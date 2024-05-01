use crate::types::{Cursor, Page, Variable};

pub trait Query: Sized {
    type Item;

    fn with_alias(self, alias: String) -> Self;
    fn write_graphql<W: std::fmt::Write>(&self, s: W) -> std::fmt::Result;
    fn variables(&self) -> impl IntoIterator<Item = (String, Variable)>;
    fn parse_response(&self, value: serde_json::Value) -> Result<Self::Item, serde_json::Error>;
}

pub trait Paginator {
    type Query: Query<Item = Page<Self::Item>>;
    type Item;

    fn for_cursor(&self, cursor: Option<&Cursor>) -> Self::Query;
}
