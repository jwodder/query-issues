use gqlient::{Cursor, Id, Page, Paginator, QueryField, Singleton, Variable};
use indoc::indoc;
use std::fmt::{self, Write};
use std::num::NonZeroUsize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetLabels {
    issue_id: Id,
    cursor: Option<Cursor>,
    label_page_size: NonZeroUsize,
}

impl GetLabels {
    pub(crate) fn new(
        issue_id: Id,
        cursor: Option<Cursor>,
        label_page_size: NonZeroUsize,
    ) -> GetLabels {
        GetLabels {
            issue_id,
            cursor,
            label_page_size,
        }
    }
}

impl Paginator for GetLabels {
    type Item = String;
    type Selection = GetLabelsQuery;

    fn for_cursor(&self, cursor: Option<&Cursor>) -> GetLabelsQuery {
        GetLabelsQuery::new(
            self.issue_id.clone(),
            match cursor {
                Some(c) => Some(c.clone()),
                None => self.cursor.clone(),
            },
            self.label_page_size,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetLabelsQuery {
    issue_id: Id,
    cursor: Option<Cursor>,
    label_page_size: NonZeroUsize,
    prefix: Option<String>,
}

impl GetLabelsQuery {
    fn new(issue_id: Id, cursor: Option<Cursor>, label_page_size: NonZeroUsize) -> GetLabelsQuery {
        GetLabelsQuery {
            issue_id,
            cursor,
            label_page_size,
            prefix: None,
        }
    }

    fn issue_id_varname(&self) -> String {
        match self.prefix {
            Some(ref prefix) => format!("{prefix}_issue_id"),
            None => String::from("issue_id"),
        }
    }

    fn cursor_varname(&self) -> String {
        match self.prefix {
            Some(ref prefix) => format!("{prefix}_cursor"),
            None => String::from("cursor"),
        }
    }
}

impl QueryField for GetLabelsQuery {
    type Output = Page<String>;

    fn with_variable_prefix(mut self, prefix: String) -> Self {
        let new_prefix = match self.prefix {
            Some(p0) => format!("{prefix}_{p0}"),
            None => prefix,
        };
        self.prefix = Some(new_prefix);
        self
    }

    fn write_field<W: Write>(&self, mut s: W) -> fmt::Result {
        write!(
            s,
            indoc! {"
            node(id: ${issue_id_varname}) {{
                ... on Issue {{
                    labels(
                        first: {label_page_size},
                        after: ${cursor_varname},
                    ) {{
                        nodes {{
                            name
                        }}
                        pageInfo {{
                            endCursor
                            hasNextPage
                        }}
                    }}
                }}
            }}
        "},
            issue_id_varname = self.issue_id_varname(),
            cursor_varname = self.cursor_varname(),
            label_page_size = self.label_page_size,
        )
    }

    fn variables(&self) -> [(String, Variable); 2] {
        [
            (
                self.issue_id_varname(),
                Variable {
                    gql_type: String::from("ID!"),
                    value: self.issue_id.clone().into(),
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

    fn parse_response(&self, value: serde_json::Value) -> Result<Self::Output, serde_json::Error> {
        let page = serde_json::from_value::<Singleton<Page<Singleton<String>>>>(value)?.0;
        Ok(Page {
            items: page.items.into_iter().map(|lb| lb.0).collect(),
            end_cursor: page.end_cursor,
            has_next_page: page.has_next_page,
        })
    }
}
