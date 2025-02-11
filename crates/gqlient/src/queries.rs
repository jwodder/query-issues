use crate::types::{Cursor, JsonMap, Page, Variable};
use crate::QueryPayload;
use indenter::indented;
use std::collections::{
    VecDeque,
    {hash_map::Entry, HashMap},
};
use std::fmt::Write;
use std::num::NonZeroUsize;

pub trait Query: Sized {
    type Output;

    fn with_variable_prefix(self, prefix: String) -> Self;
    fn write_graphql<W: Write>(&self, s: W) -> std::fmt::Result;
    fn variables(&self) -> impl IntoIterator<Item = (String, Variable)>;
    fn parse_response(&self, value: serde_json::Value) -> Result<Self::Output, serde_json::Error>;
}

pub trait Paginator {
    type Query: Query<Output = Page<Self::Item>>;
    type Item;

    fn for_cursor(&self, cursor: Option<&Cursor>) -> Self::Query;
}

pub trait QueryMachine {
    type Output;

    fn get_next_query(&mut self) -> Option<QueryPayload>;
    fn handle_response(&mut self, data: JsonMap) -> Result<(), serde_json::Error>;
    fn get_output(&mut self) -> Vec<Self::Output>;
}

#[allow(missing_debug_implementations)]
pub struct BatchPaginator<K, P: Paginator> {
    in_progress: VecDeque<PaginationState<K, P>>,
    results: Vec<PaginationResults<K, P::Item>>,
    active: HashMap<String, ActiveQuery<K, P>>,
    batch_size: NonZeroUsize,
}

impl<K, P: Paginator> BatchPaginator<K, P> {
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
                if i > 0 {
                    write!(&mut varstr, ", ").expect("writing to a string should not fail");
                }
                write!(&mut varstr, "${name}: {gql_type}")
                    .expect("writing to a string should not fail");
                variables.insert(name, value);
            }
            write!(&mut qwrite, "{alias}: ").expect("writing to a string should not fail");
            query
                .write_graphql(&mut qwrite)
                .expect("writng to a string should not fail");
            self.active.insert(alias, ActiveQuery { state, query });
        }
        let full_query = format!("query ({varstr}) {{\n{qstr}}}\n");
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
// here — but I don't need them and that's busywork, so…
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
