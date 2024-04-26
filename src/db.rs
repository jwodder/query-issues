use crate::queries::GetIssues;
use crate::types::{Cursor, Id, Ided, Issue, IssueState, RepoDetails};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::{btree_map::Entry, BTreeMap, HashMap};
use std::fmt;
use std::io;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct Database(BTreeMap<Id, Repository>);

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

    pub(crate) fn get_mut(&mut self, repo_id: &Id) -> Option<&mut Repository> {
        self.0.get_mut(repo_id)
    }

    pub(crate) fn update_repositories<I>(&mut self, iter: I) -> RepoDiff
    where
        I: IntoIterator<Item = Ided<RepoDetails>>,
    {
        let mut report = RepoDiff::default();
        let mut newmap = BTreeMap::new();
        for Ided { id, data } in iter {
            if let Some(mut repo) = self.0.remove(&id) {
                if repo.details != data {
                    report.modified += 1;
                    repo.details = data;
                }
                newmap.insert(id, repo);
            } else {
                newmap.insert(
                    id,
                    Repository {
                        details: data,
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

    pub(crate) fn issue_queries(&self) -> HashMap<Id, GetIssues> {
        self.0
            .iter()
            .map(|(id, repo)| {
                (
                    id.clone(),
                    GetIssues::new(id.clone(), repo.issue_cursor.clone()),
                )
            })
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct Repository {
    details: RepoDetails,
    issue_cursor: Option<Cursor>,
    issues: BTreeMap<Id, Issue>,
}

impl Repository {
    pub(crate) fn set_issue_cursor(&mut self, cursor: Option<Cursor>) {
        self.issue_cursor = cursor;
    }

    pub(crate) fn update_issues<I>(&mut self, issues: I) -> IssueDiff
    where
        I: IntoIterator<Item = Ided<Issue>>,
    {
        let mut report = IssueDiff::default();
        for Ided { id, data } in issues {
            match self.issues.entry(id) {
                Entry::Occupied(o) if data.state == IssueState::Closed => {
                    report.closed += 1;
                    o.remove();
                }
                Entry::Vacant(_) if data.state == IssueState::Closed => (),
                Entry::Occupied(mut o) => {
                    if o.get() != &data {
                        report.modified += 1;
                        o.insert(data);
                    }
                }
                Entry::Vacant(v) => {
                    report.added += 1;
                    v.insert(data);
                }
            }
        }
        report
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RepoDiff {
    added: usize,
    modified: usize,
    deleted: usize,
}

impl fmt::Display for RepoDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} repositories added, {} repositories modified, {} repositories deleted",
            self.added, self.modified, self.deleted
        )
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct IssueDiff {
    added: usize,
    modified: usize,
    closed: usize,
}

impl fmt::Display for IssueDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} issues added, {} issues modified, {} issues closed",
            self.added, self.modified, self.closed
        )
    }
}

impl std::ops::AddAssign for IssueDiff {
    fn add_assign(&mut self, rhs: IssueDiff) {
        self.added += rhs.added;
        self.modified += rhs.modified;
        self.closed += rhs.closed;
    }
}