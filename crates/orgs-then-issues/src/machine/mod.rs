use crate::queries::{GetIssues, GetLabels, GetOwnerRepos, Issue};
use gqlient::{BatchPaginator, Id, JsonMap, QueryMachine, QueryPayload};
use serde::Serialize;
use std::collections::HashMap;
use std::num::NonZeroUsize;

/// A [`QueryMachine`] for fetching open issues in repositories owned by given
/// users and/or organizations.
///
/// The query machine issues the following queries, in order:
///
/// - Paginated queries for the repositories owned by the given repository
///   owners
///
/// - Paginated queries for open issues in those repositories that have any
///
/// - If any open issue has more than one page of labels, paginated queries for
///   the remaining labels for those issues
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OrgsThenIssues<S> {
    /// Internal state
    state: State,

    /// Common data passed to methods of `state`
    shared: Shared<S>,
}

impl OrgsThenIssues<()> {
    /// Create a new `OrgsThenIssues` instance that will fetch open issues in
    /// repositories owned by any owners listed in `owners`
    pub(crate) fn new(owners: Vec<String>, limits: QueryLimits) -> OrgsThenIssues<()> {
        OrgsThenIssues {
            state: Start { owners }.into(),
            shared: Shared {
                limits,
                results: Vec::new(),
                subscriber: (),
            },
        }
    }
}

impl<S> OrgsThenIssues<S> {
    /// Set the [`EventSubscriber`] that the `OrgsThenIssues` will report
    /// transition events to
    pub(crate) fn with_subscriber<S2>(self, subscriber: S2) -> OrgsThenIssues<S2> {
        let OrgsThenIssues {
            state,
            shared: Shared {
                limits, results, ..
            },
        } = self;
        OrgsThenIssues {
            state,
            shared: Shared {
                limits,
                results,
                subscriber,
            },
        }
    }
}

impl<S: EventSubscriber> QueryMachine for OrgsThenIssues<S> {
    type Output = Issue;

    fn get_next_query(&mut self) -> Option<QueryPayload> {
        let (state, output) =
            std::mem::replace(&mut self.state, Error.into()).get_next_query(&mut self.shared);
        self.state = state;
        output
    }

    fn handle_response(&mut self, data: JsonMap) -> Result<(), serde_json::Error> {
        let r = self.state.handle_response(data, &mut self.shared);
        if r.is_err() {
            self.state = Error.into();
            self.shared.subscriber.handle_event(Event::Error);
        }
        r
    }

    fn get_output(&mut self) -> Vec<Issue> {
        self.shared.results.drain(..).collect()
    }
}

/// Data on the `OrgsThenIssues` instance that is accessed by the methods of
/// the individual states
#[derive(Clone, Debug, Eq, PartialEq)]
struct Shared<S> {
    /// Batching & pagination limits to obey when forming queries
    limits: QueryLimits,

    /// Output produced by the `QueryMachine` that has not yet been returned by
    /// a call to `get_output()`
    results: Vec<Issue>,

    /// An [`EventSubscriber`] instance that transition events will be reported
    /// to
    subscriber: S,
}

/// Behavior common to all states of the `OrgsThenIssues` state machine
#[enum_dispatch::enum_dispatch]
trait MachineState {
    /// Obtain the next query, if any, to perform against the GraphQL server
    /// along with the updated internal state (which may or may not be the same
    /// type as before)
    fn get_next_query<S: EventSubscriber>(
        self,
        shared: &mut Shared<S>,
    ) -> (State, Option<QueryPayload>);

    /// Provide the state with the deserialized value of the `"data"` field
    /// from a successful response to the query returned by the most recent
    /// call to some state's `get_next_query()`
    fn handle_response<S: EventSubscriber>(
        &mut self,
        data: JsonMap,
        shared: &mut Shared<S>,
    ) -> Result<(), serde_json::Error>;
}

/// Internal states of the `OrgsThenIssues` state machine
#[enum_dispatch::enum_dispatch(MachineState)]
#[derive(Clone, Debug, Eq, PartialEq)]
enum State {
    Start,
    FetchRepos,
    FetchIssues,
    FetchLabels,
    Done,
    Error,
}

/// The initial state
///
/// Calling `get_next_query()` will return the first query from a
/// [`BatchPaginator`] over [`GetOwnerRepos`] paginators and transition the
/// state to [`FetchRepos`] — unless the list of repository owners is empty, in
/// which case no query is returned and the state transitions to [`Done`].
///
/// `handle_response()` should never be called on this state; doing so will
/// result in a panic.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Start {
    /// List of repository owners for whose repositories open issues should be
    /// fetched
    owners: Vec<String>,
}

