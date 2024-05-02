mod queries;
mod types;
pub use crate::queries::{Paginator, Query};
pub use crate::types::*;
use anyhow::Context;
use indenter::indented;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::collections::{hash_map::Entry, HashMap};
use std::fmt::Write;
use ureq::{Agent, AgentBuilder};

static GRAPHQL_API_URL: &str = "https://api.github.com/graphql";
static RATE_LIMIT_URL: &str = "https://api.github.com/rate_limit";

const BATCH_SIZE: usize = 50;

#[derive(Clone, Debug)]
pub struct Client {
    inner: Agent,
}

impl Client {
    pub fn new(token: &str) -> Client {
        let auth = format!("Bearer {token}");
        let inner = AgentBuilder::new()
            .https_only(true)
            .middleware(move |req: ureq::Request, next: ureq::MiddlewareNext<'_>| {
                next.handle(
                    req.set("Authorization", &auth)
                        .set("X-Github-Next-Global-ID", "1"),
                )
            })
            .build();
        Client { inner }
    }

    pub fn get_rate_limit(&self) -> anyhow::Result<RateLimit> {
        self.inner
            .get(RATE_LIMIT_URL)
            .call()
            .context("failed to perform rate limit request")?
            .into_json::<RateLimitResponse>()
            .context("failed to deserialize rate limit response")
            .map(|r| r.resources.graphql)
    }

    pub fn query(&self, query: String, variables: JsonMap) -> anyhow::Result<JsonMap> {
        let r = self
            .inner
            .post(GRAPHQL_API_URL)
            .send_json(Payload { query, variables })
            .context("failed to perform GraphQL request")?
            .into_json::<Response>()
            .context("failed to deserialize GraphQL response")?;
        if !r.errors.is_empty() {
            let mut msg = String::from("Query errored:\n");
            let mut first = true;
            for e in r.errors {
                if !std::mem::take(&mut first) {
                    writeln!(&mut msg, "---")?;
                }
                if let Some(t) = e.err_type {
                    writeln!(&mut msg, "    Type: {t}")?;
                }
                writeln!(&mut msg, "    Message: {}", e.message)?;
                if let Some(p) = e.path {
                    writeln!(&mut msg, "    Path: {p:?}")?;
                }
            }
            Err(anyhow::Error::msg(msg))
        } else {
            Ok(r.data)
        }
    }

    pub fn batch_paginate<K, Q, I>(
        &self,
        queries: I,
    ) -> anyhow::Result<Vec<PaginationResults<K, Q::Item>>>
    where
        Q: Paginator,
        I: IntoIterator<Item = (K, Q)>,
    {
        let mut in_progress = queries
            .into_iter()
            .map(|(key, paginator)| PaginationState::new(key, paginator))
            .collect::<VecDeque<_>>();
        let mut results = Vec::new();
        while !in_progress.is_empty() {
            let mut active = HashMap::new();
            let mut variables = JsonMap::new();
            let mut varstr = String::new();
            let mut qstr = String::new();
            let mut qwrite = indented(&mut qstr).with_str("    ");
            for (i, state) in in_progress
                .drain(0..(in_progress.len().min(BATCH_SIZE)))
                .enumerate()
            {
                let alias = format!("q{i}");
                let query = state
                    .paginator
                    .for_cursor(state.cursor.as_ref())
                    .with_variable_prefix(alias.clone());
                for (name, Variable { gql_type, value }) in query.variables() {
                    if i > 0 {
                        write!(&mut varstr, ", ")?;
                    }
                    write!(&mut varstr, "${name}: {gql_type}")?;
                    variables.insert(name, value);
                }
                write!(&mut qwrite, "{alias}: ")?;
                query.write_graphql(&mut qwrite)?;
                active.insert(alias, ActiveQuery { state, query });
            }
            let full_query = format!("query ({varstr}) {{\n{qstr}}}\n");
            let data = self.query(full_query, variables)?;
            for (alias, value) in data {
                let Entry::Occupied(aqo) = active.entry(alias) else {
                    // TODO: Warn or error
                    continue;
                };
                let state = aqo.remove().process_response(value)?;
                if state.has_next_page {
                    in_progress.push_back(state);
                } else {
                    results.push(PaginationResults::from(state));
                }
            }
        }
        Ok(results)
    }
}

// This can't be replaced with Singleton because the JSON contains more than
// one field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct RateLimitResponse {
    resources: RateLimitResources,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct RateLimitResources {
    graphql: RateLimit,
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RateLimit {
    used: u32,
    reset: u64,
}

impl RateLimit {
    // Returns `None` if a reset happened in between the fetching of the two
    // rate limit values
    pub fn used_since(self, since: RateLimit) -> Option<u32> {
        (self.reset == since.reset).then(|| self.used.saturating_sub(since.used))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct Payload {
    query: String,
    variables: JsonMap,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct Response {
    #[serde(default)]
    data: JsonMap,
    #[serde(default)]
    errors: Vec<GraphQLError>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct GraphQLError {
    #[serde(default, rename = "type")]
    err_type: Option<String>,
    message: String,
    #[serde(default)]
    path: Option<Vec<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PaginationState<K, P: Paginator> {
    key: K,
    paginator: P,
    items: Vec<P::Item>,
    cursor: Option<Cursor>,
    has_next_page: bool,
}

impl<K, P: Paginator> PaginationState<K, P> {
    fn new(key: K, paginator: P) -> Self {
        PaginationState {
            key,
            paginator,
            items: Vec::new(),
            cursor: None,
            has_next_page: true,
        }
    }
}

// Implementing these traits requires matching bounds on P::Query, which the
// derive macros don't handle, so the traits can only be implemented manually
// here — but I don't needs them and that's busywork, so…
//#[derive(Clone, Debug, Eq, PartialEq)]
struct ActiveQuery<K, P: Paginator> {
    state: PaginationState<K, P>,
    query: P::Query,
}

impl<K, P: Paginator> ActiveQuery<K, P> {
    fn process_response(
        mut self,
        value: serde_json::Value,
    ) -> Result<PaginationState<K, P>, serde_json::Error> {
        let page = self.query.parse_response(value)?;
        self.state.items.extend(page.items);
        if page.end_cursor.is_some() {
            // endCursor is null when the page has no items, which happens when
            // the current cursor is already at the end, so don't update the
            // cursor to null.
            self.state.cursor = page.end_cursor;
        }
        self.state.has_next_page = page.has_next_page;
        Ok(self.state)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationResults<K, T> {
    pub key: K,
    pub items: Vec<T>,
    pub end_cursor: Option<Cursor>,
}

impl<K, Q: Paginator> From<PaginationState<K, Q>> for PaginationResults<K, Q::Item> {
    fn from(value: PaginationState<K, Q>) -> Self {
        PaginationResults {
            key: value.key,
            items: value.items,
            end_cursor: value.cursor,
        }
    }
}
