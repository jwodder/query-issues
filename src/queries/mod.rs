mod get_issues;
mod get_owner_repos;
pub(crate) use self::get_issues::GetIssues;
pub(crate) use self::get_owner_repos::GetOwnerRepos;
use crate::types::{Cursor, JsonMap, Page};

pub(crate) trait PaginatedQuery {
    type Item;

    fn set_alias(&mut self, alias: String);
    fn set_cursor(&mut self, cursor: Cursor);
    fn write_graphql(&self, s: &mut String) -> std::fmt::Result;
    fn variables(&self) -> JsonMap;
    fn parse_response(
        &self,
        value: serde_json::Value,
    ) -> Result<Page<Self::Item>, serde_json::Error>;
}
