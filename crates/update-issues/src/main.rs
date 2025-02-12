mod db;
mod machine;
mod queries;
mod types;
use crate::db::Database;
use crate::machine::{FetchReport, Output, Parameters, UpdateIssues};
use anyhow::Context;
use clap::Parser;
use gqlient::{Client, DEFAULT_BATCH_SIZE};
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
    #[arg(short = 'B', long, default_value_t = DEFAULT_BATCH_SIZE)]
    batch_size: NonZeroUsize,

    /// Load the initial database state from the given file
    #[arg(short, long)]
    infile: Option<InputArg>,

    /// Number of labels to request per page
    #[arg(short = 'L', long, default_value = "10")]
    label_page_size: NonZeroUsize,

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

    let client = Client::new_with_local_token()?;
    let start_rate_limit = client.get_rate_limit()?;

    let start = Instant::now();
    let timestamp = SystemTime::now();
    let parameters = Parameters {
        batch_size: args.batch_size,
        page_size: args.page_size,
        label_page_size: args.label_page_size,
    };
    let machine = UpdateIssues::new(&mut db, args.owners.clone(), parameters);
    let mut fetched = None;
    let mut rdiff = None;
    for output in client.run(machine) {
        match output {
            Ok(Output::Transition(t)) => eprintln!("[·] {t}"),
            Ok(Output::Report(r)) => {
                eprintln!("[·] {}", r.issue_diff);
                fetched = Some(r);
            }
            Ok(Output::RepoDiff(rd)) => {
                eprintln!("[·] {rd}");
                rdiff = Some(rd);
            }
            Err(e) => return Err(e.into()),
        }
    }
    let elapsed = start.elapsed();
    eprintln!("[·] Total fetch time: {elapsed:?}");

    let end_rate_limit = client.get_rate_limit()?;
    let rate_limit_points = end_rate_limit.used_since(start_rate_limit);
    if let Some(used) = rate_limit_points {
        eprintln!("[·] Used {used} rate limit points");
    } else {
        eprintln!("[·] Could not determine rate limit points used due to intervening reset");
    }

    if let Some(ref report_file) = args.report_file {
        eprintln!("[·] Appending report to {} …", report_file.display());
        // rdiff is None if no owners were specified, but fetched should always
        // be Some.
        let rdiff = rdiff.unwrap_or_default();
        let fetched = fetched.expect("fetched should have been yielded");
        let report = Report {
            program: env!("CARGO_BIN_NAME"),
            commit: option_env!("GIT_COMMIT"),
            timestamp: humantime::format_rfc3339(timestamp).to_string(),
            owners: args.owners.clone(),
            parameters,
            fetched,
            repos_updated: rdiff.repos_touched(),
            issues_updated: rdiff
                .closed_issues
                .saturating_add(fetched.issue_diff.issues_touched()),
            elapsed,
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
    fetched: FetchReport,
    repos_updated: usize,
    issues_updated: usize,
    elapsed: Duration,
    rate_limit_points: Option<u32>,
}
