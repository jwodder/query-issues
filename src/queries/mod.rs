mod get_issues;
mod get_owner_repos;
pub(crate) use self::get_issues::GetIssues;
pub(crate) use self::get_owner_repos::GetOwnerRepos;
use crate::types::{Cursor, JsonMap, Page};
use std::collections::HashMap;

pub(crate) trait PaginatedQuery {
    type Item;

    fn set_alias(&mut self, alias: String);
    fn get_cursor(&self) -> Option<Cursor>;
    fn set_cursor(&mut self, cursor: Option<Cursor>);
    fn write_graphql<W: std::fmt::Write>(&self, s: W) -> std::fmt::Result;
    fn variables(&self) -> JsonMap;
    fn variable_types(&self) -> HashMap<String, String>;
    fn parse_response(
        &self,
        value: serde_json::Value,
    ) -> Result<Page<Self::Item>, serde_json::Error>;
}
