mod queries;
mod types;
use crate::queries::{GetIssues, GetOwnerRepos};
use crate::types::Issue;
use anyhow::Context;
use clap::Parser;
use gqlient::{Client, Id, Ided, DEFAULT_BATCH_SIZE};
use serde::Serialize;
use serde_jsonlines::{append_json_lines, WriteExt};
use std::collections::HashMap;
use std::io::Write;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

/// Measure time to fetch open GitHub issues via GraphQL
#[derive(Clone, Debug, Eq, Parser, PartialEq)]
struct Arguments {
    /// Number of sub-queries to make per GraphQL request
    #[arg(short = 'B', long)]
    batch_size: Option<NonZeroUsize>,

    /// Number of labels to request per page
    #[arg(short = 'L', long, default_value = "10")]
    label_page_size: NonZeroUsize,

    /// Dump fetched issue information to the given file
    #[arg(short, long)]
    outfile: Option<patharg::OutputArg>,

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

fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();
    let mut client = Client::new_with_local_token()?;
    if let Some(bsz) = args.batch_size {
        client.batch_size(bsz);
    }
    let start_rate_limit = client.get_rate_limit()?;

    let big_start = Instant::now();
    let timestamp = SystemTime::now();
    let mut repo_qty = 0;
    let mut repos_with_issues_qty: usize = 0;

    eprintln!("[·] Fetching repositories …");
    let owner_queries = args
        .owners
        .clone()
        .into_iter()
        .map(|owner| (owner.clone(), GetOwnerRepos::new(owner, args.page_size)));
    let repos_start = Instant::now();
    let repos = client.batch_paginate(owner_queries)?;
    let elapsed = repos_start.elapsed();

    let mut issue_queries = Vec::new();
    for Ided { id, data: repo } in repos.into_iter().flat_map(|pr| pr.items) {
        repo_qty += 1;
        if repo.open_issues > 0 {
            repos_with_issues_qty += 1;
            issue_queries.push((
                id.clone(),
                GetIssues::new(id, args.page_size, args.label_page_size),
            ));
        }
    }
    eprintln!(
        "[·] Fetched {repo_qty} repositories ({repos_with_issues_qty} with open issues) in {elapsed:?}"
    );

    eprintln!(
        "[·] Fetching issues for {} repositories …",
        issue_queries.len()
    );
    let start = Instant::now();
    let more_issues = client.batch_paginate(issue_queries)?;
    let elapsed = start.elapsed();
    let mut issues = HashMap::<Id, Issue>::new();
    let mut label_queries = Vec::new();
    let mut issue_qty = 0;
    for iwl in more_issues.into_iter().flat_map(|pr| pr.items) {
        label_queries.extend(iwl.more_labels_query(args.label_page_size));
        issue_qty += 1;
        issues.insert(iwl.issue_id, iwl.issue);
    }
    eprintln!("[·] Fetched {issue_qty} issues in {elapsed:?}");

    let issues_with_extra_labels = label_queries.len();
    if !label_queries.is_empty() {
        eprintln!("[·] Fetching more labels for {issues_with_extra_labels} issues …",);
        let start = Instant::now();
        let more_labels = client.batch_paginate(label_queries)?;
        let elapsed = start.elapsed();
        let mut label_qty = 0;
        for res in more_labels {
            label_qty += res.items.len();
            issues
                .get_mut(&res.key)
                .expect("Issues we get labels for should have already been seen")
                .labels
                .extend(res.items);
        }
        eprintln!("[·] Fetched {label_qty} more labels in {elapsed:?}");
    }

    let big_elapsed = big_start.elapsed();
    eprintln!("[·] Total fetch time: {big_elapsed:?}");

    let end_rate_limit = client.get_rate_limit()?;
    let rate_limit_points = end_rate_limit.used_since(start_rate_limit);
    if let Some(used) = rate_limit_points {
        eprintln!("[·] Used {used} rate limit points");
    } else {
        eprintln!("[·] Could not determine rate limit points used due to intervening reset");
    }

    if let Some(report_file) = args.report_file {
        eprintln!("[·] Appending report to {} …", report_file.display());
        let report = Report {
            program: env!("CARGO_BIN_NAME"),
            commit: option_env!("GIT_COMMIT"),
            timestamp: humantime::format_rfc3339(timestamp).to_string(),
            owners: args.owners,
            parameters: Parameters {
                batch_size: match args.batch_size {
                    Some(bs) => bs.get(),
                    None => DEFAULT_BATCH_SIZE,
                },
                page_size: args.page_size,
                label_page_size: args.label_page_size,
            },
            repositories: repo_qty,
            open_issues: issues.len(),
            repos_with_open_issues: repos_with_issues_qty,
            issues_with_extra_labels,
            elapsed: big_elapsed,
            rate_limit_points,
        };
        append_json_lines(report_file, std::iter::once(report))
            .context("failed to write report")?;
    }

    if let Some(outfile) = args.outfile {
        eprintln!("[·] Dumping to {outfile:#} …");
        let mut fp = outfile.create().context("failed to open file")?;
        fp.write_json_lines(issues.values())
            .context("failed to dump issues")?;
        fp.flush().context("failed to flush filehandle")?;
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
    issues_with_extra_labels: usize,
    elapsed: Duration,
    rate_limit_points: Option<u32>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[allow(clippy::struct_field_names)]
struct Parameters {
    batch_size: usize,
    page_size: NonZeroUsize,
    label_page_size: NonZeroUsize,
}
