[![Project Status: Concept – Minimal or no implementation has been done yet, or the repository is only intended to be a limited example, demo, or proof-of-concept.](https://www.repostatus.org/badges/latest/concept.svg)](https://www.repostatus.org/#concept)
[![CI Status](https://github.com/jwodder/query-issues/actions/workflows/test.yml/badge.svg)](https://github.com/jwodder/query-issues/actions/workflows/test.yml) <!-- [![codecov.io](https://codecov.io/gh/jwodder/query-issues/branch/master/graph/badge.svg)](https://codecov.io/gh/jwodder/query-issues) -->
[![Minimum Supported Rust Version](https://img.shields.io/badge/MSRV-1.77-orange)](https://www.rust-lang.org)
[![MIT License](https://img.shields.io/github/license/jwodder/query-issues.svg)](https://opensource.org/licenses/MIT)

This is an experiment in determining the fastest way to use GitHub's GraphQL
API to fetch all open issues in all (public, non-archived, non-fork)
repositories belonging to a collection of owners/organizations.  Each binary
package in this workspace implements a different strategy:

- `orgs-then-issues` — Performs a paginated batch query to fetch all (public
  etc.) repositories belonging to each of the owners, including getting the
  number of open issues in each one.  Then, all repositories that have one or
  more open issues are queried in batches to get paginated lists of their open
  issues.

- `orgs-with-issues` — Performs a paginated batch query to fetch all (public
  etc.) repositories belonging to each of the owners; for each repository, the
  first page of open issues is also queried at the same time.  Then, all
  repositories that still have more open issues are queried repeatedly in
  batches to get their remaining pages of open isses.

  The key difference from `orgs-then-issues` is that this package fetches a
  page of issues for each repository as part of the same requests that fetch
  the repositories themselves.

- `update-issues` — This package creates or updates a JSON database of open
  issues on each run and only makes requests for issues that have been updated
  since the last run.  The program performs a paginated batch query to fetch
  all (public etc.) repositories belonging to each of the owners, including
  getting the number of open issues in each repository.  Then, all repositories
  that have one or more open issues are queried in batches to get paginated
  lists of their issues ordered by update time.  For repositories that had open
  issues when the database was last updated, this query fetches details on all
  issues, open & closed, that have been updated since the last run; for other
  repositories, only open issues are queried, but the query starts from the
  beginning of time.  The issues thus fetched are then added or (for closed
  issues) removed from the database as appropriate.

    - This strategy is unable to update the database to remove issues that have
      been deleted, transferred to another repository, or converted to
      discussions.
