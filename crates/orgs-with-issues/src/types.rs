use crate::queries::{GetIssues, GetLabels};
use gqlient::{Cursor, Id, Page, Singleton};
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;

/// A page of open issues for a GitHub repository
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(from = "RawRepoDetails")]
pub(crate) struct RepoWithIssues {
    /// The repository's GraphQL node ID
    pub(crate) id: Id,

    /// A page of open issues for the repository
    pub(crate) issues: Vec<IssueWithLabels>,

    /// Cursor for the start of any remaining issues for the repository
    pub(crate) issue_cursor: Option<Cursor>,

    /// Are there more pages of issues after the ones here?
    pub(crate) has_more_issues: bool,
}

impl RepoWithIssues {
    /// If this repository has more than one page of issues, return its ID and
    /// a [`GetIssues`] instance for retrieving the remaining issues
    pub(crate) fn more_issues_query(
        &self,
        page_size: NonZeroUsize,
        label_page_size: NonZeroUsize,
    ) -> Option<(Id, GetIssues)> {
        self.has_more_issues.then(|| {
            (
                self.id.clone(),
                GetIssues::new(
                    self.id.clone(),
                    self.issue_cursor.clone(),
                    page_size,
                    label_page_size,
                ),
            )
        })
    }
}

impl From<RawRepoDetails> for RepoWithIssues {
    fn from(value: RawRepoDetails) -> RepoWithIssues {
        RepoWithIssues {
            id: value.id,
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
                        labels: ri.labels.items.into_iter().map(|lb| lb.0).collect(),
                        url: ri.url,
                        created: ri.created_at,
                        updated: ri.updated_at,
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

/// Information on an open GitHub issue and pagination of its labels
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
    /// The full name of the issue's repository in "OWNER/NAME" format
    pub(crate) repo: String,

    /// The issue's number
    pub(crate) number: u64,

    /// The issue's title
    pub(crate) title: String,

    /// The names of the issue's labels
    pub(crate) labels: Vec<String>,

    /// The HTTP URL to the web view for the issue
    pub(crate) url: String,

    /// The timestamp at which the issue was created
    pub(crate) created: String,

    /// The timestamp at which the issue was last modified
    pub(crate) updated: String,
}

/// The "raw" deserialized representation of the data queried by
/// `GetOwnerRepos`
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct RawRepoDetails {
    id: Id,
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
