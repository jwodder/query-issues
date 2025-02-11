use crate::db::{Database, IssueDiff, RepoDiff};
use crate::queries::{GetIssues, GetLabels, GetOwnerRepos};
use crate::types::{Issue, RepoDetails};
use gqlient::{BatchPaginator, Id, Ided, JsonMap, PaginationResults, QueryMachine, QueryPayload};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

pub(crate) struct UpdateIssues<'a> {
    db: &'a mut Database,
    parameters: Parameters,
    state: State,
    results: Vec<Output>,
    report: FetchReport,
    // The first Id is the issue ID; the second Id is the ID of the repo the
    // issue belongs to.
    issues: HashMap<Id, (Id, Issue)>,
}

impl<'a> UpdateIssues<'a> {
    pub(crate) fn new(db: &'a mut Database, owners: Vec<String>, parameters: Parameters) -> Self {
        let submachine = BatchPaginator::new(
            owners.into_iter().map(|owner| {
                (
                    owner.clone(),
                    GetOwnerRepos::new(owner, parameters.page_size),
                )
            }),
            parameters.batch_size,
        );
        UpdateIssues {
            db,
            parameters,
            state: State::Start { submachine },
            results: Vec::new(),
            report: FetchReport::default(),
            issues: HashMap::new(),
        }
    }

    fn done(&mut self) {
        let mut idiff = IssueDiff::default();
        for (issue_id, (repo_id, issue)) in std::mem::take(&mut self.issues) {
            let Some(repo) = self.db.get_mut(&repo_id) else {
                // TODO: Warn? Error?
                continue;
            };
            idiff += repo.update_issue(issue_id, issue);
        }
        self.results.push(Output::IssueDiff(idiff));
        self.results.push(Output::Report(self.report));
        self.state = State::Done;
    }
}

impl QueryMachine for UpdateIssues<'_> {
    type Output = Output;

    fn get_next_query(&mut self) -> Option<QueryPayload> {
        match &mut self.state {
            State::Start { submachine } => {
                let query = submachine.get_next_query();
                if query.is_some() {
                    self.state = State::FetchRepos {
                        submachine: std::mem::take(submachine),
                        repos: Vec::new(),
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
                repos,
                start,
            } => {
                let query = submachine.get_next_query();
                if query.is_some() {
                    query
                } else {
                    self.report.repositories = repos.len();
                    self.results
                        .push(Output::Transition(Transition::EndFetchRepos {
                            repo_qty: self.report.repositories,
                            elapsed: start.elapsed(),
                        }));
                    let rdiff = self.db.update_repositories(std::mem::take(repos));
                    self.results.push(Output::RepoDiff(rdiff));
                    let paginators = self.db.issue_paginators(
                        self.parameters.page_size,
                        self.parameters.label_page_size,
                    );
                    self.report.repos_with_open_issues = paginators.len();
                    let mut submachine =
                        BatchPaginator::new(paginators, self.parameters.batch_size);
                    let query = submachine.get_next_query();
                    if query.is_some() {
                        self.results
                            .push(Output::Transition(Transition::StartFetchIssues));
                        self.state = State::FetchIssues {
                            submachine,
                            start: Instant::now(),
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
            } => {
                let query = submachine.get_next_query();
                if query.is_some() {
                    query
                } else {
                    self.results
                        .push(Output::Transition(Transition::EndFetchIssues {
                            issue_qty: self.report.open_issues,
                            repo_qty: self.report.repos_with_open_issues,
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
                        };
                    } else {
                        self.done();
                    }
                    query
                }
            }
            State::FetchLabels { submachine, start } => {
                let query = submachine.get_next_query();
                if query.is_some() {
                    query
                } else {
                    self.results
                        .push(Output::Transition(Transition::EndFetchLabels {
                            label_qty: self.report.extra_labels,
                            elapsed: start.elapsed(),
                        }));
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
                submachine, repos, ..
            } => {
                submachine.handle_response(data)?;
                repos.extend(submachine.get_output().into_iter().flat_map(|pr| pr.items));
            }
            State::FetchIssues {
                submachine,
                label_queries,
                ..
            } => {
                submachine.handle_response(data)?;
                for PaginationResults {
                    key: repo_id,
                    items,
                    end_cursor,
                } in submachine.get_output()
                {
                    let Some(repo) = self.db.get_mut(&repo_id) else {
                        // TODO: Warn? Error?
                        continue;
                    };
                    repo.set_issue_cursor(end_cursor);
                    for iwl in items {
                        self.report.open_issues += 1;
                        if let Some(q) = iwl.more_labels_query(self.parameters.label_page_size) {
                            self.report.issues_with_extra_labels += 1;
                            label_queries.push(q);
                        }
                        self.issues
                            .insert(iwl.issue_id, (repo_id.clone(), iwl.issue));
                    }
                }
            }
            State::FetchLabels { submachine, .. } => {
                submachine.handle_response(data)?;
                for res in submachine.get_output() {
                    self.report.extra_labels += res.items.len();
                    self.issues
                        .get_mut(&res.key)
                        .expect("Issues we get labels for should have already been seen")
                        .1
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
        start: Instant,
        repos: Vec<Ided<RepoDetails>>,
    },
    FetchIssues {
        submachine: BatchPaginator<Id, GetIssues>,
        start: Instant,
        label_queries: Vec<(Id, GetLabels)>,
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
    Report(FetchReport),
    RepoDiff(RepoDiff),
    IssueDiff(IssueDiff),
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
        repo_qty: usize,
        elapsed: Duration,
    },
    StartFetchIssues,
    EndFetchIssues {
        issue_qty: usize,
        repo_qty: usize,
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
            Transition::EndFetchRepos { repo_qty, elapsed } => {
                write!(f, "Fetched {repo_qty} repositories in {elapsed:?}")
            }
            Transition::StartFetchIssues => write!(f, "Fetching issues …"),
            Transition::EndFetchIssues {
                issue_qty,
                repo_qty,
                elapsed,
            } => {
                write!(
                    f,
                    "Fetched {issue_qty} issues from {repo_qty} repositories in {elapsed:?}"
                )
            }
            Transition::StartFetchLabels {
                issues_with_extra_labels,
            } => write!(
                f,
                "Fetching more labels for {issues_with_extra_labels} issues …"
            ),
            Transition::EndFetchLabels { label_qty, elapsed } => {
                write!(f, "Fetched {label_qty} more labels in {elapsed:?}")
            }
        }
    }
}