impl MachineState for Start {
    fn get_next_query<S: EventSubscriber>(
        self,
        shared: &mut Shared<S>,
    ) -> (State, Option<QueryPayload>) {
        shared.subscriber.handle_event(Event::Start);
        FetchRepos::start(self.owners, shared)
    }

    fn handle_response<S: EventSubscriber>(
        &mut self,
        _data: JsonMap,
        _shared: &mut Shared<S>,
    ) -> Result<(), serde_json::Error> {
        panic!("handle_response() called before get_next_query()")
    }
}

/// State when issuing queries for repositories owned by the given repository
/// owners
///
/// Once all repositories are retrieved, the next call to `get_next_query()`
/// will result in a transition to [`FetchIssues`], unless no repositories had
/// any open issues, in which case we transition to [`Done`] instead.
#[derive(Clone, Debug, Eq, PartialEq)]
struct FetchRepos {
    /// An inner `QueryMachine` for performing the queries for this state
    submachine: BatchPaginator<String, GetOwnerRepos>,

    /// A list of (repository ID, [`GetIssues`] paginator) pairs for any
    /// repositories fetched so far that have open issues
    issue_queries: Vec<(Id, GetIssues)>,

    /// Total number of repositories retrieved so far
    repositories: usize,
}

impl FetchRepos {
    /// Transition into this state and return the state's first query — unless
    /// `owners` is empty, in which case the state transitions to [`Done`]
    /// instead and no query is returned
    fn start<S: EventSubscriber>(
        owners: Vec<String>,
        shared: &mut Shared<S>,
    ) -> (State, Option<QueryPayload>) {
        let mut submachine = BatchPaginator::new(
            owners.into_iter().map(|owner| {
                (
                    owner.clone(),
                    GetOwnerRepos::new(owner, shared.limits.page_size),
                )
            }),
            shared.limits.batch_size,
        );
        if let query @ Some(_) = submachine.get_next_query() {
            shared.subscriber.handle_event(Event::StartFetchRepos);
            let state = FetchRepos {
                submachine,
                issue_queries: Vec::new(),
                repositories: 0,
            }
            .into();
            (state, query)
        } else {
            Done::start(shared)
        }
    }
}

impl MachineState for FetchRepos {
    fn get_next_query<S: EventSubscriber>(
        mut self,
        shared: &mut Shared<S>,
    ) -> (State, Option<QueryPayload>) {
        if let query @ Some(_) = self.submachine.get_next_query() {
            (self.into(), query)
        } else {
            shared.subscriber.handle_event(Event::EndFetchRepos {
                repositories: self.repositories,
                repos_with_open_issues: self.issue_queries.len(),
            });
            FetchIssues::start(self.issue_queries, shared)
        }
    }

    fn handle_response<S: EventSubscriber>(
        &mut self,
        data: JsonMap,
        shared: &mut Shared<S>,
    ) -> Result<(), serde_json::Error> {
        self.submachine.handle_response(data)?;
        for repo in self
            .submachine
            .get_output()
            .into_iter()
            .flat_map(|pr| pr.items)
        {
            self.repositories += 1;
            if let Some(q) =
                repo.issues_query(shared.limits.page_size, shared.limits.label_page_size)
            {
                self.issue_queries.push(q);
            }
        }
        Ok(())
    }
}

/// State when issuing queries for open issues in previously-retrieved
/// repositories
///
/// Once all issues are retrieved, the next call to `get_next_query()` will
/// result in a transition to [`FetchLabels`], unless no issues had any
/// additional pages of labels, in which case we transition to [`Done`]
/// instead.
#[derive(Clone, Debug, Eq, PartialEq)]
struct FetchIssues {
    /// An inner `QueryMachine` for performing the queries for this state
    submachine: BatchPaginator<Id, GetIssues>,

    /// A list of (issue ID, [`GetLabels`] paginator) pairs for any issues
    /// fetched so far that have more than one page of labels
    label_queries: Vec<(Id, GetLabels)>,

    /// A collection of all issues fetched so far that have more than one page
    /// of labels, keyed by issue ID
    issues_needing_labels: HashMap<Id, Issue>,

    /// Total number of open issues retrieved so far
    open_issues: usize,
}

