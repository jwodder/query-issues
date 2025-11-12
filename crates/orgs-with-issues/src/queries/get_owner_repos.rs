use crate::types::RepoWithIssues;
use gqlient::{Cursor, Page, Paginator, QueryField, Singleton, Variable};
use indoc::indoc;
use std::fmt::{self, Write};
use std::num::NonZeroUsize;

/// A [`Paginator`] for retrieving public, non-archived, non-fork repositories
/// belonging to a given GitHub repository owner, along with the first page of
/// each repository's open issues, as pages of [`RepoWithIssues`] values
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetOwnerRepos {
    /// The repository owner (user or organization) for which to retrieve
    /// repositories
    owner: String,

    /// How many repositories to request per page
    page_size: NonZeroUsize,

    /// How many issue labels to request per page
    label_page_size: NonZeroUsize,
}

impl GetOwnerRepos {
    pub(crate) fn new(
        owner: String,
        page_size: NonZeroUsize,
        label_page_size: NonZeroUsize,
    ) -> GetOwnerRepos {
        GetOwnerRepos {
            owner,
            page_size,
            label_page_size,
        }
    }
}

impl Paginator for GetOwnerRepos {
    type Item = RepoWithIssues;
    type Query = GetOwnerReposQuery;

    fn for_cursor(&self, cursor: Option<&Cursor>) -> GetOwnerReposQuery {
        GetOwnerReposQuery::new(
            self.owner.clone(),
            cursor.cloned(),
            self.page_size,
            self.label_page_size,
        )
    }
}

/// A [`QueryField`] for retrieving a page of repositories and open issues (as
/// [`RepoWithIssues`] values) belonging to a given GitHub repository owner
/// starting at a given repository cursor
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GetOwnerReposQuery {
    /// The repository owner (user or organization) for which to retrieve
    /// repositories
    owner: String,

    /// The pagination cursor after which to retrieve repositories
    cursor: Option<Cursor>,

    /// How many repositories to request per page
    page_size: NonZeroUsize,

    /// How many issue labels to request per page
    label_page_size: NonZeroUsize,

    /// The prefix to prepend to the variable names, if any
    prefix: Option<String>,
}

impl GetOwnerReposQuery {
    fn new(
        owner: String,
        cursor: Option<Cursor>,
        page_size: NonZeroUsize,
        label_page_size: NonZeroUsize,
    ) -> GetOwnerReposQuery {
        GetOwnerReposQuery {
            owner,
            cursor,
            page_size,
            label_page_size,
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
    type Output = Page<RepoWithIssues>;

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
                        issues(
                            first: {page_size},
                            orderBy: {{field: CREATED_AT, direction: ASC}},
                            states: [OPEN],
                        ) {{
                            nodes {{
                                id
                                number
                                title
                                url
                                createdAt
                                updatedAt
                                labels (first: {label_page_size}) {{
                                    nodes {{
                                        name
                                    }}
                                    pageInfo {{
                                        endCursor
                                        hasNextPage
                                    }}
                                }}
                            }}
                            pageInfo {{
                                endCursor
                                hasNextPage
                            }}
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
            label_page_size = self.label_page_size,
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
