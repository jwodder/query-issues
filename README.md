[![Project Status: Concept – Minimal or no implementation has been done yet, or the repository is only intended to be a limited example, demo, or proof-of-concept.](https://www.repostatus.org/badges/latest/concept.svg)](https://www.repostatus.org/#concept)
[![CI Status](https://github.com/jwodder/query-issues/actions/workflows/test.yml/badge.svg)](https://github.com/jwodder/query-issues/actions/workflows/test.yml)
[![codecov.io](https://codecov.io/gh/jwodder/query-issues/branch/main/graph/badge.svg)](https://codecov.io/gh/jwodder/query-issues)
[![Minimum Supported Rust Version](https://img.shields.io/badge/MSRV-1.83-orange)](https://www.rust-lang.org)
[![MIT License](https://img.shields.io/github/license/jwodder/query-issues.svg)](https://opensource.org/licenses/MIT)

This is an experiment in determining the most efficient way to use GitHub's
GraphQL API to fetch all open issues in all (public, non-archived, non-fork)
repositories belonging to a collection of owners/organizations.  Each binary
package in this workspace implements a different strategy, detailed below.

Usage
=====

`orgs-then-issues`
------------------

    cargo run [--release] -p orgs-then-issues -- [<options>] <owner> ...

`orgs-then-issues` performs a paginated batch query to fetch all (public etc.)
repositories belonging to the owners specified on the command line, including
getting the number of open issues in each repository.  Then, all repositories
that have one or more open issues are queried in batches to get paginated lists
of their open issues.

When querying issues, the first 10 (by default) labels are retrieved for each
issue.  If any issue has more than this many labels, the remaining labels are
queried in batches at this point.

The program logs to stderr the number of repositories fetched (including how
many had open issues), the number of open issues fetched, the elapsed time, and
(if possible) the number of API rate limit points used.

### Options

- `-B <int>`/`--batch-size <int>` — Set the number of sub-queries to make per
  GraphQL request [default: 50]

- `-L <int>`/`--label-page-size <int>` — Set the number of labels to request
  per page [default: 10]

- `-o <path>`/`--outfile <path>` — Dump fetched issue information to the given
  file as JSON Lines.  `<path>` may be `-` to write to standard output.

- `-P <int>`/`--page-size <int>` — Set the number of items to request per page
  of results [default: 100]

- `-R <path>`/`--report-file <path>` — Append a report of the run to the given
  file as a JSON Lines entry


`orgs-with-issues`
------------------

    cargo run [--release] -p orgs-with-issues -- [<options>] <owner> ...

`orgs-with-issues` performs a paginated batch query to fetch all (public etc.)
repositories belonging to the owners specified on the command line; for each
repository, the first page of open issues is also queried at the same time.
Then, all repositories that still have more open issues are queried repeatedly
in batches to get their remaining pages of open issues.

When querying issues, the first 10 (by default) labels are retrieved for each
issue.  If any issue has more than this many labels, the remaining labels are
queried in batches at this point.

The key difference from `orgs-then-issues` is that this command fetches an
initial page of issues for each repository as part of the same requests that
fetch the repositories themselves.

The program logs to stderr the number of repositories fetched, the number of
open issues fetched, the elapsed time, and (if possible) the number of API rate
limit points used.

### Options

- `-B <int>`/`--batch-size <int>` — Set the number of sub-queries to make per
  GraphQL request [default: 50]

- `-L <int>`/`--label-page-size <int>` — Set the number of labels to request
  per page [default: 10]

- `-o <path>`/`--outfile <path>` — Dump fetched issue information to the given
  file as JSON Lines.  `<path>` may be `-` to write to standard output.

- `-P <int>`/`--page-size <int>` — Set the number of items to request per page
  of results [default: 100]

- `-R <path>`/`--report-file <path>` — Append a report of the run to the given
  file as a JSON Lines entry


Authentication
--------------

All commands require a GitHub access token with appropriate permissions in
order to run.  Specify the token via the `GH_TOKEN` or `GITHUB_TOKEN`
environment variable or by storing a token with the
[`gh`](https://github.com/cli/cli) command.
