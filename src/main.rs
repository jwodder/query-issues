use indoc::indoc;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Write};
use ureq::{Agent, AgentBuilder};

static OWNERS: &[&str] = &["jwodder", "wheelodex"];

static GRAPHQL_API_URL: &str = "https://api.github.com/graphql";

const PAGE_SIZE: usize = 100;
const BATCH_SIZE: usize = 50;

type JsonMap = serde_json::Map<String, serde_json::Value>;

trait PaginatedQuery {
    type Item;

    fn set_alias(&mut self, alias: String);
    fn set_cursor(&mut self, cursor: Cursor);
    fn write_graphql(&self, s: &mut String) -> fmt::Result;
    fn variables(&self) -> JsonMap;
    fn parse_response(
        &self,
        value: serde_json::Value,
    ) -> Result<Page<Self::Item>, serde_json::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Page<T> {
    items: Vec<T>,
    end_cursor: Option<Cursor>,
    has_next_page: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct Response {
    #[serde(default)]
    data: JsonMap,
    #[serde(default)]
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct GraphQLError {
    #[serde(default, rename = "type")]
    err_type: Option<String>,
    message: String,
    #[serde(default)]
    path: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
struct Id(String);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
struct Cursor(String);

impl From<Cursor> for serde_json::Value {
    fn from(value: Cursor) -> serde_json::Value {
        value.0.into()
    }
}

#[derive(Clone, Debug)]
struct GitHub {
    client: Agent,
}

impl GitHub {
    fn new(token: &str) -> GitHub {
        let auth = format!("Bearer {token}");
        let client = AgentBuilder::new()
            .https_only(true)
            .middleware(move |req: ureq::Request, next: ureq::MiddlewareNext<'_>| {
                next.handle(
                    req.set("Authorization", &auth)
                        .set("X-Github-Next-Global-ID", "1"),
                )
            })
            .build();
        GitHub { client }
    }

    fn query(&self, query: &str, variables: JsonMap) -> anyhow::Result<Response> {
        todo!()
    }

    fn batch_paginate<K, Q: PaginatedQuery>(
        &self,
        queries: HashMap<K, Q>,
    ) -> anyhow::Result<HashMap<K, (Vec<Q::Item>, Cursor)>> {
        todo!()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct IdPair<T> {
    id: Id,
    #[serde(flatten)]
    inner: T,
}

impl<T> From<IdPair<T>> for (Id, T) {
    fn from(value: IdPair<T>) -> (Id, T) {
        (value.id, value.inner)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RepoDetails {
    owner: String,
    name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GetOwnerRepos {
    owner: String,
    cursor: Option<Cursor>,
    alias: Option<String>,
}

impl GetOwnerRepos {
    fn new(owner: String) -> GetOwnerRepos {
        GetOwnerRepos {
            owner,
            cursor: None,
            alias: None,
        }
    }

    fn owner_varname(&self) -> String {
        match self.alias {
            Some(ref alias) => format!("{alias}_owner"),
            None => self.owner.clone(),
        }
    }

    fn cursor_varname(&self) -> String {
        match self.alias.as_ref() {
            Some(alias) => format!("{alias}_cursor"),
            None => String::from("cursor"),
        }
    }
}

impl PaginatedQuery for GetOwnerRepos {
    type Item = (Id, RepoDetails);

    fn set_alias(&mut self, alias: String) {
        self.alias = Some(alias);
    }

    fn set_cursor(&mut self, cursor: Cursor) {
        self.cursor = Some(cursor);
    }

    fn write_graphql(&self, s: &mut String) -> fmt::Result {
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

    fn parse_response(
        &self,
        value: serde_json::Value,
    ) -> Result<Page<Self::Item>, serde_json::Error> {
        let raw: Connection<IdPair<RepoDetails>> = serde_json::from_value(value)?;
        Ok(Page {
            items: raw.nodes.into_iter().map(Into::into).collect(),
            end_cursor: raw.page_info.end_cursor,
            has_next_page: raw.page_info.has_next_page,
        })
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

fn main() -> anyhow::Result<()> {
    let token = gh_token::get().context("unable to fetch GitHub access token")?;
    let client = GitHub::new(&token);

    // Load database file
    // Time: Fetch all repositories for OWNERS
    // Report added/deleted/modified repos
    // Time: For each repository, fetch recently-updated issues (open & closed)
    //  - If there's no saved cursor for the repo, fetch all issues (open only)
    // Report added/deleted/closed/modified issues
    // Dump database file

    todo!();

    Ok(())
}
