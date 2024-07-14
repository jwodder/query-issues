use crate::queries::GetLabels;
use gqlient::{Cursor, Id, Page, Singleton};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroUsize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(from = "RawRepoDetails")]
pub(crate) struct RepoWithIssues {
    pub(crate) issues: Vec<IssueWithLabels>,
    pub(crate) issue_cursor: Option<Cursor>,
    pub(crate) has_more_issues: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct RawRepoDetails {
    name_with_owner: String,
    issues: Page<RawIssue>,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Issue<L> {
    pub(crate) repo: String,
    pub(crate) number: u64,
    pub(crate) title: String,
    pub(crate) labels: Vec<L>,
    pub(crate) url: String,
}

impl Issue<Id> {
    pub(crate) fn name_labels(self, names: &HashMap<Id, String>) -> Issue<String> {
        Issue {
            repo: self.repo,
            number: self.number,
            title: self.title,
            labels: self.labels.iter().map(|id| names[id].clone()).collect(),
            url: self.url,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct IssueWithLabels {
    pub(crate) issue_id: Id,
    pub(crate) issue: Issue<Id>,
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
struct RawIssue {
    id: Id,
    number: u64,
    title: String,
    url: String,
    labels: Page<Singleton<Id>>,
}
