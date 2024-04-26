use super::PaginatedQuery;
use crate::config::PAGE_SIZE;
use crate::types::{Connection, Cursor, Ided, JsonMap, Page, RepoDetails};
use indoc::indoc;
use std::collections::HashMap;
use std::fmt::{self, Write};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetOwnerRepos {
    owner: String,
    cursor: Option<Cursor>,
    alias: Option<String>,
}

impl GetOwnerRepos {
    pub(crate) fn new(owner: String) -> GetOwnerRepos {
        GetOwnerRepos {
            owner,
            cursor: None,
            alias: None,
        }
    }

    fn owner_varname(&self) -> String {
        match self.alias {
            Some(ref alias) => format!("{alias}_owner"),
            None => String::from("owner"),
        }
    }

    fn cursor_varname(&self) -> String {
        match self.alias {
            Some(ref alias) => format!("{alias}_cursor"),
            None => String::from("cursor"),
        }
    }
}

impl PaginatedQuery for GetOwnerRepos {
    type Item = Ided<RepoDetails>;

    fn set_alias(&mut self, alias: String) {
        self.alias = Some(alias);
    }

    fn get_cursor(&self) -> Option<Cursor> {
        self.cursor.clone()
    }

    fn set_cursor(&mut self, cursor: Option<Cursor>) {
        self.cursor = cursor;
    }

    fn write_graphql<W: Write>(&self, mut s: W) -> fmt::Result {
        if let Some(ref alias) = self.alias {
            write!(s, "{alias}: ")?;
        }
        writeln!(
            s,
            indoc! {"
            repositoryOwner(login: ${owner_varname}) {{
                repositories(
                    orderBy: {{field: NAME, direction: ASC}},
                    ownerAffiliations: [OWNER],
                    isArchived: false,
                    isFork: false,
                    privacy: PUBLIC,
                    first: {page_size},
                    after: ${cursor_varname},
                ) {{
                    nodes {{
                        id
                        owner
                        name
                    }}
                    pageInfo {{
                        endCursor
                        hasNextPage
                    }}
                }}
            }}
        "},
            owner_varname = self.owner_varname(),
            cursor_varname = self.cursor_varname(),
            page_size = PAGE_SIZE,
        )
    }

    fn variables(&self) -> JsonMap {
        let mut vars = JsonMap::new();
        vars.insert(self.owner_varname(), self.owner.clone().into());
        vars.insert(self.cursor_varname(), self.cursor.clone().into());
        vars
    }

    fn variable_types(&self) -> HashMap<String, String> {
        HashMap::from([
            (self.owner_varname(), String::from("String!")),
            (self.cursor_varname(), String::from("String")),
        ])
    }

    fn parse_response(
        &self,
        value: serde_json::Value,
    ) -> Result<Page<Self::Item>, serde_json::Error> {
        let raw: Connection<Ided<RepoDetails>> = serde_json::from_value(value)?;
        Ok(Page {
            items: raw.nodes,
            end_cursor: raw.page_info.end_cursor,
            has_next_page: raw.page_info.has_next_page,
        })
    }
}
