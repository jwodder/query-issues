// See <https://users.rust-lang.org/t/125565/2> for what's up with the tricky
// generic bounds on some structs
use crate::types::{Cursor, JsonMap, Page, Variable};
use crate::{DEFAULT_BATCH_SIZE, QueryPayload};
use indenter::indented;
use std::collections::{HashMap, VecDeque, hash_map::Entry};
use std::fmt::{self, Write};
use std::num::NonZeroUsize;

/// A trait for a GraphQL *selection set* (a brace-enclosed list of fields &
/// fragments) that can be combined with other `QuerySelection` instances to
/// form a complete query
pub trait QuerySelection: Sized {
    type Output;

    /// Return a modified instance of `self` in which all variable names are
    /// prefixed with `prefix`.
    ///
    /// This allows for combining `QuerySelection` instances that would
    /// otherwise have overlapping variable names.
    fn with_variable_prefix(self, prefix: String) -> Self;

    /// Write the text of the GraphQL selection set to `s`
    fn write_selection<W: Write>(&self, s: W) -> fmt::Result;

    /// Return an iterator of variable names (including any prefixes previously
    /// specified with `with_variable_prefix()`) paired with the variables'
    /// types & values
    fn variables(&self) -> impl IntoIterator<Item = (String, Variable)>;

    /// Parse the portion of a GraphQL response corresponding to this selection
    /// set into an `Output` instance
    fn parse_response(&self, value: serde_json::Value) -> Result<Self::Output, serde_json::Error>;
}

/// A trait for values that can produce [`QuerySelection`]s that start
/// paginating at a given cursor value and output [`Page`]s of items
pub trait Paginator {
    /// The type of [`QuerySelection`] that this trait instance produces
    type Selection: QuerySelection<Output = Page<Self::Item>>;

    /// The type of items that the [`QuerySelection`]s paginate over
    type Item;

    /// Produce a [`QuerySelection`] that requests a page starting at the given
    /// cursor value
    fn for_cursor(&self, cursor: Option<&Cursor>) -> Self::Selection;
}

/// A trait for a sans-IO state machine for issuing multiple GraphQL queries
/// and processing the responses.
///
/// A `QueryMachine` is intended to be used in a loop of the following steps:
///
/// - Call `get_next_query()`; if the result is non-`None`, perform the given
///   query against the GraphQL server
///
/// - Call `get_output()` and process the results
///
/// - If the previous call to `get_next_query()` returned `None` or the GraphQL
///   request to the server failed/returned an error response, terminate the
///   loop
///
/// - Call `handle_response()` with the deserialized `"data"` field from the
///   server's response to the query returned by `get_next_query()`; if an
///   error is returned, terminate the loop
pub trait QueryMachine {
    type Output;

    /// Obtain the next query, if any, to perform against the GraphQL server.
    ///
    /// If the query is successful, the result is to be passed to the
    /// `handle_response()` method.
    ///
    /// `get_next_query()` MUST NOT be called multiple times without
    /// intervening calls to `handle_response()`.
    ///
    /// Once this method returns `None`, no further calls to `get_next_query()`
    /// or `handle_response()` should be performed.
    fn get_next_query(&mut self) -> Option<QueryPayload>;

    /// Provide the machine with the deserialized value of the `"data"` field
    /// from a successful response to the query returned by the most recent
    /// call to `get_next_query()`.
    ///
    /// This method MUST be called only after calling `get_next_query()` and
    /// receiving a `Some` value, and the `data` argument MUST be in response
    /// to the query returned from the most recent call to `get_next_query()`.
    ///
    /// If this method returns `Err`, the `get_next_query()` and
    /// `handle_response()` methods MUST NOT be called again.
    fn handle_response(&mut self, data: JsonMap) -> Result<(), serde_json::Error>;

    /// Retrieve a (possibly empty) list of output values produced so far by
    /// the machine.  Subsequent calls will not return previously-returned
    /// values again and may return new values.  Users of a `QueryMachine` are
    /// advised to call `get_output()` after each call to `handle_response()`
    /// and process the results as a whole.
    ///
    /// `get_output()` MAY be called at any point relative to
    /// `get_next_query()` and `handle_response()`.
    ///
    /// If `get_output()` is called multiple times without an intervening
    /// `handle_response()`, all calls after the first SHOULD return an empty
    /// list.
    ///
    /// Once `get_next_query()` returns `None`, the next call to `get_output()`
    /// MAY return a nonempty list, and all subsequent calls SHOULD return an
    /// empty list.
    ///
    /// If `handle_response()` returns `Err`, the next call to `get_output()`
    /// MAY return a nonempty list, and all subsequent calls SHOULD return an
    /// empty list, but making use of any values after the error is
    /// discouraged.
    fn get_output(&mut self) -> Vec<Self::Output>;
}

/// A [`QueryMachine`] that requests [`Paginator`]s in batches of up to a
/// certain number of selection sets per query and keeps requesting subsequent
/// pages of results until they all reach their end.
///
/// Each input `Paginator` is associated with a user-defined key, and when the
/// queries for a given `Paginator` reach their end, the key is returned
/// alongside all items obtained for the `Paginator` from the queries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchPaginator<K, P: Paginator<Item = Item>, Item = <P as Paginator>::Item> {
    /// A queue of paginators to issue queries for.  Entries are popped off in
    /// batches of `batch_size` to compose a query, and if the query response
    /// indicates that a paginator still has pages left, the paginator is added
    /// back to the end of the queue.
    ///
    /// Paginators currently in `active` are not included here.
    in_progress: VecDeque<PaginationState<K, P>>,

    /// Any [`PaginationResults`] that were produced by `handle_response()` but
    /// not yet returned by `get_output()`
    results: Vec<PaginationResults<K, P::Item>>,

    /// After a call to `get_next_query()` and before the corresponding call to
    /// `handle_response()`, `active` contains information on the paginators
    /// that were queried by the return value from `get_next_query()`.  The
    /// keys of the map are the GraphQL field aliases applied to the
    /// corresponding selection sets.
    active: HashMap<String, ActiveQuery<K, P, P::Selection>>,

    /// Maximum number of `QuerySelection`s to combine into a single query
    batch_size: NonZeroUsize,
}

