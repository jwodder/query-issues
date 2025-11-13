mod machine;
mod queries;
use crate::machine::{Event, EventSubscriber, OrgsThenIssues, QueryLimits};
use anyhow::Context;
use clap::Parser;
use gqlient::{Client, DEFAULT_BATCH_SIZE};
use serde::Serialize;
use serde_jsonlines::{WriteExt, append_json_lines};
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
    let parameters = QueryLimits {
        batch_size: args.batch_size,
        page_size: args.page_size,
        label_page_size: args.label_page_size,
    };
    let mut recorder = EventReporter::new();
    let machine =
        OrgsThenIssues::new(args.owners.clone(), parameters).with_subscriber(&mut recorder);
    let issues = client.run(machine).collect::<Result<Vec<_>, _>>()?;
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
            fetched: recorder.report,
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct Report {
    program: &'static str,
    commit: Option<&'static str>,
    timestamp: String,
    owners: Vec<String>,
    parameters: QueryLimits,
    fetched: FetchReport,
    elapsed: Duration,
    rate_limit_points: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EventReporter {
    last_start_time: Option<Instant>,
    report: FetchReport,
}

impl EventReporter {
    fn new() -> EventReporter {
        EventReporter {
            last_start_time: None,
            report: FetchReport::default(),
        }
    }

    fn start_timer(&mut self) {
        self.last_start_time = Some(Instant::now());
    }

    fn state_duration(&self) -> Duration {
        self.last_start_time
            .expect("ending state should have started")
            .elapsed()
    }
}

impl EventSubscriber for &mut EventReporter {
    fn handle_event(&mut self, ev: Event) {
        match ev {
            Event::Start => (),
            Event::StartFetchRepos => {
                self.start_timer();
                eprintln!("[·] Fetching repositories …");
            }
            Event::EndFetchRepos {
                repositories,
                repos_with_open_issues,
            } => {
                let elapsed = self.state_duration();
                self.report.repositories = repositories;
                self.report.repos_with_open_issues = repos_with_open_issues;
                eprintln!(
                    "[·] Fetched {repositories} repositories ({repos_with_open_issues} with open issues) in {elapsed:?}"
                );
            }
            Event::StartFetchIssues => {
                self.start_timer();
                let repos_with_open_issues = self.report.repos_with_open_issues;
                eprintln!("[·] Fetching issues for {repos_with_open_issues} repositories …");
            }
            Event::EndFetchIssues { open_issues } => {
                let elapsed = self.state_duration();
                self.report.open_issues = open_issues;
                eprintln!("[·] Fetched {open_issues} issues in {elapsed:?}");
            }
            Event::StartFetchLabels {
                issues_with_extra_labels,
            } => {
                self.start_timer();
                self.report.issues_with_extra_labels = issues_with_extra_labels;
                eprintln!("[·] Fetching more labels for {issues_with_extra_labels} issues …");
            }
            Event::EndFetchLabels { extra_labels } => {
                let elapsed = self.state_duration();
                self.report.extra_labels = extra_labels;
                eprintln!("[·] Fetched {extra_labels} more labels in {elapsed:?}");
            }
            Event::Done => (),
            Event::Error => (),
        }
    }
}

/// Information on how many of each type of thing were retrieved from the
/// server
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub(crate) struct FetchReport {
    /// Total number of repositories retrieved
    repositories: usize,

    /// Total number of open issues retrieved
    open_issues: usize,

    /// Number of repositories retrieved that had open issues
    repos_with_open_issues: usize,

    /// Number of issues retrieved that required extra queries to fetch all
    /// labels
    issues_with_extra_labels: usize,

    /// Total number of labels that required extra queries to retrieve
    extra_labels: usize,
}
