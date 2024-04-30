This is an experiment in determining the fastest way to use GitHub's GraphQL
API to fetch all open issues in all (public, non-archived, non-fork)
repositories belonging to a collection of owners/organizations.

The strategies implemented by the binaries in this workspace are as follows:

- `orgs-with-issues` — Performs a paginated batch query to fetch all (public
  etc.) repositories belonging to each of the owners in question; for each
  repository, the first page of open issues is also queried at the same time.
  Then, all repositories that still have more open issues are queried
  repeatedly in batches to get their remaining pages of open isses.

- `update-issues` — This binary creates or updates a JSON database of open
  issues on each run and only makes requests for issues that have been updated
  since the last run.  On every execution, the program performs a paginated
  batch query to fetch all (public etc.) repositories belonging to each of the
  owners in question, including getting the number of open issues in each
  repository.  Then, all repositories that have one or more open issues are
  queried in batches to get paginated lists of their issues ordered by update
  time.  For repositories that had open issues when the database was last
  updated, this query fetches details on all issues, open and closed, that have
  been updated since the last run; for other repositories, only open issues are
  queried, but the query starts from the beginning of time.  The issues thus
  fetched are then added or (for closed issues) removed from the database as
  appropriate.

    - This strategy is unable to update the database to remove issues that have
      been deleted, transferred to another repository, or converted to
      discussions.
