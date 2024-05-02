use crate::config::PAGE_SIZE;
use crate::types::RepoWithIssues;
use gqlient::{Cursor, Ided, Page, Paginator, Query, Singleton, Variable};
use indoc::indoc;
use std::fmt::{self, Write};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetOwnerRepos {
    owner: String,
}

impl GetOwnerRepos {
    pub(crate) fn new(owner: String) -> GetOwnerRepos {
        GetOwnerRepos { owner }
    }
}

impl Paginator for GetOwnerRepos {
    type Item = Ided<RepoWithIssues>;
    type Query = GetOwnerReposQuery;

    fn for_cursor(&self, cursor: Option<&Cursor>) -> GetOwnerReposQuery {
        GetOwnerReposQuery::new(self.owner.clone(), cursor.cloned())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetOwnerReposQuery {
    owner: String,
    cursor: Option<Cursor>,
    prefix: Option<String>,
}

impl GetOwnerReposQuery {
    fn new(owner: String, cursor: Option<Cursor>) -> GetOwnerReposQuery {
        GetOwnerReposQuery {
            owner,
            cursor,
            prefix: None,
        }
    }

    fn owner_varname(&self) -> String {
        match self.prefix {
            Some(ref prefix) => format!("{prefix}_owner"),
            None => String::from("owner"),
        }
    }

    fn cursor_varname(&self) -> String {
        match self.prefix {
            Some(ref prefix) => format!("{prefix}_cursor"),
            None => String::from("cursor"),
        }
    }
}

impl Query for GetOwnerReposQuery {
    type Item = Page<Ided<RepoWithIssues>>;

    fn with_variable_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }

    fn write_graphql<W: Write>(&self, mut s: W) -> fmt::Result {
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

    fn variables(&self) -> [(String, Variable); 2] {
        [
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
        ]
    }

    fn parse_response(&self, value: serde_json::Value) -> Result<Self::Item, serde_json::Error> {
        serde_json::from_value::<Singleton<Self::Item>>(value).map(|r| r.0)
    }
}
