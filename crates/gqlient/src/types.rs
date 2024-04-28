use serde::{Deserialize, Serialize};

pub type JsonMap = serde_json::Map<String, serde_json::Value>;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Id(String);

impl From<Id> for serde_json::Value {
    fn from(value: Id) -> serde_json::Value {
        value.0.into()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Cursor(String);

impl From<Cursor> for serde_json::Value {
    fn from(value: Cursor) -> serde_json::Value {
        value.0.into()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Ided<T> {
    pub id: Id,
    #[serde(flatten)]
    pub data: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub end_cursor: Option<Cursor>,
    pub has_next_page: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Connection<T> {
    pub nodes: Vec<T>,
    pub page_info: PageInfo,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub end_cursor: Option<Cursor>,
    pub has_next_page: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Variable {
    pub gql_type: String,
    pub value: serde_json::Value,
}
