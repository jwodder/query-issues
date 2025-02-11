use crate::queries::{GetIssues, GetLabels, GetOwnerRepos};
use crate::types::{Issue, Repository};
use gqlient::{BatchPaginator, Id, Ided, JsonMap, QueryMachine, QueryPayload};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

pub(crate) struct OrgsThenIssues {
    parameters: Parameters,
    state: State,
    results: Vec<Output>,
    report: MachineReport,
}

impl OrgsThenIssues {
    pub(crate) fn new(owners: Vec<String>, parameters: Parameters) -> OrgsThenIssues {
        // TODO: If owners is empty, go straight to Done
        let submachine = BatchPaginator::new(
            owners.into_iter().map(|owner| {
                (
                    owner.clone(),
                    GetOwnerRepos::new(owner, parameters.page_size),
                )
            }),
            parameters.batch_size,
        );
        OrgsThenIssues {
            parameters,
            state: State::Start { submachine },
            results: Vec::new(),
            report: MachineReport::default(),
        }
    }
}

impl QueryMachine for OrgsThenIssues {
    type Output = Output;

    fn get_next_query(&mut self) -> Option<QueryPayload> {
        match &mut self.state {
            State::Start { submachine } => {
                let query = submachine.get_next_query();
                let start = Instant::now();
                self.state = State::FetchRepos {
                    submachine: std::mem::take(submachine),
                    repos: Vec::new(),
                    start,
                };
                self.results
                    .push(Output::Transition(Transition::StartFetchRepos));
                query
            }
            State::FetchRepos {
                submachine,
                repos,
                start,
            } => {
                let query = submachine.get_next_query();
                if query.is_some() {
                    query
                } else {
                    let elapsed = start.elapsed();
                    let mut issue_queries = Vec::new();
                    for Ided { id, data: repo } in std::mem::take(repos) {
                        self.report.repositories += 1;
                        if repo.open_issues > 0 {
                            self.report.repos_with_open_issues += 1;
                            issue_queries.push((
                                id.clone(),
                                GetIssues::new(
                                    id,
                                    self.parameters.page_size,
                                    self.parameters.label_page_size,
                                ),
                            ));
                        }
                    }
                    self.results
                        .push(Output::Transition(Transition::EndFetchRepos {
                            repo_qty: self.report.repositories,
                            repos_with_issues_qty: self.report.repos_with_open_issues,
                            elapsed,
                        }));
                    let mut submachine =
                        BatchPaginator::new(issue_queries, self.parameters.batch_size);
                    let query = submachine.get_next_query();
                    if query.is_some() {
                        self.results
                            .push(Output::Transition(Transition::StartFetchIssues {
                                repo_qty: self.report.repos_with_open_issues,
                            }));
                        self.state = State::FetchIssues {
                            submachine,
                            start: Instant::now(),
                            issues: HashMap::new(),
                            label_queries: Vec::new(),
                        };
                    } else {
                        self.results.push(Output::Report(self.report));
                        self.state = State::Done;
                    }
                    query
                }
            }
            State::FetchIssues {
                submachine,
                start,
                label_queries,
                issues,
            } => {
                let query = submachine.get_next_query();
                if query.is_some() {
                    query
                } else {
                    let elapsed = start.elapsed();
                    self.results
                        .push(Output::Transition(Transition::EndFetchIssues {
                            issue_qty: self.report.open_issues,
                            elapsed,
                        }));
                    if !label_queries.is_empty() {
                        self.report.issues_with_extra_labels = label_queries.len();
                        self.results
                            .push(Output::Transition(Transition::StartFetchLabels {
                                issues_with_extra_labels: self.report.issues_with_extra_labels,
                            }));
                        let start = Instant::now();
                        let mut submachine = BatchPaginator::new(
                            std::mem::take(label_queries),
                            self.parameters.batch_size,
                        );
                        let query = submachine.get_next_query();
                        self.state = State::FetchLabels {
                            submachine,
                            start,
                            issues: std::mem::take(issues),
                            label_qty: 0,
                        };
                        query
                    } else {
                        self.results.push(Output::Issues(
                            std::mem::take(issues).into_values().collect(),
                        ));
                        self.results.push(Output::Report(self.report));
                        self.state = State::Done;
                        None
                    }
                }
            }
            State::FetchLabels {
                submachine,
                start,
                issues,
                label_qty,
            } => {
                let query = submachine.get_next_query();
                if query.is_some() {
                    query
                } else {
                    let elapsed = start.elapsed();
                    self.results
                        .push(Output::Transition(Transition::EndFetchLabels {
                            label_qty: *label_qty,
                            elapsed,
                        }));
                    self.results.push(Output::Issues(
                        std::mem::take(issues).into_values().collect(),
                    ));
                    self.results.push(Output::Report(self.report));
                    self.state = State::Done;
                    None
                }
            }
            State::Done => None,
        }
    }

