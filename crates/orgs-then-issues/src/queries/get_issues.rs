use super::GetLabels;
use gqlient::{Cursor, Id, Page, Paginator, QueryField, Singleton, Variable};
use indoc::indoc;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Write};
use std::num::NonZeroUsize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetIssues {
    repo_id: Id,
    page_size: NonZeroUsize,
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
    type Selection = GetIssuesQuery;

    fn for_cursor(&self, cursor: Option<&Cursor>) -> GetIssuesQuery {
        GetIssuesQuery::new(
            self.repo_id.clone(),
            cursor.cloned(),
            self.page_size,
            self.label_page_size,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetIssuesQuery {
    repo_id: Id,
    cursor: Option<Cursor>,
    page_size: NonZeroUsize,
    label_page_size: NonZeroUsize,
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

impl QueryField for GetIssuesQuery {
    type Output = Page<IssueWithLabels>;

    fn with_variable_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }

    fn write_field<W: Write>(&self, mut s: W) -> fmt::Result {
        write!(
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

    fn parse_response(
        &self,
        value: serde_json::Value,
    ) -> Result<Page<IssueWithLabels>, serde_json::Error> {
        let raw = serde_json::from_value::<RepoWithIssues>(value)?;
        Ok(Page {
            items: raw.issues,
            end_cursor: raw.issue_cursor,
            has_next_page: raw.has_more_issues,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(from = "RawRepoDetails")]
pub(crate) struct RepoWithIssues {
    pub(crate) issues: Vec<IssueWithLabels>,
    pub(crate) issue_cursor: Option<Cursor>,
    pub(crate) has_more_issues: bool,
}

impl From<RawRepoDetails> for RepoWithIssues {
    fn from(value: RawRepoDetails) -> RepoWithIssues {
        RepoWithIssues {
            issues: value
                .issues
                .items
                .into_iter()
                .map(|ri| IssueWithLabels {
                    issue_id: ri.id,
                    issue: Issue {
                        repo: value.name_with_owner.clone(),
                        number: ri.number,
                        title: ri.title,
                        url: ri.url,
                        created: ri.created_at,
                        updated: ri.updated_at,
                        labels: ri.labels.items.into_iter().map(|lb| lb.0).collect(),
                    },
                    labels_cursor: ri.labels.end_cursor,
                    has_more_labels: ri.labels.has_next_page,
                })
                .collect(),
            issue_cursor: value.issues.end_cursor,
            has_more_issues: value.issues.has_next_page,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct Issue {
    pub(crate) repo: String,
    pub(crate) number: u64,
    pub(crate) title: String,
    //pub(crate) author: String,
    // Note: Reportedly, the max number of labels on an issue is 100
    pub(crate) labels: Vec<String>,
    pub(crate) url: String,
    pub(crate) created: String,
    pub(crate) updated: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IssueWithLabels {
    pub(crate) issue_id: Id,
    pub(crate) issue: Issue,
    pub(crate) labels_cursor: Option<Cursor>,
    pub(crate) has_more_labels: bool,
}

impl IssueWithLabels {
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct RawRepoDetails {
    name_with_owner: String,
    issues: Page<RawIssue>,
}

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
}
