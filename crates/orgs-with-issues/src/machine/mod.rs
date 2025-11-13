use crate::queries::{GetIssues, GetLabels, GetOwnerRepos};
use crate::types::Issue;
use gqlient::{BatchPaginator, Id, JsonMap, QueryMachine, QueryPayload};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

/// A [`QueryMachine`] for fetching open issues in repositories owned by given
/// users and/or organizations.
///
/// The query machine issues the following queries, in order:
///
/// - Paginated queries for the repositories owned by the given repository
///   owners along with the first page of open issues for each repository
///
/// - If any repository has more than one page of open issues, paginated
///   queries for the remaining open issues for those repositories
///
/// - If any open issue has more than one page of labels, paginated queries for
///   the remaining labels for those issues
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OrgsWithIssues {
    /// Internal state
    state: State,

    /// Common data passed to methods of `state`
    shared: Shared,
}

impl OrgsWithIssues {
    /// Create a new `OrgsWithIssues` instance that will fetch open issues in
    /// repositories owned by any owners listed in `owners`
    pub(crate) fn new(owners: Vec<String>, limits: QueryLimits) -> OrgsWithIssues {
        OrgsWithIssues {
            state: Start { owners }.into(),
            shared: Shared {
                limits,
                results: Vec::new(),
                report: FetchReport::default(),
            },
        }
    }
}

impl QueryMachine for OrgsWithIssues {
    type Output = Output;

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
        }
        r
    }

    fn get_output(&mut self) -> Vec<Self::Output> {
        self.shared.results.drain(..).collect()
    }
}

/// Data on the `OrgsWithIssues` instance that is accessed by the methods of
/// the individual states
#[derive(Clone, Debug, Eq, PartialEq)]
struct Shared {
    /// Batching & pagination limits to obey when forming queries
    limits: QueryLimits,

    /// Output produced by the `QueryMachine` that has not yet been returned by
    /// a call to `get_output()`
    results: Vec<Output>,

    /// Information on how many things were retrieved from the server
    report: FetchReport,
}

impl Shared {
    /// Append a [`Transition`] instance to `results`
    fn transition(&mut self, t: Transition) {
        self.results.push(Output::Transition(t));
    }
}

/// Behavior common to all states of the `OrgsWithIssues` state machine
#[enum_dispatch::enum_dispatch]
trait MachineState {
    /// Obtain the next query, if any, to perform against the GraphQL server
    /// along with the updated internal state (which may or may not be the same
    /// type as before)
    fn get_next_query(self, shared: &mut Shared) -> (State, Option<QueryPayload>);

    /// Provide the state with the deserialized value of the `"data"` field
    /// from a successful response to the query returned by the most recent
    /// call to some state's `get_next_query()`
    fn handle_response(
        &mut self,
        data: JsonMap,
        shared: &mut Shared,
    ) -> Result<(), serde_json::Error>;
}

/// Internal states of the `OrgsWithIssues` state machine
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
    fn get_next_query(self, shared: &mut Shared) -> (State, Option<QueryPayload>) {
        FetchRepos::start(self.owners, shared)
    }

    fn handle_response(
        &mut self,
        _data: JsonMap,
        _shared: &mut Shared,
    ) -> Result<(), serde_json::Error> {
        panic!("handle_response() called before get_next_query()")
    }
}

/// State when issuing queries for repositories owned by the given repository
/// owners and the first page of each repository's open issues
///
/// Once all repositories are retrieved, the next call to `get_next_query()`
/// will result in a transition to [`FetchIssues`], unless no repositories had
/// any additional pages of open issues, in which case we transition to
/// [`FetchLabels`], unless no issues had any additional pages of labels, in
/// which case we transition to [`Done`].
#[derive(Clone, Debug, Eq, PartialEq)]
struct FetchRepos {
    /// An inner `QueryMachine` for performing the queries for this state
    submachine: BatchPaginator<String, GetOwnerRepos>,

    /// The time at which this state began
    start: Instant,

    /// A list of (repository ID, [`GetIssues`] paginator) pairs for any
    /// repositories fetched so far that have more than one page of open issues
    issue_queries: Vec<(Id, GetIssues)>,

