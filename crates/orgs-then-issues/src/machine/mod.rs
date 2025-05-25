use crate::queries::{GetIssues, GetLabels, GetOwnerRepos};
use crate::types::Issue;
use gqlient::{BatchPaginator, Id, JsonMap, QueryMachine, QueryPayload};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OrgsThenIssues {
    parameters: Parameters,
    state: State,
    owners: Vec<String>,
    results: Vec<Output>,
    report: FetchReport,
    issue_queries: Vec<(Id, GetIssues)>,
    label_queries: Vec<(Id, GetLabels)>,
    issues_needing_labels: HashMap<Id, Issue>,
}

impl OrgsThenIssues {
    pub(crate) fn new(owners: Vec<String>, parameters: Parameters) -> OrgsThenIssues {
        OrgsThenIssues {
            parameters,
            state: State::Start,
            owners,
            results: Vec::new(),
            report: FetchReport::default(),
            issue_queries: Vec::new(),
            label_queries: Vec::new(),
            issues_needing_labels: HashMap::new(),
        }
    }

    fn start_fetch_repos(&mut self) -> Option<QueryPayload> {
        let mut submachine = BatchPaginator::new(
            std::mem::take(&mut self.owners).into_iter().map(|owner| {
                (
                    owner.clone(),
                    GetOwnerRepos::new(owner, self.parameters.page_size),
                )
            }),
            self.parameters.batch_size,
        );
        let query = submachine.get_next_query()?;
        self.results
            .push(Output::Transition(Transition::StartFetchRepos));
        self.state = State::FetchRepos {
            submachine,
            start: Instant::now(),
        };
        Some(query)
    }

    fn start_fetch_issues(&mut self) -> Option<QueryPayload> {
        let mut submachine = BatchPaginator::new(
            std::mem::take(&mut self.issue_queries),
            self.parameters.batch_size,
        );
        let query = submachine.get_next_query()?;
        self.results
            .push(Output::Transition(Transition::StartFetchIssues {
                repos_with_open_issues: self.report.repos_with_open_issues,
            }));
        self.state = State::FetchIssues {
            submachine,
            start: Instant::now(),
        };
        Some(query)
    }

    fn start_fetch_labels(&mut self) -> Option<QueryPayload> {
        let mut submachine = BatchPaginator::new(
            std::mem::take(&mut self.label_queries),
            self.parameters.batch_size,
        );
        let query = submachine.get_next_query()?;
        self.results
            .push(Output::Transition(Transition::StartFetchLabels {
                issues_with_extra_labels: self.report.issues_with_extra_labels,
            }));
        self.state = State::FetchLabels {
            submachine,
            start: Instant::now(),
        };
        Some(query)
    }

    fn done(&mut self) -> Option<QueryPayload> {
        self.results.push(Output::Report(self.report));
        self.state = State::Done;
        None
    }
}

impl QueryMachine for OrgsThenIssues {
    type Output = Output;

    fn get_next_query(&mut self) -> Option<QueryPayload> {
        match &mut self.state {
            State::Start => self.start_fetch_repos().or_else(|| self.done()),
            State::FetchRepos { submachine, start } => {
                if let query @ Some(_) = submachine.get_next_query() {
                    query
                } else {
                    self.results
                        .push(Output::Transition(Transition::EndFetchRepos {
                            repositories: self.report.repositories,
                            repos_with_open_issues: self.report.repos_with_open_issues,
                            elapsed: start.elapsed(),
                        }));
                    self.start_fetch_issues().or_else(|| self.done())
                }
            }
            State::FetchIssues { submachine, start } => {
                if let query @ Some(_) = submachine.get_next_query() {
                    query
                } else {
                    self.results
                        .push(Output::Transition(Transition::EndFetchIssues {
                            open_issues: self.report.open_issues,
                            elapsed: start.elapsed(),
                        }));
                    self.start_fetch_labels().or_else(|| self.done())
                }
            }
            State::FetchLabels { submachine, start } => {
                if let query @ Some(_) = submachine.get_next_query() {
                    query
                } else {
                    self.results
                        .push(Output::Transition(Transition::EndFetchLabels {
                            extra_labels: self.report.extra_labels,
                            elapsed: start.elapsed(),
                        }));
                    self.results.push(Output::Issues(
                        std::mem::take(&mut self.issues_needing_labels)
                            .into_values()
                            .collect(),
                    ));
                    self.done()
                }
            }
            State::Done => None,
        }
    }

    fn handle_response(&mut self, data: JsonMap) -> Result<(), serde_json::Error> {
        match &mut self.state {
            State::Start => {
                panic!("handle_response() called before get_next_query()")
            }
            State::FetchRepos { submachine, .. } => {
                submachine.handle_response(data)?;
                for repo in submachine.get_output().into_iter().flat_map(|pr| pr.items) {
                    self.report.repositories += 1;
                    if let Some(q) = repo
                        .issues_query(self.parameters.page_size, self.parameters.label_page_size)
                    {
                        self.report.repos_with_open_issues += 1;
                        self.issue_queries.push(q);
                    }
                }
            }
            State::FetchIssues { submachine, .. } => {
                submachine.handle_response(data)?;
                let mut issues_out = Vec::new();
                for iwl in submachine.get_output().into_iter().flat_map(|pr| pr.items) {
                    self.report.open_issues += 1;
                    if let Some(q) = iwl.more_labels_query(self.parameters.label_page_size) {
                        self.report.issues_with_extra_labels += 1;
                        self.label_queries.push(q);
                        self.issues_needing_labels.insert(iwl.issue_id, iwl.issue);
                    } else {
                        issues_out.push(iwl.issue);
                    }
                }
                if !issues_out.is_empty() {
                    self.results.push(Output::Issues(issues_out));
                }
            }
            State::FetchLabels { submachine, .. } => {
                submachine.handle_response(data)?;
                for res in submachine.get_output() {
                    self.report.extra_labels += res.items.len();
                    self.issues_needing_labels
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

#[derive(Clone, Debug, Eq, PartialEq)]
enum State {
    Start,
    FetchRepos {
        submachine: BatchPaginator<String, GetOwnerRepos>,
        start: Instant,
    },
    FetchIssues {
        submachine: BatchPaginator<Id, GetIssues>,
        start: Instant,
    },
    FetchLabels {
        submachine: BatchPaginator<Id, GetLabels>,
        start: Instant,
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

#[cfg(test)]
mod tests;
