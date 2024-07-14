[![Project Status: Concept – Minimal or no implementation has been done yet, or the repository is only intended to be a limited example, demo, or proof-of-concept.](https://www.repostatus.org/badges/latest/concept.svg)](https://www.repostatus.org/#concept)
[![CI Status](https://github.com/jwodder/query-issues/actions/workflows/test.yml/badge.svg)](https://github.com/jwodder/query-issues/actions/workflows/test.yml) <!-- [![codecov.io](https://codecov.io/gh/jwodder/query-issues/branch/master/graph/badge.svg)](https://codecov.io/gh/jwodder/query-issues) -->
[![Minimum Supported Rust Version](https://img.shields.io/badge/MSRV-1.77-orange)](https://www.rust-lang.org)
[![MIT License](https://img.shields.io/github/license/jwodder/query-issues.svg)](https://opensource.org/licenses/MIT)

This is an experiment in determining the fastest way to use GitHub's GraphQL
API to fetch all open issues in all (public, non-archived, non-fork)
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

The program logs to stderr the number of repositories fetched (including how
many had open issues), the number of open issues fetched, the elapsed time, and
(if possible) the number of API rate limit points used.

### Options

- `-B <int>`/`--batch-size <int>` — Set the number of sub-queries to make per
  GraphQL request [default: 50]

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

The key difference from `orgs-then-issues` is that this command fetches a page
of issues for each repository as part of the same requests that fetch the
repositories themselves.

The program logs to stderr the number of repositories fetched, the number of
open issues fetched, the elapsed time, and (if possible) the number of API rate
limit points used.

### Options

- `-B <int>`/`--batch-size <int>` — Set the number of sub-queries to make per
  GraphQL request [default: 50]

- `-o <path>`/`--outfile <path>` — Dump fetched issue information to the given
  file as JSON Lines.  `<path>` may be `-` to write to standard output.

- `-P <int>`/`--page-size <int>` — Set the number of items to request per page
  of results [default: 100]

- `-R <path>`/`--report-file <path>` — Append a report of the run to the given
  file as a JSON Lines entry


`update-issues`
---------------

    cargo run [--release] -p update-issues -- [<options>] <owner> ...

`update-issues` creates or updates a JSON database of open issues, only making
requests for issues that have been updated since the database was last updated.

If an `-i <path>`/`--infile <path>` option is given on the command line, the
program reads a JSON database of repositories and their open issues from
`<path>`; otherwise, it starts out with an empty database.  The program then
performs a paginated batch query to fetch all (public etc.) repositories
belonging to the owners specified on the command line, including getting the
number of open issues in each repository.  Then, all repositories that have one
or more open issues are queried in batches to get paginated lists of their
issues ordered by update time.  For repositories that had open issues when the
database was last updated, this query fetches details on all issues, open &
closed, that have been updated since the last run; for other repositories, only
open issues are queried, but the query starts from the beginning of time.  The
issues thus fetched are then added or (for closed issues) removed from the
database as appropriate.

`update-issues` logs to stderr the number of repositories fetched (including
how many had open issues), the number of open issues fetched, the numbers of
repositories & issues in the database that were added/modified/removed, the
elapsed time, and (if possible) the number of API rate limit points used.

> [!NOTE]
> This strategy is unable to update a database to remove issues that have since
> been deleted, transferred to another repository, or converted to discussions.

### Options

- `-B <int>`/`--batch-size <int>` — Set the number of sub-queries to make per
  GraphQL request [default: 50]

- `-i <path>`/`--infile <path>` — Load the database at `<path>` at start of
  program execution.  If not specified, an empty database is used.  `<path>`
  may be `-` to read from standard input.

  If this option is specified on the command line and neither `--no-save` nor
  `-o`/`--outfile` is specified, then the updated database will be written back
  out to this file at end of program execution.

- `--no-save` — If the `-i`/`--infile` option was also supplied, do not write
  the updated database back to the infile at end of program execution.

  This option is mutually exclusive with `--outfile`.

- `-o <path>`/`--outfile <path>` — Dump the final database to `<path>` at end
  of program execution.  `<path>` may be `-` to write to standard output.

  This option is mutually exclusive with `--no-save`.

- `-P <int>`/`--page-size <int>` — Set the number of items to request per page
  of results [default: 100]


Authentication
--------------

All commands require a GitHub access token with appropriate permissions in
order to run.  Specify the token via the `GH_TOKEN` or `GITHUB_TOKEN`
environment variable or by storing a token with the
[`gh`](https://github.com/cli/cli) command.
