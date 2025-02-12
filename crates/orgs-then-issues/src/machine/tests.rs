use super::*;
use assert_matches::assert_matches;
use gqlient::DEFAULT_BATCH_SIZE;
use indoc::indoc;
use pretty_assertions::assert_eq;

#[test]
fn no_owners() {
    let parameters = Parameters {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(100).unwrap(),
        label_page_size: NonZeroUsize::new(10).unwrap(),
    };
    let mut machine = OrgsThenIssues::new(Vec::new(), parameters);
    assert_eq!(machine.get_next_query(), None);
    assert_eq!(
        machine.get_output(),
        vec![Output::Report(FetchReport::default())]
    );
}

#[test]
fn no_repos() {
    let parameters = Parameters {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(100).unwrap(),
        label_page_size: NonZeroUsize::new(10).unwrap(),
    };
    let mut machine = OrgsThenIssues::new(vec!["octocat".into(), "achtkatze".into()], parameters);
    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_owner: String!, $q0_cursor: String, $q1_owner: String!, $q1_cursor: String) {
            q0: repositoryOwner(login: $q0_owner) {
                repositories(
                    orderBy: {field: NAME, direction: ASC},
                    ownerAffiliations: [OWNER],
                    isArchived: false,
                    isFork: false,
                    privacy: PUBLIC,
                    first: 100,
                    after: $q0_cursor,
                ) {
                    nodes {
                        id
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                    }
                    pageInfo {
                        endCursor
                        hasNextPage
                    }
                }
            }
            q1: repositoryOwner(login: $q1_owner) {
                repositories(
                    orderBy: {field: NAME, direction: ASC},
                    ownerAffiliations: [OWNER],
                    isArchived: false,
                    isFork: false,
                    privacy: PUBLIC,
                    first: 100,
                    after: $q1_cursor,
                ) {
                    nodes {
                        id
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                    }
                    pageInfo {
                        endCursor
                        hasNextPage
                    }
                }
            }
        }"}
    );
    assert_eq!(
        payload.variables,
        JsonMap::from_iter([
            ("q0_owner".into(), "octocat".into()),
            ("q0_cursor".into(), serde_json::Value::Null),
            ("q1_owner".into(), "achtkatze".into()),
            ("q1_cursor".into(), serde_json::Value::Null),
        ])
    );
    assert_eq!(
        machine.get_output(),
        vec![Output::Transition(Transition::StartFetchRepos)]
    );
    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
                "repositories": {
                    "nodes": [],
                    "pageInfo": {
                        "endCursor": null,
                        "hasNextPage": false,
                    }
                }
            }),
        ),
        (
            "q1".into(),
            serde_json::json!({
                "repositories": {
                    "nodes": [],
                    "pageInfo": {
                        "endCursor": null,
                        "hasNextPage": false,
                    }
                }
            }),
        ),
    ]);
    assert!(machine.handle_response(response).is_ok());
    assert_eq!(machine.get_next_query(), None);
    let outputs = machine.get_output();
    assert_eq!(outputs.len(), 2);
    assert_matches!(
        outputs[0],
        Output::Transition(Transition::EndFetchRepos {
            repositories: 0,
            repos_with_open_issues: 0,
            ..
        })
    );
    assert_eq!(outputs[1], Output::Report(FetchReport::default()));
}