    /// A list of (issue ID, [`GetLabels`] paginator) pairs for any issues
    /// fetched so far that have more than one page of labels
    label_queries: Vec<(Id, GetLabels)>,

    /// A collection of all issues fetched so far that have more than one page
    /// of labels, keyed by issue ID
    issues_needing_labels: HashMap<Id, Issue>,
}

impl FetchRepos {
    /// Transition into this state and return the state's first query — unless
    /// `owners` is empty, in which case the state transitions to [`Done`]
    /// instead and no query is returned
    fn start(owners: Vec<String>, shared: &mut Shared) -> (State, Option<QueryPayload>) {
        let mut submachine = BatchPaginator::new(
            owners.into_iter().map(|owner| {
                (
                    owner.clone(),
                    GetOwnerRepos::new(
                        owner,
                        shared.limits.page_size,
                        shared.limits.label_page_size,
                    ),
                )
            }),
            shared.limits.batch_size,
        );
        if let query @ Some(_) = submachine.get_next_query() {
            shared.transition(Transition::StartFetchRepos);
            let state = FetchRepos {
                submachine,
                start: Instant::now(),
                issue_queries: Vec::new(),
                label_queries: Vec::new(),
                issues_needing_labels: HashMap::new(),
            }
            .into();
            (state, query)
        } else {
            Done::start(shared)
        }
    }
}

impl MachineState for FetchRepos {
    fn get_next_query(mut self, shared: &mut Shared) -> (State, Option<QueryPayload>) {
        if let query @ Some(_) = self.submachine.get_next_query() {
            (self.into(), query)
        } else {
            shared.transition(Transition::EndFetchRepos {
                repositories: shared.report.repositories,
                repos_with_open_issues: shared.report.repos_with_open_issues,
                open_issues: shared.report.open_issues,
                elapsed: self.start.elapsed(),
            });
            FetchIssues::start(
                self.issue_queries,
                self.label_queries,
                self.issues_needing_labels,
                shared,
            )
        }
    }

    fn handle_response(
        &mut self,
        data: JsonMap,
        shared: &mut Shared,
    ) -> Result<(), serde_json::Error> {
        self.submachine.handle_response(data)?;
        let mut issues_out = Vec::new();
        for repo in self
            .submachine
            .get_output()
            .into_iter()
            .flat_map(|pr| pr.items)
        {
            shared.report.repositories += 1;
            if let Some(q) =
                repo.more_issues_query(shared.limits.page_size, shared.limits.label_page_size)
            {
                shared.report.repos_with_extra_issues += 1;
                self.issue_queries.push(q);
            }
            if !repo.issues.is_empty() {
                shared.report.repos_with_open_issues += 1;
                for iwl in repo.issues {
                    shared.report.open_issues += 1;
                    if let Some(q) = iwl.more_labels_query(shared.limits.label_page_size) {
                        shared.report.issues_with_extra_labels += 1;
                        self.label_queries.push(q);
                        self.issues_needing_labels.insert(iwl.issue_id, iwl.issue);
                    } else {
                        issues_out.push(iwl.issue);
                    }
                }
            }
        }
        if !issues_out.is_empty() {
            shared.results.push(Output::Issues(issues_out));
        }
        Ok(())
    }
}

/// State when issuing queries for additional pages of open issues in
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

    /// The time at which this state began
    start: Instant,

    /// A list of (issue ID, [`GetLabels`] paginator) pairs for any issues
    /// fetched so far that have more than one page of labels
    label_queries: Vec<(Id, GetLabels)>,

    /// A collection of all issues fetched so far that have more than one page
    /// of labels, keyed by issue ID
    issues_needing_labels: HashMap<Id, Issue>,
}

