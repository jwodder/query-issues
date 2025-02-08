use serde::{Deserialize, Serialize};

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
    //pub(crate) labels: Vec<String>,
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
