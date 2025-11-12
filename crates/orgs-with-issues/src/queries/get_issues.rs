use crate::types::{IssueWithLabels, RepoWithIssues};
use gqlient::{Cursor, Id, Page, Paginator, QueryField, Variable};
use indoc::indoc;
use std::fmt::{self, Write};
use std::num::NonZeroUsize;

/// A [`Paginator`] for retrieving open issues from a given GitHub repository
/// as pages of [`IssueWithLabels`] values
///
/// For each issue, only the first page of labels is retrieved; any additional
/// labels must be queried via `GetLabels`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetIssues {
    /// The GraphQL node ID of the repository for which to retrieve open issues
    repo_id: Id,

    /// The pagination cursor after which to retrieve issues
    cursor: Option<Cursor>,

    /// How many issues to request per page
    page_size: NonZeroUsize,

    /// How many issue labels to request per page
    label_page_size: NonZeroUsize,
}

impl GetIssues {
    pub(crate) fn new(
        repo_id: Id,
        cursor: Option<Cursor>,
        page_size: NonZeroUsize,
        label_page_size: NonZeroUsize,
    ) -> GetIssues {
        GetIssues {
            repo_id,
            cursor,
            page_size,
            label_page_size,
        }
    }
}

impl Paginator for GetIssues {
    type Item = IssueWithLabels;
    type Selection = GetIssuesQuery;

    fn for_cursor(&self, cursor: Option<&Cursor>) -> GetIssuesQuery {
        GetIssuesQuery::new(
            self.repo_id.clone(),
            match cursor {
                Some(c) => Some(c.clone()),
                None => self.cursor.clone(),
            },
            self.page_size,
            self.label_page_size,
        )
    }
}

/// A [`QueryField`] for retrieving a page of open issues (as
/// [`IssueWithLabels`] values) from a given GitHub repository starting at a
/// given cursor
///
/// For each issue, only the first page of labels is retrieved; any additional
/// labels must be queried via `GetLabels`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetIssuesQuery {
    /// The GraphQL node ID of the repository for which to retrieve open issues
    repo_id: Id,

    /// The pagination cursor after which to retrieve issues
    cursor: Option<Cursor>,

    /// How many issues to request per page
    page_size: NonZeroUsize,

    /// How many issue labels to request per page
    label_page_size: NonZeroUsize,

    /// The prefix to prepend to the variable names, if any
    prefix: Option<String>,
}

impl GetIssuesQuery {
    fn new(
        repo_id: Id,
        cursor: Option<Cursor>,
        page_size: NonZeroUsize,
        label_page_size: NonZeroUsize,
    ) -> GetIssuesQuery {
        GetIssuesQuery {
            repo_id,
            cursor,
            page_size,
            label_page_size,
            prefix: None,
        }
    }

    /// Returns the name of the GraphQL variable used to refer to the
    /// repository ID, including any added prefixes
    fn repo_id_varname(&self) -> String {
        match self.prefix {
            Some(ref prefix) => format!("{prefix}_repo_id"),
            None => String::from("repo_id"),
        }
    }

    /// Returns the name of the GraphQL variable used to refer to the issue
    /// cursor, including any added prefixes
    fn cursor_varname(&self) -> String {
        match self.prefix {
            Some(ref prefix) => format!("{prefix}_cursor"),
            None => String::from("cursor"),
        }
    }
}

impl QueryField for GetIssuesQuery {
    type Output = Page<IssueWithLabels>;

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
            node(id: ${repo_id_varname}) {{
                ... on Repository {{
                    id
                    nameWithOwner
                    issues(
                        first: {page_size},
                        after: ${cursor_varname},
                        orderBy: {{field: CREATED_AT, direction: ASC}},
                        states: [OPEN],
                    ) {{
                        nodes {{
                            id
                            number
                            title
                            url
                            createdAt
                            updatedAt
                            labels (first: {label_page_size}) {{
                                nodes {{
                                    name
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
            }}
        "},
            repo_id_varname = self.repo_id_varname(),
            cursor_varname = self.cursor_varname(),
            page_size = self.page_size,
            label_page_size = self.label_page_size,
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

    fn parse_response(&self, value: serde_json::Value) -> Result<Self::Output, serde_json::Error> {
        let raw = serde_json::from_value::<RepoWithIssues>(value)?;
        Ok(Page {
            items: raw.issues,
            end_cursor: raw.issue_cursor,
            has_next_page: raw.has_more_issues,
        })
    }
}
