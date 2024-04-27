mod pquery;
mod types;
pub use crate::pquery::PaginatedQuery;
pub use crate::types::*;
use anyhow::Context;
use indenter::indented;
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::Entry, HashMap};
use std::fmt::Write;
use ureq::{Agent, AgentBuilder};

static GRAPHQL_API_URL: &str = "https://api.github.com/graphql";

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
        Q: PaginatedQuery,
        I: IntoIterator<Item = (K, Q)>,
    {
        let mut query_queue = queries.into_iter();
        let mut next_alias_index = 0;
        let mut active = HashMap::new();
        let mut results = Vec::new();
        loop {
            let open_slots = BATCH_SIZE.saturating_sub(active.len());
            for _ in 0..open_slots {
                let Some((key, mut quy)) = query_queue.next() else {
                    break;
                };
                let alias = format!("q{next_alias_index}");
                quy.set_alias(alias.clone());
                next_alias_index += 1;
                active.insert(alias, ActiveQuery::new(key, quy));
            }
            if active.is_empty() {
                break;
            }

            let mut variables = JsonMap::new();
            let mut qstr = String::from("query(");
            let mut first = true;
            for quy in active.values() {
                for (name, Variable { gql_type, value }) in quy.query.variables() {
                    if !std::mem::take(&mut first) {
                        write!(&mut qstr, ", ")?;
                    }
                    write!(&mut qstr, "${name}: {gql_type}")?;
                    variables.insert(name, value);
                }
            }
            writeln!(&mut qstr, ") {{")?;
            for quy in active.values() {
                quy.query
                    .write_graphql(indented(&mut qstr).with_str("    "))?;
            }
            writeln!(&mut qstr, "}}")?;

            let data = self.query(qstr, variables)?;
            for (alias, value) in data {
                let Entry::Occupied(mut aqo) = active.entry(alias) else {
                    // TODO: Warn or error
                    continue;
                };
                if !aqo.get_mut().process_response(value)? {
                    results.push(PaginationResults::from(aqo.remove()));
                }
            }
        }
        Ok(results)
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
struct ActiveQuery<K, Q: PaginatedQuery> {
    key: K,
    query: Q,
    items: Vec<Q::Item>,
}

impl<K, Q: PaginatedQuery> ActiveQuery<K, Q> {
    fn new(key: K, query: Q) -> Self {
        ActiveQuery {
            key,
            query,
            items: Vec::new(),
        }
    }

    // Returns `true` if there are more pages left
    fn process_response(&mut self, value: serde_json::Value) -> Result<bool, serde_json::Error> {
        let page = self.query.parse_response(value)?;
        self.items.extend(page.items);
        if page.end_cursor.is_some() {
            // endCursor is null when the page has no items, which happens when
            // the current cursor is already at the end, so don't update the
            // cursor to null.
            self.query.set_cursor(page.end_cursor);
        }
        Ok(page.has_next_page)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationResults<K, T> {
    pub key: K,
    pub items: Vec<T>,
    pub end_cursor: Option<Cursor>,
}

impl<K, Q: PaginatedQuery> From<ActiveQuery<K, Q>> for PaginationResults<K, Q::Item> {
    fn from(value: ActiveQuery<K, Q>) -> Self {
        PaginationResults {
            key: value.key,
            items: value.items,
            end_cursor: value.query.get_cursor(),
        }
    }
}
