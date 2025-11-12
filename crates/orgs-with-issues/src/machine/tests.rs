use super::*;
use assert_matches::assert_matches;
use gqlient::DEFAULT_BATCH_SIZE;
use indoc::indoc;
use pretty_assertions::assert_eq;

#[test]
fn no_owners() {
    let parameters = QueryLimits {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(100).unwrap(),
        label_page_size: NonZeroUsize::new(10).unwrap(),
    };
    let mut machine = OrgsWithIssues::new(Vec::new(), parameters);
    assert_eq!(machine.get_next_query(), None);
    assert_eq!(
        machine.get_output(),
        vec![Output::Report(FetchReport::default())]
    );
}

#[test]
fn no_repos() {
    let parameters = QueryLimits {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(100).unwrap(),
        label_page_size: NonZeroUsize::new(10).unwrap(),
    };
    let mut machine = OrgsWithIssues::new(vec!["octocat".into(), "achtkatze".into()], parameters);

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
                        issues(
                            first: 100,
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
                        issues(
                            first: 100,
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

    assert!(machine.get_output().is_empty());

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
    let parameters = QueryLimits {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(100).unwrap(),
        label_page_size: NonZeroUsize::new(10).unwrap(),
    };
    let mut machine = OrgsWithIssues::new(vec!["octocat".into(), "achtkatze".into()], parameters);

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
                        issues(
                            first: 100,
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
                        issues(
                            first: 100,
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
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
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
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                        {
                            "id": "r3",
                            "nameWithOwner": "achtkatze/repo3",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
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

    assert!(machine.get_output().is_empty());

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
    let parameters = QueryLimits {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(100).unwrap(),
        label_page_size: NonZeroUsize::new(10).unwrap(),
    };
    let mut machine = OrgsWithIssues::new(vec!["octocat".into(), "achtkatze".into()], parameters);

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
                        issues(
                            first: 100,
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
                        issues(
                            first: 100,
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
                                },
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
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                }
                            }
                        },
                        {
                            "id": "r3",
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

    let outputs = machine.get_output();
    assert_eq!(
        outputs,
        vec![Output::Issues(vec![
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
        ])]
    );

    assert_eq!(machine.get_next_query(), None);

    let outputs = machine.get_output();
    assert_eq!(outputs.len(), 2);
    assert_matches!(
        outputs[0],
        Output::Transition(Transition::EndFetchRepos {
            repositories: 3,
            repos_with_open_issues: 2,
            open_issues: 5,
            ..
        })
    );
    assert_eq!(
        outputs[1],
        Output::Report(FetchReport {
            repositories: 3,
            repos_with_open_issues: 2,
            open_issues: 5,
            ..FetchReport::default()
        })
    );
}

#[test]
fn extra_labels() {
    let parameters = QueryLimits {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(100).unwrap(),
        label_page_size: NonZeroUsize::new(5).unwrap(),
    };
    let mut machine = OrgsWithIssues::new(vec!["monocat".into()], parameters);

    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_owner: String!, $q0_cursor: String) {
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
                        issues(
                            first: 100,
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
                                labels (first: 5) {
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
            ("q0_owner".into(), "monocat".into()),
            ("q0_cursor".into(), serde_json::Value::Null),
        ])
    );

    assert_eq!(
        machine.get_output(),
        vec![Output::Transition(Transition::StartFetchRepos)]
    );

    let response = JsonMap::from_iter([(
        "q0".into(),
        serde_json::json!({
            "repositories": {
                "nodes": [
                    {
                        "id": "r1",
                        "nameWithOwner": "monocat/rainbow",
                        "issues": {
                            "nodes": [
                                {
                                    "id": "i1",
                                    "number": 1,
                                    "title": "We need more colors",
                                    "url": "https://example.github/monocat/rainbow/issues/1",
                                    "createdAt": "2020-01-01T00:00:00Z",
                                    "updatedAt": "2020-01-01T00:00:00Z",
                                    "labels": {
                                        "nodes": [
                                            {"name": "colors"},
                                            {"name": "red"},
                                            {"name": "orange"},
                                            {"name": "yellow"},
                                            {"name": "green"},
                                        ],
                                        "pageInfo": {
                                            "endCursor": "cursor:p1:monocat/rainbow:1",
                                            "hasNextPage": true
                                        }
                                    }
                                },
                            ],
                            "pageInfo": {
                                "endCursor": "cursor:end:monocat/rainbow",
                                "hasNextPage": false,
                            }
                        }
                    },
                ],
                "pageInfo": {
                    "endCursor": "cursor:end:monocat",
                    "hasNextPage": false,
                }
            }
        }),
    )]);
    assert!(machine.handle_response(response).is_ok());

    assert!(machine.get_output().is_empty());

    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_issue_id: ID!, $q0_cursor: String) {
            q0: node(id: $q0_issue_id) {
                ... on Issue {
                    labels(
                        first: 5,
                        after: $q0_cursor,
                    ) {
                        nodes {
                            name
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
            ("q0_issue_id".into(), "i1".into()),
            ("q0_cursor".into(), "cursor:p1:monocat/rainbow:1".into()),
        ])
    );

    let outputs = machine.get_output();
    assert_eq!(outputs.len(), 2);
    assert_matches!(
        outputs[0],
        Output::Transition(Transition::EndFetchRepos {
            repositories: 1,
            repos_with_open_issues: 1,
            ..
        })
    );
    assert_matches!(
        outputs[1],
        Output::Transition(Transition::StartFetchLabels {
            issues_with_extra_labels: 1,
        })
    );

    let response = JsonMap::from_iter([(
        "q0".into(),
        serde_json::json!({
            "labels": {
                "nodes": [
                    {"name": "blue"},
                    {"name": "indigo"},
                    {"name": "violet"},
                ],
                "pageInfo": {
                    "endCursor": "cursor:end:monocat/rainbow:1",
                    "hasNextPage": false,
                }
            }
        }),
    )]);
    assert!(machine.handle_response(response).is_ok());

    assert!(machine.get_output().is_empty());

    assert_eq!(machine.get_next_query(), None);

    let outputs = machine.get_output();
    assert_eq!(outputs.len(), 3);
    assert_matches!(
        outputs[0],
        Output::Transition(Transition::EndFetchLabels {
            extra_labels: 3,
            ..
        })
    );
    assert_eq!(
        outputs[1],
        Output::Issues(vec![Issue {
            repo: "monocat/rainbow".into(),
            number: 1,
            title: "We need more colors".into(),
            labels: vec![
                "colors".into(),
                "red".into(),
                "orange".into(),
                "yellow".into(),
                "green".into(),
                "blue".into(),
                "indigo".into(),
                "violet".into()
            ],
            url: "https://example.github/monocat/rainbow/issues/1".into(),
            created: "2020-01-01T00:00:00Z".into(),
            updated: "2020-01-01T00:00:00Z".into(),
        },])
    );
    assert_eq!(
        outputs[2],
        Output::Report(FetchReport {
            repositories: 1,
            repos_with_open_issues: 1,
            repos_with_extra_issues: 0,
            open_issues: 1,
            issues_with_extra_labels: 1,
            extra_issues: 0,
            extra_labels: 3,
        })
    );
}

#[test]
fn multiple_pages() {
    let parameters = QueryLimits {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(5).unwrap(),
        label_page_size: NonZeroUsize::new(5).unwrap(),
    };
    let mut machine = OrgsWithIssues::new(vec!["quadcat".into(), "ochocat".into()], parameters);

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
                    first: 5,
                    after: $q0_cursor,
                ) {
                    nodes {
                        id
                        nameWithOwner
                        issues(
                            first: 5,
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
                                labels (first: 5) {
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
                    first: 5,
                    after: $q1_cursor,
                ) {
                    nodes {
                        id
                        nameWithOwner
                        issues(
                            first: 5,
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
                                labels (first: 5) {
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
            ("q0_owner".into(), "quadcat".into()),
            ("q0_cursor".into(), serde_json::Value::Null),
            ("q1_owner".into(), "ochocat".into()),
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
                            "nameWithOwner": "quadcat/code-only",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                        {
                            "id": "r2",
                            "nameWithOwner": "quadcat/empty",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                        {
                            "id": "r3",
                            "nameWithOwner": "quadcat/example",
                            "issues": {
                                "nodes": [
                                    {
                                        "id": "i1",
                                        "number": 1,
                                        "title": "My First Issue™",
                                        "url": "https://example.github/quadcat/example/issues/1",
                                        "createdAt": "2020-01-01T00:00:00Z",
                                        "updatedAt": "2020-01-01T00:00:00Z",
                                        "labels": {
                                            "nodes": [
                                                {"name": "example"},
                                                {"name": "test"},
                                                {"name": "testing"},
                                                {"name": "sample"},
                                                {"name": "first issue"},
                                            ],
                                            "pageInfo": {
                                                "endCursor": "cursor:p1:quadcat/example:1",
                                                "hasNextPage": true
                                            }
                                        }
                                    },
                                    {
                                        "id": "i2",
                                        "number": 2,
                                        "title": "My Second Issue™",
                                        "url": "https://example.github/quadcat/example/issues/2",
                                        "createdAt": "2020-02-14T23:46:10Z",
                                        "updatedAt": "2020-02-14T23:46:10Z",
                                        "labels": {
                                            "nodes": [
                                                {"name": "example"},
                                                {"name": "test"},
                                                {"name": "not as exciting as the first"},
                                            ],
                                            "pageInfo": {
                                                "endCursor": "cursor:end:quadcat/example:2",
                                                "hasNextPage": false
                                            }
                                        }
                                    },
                                    {
                                        "id": "i3",
                                        "number": 3,
                                        "title": "My Third Issue™",
                                        "url": "https://example.github/quadcat/example/issues/3",
                                        "createdAt": "2020-07-10T18:38:04Z",
                                        "updatedAt": "2020-07-10T18:38:04Z",
                                        "labels": {
                                            "nodes": [
                                                {"name": "example"},
                                                {"name": "test"},
                                                {"name": "diminishing returns"},
                                            ],
                                            "pageInfo": {
                                                "endCursor": "cursor:end:quadcat/example:3",
                                                "hasNextPage": false
                                            }
                                        }
                                    },
                                    {
                                        "id": "i4",
                                        "number": 4,
                                        "title": "My Fourth Issue™",
                                        "url": "https://example.github/quadcat/example/issues/4",
                                        "createdAt": "2022-04-29T07:51:44Z",
                                        "updatedAt": "2022-04-29T07:51:44Z",
                                        "labels": {
                                            "nodes": [
                                                {"name": "example"},
                                                {"name": "test"},
                                                {"name": "getting kind of bored"},
                                            ],
                                            "pageInfo": {
                                                "endCursor": "cursor:end:quadcat/example:4",
                                                "hasNextPage": false
                                            }
                                        }
                                    },
                                    {
                                        "id": "i5",
                                        "number": 5,
                                        "title": "My Fifth Issue™",
                                        "url": "https://example.github/quadcat/example/issues/5",
                                        "createdAt": "2022-11-22T19:10:13Z",
                                        "updatedAt": "2022-11-22T19:10:13Z",
                                        "labels": {
                                            "nodes": [
                                                {"name": "example"},
                                                {"name": "test"},
                                                {"name": "so bored now"},
                                            ],
                                            "pageInfo": {
                                                "endCursor": "cursor:end:quadcat/example:5",
                                                "hasNextPage": false
                                            }
                                        }
                                    },
                                ],
                                "pageInfo": {
                                    "endCursor": "cursor:p1:quadcat/example",
                                    "hasNextPage": true,
                                },
                            }
                        },
                        {
                            "id": "r4",
                            "nameWithOwner": "quadcat/nil",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                        {
                            "id": "r5",
                            "nameWithOwner": "quadcat/no-issues",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:p1:quadcat",
                        "hasNextPage": true,
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
                            "id": "r6",
                            "nameWithOwner": "ochocat/black",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                        {
                            "id": "r7",
                            "nameWithOwner": "ochocat/blue",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                        {
                            "id": "r8",
                            "nameWithOwner": "ochocat/cyan",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                        {
                            "id": "r9",
                            "nameWithOwner": "ochocat/fuchsia",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                        {
                            "id": "r10",
                            "nameWithOwner": "ochocat/green",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:p1:ochocat",
                        "hasNextPage": true,
                    }
                }
            }),
        ),
    ]);
    assert!(machine.handle_response(response).is_ok());

    assert!(machine.get_output().is_empty());

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
                    first: 5,
                    after: $q0_cursor,
                ) {
                    nodes {
                        id
                        nameWithOwner
                        issues(
                            first: 5,
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
                                labels (first: 5) {
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
                    first: 5,
                    after: $q1_cursor,
                ) {
                    nodes {
                        id
                        nameWithOwner
                        issues(
                            first: 5,
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
                                labels (first: 5) {
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
            ("q0_owner".into(), "quadcat".into()),
            ("q0_cursor".into(), "cursor:p1:quadcat".into()),
            ("q1_owner".into(), "ochocat".into()),
            ("q1_cursor".into(), "cursor:p1:ochocat".into()),
        ])
    );

    assert!(machine.get_output().is_empty());

    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
                "repositories": {
                    "nodes": [
                        {
                            "id": "r11",
                            "nameWithOwner": "quadcat/nothing",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                        {
                            "id": "r12",
                            "nameWithOwner": "quadcat/test",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:end:quadcat",
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
                            "id": "r13",
                            "nameWithOwner": "ochocat/grey",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                        {
                            "id": "r14",
                            "nameWithOwner": "ochocat/indigo",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                        {
                            "id": "r15",
                            "nameWithOwner": "ochocat/magenta",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                        {
                            "id": "r16",
                            "nameWithOwner": "ochocat/orange",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                        {
                            "id": "r17",
                            "nameWithOwner": "ochocat/purple",
                            "issues": {
                                "nodes": [],
                                "pageInfo": {
                                    "endCursor": null,
                                    "hasNextPage": false,
                                },
                            }
                        },
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:p2:ochocat",
                        "hasNextPage": true,
                    }
                }
            }),
        ),
    ]);
    assert!(machine.handle_response(response).is_ok());

    let outputs = machine.get_output();
    assert_eq!(
        outputs,
        vec![Output::Issues(vec![
            Issue {
                repo: "quadcat/example".into(),
                number: 2,
                title: "My Second Issue™".into(),
                labels: vec![
                    "example".into(),
                    "test".into(),
                    "not as exciting as the first".into(),
                ],
                url: "https://example.github/quadcat/example/issues/2".into(),
                created: "2020-02-14T23:46:10Z".into(),
                updated: "2020-02-14T23:46:10Z".into(),
            },
            Issue {
                repo: "quadcat/example".into(),
                number: 3,
                title: "My Third Issue™".into(),
                labels: vec![
                    "example".into(),
                    "test".into(),
                    "diminishing returns".into(),
                ],
                url: "https://example.github/quadcat/example/issues/3".into(),
                created: "2020-07-10T18:38:04Z".into(),
                updated: "2020-07-10T18:38:04Z".into(),
            },
            Issue {
                repo: "quadcat/example".into(),
                number: 4,
                title: "My Fourth Issue™".into(),
                labels: vec![
                    "example".into(),
                    "test".into(),
                    "getting kind of bored".into(),
                ],
                url: "https://example.github/quadcat/example/issues/4".into(),
                created: "2022-04-29T07:51:44Z".into(),
                updated: "2022-04-29T07:51:44Z".into(),
            },
            Issue {
                repo: "quadcat/example".into(),
                number: 5,
                title: "My Fifth Issue™".into(),
                labels: vec!["example".into(), "test".into(), "so bored now".into(),],
                url: "https://example.github/quadcat/example/issues/5".into(),
                created: "2022-11-22T19:10:13Z".into(),
                updated: "2022-11-22T19:10:13Z".into(),
            },
        ])]
    );

    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_owner: String!, $q0_cursor: String) {
            q0: repositoryOwner(login: $q0_owner) {
                repositories(
                    orderBy: {field: NAME, direction: ASC},
                    ownerAffiliations: [OWNER],
                    isArchived: false,
                    isFork: false,
                    privacy: PUBLIC,
                    first: 5,
                    after: $q0_cursor,
                ) {
                    nodes {
                        id
                        nameWithOwner
                        issues(
                            first: 5,
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
                                labels (first: 5) {
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
            ("q0_owner".into(), "ochocat".into()),
            ("q0_cursor".into(), "cursor:p2:ochocat".into()),
        ])
    );

    assert!(machine.get_output().is_empty());

    let response = JsonMap::from_iter([(
        "q0".into(),
        serde_json::json!({
            "repositories": {
                "nodes": [
                    {
                        "id": "r18",
                        "nameWithOwner": "ochocat/rainbow",
                        "issues": {
                            "nodes": [
                                {
                                    "id": "i6",
                                    "number": 1,
                                    "title": "The red isn't red enough",
                                    "url": "https://example.github/ochocat/rainbow/issues/1",
                                    "createdAt": "2020-01-01T00:00:00Z",
                                    "updatedAt": "2020-01-01T00:00:00Z",
                                    "labels": {
                                        "nodes": [
                                            {"name": "colors"},
                                            {"name": "red"},
                                        ],
                                        "pageInfo": {
                                            "endCursor": "cursor:end:ochocat/rainbow:1",
                                            "hasNextPage": false
                                        }
                                    }
                                },
                                {
                                    "id": "i7",
                                    "number": 2,
                                    "title": "The orange isn't orange enough",
                                    "url": "https://example.github/ochocat/rainbow/issues/2",
                                    "createdAt": "2020-01-01T00:01:00Z",
                                    "updatedAt": "2020-01-01T00:01:00Z",
                                    "labels": {
                                        "nodes": [
                                            {"name": "colors"},
                                            {"name": "orange"},
                                        ],
                                        "pageInfo": {
                                            "endCursor": "cursor:end:ochocat/rainbow:2",
                                            "hasNextPage": false
                                        }
                                    }
                                },
                                {
                                    "id": "i8",
                                    "number": 3,
                                    "title": "The yellow isn't yellow enough",
                                    "url": "https://example.github/ochocat/rainbow/issues/3",
                                    "createdAt": "2020-01-01T00:02:00Z",
                                    "updatedAt": "2020-01-01T00:02:00Z",
                                    "labels": {
                                        "nodes": [
                                            {"name": "colors"},
                                            {"name": "yellow"},
                                        ],
                                        "pageInfo": {
                                            "endCursor": "cursor:end:ochocat/rainbow:3",
                                            "hasNextPage": false
                                        }
                                    }
                                },
                                {
                                    "id": "i9",
                                    "number": 4,
                                    "title": "The green isn't green enough",
                                    "url": "https://example.github/ochocat/rainbow/issues/4",
                                    "createdAt": "2020-01-01T00:03:00Z",
                                    "updatedAt": "2020-01-01T00:03:00Z",
                                    "labels": {
                                        "nodes": [
                                            {"name": "colors"},
                                            {"name": "green"},
                                        ],
                                        "pageInfo": {
                                            "endCursor": "cursor:end:ochocat/rainbow:4",
                                            "hasNextPage": false
                                        }
                                    }
                                },
                                {
                                    "id": "i10",
                                    "number": 5,
                                    "title": "The blue isn't blue enough",
                                    "url": "https://example.github/ochocat/rainbow/issues/5",
                                    "createdAt": "2020-01-01T00:04:00Z",
                                    "updatedAt": "2020-01-01T00:04:00Z",
                                    "labels": {
                                        "nodes": [
                                            {"name": "colors"},
                                            {"name": "blue"},
                                        ],
                                        "pageInfo": {
                                            "endCursor": "cursor:end:ochocat/rainbow:5",
                                            "hasNextPage": false
                                        }
                                    }
                                },
                            ],
                            "pageInfo": {
                                "endCursor": "cursor:p1:ochocat/rainbow",
                                "hasNextPage": true,
                            },
                        }
                    },
                    {
                        "id": "r19",
                        "nameWithOwner": "ochocat/red",
                        "issues": {
                            "nodes": [],
                            "pageInfo": {
                                "endCursor": null,
                                "hasNextPage": false,
                            },
                        }
                    },
                    {
                        "id": "r20",
                        "nameWithOwner": "ochocat/violet",
                        "issues": {
                            "nodes": [],
                            "pageInfo": {
                                "endCursor": null,
                                "hasNextPage": false,
                            },
                        }
                    },
                    {
                        "id": "r21",
                        "nameWithOwner": "ochocat/white",
                        "issues": {
                            "nodes": [],
                            "pageInfo": {
                                "endCursor": null,
                                "hasNextPage": false,
                            },
                        }
                    },
                    {
                        "id": "r22",
                        "nameWithOwner": "ochocat/yellow",
                        "issues": {
                            "nodes": [],
                            "pageInfo": {
                                "endCursor": null,
                                "hasNextPage": false,
                            },
                        }
                    },
                ],
                "pageInfo": {
                    "endCursor": "cursor:end:ochocat",
                    "hasNextPage": false,
                }
            }
        }),
    )]);
    assert!(machine.handle_response(response).is_ok());

    let outputs = machine.get_output();
    assert_eq!(
        outputs,
        vec![Output::Issues(vec![
            Issue {
                repo: "ochocat/rainbow".into(),
                number: 1,
                title: "The red isn't red enough".into(),
                labels: vec!["colors".into(), "red".into(),],
                url: "https://example.github/ochocat/rainbow/issues/1".into(),
                created: "2020-01-01T00:00:00Z".into(),
                updated: "2020-01-01T00:00:00Z".into(),
            },
            Issue {
                repo: "ochocat/rainbow".into(),
                number: 2,
                title: "The orange isn't orange enough".into(),
                labels: vec!["colors".into(), "orange".into(),],
                url: "https://example.github/ochocat/rainbow/issues/2".into(),
                created: "2020-01-01T00:01:00Z".into(),
                updated: "2020-01-01T00:01:00Z".into(),
            },
            Issue {
                repo: "ochocat/rainbow".into(),
                number: 3,
                title: "The yellow isn't yellow enough".into(),
                labels: vec!["colors".into(), "yellow".into(),],
                url: "https://example.github/ochocat/rainbow/issues/3".into(),
                created: "2020-01-01T00:02:00Z".into(),
                updated: "2020-01-01T00:02:00Z".into(),
            },
            Issue {
                repo: "ochocat/rainbow".into(),
                number: 4,
                title: "The green isn't green enough".into(),
                labels: vec!["colors".into(), "green".into(),],
                url: "https://example.github/ochocat/rainbow/issues/4".into(),
                created: "2020-01-01T00:03:00Z".into(),
                updated: "2020-01-01T00:03:00Z".into(),
            },
            Issue {
                repo: "ochocat/rainbow".into(),
                number: 5,
                title: "The blue isn't blue enough".into(),
                labels: vec!["colors".into(), "blue".into(),],
                url: "https://example.github/ochocat/rainbow/issues/5".into(),
                created: "2020-01-01T00:04:00Z".into(),
                updated: "2020-01-01T00:04:00Z".into(),
            },
        ])]
    );

    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_repo_id: ID!, $q0_cursor: String, $q1_repo_id: ID!, $q1_cursor: String) {
            q0: node(id: $q0_repo_id) {
                ... on Repository {
                    id
                    nameWithOwner
                    issues(
                        first: 5,
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
                            labels (first: 5) {
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
                    id
                    nameWithOwner
                    issues(
                        first: 5,
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
                            labels (first: 5) {
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
            ("q0_repo_id".into(), "r3".into()),
            ("q0_cursor".into(), "cursor:p1:quadcat/example".into()),
            ("q1_repo_id".into(), "r18".into()),
            ("q1_cursor".into(), "cursor:p1:ochocat/rainbow".into()),
        ])
    );

    let outputs = machine.get_output();
    assert_eq!(outputs.len(), 2);
    assert_matches!(
        outputs[0],
        Output::Transition(Transition::EndFetchRepos {
            repositories: 22,
            repos_with_open_issues: 2,
            ..
        })
    );
    assert_matches!(
        outputs[1],
        Output::Transition(Transition::StartFetchIssues {
            repos_with_extra_issues: 2,
        })
    );

    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
                "id": "r3",
                "nameWithOwner": "quadcat/example",
                "issues": {
                    "nodes": [
                        {
                            "id": "i11",
                            "number": 6,
                            "title": "My Last Issue™",
                            "url": "https://example.github/quadcat/example/issues/6",
                            "createdAt": "2023-12-23T05:52:34Z",
                            "updatedAt": "2023-12-23T05:52:34Z",
                            "labels": {
                                "nodes": [
                                    {"name": "example"},
                                    {"name": "test"},
                                    {"name": "never again"},
                                ],
                                "pageInfo": {
                                    "endCursor": "cursor:end:quadcat/example:6",
                                    "hasNextPage": false,
                                }
                            }
                        },
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:end:quadcat/example",
                        "hasNextPage": false,
                    }
                }
            }),
        ),
        (
            "q1".into(),
            serde_json::json!({
                "id": "r18",
                "nameWithOwner": "ochocat/rainbow",
                "issues": {
                    "nodes": [
                        {
                            "id": "i12",
                            "number": 6,
                            "title": "The indigo isn't indigo enough",
                            "url": "https://example.github/ochocat/rainbow/issues/6",
                            "createdAt": "2020-01-01T00:05:00Z",
                            "updatedAt": "2020-01-01T00:05:00Z",
                            "labels": {
                                "nodes": [
                                    {"name": "colors"},
                                    {"name": "indigo"},
                                ],
                                "pageInfo": {
                                    "endCursor": "cursor:end:ochocat/rainbow:6",
                                    "hasNextPage": false
                                }
                            }
                        },
                        {
                            "id": "i13",
                            "number": 7,
                            "title": "The violet isn't violet enough",
                            "url": "https://example.github/ochocat/rainbow/issues/7",
                            "createdAt": "2020-01-01T00:06:00Z",
                            "updatedAt": "2020-01-01T00:06:00Z",
                            "labels": {
                                "nodes": [
                                    {"name": "colors"},
                                    {"name": "violet"},
                                ],
                                "pageInfo": {
                                    "endCursor": "cursor:end:ochocat/rainbow:7",
                                    "hasNextPage": false
                                }
                            }
                        },
                        {
                            "id": "i14",
                            "number": 8,
                            "title": "The magenta isn't magenta enough",
                            "url": "https://example.github/ochocat/rainbow/issues/8",
                            "createdAt": "2020-01-01T00:07:00Z",
                            "updatedAt": "2020-01-01T00:07:00Z",
                            "labels": {
                                "nodes": [
                                    {"name": "colors"},
                                    {"name": "magenta"},
                                ],
                                "pageInfo": {
                                    "endCursor": "cursor:end:ochocat/rainbow:8",
                                    "hasNextPage": false
                                }
                            }
                        },
                        {
                            "id": "i15",
                            "number": 9,
                            "title": "The cyan isn't cyan enough",
                            "url": "https://example.github/ochocat/rainbow/issues/9",
                            "createdAt": "2020-01-01T00:08:00Z",
                            "updatedAt": "2020-01-01T00:08:00Z",
                            "labels": {
                                "nodes": [
                                    {"name": "colors"},
                                    {"name": "cyan"},
                                ],
                                "pageInfo": {
                                    "endCursor": "cursor:end:ochocat/rainbow:9",
                                    "hasNextPage": false
                                }
                            }
                        },
                        {
                            "id": "i16",
                            "number": 10,
                            "title": "The fuchsia isn't fuchsia enough",
                            "url": "https://example.github/ochocat/rainbow/issues/10",
                            "createdAt": "2020-01-01T00:09:00Z",
                            "updatedAt": "2020-01-01T00:09:00Z",
                            "labels": {
                                "nodes": [
                                    {"name": "colors"},
                                    {"name": "fuchsia"},
                                ],
                                "pageInfo": {
                                    "endCursor": "cursor:end:ochocat/rainbow:10",
                                    "hasNextPage": false
                                }
                            }
                        },
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:p2:ochocat/rainbow",
                        "hasNextPage": true,
                    }
                }
            }),
        ),
    ]);
    assert!(machine.handle_response(response).is_ok());

    let outputs = machine.get_output();
    assert_eq!(
        outputs,
        vec![Output::Issues(vec![Issue {
            repo: "quadcat/example".into(),
            number: 6,
            title: "My Last Issue™".into(),
            labels: vec!["example".into(), "test".into(), "never again".into(),],
            url: "https://example.github/quadcat/example/issues/6".into(),
            created: "2023-12-23T05:52:34Z".into(),
            updated: "2023-12-23T05:52:34Z".into(),
        },])]
    );

    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_repo_id: ID!, $q0_cursor: String) {
            q0: node(id: $q0_repo_id) {
                ... on Repository {
                    id
                    nameWithOwner
                    issues(
                        first: 5,
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
                            labels (first: 5) {
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
            ("q0_repo_id".into(), "r18".into()),
            ("q0_cursor".into(), "cursor:p2:ochocat/rainbow".into()),
        ])
    );

    assert!(machine.get_output().is_empty());

    let response = JsonMap::from_iter([(
        "q0".into(),
        serde_json::json!({
            "id": "r18",
            "nameWithOwner": "ochocat/rainbow",
            "issues": {
                "nodes": [
                    {
                        "id": "i17",
                        "number": 11,
                        "title": "We need more colors",
                        "url": "https://example.github/ochocat/rainbow/issues/11",
                        "createdAt": "2020-01-01T00:10:00Z",
                        "updatedAt": "2020-01-01T00:10:00Z",
                        "labels": {
                            "nodes": [
                                {"name": "colors"},
                                {"name": "red"},
                                {"name": "orange"},
                                {"name": "yellow"},
                                {"name": "green"},
                            ],
                            "pageInfo": {
                                "endCursor": "cursor:p1:ochocat/rainbow:11",
                                "hasNextPage": true
                            }
                        }
                    },
                ],
                "pageInfo": {
                    "endCursor": "cursor:end:ochocat/rainbow",
                    "hasNextPage": false,
                }
            }
        }),
    )]);
    assert!(machine.handle_response(response).is_ok());

    let outputs = machine.get_output();
    assert_eq!(
        outputs,
        vec![Output::Issues(vec![
            Issue {
                repo: "ochocat/rainbow".into(),
                number: 6,
                title: "The indigo isn't indigo enough".into(),
                labels: vec!["colors".into(), "indigo".into(),],
                url: "https://example.github/ochocat/rainbow/issues/6".into(),
                created: "2020-01-01T00:05:00Z".into(),
                updated: "2020-01-01T00:05:00Z".into(),
            },
            Issue {
                repo: "ochocat/rainbow".into(),
                number: 7,
                title: "The violet isn't violet enough".into(),
                labels: vec!["colors".into(), "violet".into(),],
                url: "https://example.github/ochocat/rainbow/issues/7".into(),
                created: "2020-01-01T00:06:00Z".into(),
                updated: "2020-01-01T00:06:00Z".into(),
            },
            Issue {
                repo: "ochocat/rainbow".into(),
                number: 8,
                title: "The magenta isn't magenta enough".into(),
                labels: vec!["colors".into(), "magenta".into(),],
                url: "https://example.github/ochocat/rainbow/issues/8".into(),
                created: "2020-01-01T00:07:00Z".into(),
                updated: "2020-01-01T00:07:00Z".into(),
            },
            Issue {
                repo: "ochocat/rainbow".into(),
                number: 9,
                title: "The cyan isn't cyan enough".into(),
                labels: vec!["colors".into(), "cyan".into(),],
                url: "https://example.github/ochocat/rainbow/issues/9".into(),
                created: "2020-01-01T00:08:00Z".into(),
                updated: "2020-01-01T00:08:00Z".into(),
            },
            Issue {
                repo: "ochocat/rainbow".into(),
                number: 10,
                title: "The fuchsia isn't fuchsia enough".into(),
                labels: vec!["colors".into(), "fuchsia".into(),],
                url: "https://example.github/ochocat/rainbow/issues/10".into(),
                created: "2020-01-01T00:09:00Z".into(),
                updated: "2020-01-01T00:09:00Z".into(),
            },
        ])]
    );

    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_issue_id: ID!, $q0_cursor: String, $q1_issue_id: ID!, $q1_cursor: String) {
            q0: node(id: $q0_issue_id) {
                ... on Issue {
                    labels(
                        first: 5,
                        after: $q0_cursor,
                    ) {
                        nodes {
                            name
                        }
                        pageInfo {
                            endCursor
                            hasNextPage
                        }
                    }
                }
            }
            q1: node(id: $q1_issue_id) {
                ... on Issue {
                    labels(
                        first: 5,
                        after: $q1_cursor,
                    ) {
                        nodes {
                            name
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
            ("q0_issue_id".into(), "i1".into()),
            ("q0_cursor".into(), "cursor:p1:quadcat/example:1".into()),
            ("q1_issue_id".into(), "i17".into()),
            ("q1_cursor".into(), "cursor:p1:ochocat/rainbow:11".into()),
        ])
    );

    let outputs = machine.get_output();
    assert_eq!(outputs.len(), 2);
    assert_matches!(
        outputs[0],
        Output::Transition(Transition::EndFetchIssues {
            extra_issues: 7,
            ..
        })
    );
    assert_matches!(
        outputs[1],
        Output::Transition(Transition::StartFetchLabels {
            issues_with_extra_labels: 2
        })
    );

    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
                "labels": {
                    "nodes": [
                        {"name": "good first issue"},
                        {"name": "introduction"},
                        {"name": "hello"},
                        {"name": "first post"},
                        {"name": "hello world"},
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:p2:quadcat/example:1",
                        "hasNextPage": true,
                    }
                }
            }),
        ),
        (
            "q1".into(),
            serde_json::json!({
                "labels": {
                    "nodes": [
                        {"name": "blue"},
                        {"name": "indigo"},
                        {"name": "violet"},
                        {"name": "purple"},
                        {"name": "cyan"},
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:p2:ochocat/rainbow:11",
                        "hasNextPage": true,
                    }
                }
            }),
        ),
    ]);
    assert!(machine.handle_response(response).is_ok());

    assert!(machine.get_output().is_empty());

    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_issue_id: ID!, $q0_cursor: String, $q1_issue_id: ID!, $q1_cursor: String) {
            q0: node(id: $q0_issue_id) {
                ... on Issue {
                    labels(
                        first: 5,
                        after: $q0_cursor,
                    ) {
                        nodes {
                            name
                        }
                        pageInfo {
                            endCursor
                            hasNextPage
                        }
                    }
                }
            }
            q1: node(id: $q1_issue_id) {
                ... on Issue {
                    labels(
                        first: 5,
                        after: $q1_cursor,
                    ) {
                        nodes {
                            name
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
            ("q0_issue_id".into(), "i1".into()),
            ("q0_cursor".into(), "cursor:p2:quadcat/example:1".into()),
            ("q1_issue_id".into(), "i17".into()),
            ("q1_cursor".into(), "cursor:p2:ochocat/rainbow:11".into()),
        ])
    );

    assert!(machine.get_output().is_empty());

    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
                "labels": {
                    "nodes": [
                        {"name": "github"},
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:end:quadcat/example:1",
                        "hasNextPage": false,
                    }
                }
            }),
        ),
        (
            "q1".into(),
            serde_json::json!({
                "labels": {
                    "nodes": [
                        {"name": "fuchsia"},
                        {"name": "magenta"},
                        {"name": "white"},
                        {"name": "black"},
                        {"name": "grey"},
                    ],
                    "pageInfo": {
                        "endCursor": "cursor:p3:ochocat/rainbow:11",
                        "hasNextPage": true,
                    }
                }
            }),
        ),
    ]);
    assert!(machine.handle_response(response).is_ok());

    assert!(machine.get_output().is_empty());

    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_issue_id: ID!, $q0_cursor: String) {
            q0: node(id: $q0_issue_id) {
                ... on Issue {
                    labels(
                        first: 5,
                        after: $q0_cursor,
                    ) {
                        nodes {
                            name
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
            ("q0_issue_id".into(), "i17".into()),
            ("q0_cursor".into(), "cursor:p3:ochocat/rainbow:11".into()),
        ])
    );

    assert!(machine.get_output().is_empty());

    let response = JsonMap::from_iter([(
        "q0".into(),
        serde_json::json!({
            "labels": {
                "nodes": [
                    {"name": "silver"},
                    {"name": "gold"},
                ],
                "pageInfo": {
                    "endCursor": "cursor:end:ochocat/rainbow:11",
                    "hasNextPage": false,
                }
            }
        }),
    )]);
    assert!(machine.handle_response(response).is_ok());

    assert!(machine.get_output().is_empty());

    assert_eq!(machine.get_next_query(), None);

    let mut outputs = machine.get_output();
    assert_eq!(outputs.len(), 3);
    assert_matches!(
        outputs[0],
        Output::Transition(Transition::EndFetchLabels {
            extra_labels: 18,
            ..
        })
    );
    if let Output::Issues(issues) = &mut outputs[1] {
        issues.sort_unstable_by_key(|ish| (ish.repo.clone(), ish.number));
    }
    assert_eq!(
        outputs[1],
        Output::Issues(vec![
            Issue {
                repo: "ochocat/rainbow".into(),
                number: 11,
                title: "We need more colors".into(),
                labels: vec![
                    "colors".into(),
                    "red".into(),
                    "orange".into(),
                    "yellow".into(),
                    "green".into(),
                    "blue".into(),
                    "indigo".into(),
                    "violet".into(),
                    "purple".into(),
                    "cyan".into(),
                    "fuchsia".into(),
                    "magenta".into(),
                    "white".into(),
                    "black".into(),
                    "grey".into(),
                    "silver".into(),
                    "gold".into(),
                ],
                url: "https://example.github/ochocat/rainbow/issues/11".into(),
                created: "2020-01-01T00:10:00Z".into(),
                updated: "2020-01-01T00:10:00Z".into(),
            },
            Issue {
                repo: "quadcat/example".into(),
                number: 1,
                title: "My First Issue™".into(),
                labels: vec![
                    "example".into(),
                    "test".into(),
                    "testing".into(),
                    "sample".into(),
                    "first issue".into(),
                    "good first issue".into(),
                    "introduction".into(),
                    "hello".into(),
                    "first post".into(),
                    "hello world".into(),
                    "github".into(),
                ],
                url: "https://example.github/quadcat/example/issues/1".into(),
                created: "2020-01-01T00:00:00Z".into(),
                updated: "2020-01-01T00:00:00Z".into(),
            },
        ])
    );
    assert_eq!(
        outputs[2],
        Output::Report(FetchReport {
            repositories: 22,
            repos_with_open_issues: 2,
            repos_with_extra_issues: 2,
            open_issues: 17,
            issues_with_extra_labels: 2,
            extra_issues: 7,
            extra_labels: 18,
        })
    );
}