impl FetchIssues {
    /// Transition into this state and return the state's first query — unless
    /// `issue_queries` is empty, in which case the state transitions to
    /// [`Done`] instead and no query is returned.
    ///
    /// `issue_queries` is a list of (repository ID, [`GetIssues`] paginator)
    /// pairs corresponding to repositories that have open issues.
    fn start<S: EventSubscriber>(
        issue_queries: Vec<(Id, GetIssues)>,
        shared: &mut Shared<S>,
    ) -> (State, Option<QueryPayload>) {
        let mut submachine = BatchPaginator::new(issue_queries, shared.limits.batch_size);
        if let query @ Some(_) = submachine.get_next_query() {
            shared.subscriber.handle_event(Event::StartFetchIssues);
            let state = FetchIssues {
                submachine,
                label_queries: Vec::new(),
                issues_needing_labels: HashMap::new(),
                open_issues: 0,
            }
            .into();
            (state, query)
        } else {
            Done::start(shared)
        }
    }
}

impl MachineState for FetchIssues {
    fn get_next_query<S: EventSubscriber>(
        mut self,
        shared: &mut Shared<S>,
    ) -> (State, Option<QueryPayload>) {
        if let query @ Some(_) = self.submachine.get_next_query() {
            (self.into(), query)
        } else {
            shared.subscriber.handle_event(Event::EndFetchIssues {
                open_issues: self.open_issues,
            });
            FetchLabels::start(self.label_queries, self.issues_needing_labels, shared)
        }
    }

    fn handle_response<S: EventSubscriber>(
        &mut self,
        data: JsonMap,
        shared: &mut Shared<S>,
    ) -> Result<(), serde_json::Error> {
        self.submachine.handle_response(data)?;
        for iwl in self
            .submachine
            .get_output()
            .into_iter()
            .flat_map(|pr| pr.items)
        {
            self.open_issues += 1;
            if let Some(q) = iwl.more_labels_query(shared.limits.label_page_size) {
                self.label_queries.push(q);
                self.issues_needing_labels.insert(iwl.issue_id, iwl.issue);
            } else {
                shared.results.push(iwl.issue);
            }
        }
        Ok(())
    }
}

/// State when issuing queries for additional pages of labels for open issues
///
/// Once all labels are retrieved, the next call to `get_next_query()` will
/// result in a transition to [`Done`].
#[derive(Clone, Debug, Eq, PartialEq)]
struct FetchLabels {
    /// An inner `QueryMachine` for performing the queries for this state
    submachine: BatchPaginator<Id, GetLabels>,

    /// A collection of all issues fetched that have more than one page of
    /// labels, keyed by issue ID
    issues_needing_labels: HashMap<Id, Issue>,

    /// Total number of labels fetched so far that required extra queries to
    /// retrieve
    extra_labels: usize,
}

impl FetchLabels {
    /// Transition into this state and return the state's first query — unless
    /// `label_queries` is empty, in which case the state transitions to
    /// [`Done`] instead and no query is returned.
    ///
    /// `label_queries` is a list of (issue ID, [`GetLabels`] paginator) pairs
    /// corresponding to issues that have more than one page of labels.
    ///
    /// `issues_needing_labels` is a collection of all issues that have more
    /// than one page of labels, keyed by issue ID.
    fn start<S: EventSubscriber>(
        label_queries: Vec<(Id, GetLabels)>,
        issues_needing_labels: HashMap<Id, Issue>,
        shared: &mut Shared<S>,
    ) -> (State, Option<QueryPayload>) {
        let mut submachine = BatchPaginator::new(label_queries, shared.limits.batch_size);
        if let query @ Some(_) = submachine.get_next_query() {
            shared.subscriber.handle_event(Event::StartFetchLabels {
                issues_with_extra_labels: issues_needing_labels.len(),
            });
            let state = FetchLabels {
                submachine,
                issues_needing_labels,
                extra_labels: 0,
            }
            .into();
            (state, query)
        } else {
            Done::start(shared)
        }
    }
}

impl MachineState for FetchLabels {
    fn get_next_query<S: EventSubscriber>(
        mut self,
        shared: &mut Shared<S>,
    ) -> (State, Option<QueryPayload>) {
        if let query @ Some(_) = self.submachine.get_next_query() {
            (self.into(), query)
        } else {
            shared.subscriber.handle_event(Event::EndFetchLabels {
                extra_labels: self.extra_labels,
            });
            shared
                .results
                .extend(self.issues_needing_labels.into_values());
            Done::start(shared)
        }
    }

