[![Project Status: Concept – Minimal or no implementation has been done yet, or the repository is only intended to be a limited example, demo, or proof-of-concept.](https://www.repostatus.org/badges/latest/concept.svg)](https://www.repostatus.org/#concept)
[![CI Status](https://github.com/jwodder/query-issues/actions/workflows/test.yml/badge.svg)](https://github.com/jwodder/query-issues/actions/workflows/test.yml)
[![codecov.io](https://codecov.io/gh/jwodder/query-issues/branch/main/graph/badge.svg)](https://codecov.io/gh/jwodder/query-issues)
[![Minimum Supported Rust Version](https://img.shields.io/badge/MSRV-1.88-orange)](https://www.rust-lang.org)
[![MIT License](https://img.shields.io/github/license/jwodder/query-issues.svg)](https://opensource.org/licenses/MIT)

This is an experiment in determining the most efficient way to use GitHub's
GraphQL API to fetch all open issues in all (public, non-archived, non-fork)
repositories belonging to a collection of users/organizations.  Each binary
package in this workspace implements a different strategy, as follows:

- [`orgs-then-issues`][] — fetches repositories, then issues

- [`orgs-with-issues`][] — fetches repositories along with each one's first page of
  issues, then fetches any additional issues

- [`repos-and-issues`][] — like `orgs-then-issues`, except it also fetches &
  outputs various details about all repositories

[`orgs-then-issues`]: https://github.com/jwodder/query-issues/tree/main/crates/orgs-then-issues
[`orgs-with-issues`]: https://github.com/jwodder/query-issues/tree/main/crates/orgs-with-issues
[`repos-and-issues`]: https://github.com/jwodder/query-issues/tree/main/crates/repos-and-issues
