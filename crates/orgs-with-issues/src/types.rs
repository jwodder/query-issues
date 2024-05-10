use gqlient::{Cursor, Page};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(from = "RawRepoDetails")]
pub(crate) struct RepoWithIssues {
    pub(crate) issues: Vec<Issue>,
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
                .map(|ri| Issue {
                    repo: value.name_with_owner.clone(),
                    number: ri.number,
                    title: ri.title,
                    url: ri.url,
                })
                .collect(),
            issue_cursor: value.issues.end_cursor,
            has_more_issues: value.issues.has_next_page,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Issue {
    pub(crate) repo: String,
    pub(crate) number: u64,
    pub(crate) title: String,
    //pub(crate) author: String,
    // Note: Reportedly, the max number of labels on an issue is 100
    //pub(crate) labels: Vec<String>,
    pub(crate) url: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct RawIssue {
    number: u64,
    title: String,
    url: String,
}
