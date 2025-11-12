use super::GetIssues;
use gqlient::{Cursor, Id, Page, Paginator, QueryField, Singleton, Variable};
use indoc::indoc;
use serde::Deserialize;
use std::fmt::{self, Write};
use std::num::NonZeroUsize;

/// A [`Paginator`] for retrieving public, non-archived, non-fork repositories
/// belonging to a given GitHub repository owner as pages of [`Repository`]
/// values
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetOwnerRepos {
    /// The repository owner (user or organization) for which to retrieve
    /// repositories
    owner: String,

    /// How many repositories to request per page
    page_size: NonZeroUsize,
}

impl GetOwnerRepos {
    pub(crate) fn new(owner: String, page_size: NonZeroUsize) -> GetOwnerRepos {
        GetOwnerRepos { owner, page_size }
    }
}

impl Paginator for GetOwnerRepos {
    type Item = Repository;
    type Query = GetOwnerReposQuery;

    fn for_cursor(&self, cursor: Option<&Cursor>) -> GetOwnerReposQuery {
        GetOwnerReposQuery::new(self.owner.clone(), cursor.cloned(), self.page_size)
    }
}

/// A [`QueryField`] for retrieving a page of repositories (as [`Repository`]
/// values) belonging to a given GitHub repository owner starting at a given
/// cursor
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetOwnerReposQuery {
    /// The repository owner (user or organization) for which to retrieve
    /// repositories
    owner: String,

    /// The pagination cursor after which to retrieve repositories
    cursor: Option<Cursor>,

    /// How many repositories to request per page
    page_size: NonZeroUsize,

    /// The prefix to prepend to the variable names, if any
    prefix: Option<String>,
}

impl GetOwnerReposQuery {
    fn new(owner: String, cursor: Option<Cursor>, page_size: NonZeroUsize) -> GetOwnerReposQuery {
        GetOwnerReposQuery {
            owner,
            cursor,
            page_size,
            prefix: None,
        }
    }

    /// Returns the name of the GraphQL variable used to refer to the
    /// repository owner, including any added prefixes
    fn owner_varname(&self) -> String {
        match self.prefix {
            Some(ref prefix) => format!("{prefix}_owner"),
            None => String::from("owner"),
        }
    }

    /// Returns the name of the GraphQL variable used to refer to the
    /// repository cursor, including any added prefixes
    fn cursor_varname(&self) -> String {
        match self.prefix {
            Some(ref prefix) => format!("{prefix}_cursor"),
            None => String::from("cursor"),
        }
    }
}

impl QueryField for GetOwnerReposQuery {
    type Output = Page<Repository>;

    fn with_variable_prefix(mut self, prefix: String) -> Self {
        let new_prefix = match self.prefix {
            Some(p0) => format!("{prefix}_{p0}"),
            None => prefix,
        };
        self.prefix = Some(new_prefix);
        self
    }

    fn write_field<W: Write>(&self, mut s: W) -> fmt::Result {
        write!(
            s,
            indoc! {"
            repositoryOwner(login: ${owner_varname}) {{
                repositories(
                    orderBy: {{field: NAME, direction: ASC}},
                    ownerAffiliations: [OWNER],
                    isArchived: false,
                    isFork: false,
                    privacy: PUBLIC,
                    first: {page_size},
                    after: ${cursor_varname},
                ) {{
                    nodes {{
                        id
                        nameWithOwner
                        issues(states: [OPEN]) {{
                            totalCount
                        }}
                    }}
                    pageInfo {{
                        endCursor
                        hasNextPage
                    }}
                }}
            }}
        "},
            owner_varname = self.owner_varname(),
            cursor_varname = self.cursor_varname(),
            page_size = self.page_size,
        )
    }

    fn variables(&self) -> [(String, Variable); 2] {
        [
            (
                self.owner_varname(),
                Variable {
                    gql_type: String::from("String!"),
                    value: self.owner.clone().into(),
                },
            ),
            (
                self.cursor_varname(),
                Variable {
                    gql_type: String::from("String"),
                    value: self.cursor.clone().into(),
                },
            ),
        ]
    }

    fn parse_response(&self, value: serde_json::Value) -> Result<Self::Output, serde_json::Error> {
        serde_json::from_value::<Singleton<Self::Output>>(value).map(|r| r.0)
    }
}

/// Information on a GitHub repository retrieved by a [`GetOwnerRepos`]
/// paginator or [`GetOwnerReposQuery`] query field
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub(crate) struct Repository {
    /// The repository's GraphQL node ID
    pub(crate) id: Id,

    /// The repository's full name in the form "OWNER/NAME"
    #[serde(rename = "nameWithOwner")]
    pub(crate) fullname: String,

    /// The number of open issues currently in the repository
    #[serde(rename = "issues", deserialize_with = "gqlient::singleton_field")]
    pub(crate) open_issues: u64,
}

impl Repository {
    /// If this repository has any open issues, return its ID and a
    /// [`GetIssues`] instance for retrieving the open issues
    pub(crate) fn issues_query(
        &self,
        page_size: NonZeroUsize,
        label_page_size: NonZeroUsize,
    ) -> Option<(Id, GetIssues)> {
        (self.open_issues > 0).then(|| {
            (
                self.id.clone(),
                GetIssues::new(self.id.clone(), page_size, label_page_size),
            )
        })
    }
}
