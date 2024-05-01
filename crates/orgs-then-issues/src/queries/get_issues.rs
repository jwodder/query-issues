use crate::config::PAGE_SIZE;
use crate::types::{Issue, RepoWithIssues};
use gqlient::{Cursor, Id, Page, PaginatedQuery, Variable};
use indoc::indoc;
use std::collections::HashMap;
use std::fmt::{self, Write};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetIssues {
    repo_id: Id,
    cursor: Option<Cursor>,
    alias: Option<String>,
}

impl GetIssues {
    pub(crate) fn new(repo_id: Id, cursor: Option<Cursor>) -> GetIssues {
        GetIssues {
            repo_id,
            cursor,
            alias: None,
        }
    }

    fn repo_id_varname(&self) -> String {
        match self.alias {
            Some(ref alias) => format!("{alias}_repo_id"),
            None => String::from("repo_id"),
        }
    }

    fn cursor_varname(&self) -> String {
        match self.alias {
            Some(ref alias) => format!("{alias}_cursor"),
            None => String::from("cursor"),
        }
    }
}

impl PaginatedQuery for GetIssues {
    type Item = Issue;

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
            node(id: ${repo_id_varname}) {{
                ... on Repository {{
                    nameWithOwner
                    issues(
                        first: {page_size},
                        after: ${cursor_varname},
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
            }}
        "},
            repo_id_varname = self.repo_id_varname(),
            cursor_varname = self.cursor_varname(),
            page_size = PAGE_SIZE,
        )
    }

    fn variables(&self) -> HashMap<String, Variable> {
        HashMap::from([
            (
                self.repo_id_varname(),
                Variable {
                    gql_type: String::from("ID!"),
                    value: self.repo_id.clone().into(),
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
        let raw = serde_json::from_value::<RepoWithIssues>(value)?;
        Ok(Page {
            items: raw.issues,
            end_cursor: raw.issue_cursor,
            has_next_page: raw.has_more_issues,
        })
    }
}
