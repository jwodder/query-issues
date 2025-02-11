mod machine;
mod queries;
mod types;
use crate::machine::{MachineReport, OrgsThenIssues, Output, Parameters};
use anyhow::Context;
use clap::Parser;
use gqlient::{Client, DEFAULT_BATCH_SIZE};
use serde::Serialize;
use serde_jsonlines::{append_json_lines, WriteExt};
use std::io::Write;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

/// Measure time to fetch open GitHub issues via GraphQL
#[derive(Clone, Debug, Eq, Parser, PartialEq)]
struct Arguments {
    /// Number of sub-queries to make per GraphQL request
    #[arg(short = 'B', long, default_value_t = DEFAULT_BATCH_SIZE)]
    batch_size: NonZeroUsize,

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
    let client = Client::new_with_local_token()?;
    let start_rate_limit = client.get_rate_limit()?;

    let start = Instant::now();
    let timestamp = SystemTime::now();
    let parameters = Parameters {
        batch_size: args.batch_size,
        page_size: args.page_size,
        label_page_size: args.label_page_size,
    };
    let machine = OrgsThenIssues::new(args.owners.clone(), parameters);
    let mut issues = Vec::new();
    let mut machine_report = None;
    for output in client.run(machine) {
        match output {
            Ok(Output::Transition(t)) => eprintln!("[·] {t}"),
            Ok(Output::Issues(ish)) => issues.extend(ish),
            Ok(Output::Report(r)) => machine_report = Some(r),
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

    if let Some(report_file) = args.report_file {
        eprintln!("[·] Appending report to {} …", report_file.display());
        let report = Report {
            program: env!("CARGO_BIN_NAME"),
            commit: option_env!("GIT_COMMIT"),
            timestamp: humantime::format_rfc3339(timestamp).to_string(),
            owners: args.owners,
            parameters,
            machine_report: machine_report.expect("machine report should have been yielded"),
            elapsed,
            rate_limit_points,
        };
        append_json_lines(report_file, std::iter::once(report))
            .context("failed to write report")?;
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

#[allow(clippy::struct_field_names)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct Report {
    program: &'static str,
    commit: Option<&'static str>,
    timestamp: String,
    owners: Vec<String>,
    parameters: Parameters,
    #[serde(flatten)]
    machine_report: MachineReport,
    elapsed: Duration,
    rate_limit_points: Option<u32>,
}
