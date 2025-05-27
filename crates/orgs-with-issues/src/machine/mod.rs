use crate::queries::{GetIssues, GetLabels, GetOwnerRepos};
use crate::types::Issue;
use gqlient::{BatchPaginator, Id, JsonMap, QueryMachine, QueryPayload};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OrgsWithIssues {
    state: State,
    shared: Shared,
}

impl OrgsWithIssues {
    pub(crate) fn new(owners: Vec<String>, parameters: Parameters) -> OrgsWithIssues {
        OrgsWithIssues {
            state: Start { owners }.into(),
            shared: Shared {
                parameters,
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct Shared {
    parameters: Parameters,
    results: Vec<Output>,
    report: FetchReport,
}

impl Shared {
    fn transition(&mut self, t: Transition) {
        self.results.push(Output::Transition(t));
    }
}

#[enum_dispatch::enum_dispatch]
trait MachineState {
    fn get_next_query(self, shared: &mut Shared) -> (State, Option<QueryPayload>);
    fn handle_response(
        &mut self,
        data: JsonMap,
        shared: &mut Shared,
    ) -> Result<(), serde_json::Error>;
}

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

#[derive(Clone, Debug, Eq, PartialEq)]
struct Start {
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct FetchRepos {
    submachine: BatchPaginator<String, GetOwnerRepos>,
    start: Instant,
    issue_queries: Vec<(Id, GetIssues)>,
    label_queries: Vec<(Id, GetLabels)>,
    issues_needing_labels: HashMap<Id, Issue>,
}

impl FetchRepos {
    fn start(owners: Vec<String>, shared: &mut Shared) -> (State, Option<QueryPayload>) {
        let mut submachine = BatchPaginator::new(
            owners.into_iter().map(|owner| {
                (
                    owner.clone(),
                    GetOwnerRepos::new(
                        owner,
                        shared.parameters.page_size,
                        shared.parameters.label_page_size,
                    ),
                )
            }),
            shared.parameters.batch_size,
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
            if let Some(q) = repo.more_issues_query(
                shared.parameters.page_size,
                shared.parameters.label_page_size,
            ) {
                shared.report.repos_with_extra_issues += 1;
                self.issue_queries.push(q);
            }
            if !repo.issues.is_empty() {
                shared.report.repos_with_open_issues += 1;
                for iwl in repo.issues {
                    shared.report.open_issues += 1;
                    if let Some(q) = iwl.more_labels_query(shared.parameters.label_page_size) {
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct FetchIssues {
    submachine: BatchPaginator<Id, GetIssues>,
    start: Instant,
    label_queries: Vec<(Id, GetLabels)>,
    issues_needing_labels: HashMap<Id, Issue>,
}

impl FetchIssues {
    fn start(
        issue_queries: Vec<(Id, GetIssues)>,
        label_queries: Vec<(Id, GetLabels)>,
        issues_needing_labels: HashMap<Id, Issue>,
        shared: &mut Shared,
    ) -> (State, Option<QueryPayload>) {
        let mut submachine = BatchPaginator::new(issue_queries, shared.parameters.batch_size);
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
            if let Some(q) = iwl.more_labels_query(shared.parameters.label_page_size) {
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct FetchLabels {
    submachine: BatchPaginator<Id, GetLabels>,
    start: Instant,
    issues_needing_labels: HashMap<Id, Issue>,
}

impl FetchLabels {
    fn start(
        label_queries: Vec<(Id, GetLabels)>,
        issues_needing_labels: HashMap<Id, Issue>,
        shared: &mut Shared,
    ) -> (State, Option<QueryPayload>) {
        let mut submachine = BatchPaginator::new(label_queries, shared.parameters.batch_size);
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct Done;

impl Done {
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[allow(clippy::struct_field_names)]
pub(crate) struct Parameters {
    pub(crate) batch_size: NonZeroUsize,
    pub(crate) page_size: NonZeroUsize,
    pub(crate) label_page_size: NonZeroUsize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Output {
    Transition(Transition),
    Issues(Vec<Issue>),
    Report(FetchReport),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct FetchReport {
    repositories: usize,
    open_issues: usize,
    repos_with_open_issues: usize,
    repos_with_extra_issues: usize,
    issues_with_extra_labels: usize,
    extra_issues: usize,
    extra_labels: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Transition {
    StartFetchRepos,
    EndFetchRepos {
        repositories: usize,
        repos_with_open_issues: usize,
        open_issues: usize,
        elapsed: Duration,
    },
    StartFetchIssues {
        repos_with_extra_issues: usize,
    },
    EndFetchIssues {
        extra_issues: usize,
        elapsed: Duration,
    },
    StartFetchLabels {
        issues_with_extra_labels: usize,
    },
    EndFetchLabels {
        extra_labels: usize,
        elapsed: Duration,
    },
}

impl fmt::Display for Transition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Transition::StartFetchRepos => write!(f, "Fetching repositories …"),
            Transition::EndFetchRepos { repositories, repos_with_open_issues, open_issues, elapsed } => write!(f, "Fetched {repositories} repositories ({repos_with_open_issues} with open issues; {open_issues} open issues in total) in {elapsed:?}"),
            Transition::StartFetchIssues { repos_with_extra_issues } => {
                write!(f, "Fetching more issues for {repos_with_extra_issues} repositories …")
            }
            Transition::EndFetchIssues { extra_issues, elapsed } => write!(f, "Fetched {extra_issues} more issues in {elapsed:?}"),
            Transition::StartFetchLabels { issues_with_extra_labels } => write!(f, "Fetching more labels for {issues_with_extra_labels} issues …"),
            Transition::EndFetchLabels { extra_labels, elapsed } => write!(f, "Fetched {extra_labels} more labels in {elapsed:?}"),
        }
    }
}

#[cfg(test)]
mod tests;
