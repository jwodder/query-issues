use crate::queries::GetLabels;
use gqlient::{Cursor, Id, Page, Singleton};
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct RepoDetails {
    #[serde(deserialize_with = "gqlient::singleton_field")]
    pub(crate) owner: String,
    pub(crate) name: String,
    #[serde(
        rename(deserialize = "issues"),
        deserialize_with = "gqlient::singleton_field"
    )]
    pub(crate) open_issues: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Issue {
    pub(crate) number: u64,
    pub(crate) title: String,
    //pub(crate) author: String,
    // Note: Reportedly, the max number of labels on an issue is 100
    pub(crate) labels: Vec<String>,
    pub(crate) state: IssueState,
    pub(crate) url: String,
    #[serde(rename = "createdAt")]
    pub(crate) created: String,
    #[serde(rename = "updatedAt")]
    pub(crate) updated: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub(crate) enum IssueState {
    Open,
    Closed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(from = "RawIssue")]
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

impl From<RawIssue> for IssueWithLabels {
    fn from(ri: RawIssue) -> IssueWithLabels {
        let Page {
            items,
            end_cursor,
            has_next_page,
        } = ri.labels;
        IssueWithLabels {
            issue_id: ri.id,
            issue: Issue {
                number: ri.number,
                title: ri.title,
                labels: items.into_iter().map(|lb| lb.0).collect(),
                state: ri.state,
                url: ri.url,
                created: ri.created,
                updated: ri.updated,
            },
            labels_cursor: end_cursor,
            has_more_labels: has_next_page,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub(crate) struct RawIssue {
    id: Id,
    number: u64,
    title: String,
    state: IssueState,
    url: String,
    #[serde(rename = "createdAt")]
    created: String,
    #[serde(rename = "updatedAt")]
    updated: String,
    labels: Page<Singleton<String>>,
}
