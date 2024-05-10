mod config;
mod queries;
mod types;
use crate::queries::{GetIssues, GetOwnerRepos};
use anyhow::Context;
use clap::Parser;
use gqlient::{Client, Ided};
use serde_jsonlines::WriteExt;
use std::io::Write;
use std::num::NonZeroUsize;
use std::time::Instant;

/// Measure time to fetch open GitHub issues via GraphQL
#[derive(Clone, Debug, Eq, Parser, PartialEq)]
struct Arguments {
    /// Number of sub-queries to make per GraphQL request
    #[arg(short = 'B', long)]
    batch_size: Option<NonZeroUsize>,

    /// Dump fetched issue information to the given file
    #[arg(short, long)]
    outfile: Option<patharg::OutputArg>,

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
    let mut repo_qty = 0;
    let mut repos_with_issues_qty: usize = 0;
    let mut issues = Vec::new();

    eprintln!("[·] Fetching repositories …");
    let owner_queries = args
        .owners
        .into_iter()
        .map(|owner| (owner.clone(), GetOwnerRepos::new(owner)));
    let repos_start = Instant::now();
    let repos = client.batch_paginate(owner_queries)?;
    let elapsed = repos_start.elapsed();

    let mut issue_queries = Vec::new();
    for Ided { id, data: repo } in repos.into_iter().flat_map(|pr| pr.items) {
        repo_qty += 1;
        if !repo.issues.is_empty() {
            repos_with_issues_qty += 1;
            issues.extend(repo.issues);
        }
        if repo.has_more_issues {
            issue_queries.push((id.clone(), GetIssues::new(id, repo.issue_cursor)));
        }
    }
    eprintln!(
        "[·] Fetched {} repositories ({} with open issues; {} open issues in total) in {:?}",
        repo_qty,
        repos_with_issues_qty,
        issues.len(),
        elapsed
    );

    if !issue_queries.is_empty() {
        eprintln!(
            "[·] Fetching more issues for {} repositories …",
            issue_queries.len()
        );
        let start = Instant::now();
        let more_issues = client.batch_paginate(issue_queries)?;
        let elapsed = start.elapsed();
        let mut issue_qty = 0;
        issues.extend(
            more_issues
                .into_iter()
                .flat_map(|pr| pr.items)
                .inspect(|_| issue_qty += 1),
        );
        eprintln!("[·] Fetched {issue_qty} more issues in {elapsed:?}");
    }

    eprintln!(
        "[·] Total of {} issues fetched in {:?}",
        issues.len(),
        big_start.elapsed()
    );

    let end_rate_limit = client.get_rate_limit()?;
    if let Some(used) = end_rate_limit.used_since(start_rate_limit) {
        eprintln!("[·] Used {used} rate limit points");
    } else {
        eprintln!("[·] Could not determine rate limit points used due to intervening reset");
    }

    if let Some(outfile) = args.outfile {
        eprintln!("[·] Dumping to {outfile:#} …");
        let mut fp = outfile.create().context("failed to open file")?;
        fp.write_json_lines(issues)
            .context("failed to dump issues")?;
        fp.flush().context("failed to flush filehandle")?;
    }

    Ok(())
}
