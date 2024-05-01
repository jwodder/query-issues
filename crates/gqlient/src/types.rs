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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(from = "Connection<T>")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub end_cursor: Option<Cursor>,
    pub has_next_page: bool,
}

impl<T> Page<T> {
    pub fn map_items<F, U>(self, func: F) -> Page<U>
    where
        F: FnMut(T) -> U,
    {
        Page {
            items: self.items.into_iter().map(func).collect(),
            end_cursor: self.end_cursor,
            has_next_page: self.has_next_page,
        }
    }
}

impl<T> From<Connection<T>> for Page<T> {
    fn from(value: Connection<T>) -> Page<T> {
        Page {
            items: value.nodes,
            end_cursor: value.page_info.end_cursor,
            has_next_page: value.page_info.has_next_page,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Connection<T> {
    nodes: Vec<T>,
    page_info: PageInfo,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    end_cursor: Option<Cursor>,
    has_next_page: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Variable {
    pub gql_type: String,
    pub value: serde_json::Value,
}
