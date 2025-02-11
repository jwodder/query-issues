mod queries;
mod types;
pub use crate::queries::*;
pub use crate::types::*;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::num::NonZeroUsize;
use thiserror::Error;
use ureq::{Agent, AgentBuilder};

static GRAPHQL_API_URL: &str = "https://api.github.com/graphql";
static RATE_LIMIT_URL: &str = "https://api.github.com/rate_limit";

#[allow(unsafe_code)]
// SAFETY: 50 != 0
pub const DEFAULT_BATCH_SIZE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(50) };

#[derive(Clone, Debug)]
pub struct Client {
    inner: Agent,
}

impl Client {
    #[allow(clippy::missing_panics_doc)]
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

    pub fn new_with_local_token() -> anyhow::Result<Client> {
        let token = gh_token::get().context("unable to fetch GitHub access token")?;
        Ok(Client::new(&token))
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

    pub fn query(&self, payload: QueryPayload) -> Result<JsonMap, Error> {
        let mut r = self
            .inner
            .post(GRAPHQL_API_URL)
            .send_json(payload)
            .map_err(Box::new)?
            .into_reader();
        let mut bytes = Vec::new();
        r.read_to_end(&mut bytes)?;
        serde_json::from_slice::<Response>(&bytes)?
            .into_data()
            .map_err(Into::into)
    }

    pub fn run<Q: QueryMachine>(&self, query: Q) -> QueryResults<'_, Q> {
        QueryResults::new(self, query)
    }
}

#[derive(Debug)]
pub struct QueryResults<'a, Q: QueryMachine> {
    client: &'a Client,
    query: Q,
    query_done: bool,
    yielding: std::vec::IntoIter<Q::Output>,
}

impl<'a, Q: QueryMachine> QueryResults<'a, Q> {
    fn new(client: &'a Client, query: Q) -> Self {
        QueryResults {
            client,
            query,
            query_done: false,
            yielding: Vec::new().into_iter(),
        }
    }
}

impl<Q: QueryMachine> Iterator for QueryResults<'_, Q> {
    type Item = Result<Q::Output, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(value) = self.yielding.next() {
                return Some(Ok(value));
            } else if self.query_done {
                return None;
            } else {
                if let Some(payload) = self.query.get_next_query() {
                    match self.client.query(payload) {
                        Ok(data) => {
                            if let Err(e) = self.query.handle_response(data) {
                                return Some(Err(e.into()));
                            }
                        }
                        Err(e) => return Some(Err(e)),
                    }
                } else {
                    self.query_done = true;
                }
                self.yielding = self.query.get_output().into_iter();
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to perform GraphQL request")]
    Http(#[from] Box<ureq::Error>),
    #[error("failed to read GraphQL response")]
    Read(#[from] std::io::Error),
    #[error("failed to deserialize GraphQL response")]
    Json(#[from] serde_json::Error),
    #[error("GraphQL server returned error response")]
    GraphQL(#[from] GqlError),
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
pub struct QueryPayload {
    query: String,
    variables: JsonMap,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct Response {
    #[serde(default)]
    data: JsonMap,
    #[serde(default)]
    errors: GqlError,
}

impl Response {
    fn into_data(self) -> Result<JsonMap, GqlError> {
        if self.errors.is_empty() {
            Ok(self.data)
        } else {
            Err(self.errors)
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct GqlError(Vec<GqlInnerError>);

impl GqlError {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for GqlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Query errored:")?;
        let mut first = true;
        for e in &self.0 {
            if !std::mem::take(&mut first) {
                writeln!(f, "---")?;
            }
            if let Some(ref t) = e.err_type {
                writeln!(f, "    Type: {t}")?;
            }
            writeln!(f, "    Message: {}", e.message)?;
            if let Some(ref p) = e.path {
                writeln!(f, "    Path: {p:?}")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for GqlError {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct GqlInnerError {
    #[serde(default, rename = "type")]
    err_type: Option<String>,
    message: String,
    #[serde(default)]
    path: Option<Vec<String>>,
}
