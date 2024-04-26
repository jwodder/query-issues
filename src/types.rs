use serde::{Deserialize, Serialize};

pub(crate) type JsonMap = serde_json::Map<String, serde_json::Value>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct Id(String);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct Cursor(String);

impl From<Cursor> for serde_json::Value {
    fn from(value: Cursor) -> serde_json::Value {
        value.0.into()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub(crate) struct IdPair<T> {
    id: Id,
    #[serde(flatten)]
    inner: T,
}

impl<T> From<IdPair<T>> for (Id, T) {
    fn from(value: IdPair<T>) -> (Id, T) {
        (value.id, value.inner)
    }
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
pub(crate) struct RepoDetails {
    pub(crate) owner: String,
    pub(crate) name: String,
}
