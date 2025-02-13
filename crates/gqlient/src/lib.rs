mod queries;
mod types;
pub use crate::queries::*;
pub use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;
use std::num::NonZeroUsize;
use thiserror::Error;
use ureq::{
    http::{
        header::{HeaderValue, InvalidHeaderValue},
        Request,
    },
    middleware::MiddlewareNext,
    Agent, SendBody,
};

static GRAPHQL_API_URL: &str = "https://api.github.com/graphql";
static RATE_LIMIT_URL: &str = "https://api.github.com/rate_limit";

static USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_REPOSITORY"),
    ")",
);

#[allow(unsafe_code)]
// SAFETY: 50 != 0
pub const DEFAULT_BATCH_SIZE: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(50) };

#[derive(Clone, Debug)]
pub struct Client {
    inner: Agent,
}

impl Client {
    pub fn new(token: &str) -> Result<Client, BuildClientError> {
        let auth = HeaderValue::from_str(&format!("Bearer {token}"))?;
        let inner = Agent::config_builder()
            .https_only(true)
            .user_agent(USER_AGENT)
            .middleware(
                move |mut req: Request<SendBody<'_>>, next: MiddlewareNext<'_>| {
                    let _ = req.headers_mut().insert("Authorization", auth.clone());
                    let _ = req
                        .headers_mut()
                        .insert("X-Github-Next-Global-ID", HeaderValue::from_static("1"));
                    next.handle(req)
                },
            )
            .build()
            .into();
        Ok(Client { inner })
    }

    pub fn new_with_local_token() -> Result<Client, BuildClientError> {
        let token = gh_token::get()?;
        Client::new(&token)
    }

    pub fn get_rate_limit(&self) -> Result<RateLimit, RateLimitError> {
        let bytes = self
            .inner
            .get(RATE_LIMIT_URL)
            .call()
            .map_err(|e| RateLimitError::Http(Box::new(e)))?
            .into_body()
            .read_to_vec()
            .map_err(|e| RateLimitError::Read(Box::new(e)))?;
        let r = serde_json::from_slice::<RateLimitResponse>(&bytes)?;
        Ok(r.resources.graphql)
    }

    pub fn query(&self, payload: QueryPayload) -> Result<JsonMap, QueryError> {
        let bytes = self
            .inner
            .post(GRAPHQL_API_URL)
            .send_json(payload)
            .map_err(|e| QueryError::Http(Box::new(e)))?
            .into_body()
            .read_to_vec()
            .map_err(|e| QueryError::Read(Box::new(e)))?;
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
    yielding: VecDeque<Q::Output>,
    payload: Option<QueryPayload>,
}

impl<'a, Q: QueryMachine> QueryResults<'a, Q> {
    fn new(client: &'a Client, query: Q) -> Self {
        QueryResults {
            client,
            query,
            query_done: false,
            yielding: VecDeque::new(),
            payload: None,
        }
    }
}

impl<Q: QueryMachine> Iterator for QueryResults<'_, Q> {
    type Item = Result<Q::Output, QueryError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(value) = self.yielding.pop_front() {
                return Some(Ok(value));
            } else if self.query_done {
                return None;
            } else if let Some(payload) = self.payload.take() {
                match self.client.query(payload) {
                    Ok(data) => {
                        if let Err(e) = self.query.handle_response(data) {
                            return Some(Err(e.into()));
                        }
                    }
                    Err(e) => return Some(Err(e)),
                }
                self.yielding.extend(self.query.get_output());
            } else {
                if let Some(payload) = self.query.get_next_query() {
                    self.payload = Some(payload);
                } else {
                    self.query_done = true;
                }
                self.yielding.extend(self.query.get_output());
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum BuildClientError {
    #[error("invalid authorization token")]
    Auth(#[from] InvalidHeaderValue),
    #[error("failed to fetch GitHub access token")]
    GetToken(#[from] gh_token::Error),
}

#[derive(Debug, Error)]
pub enum RateLimitError {
    #[error("failed to perform rate limit request")]
    Http(#[source] Box<ureq::Error>),
    #[error("failed to read rate limit response")]
    Read(#[source] Box<ureq::Error>),
    #[error("failed to deserialize rate limit response")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("failed to perform GraphQL request")]
    Http(#[source] Box<ureq::Error>),
    #[error("failed to read GraphQL response")]
    Read(#[source] Box<ureq::Error>),
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
    pub query: String,
    pub variables: JsonMap,
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