impl FetchIssues {
    /// Transition into this state and return the state's first query — unless
    /// `issue_queries` is empty, in which case the state transitions to
    /// [`Done`] instead and no query is returned.
    ///
    /// `issue_queries` is a list of (repository ID, [`GetIssues`] paginator)
    /// pairs corresponding to repositories that have more than one page of
    /// open issues.
    ///
    /// `label_queries` is a list of (issue ID, [`GetLabels`] paginator) pairs
    /// for any issues fetched so far that have more than one page of labels
    ///
    /// `issues_needing_labels` is a collection of all issues fetched so far
    /// that have more than one page of labels, keyed by issue ID
    fn start(
        issue_queries: Vec<(Id, GetIssues)>,
        label_queries: Vec<(Id, GetLabels)>,
        issues_needing_labels: HashMap<Id, Issue>,
        shared: &mut Shared,
    ) -> (State, Option<QueryPayload>) {
        let mut submachine = BatchPaginator::new(issue_queries, shared.limits.batch_size);
        if let query @ Some(_) = submachine.get_next_query() {
            shared.transition(Transition::StartFetchIssues {
                repos_with_extra_issues: shared.report.repos_with_extra_issues,
            });
            let state = FetchIssues {
                submachine,
                start: Instant::now(),
                label_queries,
                issues_needing_labels,
            }
            .into();
            (state, query)
        } else {
            FetchLabels::start(label_queries, issues_needing_labels, shared)
        }
    }
}

impl MachineState for FetchIssues {
    fn get_next_query(mut self, shared: &mut Shared) -> (State, Option<QueryPayload>) {
        if let query @ Some(_) = self.submachine.get_next_query() {
            (self.into(), query)
        } else {
            shared.transition(Transition::EndFetchIssues {
                extra_issues: shared.report.extra_issues,
                elapsed: self.start.elapsed(),
            });
            FetchLabels::start(self.label_queries, self.issues_needing_labels, shared)
        }
    }