#[test]
fn no_issues() {
    let parameters = Parameters {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(100).unwrap(),
        label_page_size: NonZeroUsize::new(10).unwrap(),
    };
    let mut machine = OrgsThenIssues::new(vec!["octocat".into(), "achtkatze".into()], parameters);
    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_owner: String!, $q0_cursor: String, $q1_owner: String!, $q1_cursor: String) {
            q0: repositoryOwner(login: $q0_owner) {
                repositories(
                    orderBy: {field: NAME, direction: ASC},
                    ownerAffiliations: [OWNER],
                    isArchived: false,
                    isFork: false,
                    privacy: PUBLIC,
                    first: 100,
                    after: $q0_cursor,
                ) {
                    nodes {
                        id
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                    }
                    pageInfo {
                        endCursor
                        hasNextPage
                    }
                }
            }
            q1: repositoryOwner(login: $q1_owner) {
                repositories(
                    orderBy: {field: NAME, direction: ASC},
                    ownerAffiliations: [OWNER],
                    isArchived: false,
                    isFork: false,
                    privacy: PUBLIC,
                    first: 100,
                    after: $q1_cursor,
                ) {
                    nodes {
                        id
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                    }
                    pageInfo {
                        endCursor
                        hasNextPage
                    }
                }
            }
        }"}
    );
    assert_eq!(
        payload.variables,
        JsonMap::from_iter([
            ("q0_owner".into(), "octocat".into()),
            ("q0_cursor".into(), serde_json::Value::Null),
            ("q1_owner".into(), "achtkatze".into()),
            ("q1_cursor".into(), serde_json::Value::Null),
        ])
    );
    assert_eq!(
        machine.get_output(),
        vec![Output::Transition(Transition::StartFetchRepos)]
    );
    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
                "repositories": {
                    "nodes": [
                        {
                            "id": "r1",
                            "nameWithOwner": "octocat/repo1",
                            "issues": {
                                "totalCount": 0,
                            }
                        },
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:end:octocat",
                        "hasNextPage": false,
                    }
                }
            }),
        ),
        (
            "q1".into(),
            serde_json::json!({
                "repositories": {
                    "nodes": [
                        {
                            "id": "r2",
                            "nameWithOwner": "achtkatze/repo2",
                            "issues": {
                                "totalCount": 0,
                            }
                        },
                        {
                            "id": "r3",
                            "nameWithOwner": "achtkatze/repo3",
                            "issues": {
                                "totalCount": 0,
                            }
                        },
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:end:achtkatze",
                        "hasNextPage": false,
                    }
                }
            }),
        ),
    ]);
    assert!(machine.handle_response(response).is_ok());
    assert_eq!(machine.get_next_query(), None);
    let outputs = machine.get_output();
    assert_eq!(outputs.len(), 2);
    assert_matches!(
        outputs[0],
        Output::Transition(Transition::EndFetchRepos {
            repositories: 3,
            repos_with_open_issues: 0,
            ..
        })
    );
    assert_eq!(
        outputs[1],
        Output::Report(FetchReport {
            repositories: 3,
            ..FetchReport::default()
        })
    );
}