    fn handle_response<S: EventSubscriber>(
        &mut self,
        data: JsonMap,
        _shared: &mut Shared<S>,
    ) -> Result<(), serde_json::Error> {
        self.submachine.handle_response(data)?;
        for res in self.submachine.get_output() {
            self.extra_labels += res.items.len();
            self.issues_needing_labels
                .get_mut(&res.key)
                .expect("Issues we get labels for should have already been seen")
                .labels
                .extend(res.items);
        }
        Ok(())
    }
}

/// The successful final state
///
/// `handle_response()` should never be called on this state; doing so will
/// result in a panic.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Done;

impl Done {
    /// Transition to this state
    fn start<S: EventSubscriber>(shared: &mut Shared<S>) -> (State, Option<QueryPayload>) {
        shared.subscriber.handle_event(Event::Done);
        (Done.into(), None)
    }
}

impl MachineState for Done {
    fn get_next_query<S: EventSubscriber>(
        self,
        _shared: &mut Shared<S>,
    ) -> (State, Option<QueryPayload>) {
        (self.into(), None)
    }

    fn handle_response<S: EventSubscriber>(
        &mut self,
        _data: JsonMap,
        _shared: &mut Shared<S>,
    ) -> Result<(), serde_json::Error> {
        panic!("handle_response() called after machine completed")
    }
}

/// The error state, entered if a call to `handle_response()` on any other
/// state returns an error.
///
/// `get_next_query()` and `handle_response()` should never be called on this
/// state; doing so will result in a panic.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Error;

impl MachineState for Error {
    fn get_next_query<S: EventSubscriber>(
        self,
        _shared: &mut Shared<S>,
    ) -> (State, Option<QueryPayload>) {
        panic!("get_next_query() called after machine errored")
    }

    fn handle_response<S: EventSubscriber>(
        &mut self,
        _data: JsonMap,
        _shared: &mut Shared<S>,
    ) -> Result<(), serde_json::Error> {
        panic!("handle_response() called after machine errored")
    }
}

/// Configuration for batching & pagination limits when forming queries
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[allow(clippy::struct_field_names)]
pub(crate) struct QueryLimits {
    /// Maximum number of query fields to combine into a single query at once
    pub(crate) batch_size: NonZeroUsize,

    /// Number of repositories and issues to request per page
    pub(crate) page_size: NonZeroUsize,

    /// Number of labels to request per page
    pub(crate) label_page_size: NonZeroUsize,
}

/// A trait for subscribers for receiving information about state transitions
/// and fetched resources from an `OrgsThenIssues`
pub(crate) trait EventSubscriber {
    fn handle_event(&mut self, ev: Event);
}

impl EventSubscriber for () {
    fn handle_event(&mut self, _ev: Event) {}
}

/// An announcement of a state transition, marking the start or end of a state
/// or operations as a whole.
///
/// During the lifetime of an `OrgsThenIssues`, events are always passed to an
/// `EventSubscriber` in the order that they are defined here, with the
/// following exceptions:
///
/// - Some pairs of "start" and "end" events may be skipped if there is no need
///   to enter the associated states
///
/// - An `Error` event may occur at any point between `Start` & `Done` and will
///   likely result in the most recent "start" event not receiving a paired
///   "end" event
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Event {
    /// Querying has started.
    ///
    /// This is always the first event emitted.
    Start,

    /// The [`FetchRepos`] state has started
    StartFetchRepos,

    /// The [`FetchRepos`] state has finished
    EndFetchRepos {
        /// Total number of repositories retrieved
        repositories: usize,

        /// Number of repositories with open issues retrieved
        repos_with_open_issues: usize,
    },

    /// The [`FetchIssues`] state has started
    StartFetchIssues,

    /// The [`FetchIssues`] state has finished
    EndFetchIssues {
        /// Number of open issues retrieved
        open_issues: usize,
    },

    /// The [`FetchLabels`] state has started
    StartFetchLabels {
        /// Number of issues retrieved that have more than one page of labels
        issues_with_extra_labels: usize,
    },

    /// The [`FetchLabels`] state has finished
    EndFetchLabels {
        /// Total number of labels that required extra queries to retrieve
        extra_labels: usize,
    },

    /// Querying has completed successfully.
    ///
    /// No more events will be emitted after this event.
    Done,

    /// A `handle_response()` method returned an error.
    ///
    /// No more events will be emitted after this event.
    Error,
}

#[cfg(test)]
mod tests;
