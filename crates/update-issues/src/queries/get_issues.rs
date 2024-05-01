use crate::config::PAGE_SIZE;
use crate::types::Issue;
use gqlient::{Cursor, Id, Ided, Page, Paginator, Query, Variable};
use indoc::indoc;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::{self, Write};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetIssues {
    repo_id: Id,
    cursor: Option<Cursor>,
    include_closed: bool,
}

impl GetIssues {
    pub(crate) fn new(repo_id: Id, cursor: Option<Cursor>) -> GetIssues {
        let include_closed = cursor.is_some();
        GetIssues {
            repo_id,
            cursor,
            include_closed,
        }
    }
}

impl Paginator for GetIssues {
    type Item = Ided<Issue>;
    type Query = GetIssuesQuery;

    fn for_cursor(&self, cursor: Option<&Cursor>) -> GetIssuesQuery {
        let cursor = match cursor {
            Some(c) => Some(c.clone()),
            None => self.cursor.clone(),
        };
        GetIssuesQuery {
            repo_id: self.repo_id.clone(),
            cursor,
            include_closed: self.include_closed,
            alias: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetIssuesQuery {
    repo_id: Id,
    cursor: Option<Cursor>,
    include_closed: bool,
    alias: Option<String>,
}

impl GetIssuesQuery {
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

impl Query for GetIssuesQuery {
    type Item = Page<Ided<Issue>>;

    fn with_alias(mut self, alias: String) -> Self {
        self.alias = Some(alias);
        self
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
                    issues(
                        first: {page_size},
                        after: ${cursor_varname},
                        orderBy: {{field: UPDATED_AT, direction: ASC}},
                        states: [{states}],
                    ) {{
                        nodes {{
                            id
                            number
                            title
                            state
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
            states = if self.include_closed {
                "OPEN, CLOSED"
            } else {
                "OPEN"
            },
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

    fn parse_response(&self, value: serde_json::Value) -> Result<Self::Item, serde_json::Error> {
        serde_json::from_value::<Response>(value).map(|r| r.issues)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct Response {
    issues: Page<Ided<Issue>>,
}
