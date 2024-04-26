use crate::config::BATCH_SIZE;
use crate::queries::PaginatedQuery;
use crate::types::{Cursor, JsonMap};
use anyhow::Context;
use indenter::indented;
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::Entry, HashMap};
use std::fmt::Write;
use ureq::{Agent, AgentBuilder};

static GRAPHQL_API_URL: &str = "https://api.github.com/graphql";

#[derive(Clone, Debug)]
pub(crate) struct Client {
    inner: Agent,
}

impl Client {
    pub(crate) fn new(token: &str) -> Client {
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

    pub(crate) fn query(&self, query: String, variables: JsonMap) -> anyhow::Result<JsonMap> {
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

    pub(crate) fn batch_paginate<K, Q, I>(
        &self,
        queries: I,
    ) -> anyhow::Result<HashMap<K, (Vec<Q::Item>, Option<Cursor>)>>
    where
        K: Eq + std::hash::Hash,
        Q: PaginatedQuery,
        I: IntoIterator<Item = (K, Q)>,
    {
        let mut query_queue = queries.into_iter();
        let mut next_alias_index = 0;
        let mut active = HashMap::new();
        let mut results = HashMap::new();
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
            let mut var_types = HashMap::new();
            for quy in active.values() {
                variables.extend(quy.query.variables());
                var_types.extend(quy.query.variable_types());
            }

            let mut qstr = String::from("query(");
            let mut first = true;
            for (name, ty) in var_types {
                if !std::mem::take(&mut first) {
                    write!(&mut qstr, ", ")?;
                }
                write!(&mut qstr, "${name}: {ty}")?;
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
                    let q = aqo.remove();
                    results.insert(q.key, (q.items, q.query.get_cursor()));
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
        self.query.set_cursor(page.end_cursor);
        Ok(page.has_next_page)
    }
}
