use super::GetLabels;
use gqlient::{Cursor, Id, Page, Paginator, QueryField, Singleton, Variable};
use indoc::indoc;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Write};
use std::num::NonZeroUsize;

/// A [`Paginator`] for retrieving open issues from a given GitHub repository
/// as pages of [`IssueWithLabels`] values
///
/// For each issue, only the first page of labels is retrieved; any additional
/// labels must be queried via [`GetLabels`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetIssues {
    /// The GraphQL node ID of the repository for which to retrieve open issues
    repo_id: Id,

    /// How many issues to request per page
    page_size: NonZeroUsize,

    /// How many issue labels to request per page
    label_page_size: NonZeroUsize,
}

impl GetIssues {
    pub(crate) fn new(
        repo_id: Id,
        page_size: NonZeroUsize,
        label_page_size: NonZeroUsize,
    ) -> GetIssues {
        GetIssues {
            repo_id,
            page_size,
            label_page_size,
        }
    }
}

impl Paginator for GetIssues {
    type Item = IssueWithLabels;
    type Query = GetIssuesQuery;

    fn for_cursor(&self, cursor: Option<&Cursor>) -> GetIssuesQuery {
        GetIssuesQuery::new(
            self.repo_id.clone(),
            cursor.cloned(),
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
/// labels must be queried via [`GetLabels`].
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
                            milestone {{
                                title
                            }}
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

    fn parse_response(
        &self,
        value: serde_json::Value,
    ) -> Result<Page<IssueWithLabels>, serde_json::Error> {
        let value = serde_json::from_value::<Singleton<Page<RawIssue>>>(value)?.0;
        let items = value
            .items
            .into_iter()
            .map(|ri| IssueWithLabels {
                issue_id: ri.id,
                issue: Issue {
                    repo_id: self.repo_id.clone(),
                    number: ri.number,
                    title: ri.title,
                    url: ri.url,
                    created: ri.created_at,
                    updated: ri.updated_at,
                    labels: ri.labels.items.into_iter().map(|lb| lb.0).collect(),
                    milestone: ri.milestone.map(|s| s.0),
                },
                labels_cursor: ri.labels.end_cursor,
                has_more_labels: ri.labels.has_next_page,
            })
            .collect();
        let end_cursor = value.end_cursor;
        let has_next_page = value.has_next_page;
        Ok(Page {
            items,
            end_cursor,
            has_next_page,
        })
    }
}

/// Information on an open GitHub issue and pagination of its labels, retrieved
/// by a [`GetIssues`] paginator or [`GetIssuesQuery`] query field
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IssueWithLabels {
    /// The issue's GraphQL node ID
    pub(crate) issue_id: Id,

    /// Information about the issue itself
    pub(crate) issue: Issue,

    /// Cursor for the start of any remaining labels for the issue
    pub(crate) labels_cursor: Option<Cursor>,

    /// Are there more pages of labels after the ones here?
    pub(crate) has_more_labels: bool,
}

impl IssueWithLabels {
    /// If this issue has more than one page of labels, return its ID and a
    /// [`GetLabels`] instance for retrieving the remaining labels
    pub(crate) fn more_labels_query(
        &self,
        label_page_size: NonZeroUsize,
    ) -> Option<(Id, GetLabels)> {
        self.has_more_labels.then(|| {
            (
                self.issue_id.clone(),
                GetLabels::new(
                    self.issue_id.clone(),
                    self.labels_cursor.clone(),
                    label_page_size,
                ),
            )
        })
    }
}

/// GitHub issue details
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct Issue {
    /// The GraphQL node ID of the issue's repository
    pub(crate) repo_id: Id,

    /// The issue's number
    pub(crate) number: u64,

    /// The issue's title
    pub(crate) title: String,

    /// The names of the issue's labels
    // Note: Reportedly, the max number of labels on an issue is 100
    pub(crate) labels: Vec<String>,

    /// The HTTP URL to the web view for the issue
    pub(crate) url: String,

    /// The timestamp at which the issue was created
    pub(crate) created: String,

    /// The timestamp at which the issue was last modified
    pub(crate) updated: String,

    /// The name of the milestone associated with the issue, if any
    pub(crate) milestone: Option<String>,
}

/// The "raw" deserialized representation of the data queried by
/// [`GetIssuesQuery`]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct RawIssue {
    id: Id,
    number: u64,
    title: String,
    labels: Page<Singleton<String>>,
    url: String,
    created_at: String,
    updated_at: String,
    milestone: Option<Singleton<String>>,
}
