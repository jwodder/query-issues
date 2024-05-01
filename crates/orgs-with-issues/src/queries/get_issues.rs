use crate::config::PAGE_SIZE;
use crate::types::{Issue, RepoWithIssues};
use gqlient::{Cursor, Id, Page, Paginator, Query, Variable};
use indoc::indoc;
use std::fmt::{self, Write};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetIssues {
    repo_id: Id,
    cursor: Option<Cursor>,
}

impl GetIssues {
    pub(crate) fn new(repo_id: Id, cursor: Option<Cursor>) -> GetIssues {
        GetIssues { repo_id, cursor }
    }
}

impl Paginator for GetIssues {
    type Item = Issue;
    type Query = GetIssuesQuery;

    fn for_cursor(&self, cursor: Option<&Cursor>) -> GetIssuesQuery {
        GetIssuesQuery::new(
            self.repo_id.clone(),
            match cursor {
                Some(c) => Some(c.clone()),
                None => self.cursor.clone(),
            },
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetIssuesQuery {
    repo_id: Id,
    cursor: Option<Cursor>,
    prefix: Option<String>,
}

impl GetIssuesQuery {
    fn new(repo_id: Id, cursor: Option<Cursor>) -> GetIssuesQuery {
        GetIssuesQuery {
            repo_id,
            cursor,
            prefix: None,
        }
    }

    fn repo_id_varname(&self) -> String {
        match self.prefix {
            Some(ref prefix) => format!("{prefix}_repo_id"),
            None => String::from("repo_id"),
        }
    }

    fn cursor_varname(&self) -> String {
        match self.prefix {
            Some(ref prefix) => format!("{prefix}_cursor"),
            None => String::from("cursor"),
        }
    }
}

impl Query for GetIssuesQuery {
    type Item = Page<Issue>;

    fn with_variable_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }

    fn write_graphql<W: Write>(&self, mut s: W) -> fmt::Result {
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

    fn variables(&self) -> [(String, Variable); 2] {
        [
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
        ]
    }

    fn parse_response(&self, value: serde_json::Value) -> Result<Self::Item, serde_json::Error> {
        let raw = serde_json::from_value::<RepoWithIssues>(value)?;
        Ok(Page {
            items: raw.issues,
            end_cursor: raw.issue_cursor,
            has_next_page: raw.has_more_issues,
        })
    }
}
