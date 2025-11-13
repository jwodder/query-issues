use serde::{
    Deserialize, Serialize,
    de::{self, Deserializer, IgnoredAny, MapAccess, Visitor},
};
use std::fmt;
use std::marker::PhantomData;

/// A type alias for a JSON object as a [`serde_json`] value
pub type JsonMap = serde_json::Map<String, serde_json::Value>;

/// A GraphQL ID
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Id(String);

impl From<Id> for serde_json::Value {
    fn from(value: Id) -> serde_json::Value {
        value.0.into()
    }
}

impl From<&str> for Id {
    fn from(value: &str) -> Id {
        Id(value.to_owned())
    }
}

/// A GraphQL pagination cursor
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Cursor(String);

impl From<Cursor> for serde_json::Value {
    fn from(value: Cursor) -> serde_json::Value {
        value.0.into()
    }
}

/// A helper struct for adding an `"id"` field to a struct `T` on
/// deserialization
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Ided<T> {
    pub id: Id,
    #[serde(flatten)]
    pub data: T,
}

/// Deserialized representation of a single page of paginated values
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(from = "Connection<T>")]
pub struct Page<T> {
    /// The values in the page
    pub items: Vec<T>,

    /// The cursor for the start of the next page
    pub end_cursor: Option<Cursor>,

    /// Is there another page after this?
    pub has_next_page: bool,
}

impl<T> Page<T> {
    /// Map `items` through a given function
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

/// The "raw" deserialized representation of a [`Page`], directly corresponding
/// to its representation in a GraphQL response
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

/// Information on a variable in a GraphQL query
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Variable {
    /// The name of the GraphQL type of the variable as written in a GraphQL query
    pub gql_type: String,

    /// The variable's value
    pub value: serde_json::Value,
}

/// A utility type for deserializing just `foo` from a single-field map of the
/// form `{"anything": foo}`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Singleton<T>(pub T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Singleton<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SingletonVisitor<T>(PhantomData<T>);

        impl<T> SingletonVisitor<T> {
            fn new() -> Self {
                SingletonVisitor(PhantomData)
            }
        }

        impl<'de, T: Deserialize<'de>> Visitor<'de> for SingletonVisitor<T> {
            type Value = Singleton<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a string-keyed map containing a single field")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                if let Some((_, value)) = map.next_entry::<String, T>()? {
                    if map.next_entry::<String, IgnoredAny>()?.is_some() {
                        Err(de::Error::invalid_length(
                            map.size_hint().unwrap_or(0).saturating_add(2),
                            &self,
                        ))
                    } else {
                        Ok(Singleton(value))
                    }
                } else {
                    Err(de::Error::invalid_length(0, &self))
                }
            }
        }

        deserializer.deserialize_map(SingletonVisitor::new())
    }
}

/// Utility function for use in `#[serde(deserialize_with = ...)]` for
/// deserializing a single-field map as the field's value
pub fn singleton_field<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Singleton::<T>::deserialize(deserializer).map(|r| r.0)
}
