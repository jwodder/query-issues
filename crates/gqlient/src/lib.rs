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

/// The URL to which GitHub GraphQL queries are sent
static GRAPHQL_API_URL: &str = "https://api.github.com/graphql";

/// The URL to which GitHub REST API requests for rate limit information are
/// sent
static RATE_LIMIT_URL: &str = "https://api.github.com/rate_limit";

/// The [`Client`]'s "User-Agent" header value
static USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_REPOSITORY"),
    ")",
);

/// The default batch size for use by [`BatchPaginator`]s
pub const DEFAULT_BATCH_SIZE: NonZeroUsize = NonZeroUsize::new(50).unwrap();

/// A client for performing requests to the GitHub GraphQL API
#[derive(Clone, Debug)]
pub struct Client {
    inner: Agent,
}

impl Client {
    /// Create a new client instance using the given GitHub access token
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

    /// Fetch the local user's GitHub access token with [`gh_token`] and use it
    /// to create a new client instance
    pub fn new_with_local_token() -> Result<Client, BuildClientError> {
        let token = gh_token::get()?;
        Client::new(&token)
    }

    /// Get the current details on the GraphQL API rate limit for the
    /// authenticated user
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

    /// Perform a GraphQL query and return its deserialized response data on
    /// success
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

    /// Return an iterator over the results of running a [`QueryMachine`] to
    /// completion
    pub fn run<Q: QueryMachine>(&self, query: Q) -> MachineRunner<'_, Q> {
        MachineRunner::new(self, query)
    }
}

/// An iterator that runs a [`QueryMachine`] to completion against a [`Client`]
/// and yields the outputs
#[derive(Debug)]
pub struct MachineRunner<'a, Q: QueryMachine> {
    client: &'a Client,

    /// The `QueryMachine`
    query: Q,

    /// Has the `QueryMachine` terminated?
    query_done: bool,

    /// Any values returned by [`QueryMachine::get_output()`] that have yet to
    /// be yielded by the iterator
    yielding: VecDeque<Q::Output>,

    /// A `QueryPayload` returned by [`QueryMachine::get_next_query()`] to send
    /// to the client in the next request
    payload: Option<QueryPayload>,
}

impl<'a, Q: QueryMachine> MachineRunner<'a, Q> {
    fn new(client: &'a Client, query: Q) -> Self {
        MachineRunner {
            client,
            query,
            query_done: false,
            yielding: VecDeque::new(),
            payload: None,
        }
    }
}

impl<Q: QueryMachine> Iterator for MachineRunner<'_, Q> {
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
                    Err(e) => {
                        self.query_done = true;
                        return Some(Err(e));
                    }
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

/// Structure returned for requests to [`RATE_LIMIT_URL`]
// This can't be replaced with Singleton because the JSON contains more than
// one field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct RateLimitResponse {
    /// Rate limit details for different API resources
    resources: RateLimitResources,
}

/// Rate limit details for different API resources
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct RateLimitResources {
    /// The rate limit details for the GraphQL API
    graphql: RateLimit,
}

/// Information on the rate limit points for a given resource
#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RateLimit {
    /// The number of rate limit points used for the resource since the last
    /// reset
    used: u32,

    /// The UNIX timestamp at which the rate limit points for the resource will
    /// next reset
    reset: u64,
}

impl RateLimit {
    /// Returns the number of rate limit points used since a previous
    /// `RateLimit` instance was fetched.
    ///
    /// Returns `None` if a reset happened in between the fetching of the two
    /// `RateLimit` values.
    pub fn used_since(self, since: RateLimit) -> Option<u32> {
        (self.reset == since.reset).then(|| self.used.saturating_sub(since.used))
    }
}

/// A complete GraphQL query to send to the GitHub GraphQL API
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct QueryPayload {
    /// A complete GraphQL query string
    pub query: String,

    /// A collection of variables used by `query` as a map from variable names
    /// to values
    pub variables: JsonMap,
}

/// A deserialized complete response to a GraphQL query
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct Response {
    /// The data requested by the query
    #[serde(default)]
    data: JsonMap,

    /// Any errors reported by the server for the query
    #[serde(default)]
    errors: GqlError,
}

impl Response {
    /// If `errors` is empty, return `Ok(data)`; otherwise, return
    /// `Err(errors)`
    fn into_data(self) -> Result<JsonMap, GqlError> {
        if self.errors.is_empty() {
            Ok(self.data)
        } else {
            Err(self.errors)
        }
    }
}

/// A collection of errors returned for a GraphQL query
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

/// An individual GraphQL query error
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct GqlInnerError {
    #[serde(default, rename = "type")]
    err_type: Option<String>,
    message: String,
    #[serde(default)]
    path: Option<Vec<String>>,
}
