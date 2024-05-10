mod config;
mod queries;
mod types;
use crate::queries::{GetIssues, GetOwnerRepos};
use anyhow::Context;
use clap::Parser;
use gqlient::{Client, Ided};
use itertools::Itertools;
use serde_jsonlines::WriteExt;
use std::io::Write;
use std::time::Instant;

/// Measure time to fetch open GitHub issues via GraphQL
#[derive(Clone, Debug, Eq, Parser, PartialEq)]
struct Arguments {
    /// Dump fetched issue information to the given file
    #[arg(short, long)]
    outfile: Option<patharg::OutputArg>,

    /// GitHub owners/organizations of repositories to fetch open issues for
    #[arg(required = true)]
    owners: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();
    let client = Client::new_with_local_token()?;
    let start_rate_limit = client.get_rate_limit()?;

    let big_start = Instant::now();
    let mut repo_qty = 0;
    let mut repos_with_issues_qty: usize = 0;

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
        if repo.open_issues > 0 {
            repos_with_issues_qty += 1;
            issue_queries.push((id.clone(), GetIssues::new(id, None)));
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
    let issues = client.batch_paginate(issue_queries)?;
    let elapsed = start.elapsed();
    let issues = issues.into_iter().map(|pr| pr.items).concat();
    eprintln!("[·] Fetched {} issues in {:?}", issues.len(), elapsed);

    eprintln!("[·] Total fetch time: {:?}", big_start.elapsed());

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
