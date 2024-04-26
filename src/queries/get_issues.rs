use super::PaginatedQuery;
use crate::config::PAGE_SIZE;
use crate::types::{Connection, Cursor, Id, Ided, Issue, JsonMap, Page};
use indoc::indoc;
use std::fmt::{self, Write};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetIssues {
    repo_id: Id,
    cursor: Option<Cursor>,
    include_closed: bool,
    alias: Option<String>,
}

impl GetIssues {
    pub(crate) fn new(repo_id: Id, cursor: Option<Cursor>) -> GetIssues {
        let include_closed = cursor.is_some();
        GetIssues {
            repo_id,
            cursor,
            include_closed,
            alias: None,
        }
    }

    fn repo_id_varname(&self) -> String {
        match self.alias {
            Some(ref alias) => format!("{alias}_repo_id"),
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

impl PaginatedQuery for GetIssues {
    type Item = Ided<Issue>;

    fn set_alias(&mut self, alias: String) {
        self.alias = Some(alias);
    }

    fn set_cursor(&mut self, cursor: Cursor) {
        self.cursor = Some(cursor);
    }

    fn write_graphql(&self, s: &mut String) -> fmt::Result {
        if let Some(ref alias) = self.alias {
            write!(s, "{alias}: ")?;
        }
        writeln!(
            s,
            indoc! {"
            node(id: ${repo_id_varname}) {{
                ... on Repository {{
                    issues(
                        first: ${page_size},
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

    fn variables(&self) -> JsonMap {
        let mut vars = JsonMap::new();
        vars.insert(self.repo_id_varname(), self.repo_id.clone().into());
        vars.insert(self.cursor_varname(), self.cursor.clone().into());
        vars
    }

    fn parse_response(
        &self,
        value: serde_json::Value,
    ) -> Result<Page<Self::Item>, serde_json::Error> {
        let raw: Connection<Ided<Issue>> = serde_json::from_value(value)?;
        Ok(Page {
            items: raw.nodes,
            end_cursor: raw.page_info.end_cursor,
            has_next_page: raw.page_info.has_next_page,
        })
    }
}
