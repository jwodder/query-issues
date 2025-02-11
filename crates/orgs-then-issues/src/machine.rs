use crate::queries::{GetIssues, GetLabels, GetOwnerRepos};
use crate::types::Issue;
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
    report: FetchReport,
}

impl OrgsThenIssues {
    pub(crate) fn new(owners: Vec<String>, parameters: Parameters) -> OrgsThenIssues {
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
            report: FetchReport::default(),
        }
    }

    fn done(&mut self) {
        self.results.push(Output::Report(self.report));
        self.state = State::Done;
    }
}

impl QueryMachine for OrgsThenIssues {
    type Output = Output;

    fn get_next_query(&mut self) -> Option<QueryPayload> {
        match &mut self.state {
            State::Start { submachine } => {
                let query = submachine.get_next_query();
                if query.is_some() {
                    self.state = State::FetchRepos {
                        submachine: std::mem::take(submachine),
                        issue_queries: Vec::new(),
                        start: Instant::now(),
                    };
                    self.results
                        .push(Output::Transition(Transition::StartFetchRepos));
                } else {
                    self.done();
                }
                query
            }
            State::FetchRepos {
                submachine,
                issue_queries,
                start,
            } => {
                let query = submachine.get_next_query();
                if query.is_some() {
                    query
                } else {
                    self.results
                        .push(Output::Transition(Transition::EndFetchRepos {
                            repositories: self.report.repositories,
                            repos_with_open_issues: self.report.repos_with_open_issues,
                            elapsed: start.elapsed(),
                        }));
                    let mut submachine = BatchPaginator::new(
                        std::mem::take(issue_queries),
                        self.parameters.batch_size,
                    );
                    let query = submachine.get_next_query();
                    if query.is_some() {
                        self.results
                            .push(Output::Transition(Transition::StartFetchIssues {
                                repos_with_open_issues: self.report.repos_with_open_issues,
                            }));
                        self.state = State::FetchIssues {
                            submachine,
                            start: Instant::now(),
                            issues_needing_labels: HashMap::new(),
                            label_queries: Vec::new(),
                        };
                    } else {
                        self.done();
                    }
                    query
                }
            }
            State::FetchIssues {
                submachine,
                start,
                label_queries,
                issues_needing_labels,
            } => {
                let query = submachine.get_next_query();
                if query.is_some() {
                    query
                } else {
                    self.results
                        .push(Output::Transition(Transition::EndFetchIssues {
                            open_issues: self.report.open_issues,
                            elapsed: start.elapsed(),
                        }));
                    let mut submachine = BatchPaginator::new(
                        std::mem::take(label_queries),
                        self.parameters.batch_size,
                    );
                    let query = submachine.get_next_query();
                    if query.is_some() {
                        self.results
                            .push(Output::Transition(Transition::StartFetchLabels {
                                issues_with_extra_labels: self.report.issues_with_extra_labels,
                            }));
                        self.state = State::FetchLabels {
                            submachine,
                            start: Instant::now(),
                            issues_needing_labels: std::mem::take(issues_needing_labels),
                        };
                    } else {
                        debug_assert!(
                            issues_needing_labels.is_empty(),
                            "no label queries to run, but `issues_needing_labels` is nonempty"
                        );
                        self.done();
                    }
                    query
                }
            }
            State::FetchLabels {
                submachine,
                start,
                issues_needing_labels,
            } => {
                let query = submachine.get_next_query();
                if query.is_some() {
                    query
                } else {
                    self.results
                        .push(Output::Transition(Transition::EndFetchLabels {
                            extra_labels: self.report.extra_labels,
                            elapsed: start.elapsed(),
                        }));
                    self.results.push(Output::Issues(
                        std::mem::take(issues_needing_labels)
                            .into_values()
                            .collect(),
                    ));
                    self.done();
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
                submachine,
                issue_queries,
                ..
            } => {
                submachine.handle_response(data)?;
                for Ided { id, data: repo } in
                    submachine.get_output().into_iter().flat_map(|pr| pr.items)
                {
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
            }
            State::FetchIssues {
                submachine,
                label_queries,
                issues_needing_labels,
                ..
            } => {
                submachine.handle_response(data)?;
                let mut issues_out = Vec::new();
                for iwl in submachine.get_output().into_iter().flat_map(|pr| pr.items) {
                    self.report.open_issues += 1;
                    if let Some(q) = iwl.more_labels_query(self.parameters.label_page_size) {
                        self.report.issues_with_extra_labels += 1;
                        label_queries.push(q);
                        issues_needing_labels.insert(iwl.issue_id, iwl.issue);
                    } else {
                        issues_out.push(iwl.issue);
                    }
                }
                if !issues_out.is_empty() {
                    self.results.push(Output::Issues(issues_out));
                }
            }
            State::FetchLabels {
                submachine,
                issues_needing_labels,
                ..
            } => {
                submachine.handle_response(data)?;
                for res in submachine.get_output() {
                    self.report.extra_labels += res.items.len();
                    issues_needing_labels
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
        issue_queries: Vec<(Id, GetIssues)>,
        start: Instant,
    },
    FetchIssues {
        submachine: BatchPaginator<Id, GetIssues>,
        start: Instant,
        issues_needing_labels: HashMap<Id, Issue>,
        label_queries: Vec<(Id, GetLabels)>,
    },
    FetchLabels {
        submachine: BatchPaginator<Id, GetLabels>,
        start: Instant,
        issues_needing_labels: HashMap<Id, Issue>,
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
    Report(FetchReport),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct FetchReport {
    repositories: usize,
    open_issues: usize,
    repos_with_open_issues: usize,
    issues_with_extra_labels: usize,
    extra_labels: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Transition {
    StartFetchRepos,
    EndFetchRepos {
        repositories: usize,
        repos_with_open_issues: usize,
        elapsed: Duration,
    },
    StartFetchIssues {
        repos_with_open_issues: usize,
    },
    EndFetchIssues {
        open_issues: usize,
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
            Transition::EndFetchRepos { repositories, repos_with_open_issues, elapsed } => write!(f, "Fetched {repositories} repositories ({repos_with_open_issues} with open issues) in {elapsed:?}"),
            Transition::StartFetchIssues { repos_with_open_issues } => {
                write!(f, "Fetching issues for {repos_with_open_issues} repositories …")
            }
            Transition::EndFetchIssues { open_issues, elapsed } => write!(f, "Fetched {open_issues} issues in {elapsed:?}"),
            Transition::StartFetchLabels { issues_with_extra_labels } => write!(f, "Fetching more labels for {issues_with_extra_labels} issues …"),
            Transition::EndFetchLabels { extra_labels, elapsed } => write!(f, "Fetched {extra_labels} more labels in {elapsed:?}"),
        }
    }
}