    fn handle_response(
        &mut self,
        data: JsonMap,
        shared: &mut Shared,
    ) -> Result<(), serde_json::Error> {
        self.submachine.handle_response(data)?;
        let mut issues_out = Vec::new();
        for iwl in self
            .submachine
            .get_output()
            .into_iter()
            .flat_map(|pr| pr.items)
        {
            shared.report.open_issues += 1;
            shared.report.extra_issues += 1;
            if let Some(q) = iwl.more_labels_query(shared.limits.label_page_size) {
                shared.report.issues_with_extra_labels += 1;
                self.label_queries.push(q);
                self.issues_needing_labels.insert(iwl.issue_id, iwl.issue);
            } else {
                issues_out.push(iwl.issue);
            }
        }
        if !issues_out.is_empty() {
            shared.results.push(Output::Issues(issues_out));
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

    /// The time at which this state began
    start: Instant,

    /// A collection of all issues fetched that have more than one page of
    /// labels, keyed by issue ID
    issues_needing_labels: HashMap<Id, Issue>,
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
    fn start(
        label_queries: Vec<(Id, GetLabels)>,
        issues_needing_labels: HashMap<Id, Issue>,
        shared: &mut Shared,
    ) -> (State, Option<QueryPayload>) {
        let mut submachine = BatchPaginator::new(label_queries, shared.limits.batch_size);
        if let query @ Some(_) = submachine.get_next_query() {
            shared.transition(Transition::StartFetchLabels {
                issues_with_extra_labels: shared.report.issues_with_extra_labels,
            });
            let state = FetchLabels {
                submachine,
                start: Instant::now(),
                issues_needing_labels,
            }
            .into();
            (state, query)
        } else {
            Done::start(shared)
        }
    }
}

impl MachineState for FetchLabels {
    fn get_next_query(mut self, shared: &mut Shared) -> (State, Option<QueryPayload>) {
        if let query @ Some(_) = self.submachine.get_next_query() {
            (self.into(), query)
        } else {
            shared.transition(Transition::EndFetchLabels {
                extra_labels: shared.report.extra_labels,
                elapsed: self.start.elapsed(),
            });
            shared.results.push(Output::Issues(
                self.issues_needing_labels.into_values().collect(),
            ));
            Done::start(shared)
        }
    }

    fn handle_response(
        &mut self,
        data: JsonMap,
        shared: &mut Shared,
    ) -> Result<(), serde_json::Error> {
        self.submachine.handle_response(data)?;
        for res in self.submachine.get_output() {
            shared.report.extra_labels += res.items.len();
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
    /// Transition to this state and push the [`FetchReport`] into
    /// `shared.results`
    fn start(shared: &mut Shared) -> (State, Option<QueryPayload>) {
        shared.results.push(Output::Report(shared.report));
        (Done.into(), None)
    }
}

impl MachineState for Done {
    fn get_next_query(self, _shared: &mut Shared) -> (State, Option<QueryPayload>) {
        (self.into(), None)
    }

    fn handle_response(
        &mut self,
        _data: JsonMap,
        _shared: &mut Shared,
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
    fn get_next_query(self, _shared: &mut Shared) -> (State, Option<QueryPayload>) {
        panic!("get_next_query() called after machine errored")
    }

    fn handle_response(
        &mut self,
        _data: JsonMap,
        _shared: &mut Shared,
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

/// Output values produced by `OrgsWithIssues`
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Output {
    /// An announcement of a state transition, marking the start or end of a
    /// state
    Transition(Transition),

    /// A collection of issues whose data has been fully fetched
    Issues(Vec<Issue>),

    /// A report on how many things were retrieved from the server
    Report(FetchReport),
}

/// Information on how many of each type of thing were retrieved from the
/// server
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct FetchReport {
    /// Total number of repositories retrieved
    repositories: usize,

    /// Total number of open issues retrieved
    open_issues: usize,

    /// Number of repositories retrieved that had open issues
    repos_with_open_issues: usize,

    /// Number of repositories retrieved that required extra queries to fetch
    /// all issues
    repos_with_extra_issues: usize,

    /// Number of issues retrieved that required extra queries to fetch all
    /// labels
    issues_with_extra_labels: usize,

    /// Total number of issues that required extra queries to retrieve
    extra_issues: usize,

    /// Total number of labels that required extra queries to retrieve
    extra_labels: usize,
}

/// An announcement of a state transition, marking the start or end of a state
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Transition {
    /// The [`FetchRepos`] state has started
    StartFetchRepos,

    /// The [`FetchRepos`] state has finished
    EndFetchRepos {
        /// Total number of repositories retrieved
        repositories: usize,

        /// Number of repositories with open issues retrieved
        repos_with_open_issues: usize,

        /// Number of open issues retrieved
        open_issues: usize,

        /// Time elapsed since the start of the [`FetchRepos`] state
        elapsed: Duration,
    },

    /// The [`FetchIssues`] state has started
    StartFetchIssues {
        /// Number of repositories retrieved that have more than one page of
        /// issues
        repos_with_extra_issues: usize,
    },

    /// The [`FetchIssues`] state has finished
    EndFetchIssues {
        /// Total number of issues that required extra queries to retrieve
        extra_issues: usize,

        /// Time elapsed since the start of the [`FetchIssues`] state
        elapsed: Duration,
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

        /// Time elapsed since the start of the [`FetchLabels`] state
        elapsed: Duration,
    },
}

impl fmt::Display for Transition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Transition::StartFetchRepos => write!(f, "Fetching repositories …"),
            Transition::EndFetchRepos {
                repositories,
                repos_with_open_issues,
                open_issues,
                elapsed,
            } => write!(
                f,
                "Fetched {repositories} repositories ({repos_with_open_issues} with open issues; {open_issues} open issues in total) in {elapsed:?}"
            ),
            Transition::StartFetchIssues {
                repos_with_extra_issues,
            } => {
                write!(
                    f,
                    "Fetching more issues for {repos_with_extra_issues} repositories …"
                )
            }
            Transition::EndFetchIssues {
                extra_issues,
                elapsed,
            } => write!(f, "Fetched {extra_issues} more issues in {elapsed:?}"),
            Transition::StartFetchLabels {
                issues_with_extra_labels,
            } => write!(
                f,
                "Fetching more labels for {issues_with_extra_labels} issues …"
            ),
            Transition::EndFetchLabels {
                extra_labels,
                elapsed,
            } => write!(f, "Fetched {extra_labels} more labels in {elapsed:?}"),
        }
    }
}

#[cfg(test)]
mod tests;
