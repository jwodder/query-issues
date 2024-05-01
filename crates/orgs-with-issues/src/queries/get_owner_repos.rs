use crate::config::PAGE_SIZE;
use crate::types::RepoWithIssues;
use gqlient::{Cursor, Ided, Page, PaginatedQuery, Variable};
use indoc::indoc;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::{self, Write};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetOwnerRepos {
    owner: String,
    cursor: Option<Cursor>,
    alias: Option<String>,
}

impl GetOwnerRepos {
    pub(crate) fn new(owner: String) -> GetOwnerRepos {
        GetOwnerRepos {
            owner,
            cursor: None,
            alias: None,
        }
    }

    fn owner_varname(&self) -> String {
        match self.alias {
            Some(ref alias) => format!("{alias}_owner"),
            None => String::from("owner"),
        }
    }

    fn cursor_varname(&self) -> String {
        match self.alias {
            Some(ref alias) => format!("{alias}_cursor"),
            None => String::from("cursor"),
        }
    }
}

impl PaginatedQuery for GetOwnerRepos {
    type Item = Ided<RepoWithIssues>;

    fn set_alias(&mut self, alias: String) {
        self.alias = Some(alias);
    }

    fn get_cursor(&self) -> Option<Cursor> {
        self.cursor.clone()
    }

    fn set_cursor(&mut self, cursor: Option<Cursor>) {
        self.cursor = cursor;
    }

    fn write_graphql<W: Write>(&self, mut s: W) -> fmt::Result {
        if let Some(ref alias) = self.alias {
            write!(s, "{alias}: ")?;
        }
        writeln!(
            s,
            indoc! {"
            repositoryOwner(login: ${owner_varname}) {{
                repositories(
                    orderBy: {{field: NAME, direction: ASC}},
                    ownerAffiliations: [OWNER],
                    isArchived: false,
                    isFork: false,
                    privacy: PUBLIC,
                    first: {page_size},
                    after: ${cursor_varname},
                ) {{
                    nodes {{
                        id
                        nameWithOwner
                        issues(
                            first: {page_size},
                            orderBy: {{field: CREATED_AT, direction: ASC}},
                            states: [OPEN],
                        ) {{
                            nodes {{
                                number
                                title
                                url
                            }}
                            pageInfo {{
                                endCursor
                                hasNextPage
                            }}
                        }}
                    }}
                    pageInfo {{
                        endCursor
                        hasNextPage
                    }}
                }}
            }}
        "},
            owner_varname = self.owner_varname(),
            cursor_varname = self.cursor_varname(),
            page_size = PAGE_SIZE,
        )
    }

    fn variables(&self) -> HashMap<String, Variable> {
        HashMap::from([
            (
                self.owner_varname(),
                Variable {
                    gql_type: String::from("String!"),
                    value: self.owner.clone().into(),
                },
            ),
            (
                self.cursor_varname(),
                Variable {
                    gql_type: String::from("String"),
                    value: self.cursor.clone().into(),
                },
            ),
        ])
    }

    fn parse_response(
        &self,
        value: serde_json::Value,
    ) -> Result<Page<Self::Item>, serde_json::Error> {
        serde_json::from_value::<Response>(value).map(|r| r.repositories)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct Response {
    repositories: Page<Ided<RepoWithIssues>>,
}
