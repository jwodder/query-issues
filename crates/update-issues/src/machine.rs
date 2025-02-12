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
    owners: Vec<String>,
    results: Vec<Output>,
    report: FetchReport,
    repos: Vec<Ided<RepoDetails>>,
    label_queries: Vec<(Id, GetLabels)>,
    // The first Id is the issue ID; the second Id is the ID of the repo the
    // issue belongs to.
    issues_needing_labels: HashMap<Id, (Id, Issue)>,
}

impl<'a> UpdateIssues<'a> {
    pub(crate) fn new(db: &'a mut Database, owners: Vec<String>, parameters: Parameters) -> Self {
        UpdateIssues {
            db,
            parameters,
            state: State::Start,
            owners,
            results: Vec::new(),
            report: FetchReport::default(),
            repos: Vec::new(),
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
        let paginators = self
            .db
            .issue_paginators(self.parameters.page_size, self.parameters.label_page_size);
        let mut submachine = BatchPaginator::new(paginators, self.parameters.batch_size);
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
        for (issue_id, (repo_id, issue)) in std::mem::take(&mut self.issues_needing_labels) {
            self.report.issue_diff += self.db.update_issue(repo_id, issue_id, issue);
        }
        self.results.push(Output::Report(self.report));
        self.state = State::Done;
        None
    }
}

impl QueryMachine for UpdateIssues<'_> {
    type Output = Output;

    fn get_next_query(&mut self) -> Option<QueryPayload> {
        match &mut self.state {
            State::Start => self.start_fetch_repos().or_else(|| self.done()),
            State::FetchRepos { submachine, start } => {
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
                    let rdiff = self.db.update_repositories(std::mem::take(&mut self.repos));
                    self.results.push(Output::RepoDiff(rdiff));
                    self.start_fetch_issues().or_else(|| self.done())
                }
            }
            State::FetchIssues { submachine, start } => {
                let query = submachine.get_next_query();
                if query.is_some() {
                    query
                } else {
                    self.results
                        .push(Output::Transition(Transition::EndFetchIssues {
                            issues: self.report.issues,
                            elapsed: start.elapsed(),
                        }));
                    self.start_fetch_labels().or_else(|| self.done())
                }
            }
            State::FetchLabels { submachine, start } => {
                let query = submachine.get_next_query();
                if query.is_some() {
                    query
                } else {
                    self.results
                        .push(Output::Transition(Transition::EndFetchLabels {
                            extra_labels: self.report.extra_labels,
                            elapsed: start.elapsed(),
                        }));
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
                self.repos.extend(
                    submachine
                        .get_output()
                        .into_iter()
                        .flat_map(|pr| pr.items)
                        .inspect(|repo| {
                            self.report.repositories += 1;
                            if repo.data.open_issues > 0 {
                                self.report.repos_with_open_issues += 1;
                            }
                        }),
                );
            }
            State::FetchIssues { submachine, .. } => {
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
                    if end_cursor.is_some() {
                        repo.set_issue_cursor(end_cursor);
                    }
                    for iwl in items {
                        self.report.issues += 1;
                        if let Some(q) = iwl.more_labels_query(self.parameters.label_page_size) {
                            self.report.issues_with_extra_labels += 1;
                            self.label_queries.push(q);
                            self.issues_needing_labels
                                .insert(iwl.issue_id, (repo_id.clone(), iwl.issue));
                        } else {
                            self.report.issue_diff +=
                                self.db
                                    .update_issue(repo_id.clone(), iwl.issue_id, iwl.issue);
                        }
                    }
                }
            }
            State::FetchLabels { submachine, .. } => {
                submachine.handle_response(data)?;
                for res in submachine.get_output() {
                    self.report.extra_labels += res.items.len();
                    self.issues_needing_labels
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
    Report(FetchReport),
    RepoDiff(RepoDiff),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct FetchReport {
    repositories: usize,
    repos_with_open_issues: usize,
    issues: usize,
    pub(crate) issue_diff: IssueDiff,
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
        issues: usize,
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
            Transition::EndFetchRepos {
                repositories,
                repos_with_open_issues,
                elapsed,
            } => {
                write!(f, "Fetched {repositories} repositories ({repos_with_open_issues} with open issues) in {elapsed:?}")
            }
            Transition::StartFetchIssues {
                repos_with_open_issues,
            } => write!(
                f,
                "Fetching new issues for {repos_with_open_issues} repositories …"
            ),
            Transition::EndFetchIssues { issues, elapsed } => {
                write!(f, "Fetched {issues} issues in {elapsed:?}")
            }
            Transition::StartFetchLabels {
                issues_with_extra_labels,
            } => write!(
                f,
                "Fetching more labels for {issues_with_extra_labels} issues …"
            ),
            Transition::EndFetchLabels {
                extra_labels,
                elapsed,
            } => {
                write!(f, "Fetched {extra_labels} more labels in {elapsed:?}")
            }
        }
    }
}
