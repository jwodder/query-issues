mod config;
mod db;
mod queries;
mod types;
use crate::config::OWNERS;
use crate::db::{Database, IssueDiff};
use crate::queries::GetOwnerRepos;
use anyhow::Context;
use clap::Parser;
use gqlclient::{Client, PaginationResults};
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
    let mut db = match args.infile.open() {
        Ok(fp) => {
            eprintln!("[·] Loading {:#} …", args.infile);
            Database::load(fp)?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Database::default(),
        Err(e) => return Err(e.into()),
    };

    let token = gh_token::get().context("unable to fetch GitHub access token")?;
    let client = Client::new(&token);

    let big_start = Instant::now();

    eprintln!("[·] Fetching repositories …");
    let owner_queries = OWNERS
        .iter()
        .map(|owner| (owner, GetOwnerRepos::new(owner.to_string())));
    let start = Instant::now();
    let repos = client.batch_paginate(owner_queries)?;
    let elapsed = start.elapsed();
    let repos = repos.into_iter().map(|pr| pr.items).concat();
    eprintln!("[·] Fetched {} repositories in {:?}", repos.len(), elapsed);

    let rdiff = db.update_repositories(repos);
    eprintln!("[·] {rdiff}");

    eprintln!("[·] Fetching issues …");
    let start = Instant::now();
    let mut repo_qty = 0;
    let issues = client.batch_paginate(db.issue_queries().inspect(|_| repo_qty += 1))?;
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

    eprintln!("[·] Total fetch time: {:?}", big_start.elapsed());

    let outfile = match (args.outfile, args.infile) {
        (Some(f), _) => f,
        (None, InputArg::Stdin) => OutputArg::Stdout,
        (None, InputArg::Path(p)) => OutputArg::Path(p),
    };
    eprintln!("[·] Dumping to {outfile:#} …");
    db.dump(outfile.create()?)?;
    Ok(())
}