    fn handle_response(&mut self, data: JsonMap) -> Result<(), serde_json::Error> {
        match &mut self.state {
            State::Start { .. } => {
                panic!("handle_response() called before get_next_query()")
            }
            State::FetchRepos {
                submachine, repos, ..
            } => {
                submachine.handle_response(data)?;
                repos.extend(submachine.get_output().into_iter().flat_map(|pr| pr.items));
            }
            State::FetchIssues {
                submachine,
                label_queries,
                issues,
                ..
            } => {
                submachine.handle_response(data)?;
                for iwl in submachine.get_output().into_iter().flat_map(|pr| pr.items) {
                    label_queries.extend(iwl.more_labels_query(self.parameters.label_page_size));
                    self.report.open_issues += 1;
                    issues.insert(iwl.issue_id, iwl.issue);
                }
            }
            State::FetchLabels {
                submachine,
                issues,
                label_qty,
                ..
            } => {
                submachine.handle_response(data)?;
                for res in submachine.get_output() {
                    *label_qty += res.items.len();
                    issues
                        .get_mut(&res.key)
                        .expect("Issues we get labels for should have already been seen")
                        .labels
                        .extend(res.items);
                }
            }
            State::Done => (),
        }
        Ok(())
    }

    fn get_output(&mut self) -> Vec<Self::Output> {
        self.results.drain(..).collect()
    }
}

enum State {
    Start {
        submachine: BatchPaginator<String, GetOwnerRepos>,
    },
    FetchRepos {
        submachine: BatchPaginator<String, GetOwnerRepos>,
        repos: Vec<Ided<Repository>>,
        start: Instant,
    },
    FetchIssues {
        submachine: BatchPaginator<Id, GetIssues>,
        start: Instant,
        issues: HashMap<Id, Issue>,
        label_queries: Vec<(Id, GetLabels)>,
    },
    FetchLabels {
        submachine: BatchPaginator<Id, GetLabels>,
        start: Instant,
        issues: HashMap<Id, Issue>,
        label_qty: usize,
    },
    Done,
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
    Report(MachineReport),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct MachineReport {
    repositories: usize,
    open_issues: usize,
    repos_with_open_issues: usize,
    issues_with_extra_labels: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Transition {
    StartFetchRepos,
    EndFetchRepos {
        repo_qty: usize,
        repos_with_issues_qty: usize,
        elapsed: Duration,
    },
    StartFetchIssues {
        repo_qty: usize,
    },
    EndFetchIssues {
        issue_qty: usize,
        elapsed: Duration,
    },
    StartFetchLabels {
        issues_with_extra_labels: usize,
    },
    EndFetchLabels {
        label_qty: usize,
        elapsed: Duration,
    },
}

impl fmt::Display for Transition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Transition::StartFetchRepos => write!(f, "Fetching repositories …"),
            Transition::EndFetchRepos { repo_qty, repos_with_issues_qty, elapsed } => write!(f, "Fetched {repo_qty} repositories ({repos_with_issues_qty} with open issues) in {elapsed:?}"),
            Transition::StartFetchIssues { repo_qty } => {
                write!(f, "Fetching issues for {repo_qty} repositories …")
            }
            Transition::EndFetchIssues { issue_qty, elapsed } => write!(f, "Fetched {issue_qty} issues in {elapsed:?}"),
            Transition::StartFetchLabels { issues_with_extra_labels } => write!(f, "Fetching more labels for {issues_with_extra_labels} issues …"),
            Transition::EndFetchLabels { label_qty, elapsed } => write!(f, "Fetched {label_qty} more labels in {elapsed:?}"),
        }
    }
}
