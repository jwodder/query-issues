use crate::queries::{GetIssues, GetLabels};
use gqlient::{Cursor, Id, Page, Singleton};
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(from = "RawRepoDetails")]
pub(crate) struct RepoWithIssues {
    pub(crate) id: Id,
    pub(crate) issues: Vec<IssueWithLabels>,
    pub(crate) issue_cursor: Option<Cursor>,
    pub(crate) has_more_issues: bool,
}

impl RepoWithIssues {
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct Issue {
    pub(crate) repo: String,
    pub(crate) number: u64,
    pub(crate) title: String,
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