impl<K, P: Paginator> BatchPaginator<K, P> {
    /// Construct a new `BatchPaginator` from an iterable of (key, paginator)
    /// pairs and a maximum number of selection sets to combine into a single
    /// query.
    ///
    /// The keys will be used in the [`PaginationResults`] returned by the
    /// `QueryMachine` to identify the corresponding original paginators.
    pub fn new<I: IntoIterator<Item = (K, P)>>(queries: I, batch_size: NonZeroUsize) -> Self {
        let in_progress = queries
            .into_iter()
            .map(|(key, paginator)| PaginationState::new(key, paginator))
            .collect::<VecDeque<_>>();
        let results = Vec::new();
        let active = HashMap::new();
        BatchPaginator {
            in_progress,
            results,
            active,
            batch_size,
        }
    }
}

impl<K, P: Paginator> Default for BatchPaginator<K, P> {
    fn default() -> Self {
        BatchPaginator {
            in_progress: VecDeque::new(),
            results: Vec::new(),
            active: HashMap::new(),
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }
}

impl<K, P: Paginator> QueryMachine for BatchPaginator<K, P> {
    type Output = PaginationResults<K, P::Item>;

    fn get_next_query(&mut self) -> Option<QueryPayload> {
        if self.in_progress.is_empty() {
            return None;
        }
        let mut variables = JsonMap::new();
        let mut varstr = String::new();
        let mut qstr = String::new();
        let mut qwrite = indented(&mut qstr).with_str("    ");
        let mut first_var = true;
        for (i, state) in self
            .in_progress
            .drain(0..(self.in_progress.len().min(self.batch_size.get())))
            .enumerate()
        {
            let alias = format!("q{i}");
            let query = state
                .paginator
                .for_cursor(state.cursor.as_ref())
                .with_variable_prefix(alias.clone());
            for (name, Variable { gql_type, value }) in query.variables() {
                if !std::mem::replace(&mut first_var, false) {
                    write!(&mut varstr, ", ").expect("writing to a string should not fail");
                }
                write!(&mut varstr, "${name}: {gql_type}")
                    .expect("writing to a string should not fail");
                variables.insert(name, value);
            }
            write!(&mut qwrite, "{alias}: ").expect("writing to a string should not fail");
            query
                .write_selection(&mut qwrite)
                .expect("writing to a string should not fail");
            self.active.insert(alias, ActiveQuery { state, query });
        }
        let full_query = format!("query ({varstr}) {{\n{qstr}}}");
        Some(QueryPayload {
            query: full_query,
            variables,
        })
    }

    fn handle_response(&mut self, data: JsonMap) -> Result<(), serde_json::Error> {
        for (alias, value) in data {
            let Entry::Occupied(aqo) = self.active.entry(alias) else {
                // TODO: Warn or error
                continue;
            };
            let state = aqo.remove().process_response(value)?;
            if state.has_next_page {
                self.in_progress.push_back(state);
            } else {
                self.results.push(PaginationResults::from(state));
            }
        }
        Ok(())
    }

    fn get_output(&mut self) -> Vec<Self::Output> {
        self.results.drain(..).collect()
    }
}

/// Current state of paginating over pages of items with a [`Paginator`] in a
/// [`BatchPaginator`]
#[derive(Clone, Debug, Eq, PartialEq)]
struct PaginationState<K, P: Paginator<Item = Item>, Item = <P as Paginator>::Item> {
    /// The key that the user supplied for this paginator, to be used to
    /// identify it in the [`PaginationResults`]
    key: K,

    /// The [`Paginator`]
    paginator: P,

    /// All items obtained so far from querying the paginator's pages
    items: Vec<Item>,

    /// The cursor for the start of the next page of results, or `None` to
    /// start at the beginning
    cursor: Option<Cursor>,

    /// Should we request another page of results?
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

/// Information on a pagination selection set that was included in a query by a
/// [`BatchPaginator`] but has not yet had its results processed
#[derive(Clone, Debug, Eq, PartialEq)]
struct ActiveQuery<K, P: Paginator<Selection = S>, S = <P as Paginator>::Selection> {
    /// The [`PaginationState`] for the [`Paginator`]
    state: PaginationState<K, P, P::Item>,

    /// The [`QuerySelection`] that was included in the query, used to process
    /// the selection set's portion of the response
    query: S,
}

impl<K, P: Paginator> ActiveQuery<K, P> {
    /// Parse the portion of a GraphQL response corresponding to this selection
    /// set and update & return the [`PaginationState`]
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

/// Results of querying all pages of a [`Paginator`] with a [`BatchPaginator`]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaginationResults<K, T> {
    /// The key supplied by the user alongside the `Paginator` value when
    /// constructing the `BatchPaginator`
    pub key: K,

    /// All items obtained from querying the paginator's pages
    pub items: Vec<T>,

    /// The cursor for the end of the paginated results, if any
    pub end_cursor: Option<Cursor>,
}

impl<K, P: Paginator> From<PaginationState<K, P>> for PaginationResults<K, P::Item> {
    fn from(value: PaginationState<K, P>) -> Self {
        PaginationResults {
            key: value.key,
            items: value.items,
            end_cursor: value.cursor,
        }
    }
}
