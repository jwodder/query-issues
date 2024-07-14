use crate::types::{Cursor, Page, Variable};

pub trait Query: Sized {
    type Output;

    fn with_variable_prefix(self, prefix: String) -> Self;
    fn write_graphql<W: std::fmt::Write>(&self, s: W) -> std::fmt::Result;
    fn variables(&self) -> impl IntoIterator<Item = (String, Variable)>;
    fn parse_response(&self, value: serde_json::Value) -> Result<Self::Output, serde_json::Error>;
}

pub trait Paginator {
    type Query: Query<Output = Page<Self::Item>>;
    type Item;

    fn for_cursor(&self, cursor: Option<&Cursor>) -> Self::Query;
}
