mod config;
mod db;
mod queries;
mod types;
use crate::db::{Database, IssueDiff};
use crate::queries::GetOwnerRepos;
use anyhow::Context;
use clap::Parser;
use gqlient::{Client, PaginationResults};
use itertools::Itertools;
use patharg::{InputArg, OutputArg};
use std::time::Instant;

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
struct Arguments {
    #[arg(short, long)]
    infile: Option<InputArg>,

    #[arg(long, conflicts_with = "outfile")]
    no_save: bool,

    #[arg(short, long)]
    outfile: Option<OutputArg>,

    #[arg(required = true)]
    owners: Vec<String>,
}

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
struct Options {
    infile: Option<InputArg>,
    outfile: Option<OutputArg>,
    owners: Vec<String>,
}

impl From<Arguments> for Options {
    fn from(
        Arguments {
            infile,
            no_save,
            outfile,
            owners,
        }: Arguments,
    ) -> Options {
        let outfile = match (outfile, &infile, no_save) {
            (Some(f), _, _) => Some(f),
            (None, None, _) => None,
            (None, _, true) => None,
            (None, Some(InputArg::Stdin), false) => Some(OutputArg::Stdout),
            (None, Some(InputArg::Path(p)), false) => Some(OutputArg::Path(p.clone())),
        };
        Options {
            infile,
            outfile,
            owners,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let opts = Options::from(Arguments::parse());
    let mut db = if let Some(infile) = opts.infile {
        eprintln!("[·] Loading {infile:#} …");
        Database::load(infile.open()?)?
    } else {
        Database::default()
    };

    let token = gh_token::get().context("unable to fetch GitHub access token")?;
    let client = Client::new(&token);
    let start_rate_limit = client.get_rate_limit()?;

    let big_start = Instant::now();

    eprintln!("[·] Fetching repositories …");
    let owner_paginators = opts
        .owners
        .into_iter()
        .map(|owner| (owner.clone(), GetOwnerRepos::new(owner)));
    let start = Instant::now();
    let repos = client.batch_paginate(owner_paginators)?;
    let elapsed = start.elapsed();
    let repos = repos.into_iter().map(|pr| pr.items).concat();
    eprintln!("[·] Fetched {} repositories in {:?}", repos.len(), elapsed);

    let rdiff = db.update_repositories(repos);
    eprintln!("[·] {rdiff}");

    eprintln!("[·] Fetching issues …");
    let start = Instant::now();
    let mut repo_qty = 0;
    let issues = client.batch_paginate(db.issue_paginators().inspect(|_| repo_qty += 1))?;
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

    let end_rate_limit = client.get_rate_limit()?;
    if let Some(used) = end_rate_limit.used_since(start_rate_limit) {
        eprintln!("[·] Used {used} rate limit points");
    } else {
        eprintln!("[·] Could not determine rate limit points used due to intervening reset");
    }

    if let Some(outfile) = opts.outfile {
        eprintln!("[·] Dumping to {outfile:#} …");
        db.dump(outfile.create()?)?;
    }
    Ok(())
}
