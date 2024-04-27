mod client;
mod config;
mod queries;
mod types;
use crate::client::Client;
use crate::config::OWNERS;
use crate::queries::{GetIssues, GetOwnerRepos};
use anyhow::Context;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let token = gh_token::get().context("unable to fetch GitHub access token")?;
    let client = Client::new(&token);

    let start = Instant::now();
    let mut repo_qty = 0;
    let mut issue_qty = 0;

    let owner_queries = OWNERS
        .iter()
        .map(|owner| (owner, GetOwnerRepos::new(owner.to_string())));
    let repos = client.batch_paginate(owner_queries)?;

    let mut issue_queries = Vec::new();
    for id_repo in repos.into_iter().flat_map(|pr| pr.items) {
        repo_qty += 1;
        issue_qty += id_repo.data.issues.len();
        if id_repo.data.has_more_issues {
            issue_queries.push((
                id_repo.id.clone(),
                GetIssues::new(id_repo.id, id_repo.data.issue_cursor),
            ));
        }
    }

    let issues = client.batch_paginate(issue_queries)?;
    issue_qty += issues.iter().map(|pr| pr.items.len()).sum::<usize>();

    println!(
        "Fetched {} issues in {} repositories in {:?}",
        issue_qty,
        repo_qty,
        start.elapsed()
    );
    Ok(())
}
