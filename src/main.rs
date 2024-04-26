mod client;
mod config;
mod db;
mod queries;
mod types;
use crate::client::Client;
use crate::config::OWNERS;
use crate::db::{Database, IssueDiff};
use crate::queries::GetOwnerRepos;
use anyhow::Context;
use clap::Parser;
use itertools::Itertools;
use patharg::{InputArg, OutputArg};
use std::time::Instant;

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
struct Arguments {
    infile: InputArg,
    outfile: Option<OutputArg>,
}

fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();
    eprintln!("[·] Loading {:#} …", args.infile);
    let mut db = Database::load(args.infile.open()?)?;

    let token = gh_token::get().context("unable to fetch GitHub access token")?;
    let client = Client::new(&token);

    eprintln!("[·] Fetching repositories …");
    let owner_queries = OWNERS
        .iter()
        .map(|owner| (owner, GetOwnerRepos::new(owner.to_string())))
        .collect();
    let start = Instant::now();
    let repomap = client.batch_paginate(owner_queries)?;
    let elapsed = start.elapsed();
    let repos = repomap.into_values().map(|(items, _)| items).concat();
    eprintln!("[·] Fetched {} repositories in {:?}", repos.len(), elapsed);

    let rdiff = db.update_repositories(repos);
    eprintln!("[·] {rdiff}");

    eprintln!("[·] Fetching issues …");
    let issue_queries = db.issue_queries();
    let start = Instant::now();
    let issue_map = client.batch_paginate(issue_queries)?;
    let elapsed = start.elapsed();
    let qty: usize = issue_map.values().map(|(items, _)| items.len()).sum();
    eprintln!("[·] Fetched {qty} issues in {elapsed:?}");

    let mut idiff = IssueDiff::default();
    for (repo_id, (issues, cursor)) in issue_map {
        let Some(repo) = db.get_mut(&repo_id) else {
            // TODO: Warn? Error?
            continue;
        };
        repo.set_issue_cursor(cursor);
        idiff += repo.update_issues(issues);
    }
    eprintln!("[·] {idiff}");

    let outfile = match (args.outfile, args.infile) {
        (Some(f), _) => f,
        (None, InputArg::Stdin) => OutputArg::Stdout,
        (None, InputArg::Path(p)) => OutputArg::Path(p),
    };
    eprintln!("[·] Dumping to {outfile:#} …");
    db.dump(outfile.create()?)?;
    Ok(())
}
