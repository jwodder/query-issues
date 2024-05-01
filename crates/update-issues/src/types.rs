use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(from = "RawRepoDetails")]
pub(crate) struct RepoDetails {
    pub(crate) owner: String,
    pub(crate) name: String,
    pub(crate) open_issues: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct RawRepoDetails {
    owner: RepositoryOwner,
    name: String,
    issues: Issues,
}

impl From<RawRepoDetails> for RepoDetails {
    fn from(value: RawRepoDetails) -> RepoDetails {
        RepoDetails {
            owner: value.owner.login,
            name: value.name,
            open_issues: value.issues.total_count,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct RepositoryOwner {
    login: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Issues {
    total_count: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Issue {
    pub(crate) number: u64,
    pub(crate) title: String,
    //pub(crate) author: String,
    // Note: Reportedly, the max number of labels on an issue is 100
    //pub(crate) labels: Vec<String>,
    pub(crate) state: IssueState,
    pub(crate) url: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub(crate) enum IssueState {
    Open,
    Closed,
}
