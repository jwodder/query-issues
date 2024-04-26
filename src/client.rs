use crate::queries::PaginatedQuery;
use crate::types::{Cursor, JsonMap};
use serde::Deserialize;
use std::collections::HashMap;
use ureq::{Agent, AgentBuilder};

static GRAPHQL_API_URL: &str = "https://api.github.com/graphql";

#[derive(Clone, Debug)]
pub(crate) struct GitHub {
    client: Agent,
}

impl GitHub {
    pub(crate) fn new(token: &str) -> GitHub {
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

    pub(crate) fn query(&self, query: &str, variables: JsonMap) -> anyhow::Result<Response> {
        todo!()
    }

    pub(crate) fn batch_paginate<K, Q: PaginatedQuery>(
        &self,
        queries: HashMap<K, Q>,
    ) -> anyhow::Result<HashMap<K, (Vec<Q::Item>, Cursor)>> {
        todo!()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub(crate) struct Response {
    #[serde(default)]
    pub(crate) data: JsonMap,
    #[serde(default)]
    pub(crate) errors: Option<Vec<GraphQLError>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub(crate) struct GraphQLError {
    #[serde(default, rename = "type")]
    pub(crate) err_type: Option<String>,
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) path: Option<Vec<String>>,
}
