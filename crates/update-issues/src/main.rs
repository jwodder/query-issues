mod db;
mod queries;
mod types;
use crate::db::{Database, IssueDiff};
use crate::queries::GetOwnerRepos;
use anyhow::Context;
use clap::Parser;
use gqlient::{Client, PaginationResults, DEFAULT_BATCH_SIZE};
use patharg::{InputArg, OutputArg};
use serde::Serialize;
use serde_jsonlines::append_json_lines;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

/// Measure time to create & update a local database of open GitHub issues
#[derive(Clone, Debug, Eq, Parser, PartialEq)]
struct Arguments {
    /// Number of sub-queries to make per GraphQL request
    #[arg(short = 'B', long)]
    batch_size: Option<NonZeroUsize>,

    /// Load the initial database state from the given file
    #[arg(short, long)]
    infile: Option<InputArg>,

    /// Do not write the updated database state to `--infile`
    ///
    /// Mutually exclusive with `--outfile`
    #[arg(long, conflicts_with = "outfile")]
    no_save: bool,

    /// Dump the updated database state to the given file
    ///
    /// Mutually exclusive with `--no-save`
    #[arg(short, long)]
    outfile: Option<OutputArg>,

    /// Number of items to request per page of results
    #[arg(short = 'P', long, default_value = "100")]
    page_size: NonZeroUsize,

    /// Append a run report to the given file
    #[arg(short = 'R', long)]
    report_file: Option<PathBuf>,

    /// GitHub owners/organizations of repositories to fetch open issues for
    #[arg(required = true)]
    owners: Vec<String>,
}

impl Arguments {
    fn outfile(&self) -> Option<OutputArg> {
        match (&self.outfile, &self.infile, self.no_save) {
            (Some(f), _, _) => Some(f.clone()),
            (None, None, _) => None,
            (None, _, true) => None,
            (None, Some(InputArg::Stdin), false) => Some(OutputArg::Stdout),
            (None, Some(InputArg::Path(p)), false) => Some(OutputArg::Path(p.clone())),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();
    let mut db = if let Some(ref infile) = args.infile {
        eprintln!("[·] Loading {infile:#} …");
        Database::load(infile.open()?)?
    } else {
        Database::default()
    };

    let mut client = Client::new_with_local_token()?;
    if let Some(bsz) = args.batch_size {
        client.batch_size(bsz);
    }
    let start_rate_limit = client.get_rate_limit()?;

    let big_start = Instant::now();
    let timestamp = SystemTime::now();

    eprintln!("[·] Fetching repositories …");
    let owner_paginators = args.owners.iter().map(|owner| {
        (
            owner.clone(),
            GetOwnerRepos::new(owner.clone(), args.page_size),
        )
    });
    let start = Instant::now();
    let repos = client.batch_paginate(owner_paginators)?;
    let elapsed = start.elapsed();
    let repos = repos
        .into_iter()
        .flat_map(|pr| pr.items)
        .collect::<Vec<_>>();
    let all_repos_qty = repos.len();
    eprintln!("[·] Fetched {all_repos_qty} repositories in {elapsed:?}");

    let rdiff = db.update_repositories(repos);
    eprintln!("[·] {rdiff}");

    eprintln!("[·] Fetching issues …");
    let start = Instant::now();
    let mut repo_qty = 0;
    let issues = client.batch_paginate(
        db.issue_paginators(args.page_size)
            .inspect(|_| repo_qty += 1),
    )?;
    let elapsed = start.elapsed();
    let qty: usize = issues.iter().map(|pr| pr.items.len()).sum();
    eprintln!("[·] Fetched {qty} issues from {repo_qty} repositories in {elapsed:?}");

    let mut idiff = IssueDiff::default();
    for PaginationResults {
        key: repo_id,
        items,
        end_cursor,
    } in issues
    {
        let Some(repo) = db.get_mut(&repo_id) else {
            // TODO: Warn? Error?
            continue;
        };
        repo.set_issue_cursor(end_cursor);
        idiff += repo.update_issues(items);
    }
    eprintln!("[·] {idiff}");

    let big_elapsed = big_start.elapsed();
    eprintln!("[·] Total fetch time: {big_elapsed:?}");

    let end_rate_limit = client.get_rate_limit()?;
    let rate_limit_points = end_rate_limit.used_since(start_rate_limit);
    if let Some(used) = rate_limit_points {
        eprintln!("[·] Used {used} rate limit points");
    } else {
        eprintln!("[·] Could not determine rate limit points used due to intervening reset");
    }

    if let Some(ref report_file) = args.report_file {
        eprintln!("[·] Appending report to {} …", report_file.display());
        let report = Report {
            program: env!("CARGO_BIN_NAME"),
            commit: option_env!("GIT_COMMIT"),
            timestamp: humantime::format_rfc3339(timestamp).to_string(),
            owners: args.owners.clone(),
            parameters: Parameters {
                batch_size: match args.batch_size {
                    Some(bs) => bs.get(),
                    None => DEFAULT_BATCH_SIZE,
                },
                page_size: args.page_size,
            },
            repositories: all_repos_qty,
            open_issues: qty,
            repos_with_open_issues: repo_qty,
            repos_updated: rdiff.repos_touched(),
            issues_updated: rdiff.closed_issues.saturating_add(idiff.issues_touched()),
            elapsed: big_elapsed,
            rate_limit_points,
        };
        append_json_lines(report_file, std::iter::once(report))
            .context("failed to write report")?;
    }

    if let Some(outfile) = args.outfile() {
        eprintln!("[·] Dumping to {outfile:#} …");
        db.dump(outfile.create()?)?;
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct Report {
    program: &'static str,
    commit: Option<&'static str>,
    timestamp: String,
    owners: Vec<String>,
    parameters: Parameters,
    repositories: usize,
    open_issues: usize,
    repos_with_open_issues: usize,
    repos_updated: usize,
    issues_updated: usize,
    elapsed: Duration,
    rate_limit_points: Option<u32>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
struct Parameters {
    batch_size: usize,
    page_size: NonZeroUsize,
}