#[test]
fn issues() {
    let parameters = Parameters {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(100).unwrap(),
        label_page_size: NonZeroUsize::new(10).unwrap(),
    };
    let mut machine = OrgsThenIssues::new(vec!["octocat".into(), "achtkatze".into()], parameters);
    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_owner: String!, $q0_cursor: String, $q1_owner: String!, $q1_cursor: String) {
            q0: repositoryOwner(login: $q0_owner) {
                repositories(
                    orderBy: {field: NAME, direction: ASC},
                    ownerAffiliations: [OWNER],
                    isArchived: false,
                    isFork: false,
                    privacy: PUBLIC,
                    first: 100,
                    after: $q0_cursor,
                ) {
                    nodes {
                        id
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                    }
                    pageInfo {
                        endCursor
                        hasNextPage
                    }
                }
            }
            q1: repositoryOwner(login: $q1_owner) {
                repositories(
                    orderBy: {field: NAME, direction: ASC},
                    ownerAffiliations: [OWNER],
                    isArchived: false,
                    isFork: false,
                    privacy: PUBLIC,
                    first: 100,
                    after: $q1_cursor,
                ) {
                    nodes {
                        id
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                    }
                    pageInfo {
                        endCursor
                        hasNextPage
                    }
                }
            }
        }"}
    );
    assert_eq!(
        payload.variables,
        JsonMap::from_iter([
            ("q0_owner".into(), "octocat".into()),
            ("q0_cursor".into(), serde_json::Value::Null),
            ("q1_owner".into(), "achtkatze".into()),
            ("q1_cursor".into(), serde_json::Value::Null),
        ])
    );
    assert_eq!(
        machine.get_output(),
        vec![Output::Transition(Transition::StartFetchRepos)]
    );

    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
                "repositories": {
                    "nodes": [
                        {
                            "id": "r1",
                            "nameWithOwner": "octocat/repo1",
                            "issues": {
                                "totalCount": 3,
                            }
                        },
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:end:octocat",
                        "hasNextPage": false,
                    }
                }
            }),
        ),
        (
            "q1".into(),
            serde_json::json!({
                "repositories": {
                    "nodes": [
                        {
                            "id": "r2",
                            "nameWithOwner": "achtkatze/repo2",
                            "issues": {
                                "totalCount": 0,
                            }
                        },
                        {
                            "id": "r3",
                            "nameWithOwner": "achtkatze/repo3",
                            "issues": {
                                "totalCount": 2,
                            }
                        },
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:end:achtkatze",
                        "hasNextPage": false,
                    }
                }
            }),
        ),
    ]);
    assert!(machine.handle_response(response).is_ok());

    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_repo_id: ID!, $q0_cursor: String, $q1_repo_id: ID!, $q1_cursor: String) {
            q0: node(id: $q0_repo_id) {
                ... on Repository {
                    nameWithOwner
                    issues(
                        first: 100,
                        after: $q0_cursor,
                        orderBy: {field: CREATED_AT, direction: ASC},
                        states: [OPEN],
                    ) {
                        nodes {
                            id
                            number
                            title
                            url
                            createdAt
                            updatedAt
                            labels (first: 10) {
                                nodes {
                                    name
                                }
                                pageInfo {
                                    endCursor
                                    hasNextPage
                                }
                            }
                        }
                        pageInfo {
                            endCursor
                            hasNextPage
                        }
                    }
                }
            }
            q1: node(id: $q1_repo_id) {
                ... on Repository {
                    nameWithOwner
                    issues(
                        first: 100,
                        after: $q1_cursor,
                        orderBy: {field: CREATED_AT, direction: ASC},
                        states: [OPEN],
                    ) {
                        nodes {
                            id
                            number
                            title
                            url
                            createdAt
                            updatedAt
                            labels (first: 10) {
                                nodes {
                                    name
                                }
                                pageInfo {
                                    endCursor
                                    hasNextPage
                                }
                            }
                        }
                        pageInfo {
                            endCursor
                            hasNextPage
                        }
                    }
                }
            }
        }"}
    );
    assert_eq!(
        payload.variables,
        JsonMap::from_iter([
            ("q0_repo_id".into(), "r1".into()),
            ("q0_cursor".into(), serde_json::Value::Null),
            ("q1_repo_id".into(), "r3".into()),
            ("q1_cursor".into(), serde_json::Value::Null),
        ])
    );

    let outputs = machine.get_output();
    assert_eq!(outputs.len(), 2);
    assert_matches!(
        outputs[0],
        Output::Transition(Transition::EndFetchRepos {
            repositories: 3,
            repos_with_open_issues: 2,
            ..
        })
    );
    assert_matches!(
        outputs[1],
        Output::Transition(Transition::StartFetchIssues {
            repos_with_open_issues: 2,
        })
    );

    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
                "nameWithOwner": "octocat/repo1",
                "issues": {
                    "nodes": [
                        {
                            "id": "i1",
                            "number": 1,
                            "title": "I found a bug!",
                            "url": "https://example.github/octocat/repo1/issues/1",
                            "createdAt": "2020-01-01T00:00:00Z",
                            "updatedAt": "2020-01-01T00:00:00Z",
                            "labels": {
                                "nodes": [
                                    {"name": "bug"}
                                ],
                                "pageInfo": {
                                    "endCursor": "cursor:end:octocat/repo1:1",
                                    "hasNextPage": false
                                }
                            }
                        },
                        {
                            "id": "i2",
                            "number": 17,
                            "title": "How do I use this code?",
                            "url": "https://example.github/octocat/repo1/issues/17",
                            "createdAt": "2020-02-01T00:00:00Z",
                            "updatedAt": "2021-01-01T00:00:00Z",
                            "labels": {
                                "nodes": [
                                    {"name": "question"},
                                    {"name": "PEBKAC"}
                                ],
                                "pageInfo": {
                                    "endCursor": "cursor:end:octocat/repo1:17",
                                    "hasNextPage": false
                                }
                            }
                        },
                        {
                            "id": "i3",
                            "number": 42,
                            "title": "Idea to make code better",
                            "url": "https://example.github/octocat/repo1/issues/42",
                            "createdAt": "2021-01-01T00:00:00Z",
                            "updatedAt": "2022-01-01T00:00:00Z",
                            "labels": {
                                "nodes": [
                                    {"name": "enhancement"}
                                ],
                                "pageInfo": {
                                    "endCursor": "cursor:end:octocat/repo1:42",
                                    "hasNextPage": false
                                }
                            }
                        },
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:end:octocat/repo1",
                        "hasNextPage": false,
                    }
                }
            }),
        ),
        (
            "q1".into(),
            serde_json::json!({
                "nameWithOwner": "achtkatze/repo3",
                "issues": {
                    "nodes": [
                        {
                            "id": "i4",
                            "number": 23,
                            "title": "Why are we speaking German?",
                            "url": "https://example.github/achtkatze/repo3/issues/23",
                            "createdAt": "2020-06-15T12:34:56Z",
                            "updatedAt": "2020-06-15T12:34:56Z",
                            "labels": {
                                "nodes": [
                                    {"name": "german"},
                                    {"name": "language"},
                                    {"name": "question"},
                                ],
                                "pageInfo": {
                                    "endCursor": "cursor:end:achtkatze/repo3:23",
                                    "hasNextPage": false
                                }
                            }
                        },
                        {
                            "id": "i5",
                            "number": 24,
                            "title": "Wenn ist das Nunstück git und Slotermeyer?",
                            "url": "https://example.github/achtkatze/repo3/issues/24",
                            "createdAt": "2020-06-16T00:00:00Z",
                            "updatedAt": "2020-06-16T00:00:00Z",
                            "labels": {
                                "nodes": [
                                    {"name": "german"},
                                    {"name": "funny"}
                                ],
                                "pageInfo": {
                                    "endCursor": "cursor:end:achtkatze/repo3:24",
                                    "hasNextPage": false
                                }
                            }
                        },
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:end:achtkatze/repo3",
                        "hasNextPage": false,
                    }
                }
            }),
        ),
    ]);

    assert!(machine.handle_response(response).is_ok());
    assert_eq!(machine.get_next_query(), None);

    let outputs = machine.get_output();
    assert_eq!(outputs.len(), 3);
    assert_eq!(
        outputs[0],
        Output::Issues(vec![
            Issue {
                repo: "octocat/repo1".into(),
                number: 1,
                title: "I found a bug!".into(),
                labels: vec!["bug".into()],
                url: "https://example.github/octocat/repo1/issues/1".into(),
                created: "2020-01-01T00:00:00Z".into(),
                updated: "2020-01-01T00:00:00Z".into(),
            },
            Issue {
                repo: "octocat/repo1".into(),
                number: 17,
                title: "How do I use this code?".into(),
                labels: vec!["question".into(), "PEBKAC".into()],
                url: "https://example.github/octocat/repo1/issues/17".into(),
                created: "2020-02-01T00:00:00Z".into(),
                updated: "2021-01-01T00:00:00Z".into(),
            },
            Issue {
                repo: "octocat/repo1".into(),
                number: 42,
                title: "Idea to make code better".into(),
                labels: vec!["enhancement".into()],
                url: "https://example.github/octocat/repo1/issues/42".into(),
                created: "2021-01-01T00:00:00Z".into(),
                updated: "2022-01-01T00:00:00Z".into(),
            },
            Issue {
                repo: "achtkatze/repo3".into(),
                number: 23,
                title: "Why are we speaking German?".into(),
                labels: vec!["german".into(), "language".into(), "question".into()],
                url: "https://example.github/achtkatze/repo3/issues/23".into(),
                created: "2020-06-15T12:34:56Z".into(),
                updated: "2020-06-15T12:34:56Z".into(),
            },
            Issue {
                repo: "achtkatze/repo3".into(),
                number: 24,
                title: "Wenn ist das Nunstück git und Slotermeyer?".into(),
                labels: vec!["german".into(), "funny".into()],
                url: "https://example.github/achtkatze/repo3/issues/24".into(),
                created: "2020-06-16T00:00:00Z".into(),
                updated: "2020-06-16T00:00:00Z".into(),
            },
        ])
    );
    assert_matches!(
        outputs[1],
        Output::Transition(Transition::EndFetchIssues { open_issues: 5, .. })
    );
    assert_eq!(
        outputs[2],
        Output::Report(FetchReport {
            repositories: 3,
            repos_with_open_issues: 2,
            open_issues: 5,
            ..FetchReport::default()
        })
    );
}

// multiple pages of repos
// multiple pages of issues
// issues with extra labels
// issues with multiple pages of labels
