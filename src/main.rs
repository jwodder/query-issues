mod client;
mod config;
mod queries;
mod types;
use crate::client::*;
use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let token = gh_token::get().context("unable to fetch GitHub access token")?;
    let client = GitHub::new(&token);

    // Load database file
    // Time: Fetch all repositories for OWNERS
    // Report added/deleted/modified repos
    // Time: For each repository, fetch recently-updated issues (open & closed)
    //  - If there's no saved cursor for the repo, fetch all issues (open only)
    // Report added/deleted/closed/modified issues
    // Dump database file

    todo!();

    Ok(())
}
