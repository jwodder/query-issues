use serde::{Deserialize, Serialize};

pub(crate) type JsonMap = serde_json::Map<String, serde_json::Value>;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct Id(String);

impl From<Id> for serde_json::Value {
    fn from(value: Id) -> serde_json::Value {
        value.0.into()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct Cursor(String);

impl From<Cursor> for serde_json::Value {
    fn from(value: Cursor) -> serde_json::Value {
        value.0.into()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub(crate) struct Ided<T> {
    pub(crate) id: Id,
    #[serde(flatten)]
    pub(crate) data: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Page<T> {
    pub(crate) items: Vec<T>,
    pub(crate) end_cursor: Option<Cursor>,
    pub(crate) has_next_page: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Connection<T> {
    pub(crate) nodes: Vec<T>,
    pub(crate) page_info: PageInfo,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageInfo {
    pub(crate) end_cursor: Option<Cursor>,
    pub(crate) has_next_page: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct RepoWithIssues {
    pub(crate) issues: Vec<Issue>,
    pub(crate) issue_cursor: Option<Cursor>,
    pub(crate) has_more_issues: bool,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Variable {
    pub(crate) gql_type: String,
    pub(crate) value: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub(crate) struct RawIssue {
    pub(crate) number: u64,
    pub(crate) title: String,
    pub(crate) url: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawRepoDetails {
    pub(crate) name_with_owner: String,
    pub(crate) issues: Connection<RawIssue>,
}

impl From<RawRepoDetails> for RepoWithIssues {
    fn from(value: RawRepoDetails) -> RepoWithIssues {
        RepoWithIssues {
            issues: value
                .issues
                .nodes
                .into_iter()
                .map(|ri| Issue {
                    repo: value.name_with_owner.clone(),
                    number: ri.number,
                    title: ri.title,
                    url: ri.url,
                })
                .collect(),
            issue_cursor: value.issues.page_info.end_cursor,
            has_more_issues: value.issues.page_info.has_next_page,
        }
    }
}
