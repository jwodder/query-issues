use crate::queries::GetIssues;
use crate::types::{Issue, IssueState, RepoDetails};
use anyhow::Context;
use gqlient::{Cursor, Id, Ided};
use serde::{de::Deserializer, Deserialize, Serialize};
use std::collections::{btree_map::Entry, BTreeMap};
use std::fmt;
use std::io;
use std::num::NonZeroUsize;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct Database(BTreeMap<Id, RepoWithIssues>);

impl Database {
    pub(crate) fn load<R: io::Read>(reader: R) -> serde_json::Result<Self> {
        serde_json::from_reader(reader)
    }

    pub(crate) fn dump<W: io::Write>(&self, mut writer: W) -> anyhow::Result<()> {
        serde_json::to_writer_pretty(&mut writer, self).context("failed to dump database")?;
        writer
            .write_all(b"\n")
            .context("failed to append newline to database dump")?;
        Ok(())
    }

    pub(crate) fn get_mut(&mut self, repo_id: &Id) -> Option<&mut RepoWithIssues> {
        self.0.get_mut(repo_id)
    }

    pub(crate) fn update_repositories<I>(&mut self, iter: I) -> RepoDiff
    where
        I: IntoIterator<Item = Ided<RepoDetails>>,
    {
        let mut report = RepoDiff::default();
        let mut newmap = BTreeMap::new();
        for Ided { id, data: repo } in iter {
            if let Some(mut repo_w_issues) = self.0.remove(&id) {
                if repo_w_issues.repository != repo {
                    report.modified += 1;
                    if repo.open_issues == 0 {
                        report.closed_issues += repo_w_issues.issues.len();
                        repo_w_issues.issue_cursor = None;
                        repo_w_issues.issues.clear();
                    }
                    repo_w_issues.repository = repo;
                }
                newmap.insert(id, repo_w_issues);
            } else {
                newmap.insert(
                    id,
                    RepoWithIssues {
                        repository: repo,
                        issue_cursor: None,
                        issues: BTreeMap::new(),
                    },
                );
                report.added += 1;
            }
        }
        report.deleted = self.0.len();
        self.0 = newmap;
        report
    }

    pub(crate) fn issue_paginators(
        &self,
        page_size: NonZeroUsize,
        label_page_size: NonZeroUsize,
    ) -> Vec<(Id, GetIssues)> {
        self.0
            .iter()
            .filter(|(_, repo)| repo.repository.open_issues != 0)
            .map(move |(id, repo)| {
                (
                    id.clone(),
                    GetIssues::new(
                        id.clone(),
                        repo.issue_cursor.clone(),
                        page_size,
                        label_page_size,
                    ),
                )
            })
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct RepoWithIssues {
    #[serde(deserialize_with = "deser_repo_details")]
    repository: RepoDetails,
    issue_cursor: Option<Cursor>,
    issues: BTreeMap<Id, Issue>,
}

impl RepoWithIssues {
    pub(crate) fn set_issue_cursor(&mut self, cursor: Option<Cursor>) {
        self.issue_cursor = cursor;
    }

    pub(crate) fn update_issue(&mut self, issue_id: Id, issue: Issue) -> IssueDiff {
        let mut report = IssueDiff::default();
        match self.issues.entry(issue_id) {
            Entry::Occupied(o) if issue.state == IssueState::Closed => {
                report.open_closed += 1;
                o.remove();
            }
            Entry::Vacant(_) if issue.state == IssueState::Closed => report.already_closed += 1,
            Entry::Occupied(mut o) => {
                if o.get() != &issue {
                    report.modified += 1;
                    o.insert(issue);
                }
            }
            Entry::Vacant(v) => {
                report.added += 1;
                v.insert(issue);
            }
        }
        report
    }
}

fn deser_repo_details<'de, D>(deserializer: D) -> Result<RepoDetails, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
    struct DirectDetails {
        owner: String,
        name: String,
        open_issues: u64,
    }

    let DirectDetails {
        owner,
        name,
        open_issues,
    } = DirectDetails::deserialize(deserializer)?;
    Ok(RepoDetails {
        owner,
        name,
        open_issues,
    })
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RepoDiff {
    added: usize,
    modified: usize,
    deleted: usize,
    pub(crate) closed_issues: usize,
}

impl RepoDiff {
    pub(crate) fn repos_touched(&self) -> usize {
        self.added
            .saturating_add(self.modified)
            .saturating_add(self.deleted)
    }
}

impl fmt::Display for RepoDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} repositories added, {} repositories modified, {} repositories deleted, {} issues bulk closed",
            self.added, self.modified, self.deleted, self.closed_issues
        )
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct IssueDiff {
    added: usize,
    modified: usize,
    open_closed: usize,
    already_closed: usize,
}

impl IssueDiff {
    pub(crate) fn issues_touched(&self) -> usize {
        self.added
            .saturating_add(self.modified)
            .saturating_add(self.open_closed)
    }
}

impl fmt::Display for IssueDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} issues added, {} issues modified, {} open issues closed, {} issues already closed",
            self.added, self.modified, self.open_closed, self.already_closed
        )
    }
}

impl std::ops::AddAssign for IssueDiff {
    fn add_assign(&mut self, rhs: IssueDiff) {
        self.added += rhs.added;
        self.modified += rhs.modified;
        self.open_closed += rhs.open_closed;
        self.already_closed += rhs.already_closed;
    }
}
