use crate::queries::PaginatedQuery;
use crate::types::{Cursor, JsonMap};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    pub(crate) fn batch_paginate<K, Q: PaginatedQuery>(
        &self,
        queries: HashMap<K, Q>,
    ) -> anyhow::Result<HashMap<K, (Vec<Q::Item>, Option<Cursor>)>> {
        todo!()
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
