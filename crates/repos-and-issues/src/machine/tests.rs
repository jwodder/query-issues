use super::*;
use gqlient::DEFAULT_BATCH_SIZE;
use indoc::indoc;
use pretty_assertions::assert_eq;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, Eq, PartialEq)]
struct EventRecorder(RefCell<Vec<Event>>);

impl EventRecorder {
    fn new() -> Rc<EventRecorder> {
        Rc::new(EventRecorder(RefCell::new(Vec::new())))
    }

    fn get_new_events(self: &Rc<Self>) -> Vec<Event> {
        self.0.borrow_mut().drain(..).collect()
    }
}

impl EventSubscriber for Rc<EventRecorder> {
    fn handle_event(&mut self, ev: Event) {
        self.0.borrow_mut().push(ev);
    }
}

#[test]
fn no_owners() {
    let parameters = QueryLimits {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(100).unwrap(),
        label_page_size: NonZeroUsize::new(10).unwrap(),
    };
    let record = EventRecorder::new();
    let mut machine =
        ReposAndIssues::new(Vec::new(), parameters).with_subscriber(Rc::clone(&record));
    assert_eq!(machine.get_next_query(), None);
    assert!(machine.get_output().is_empty());
    assert_eq!(record.get_new_events(), [Event::Start, Event::Done]);
}

#[test]
fn no_repos() {
    let parameters = QueryLimits {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(100).unwrap(),
        label_page_size: NonZeroUsize::new(10).unwrap(),
    };
    let record = EventRecorder::new();
    let mut machine = ReposAndIssues::new(vec!["octocat".into(), "achtkatze".into()], parameters)
        .with_subscriber(Rc::clone(&record));

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
                        name
                        owner {
                            login
                        }
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                        primaryLanguage {
                            name
                        }
                        repositoryTopics(first: 20) {
                            nodes {
                                topic {
                                    name
                                }
                            }
                        }
                        url
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
                        name
                        owner {
                            login
                        }
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                        primaryLanguage {
                            name
                        }
                        repositoryTopics(first: 20) {
                            nodes {
                                topic {
                                    name
                                }
                            }
                        }
                        url
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

    assert!(machine.get_output().is_empty());
    assert_eq!(
        record.get_new_events(),
        [Event::Start, Event::StartFetchRepos]
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
    assert!(record.get_new_events().is_empty());

    assert_eq!(machine.get_next_query(), None);

    assert!(machine.get_output().is_empty());
    assert_eq!(
        record.get_new_events(),
        [
            Event::EndFetchRepos {
                repositories: 0,
                repos_with_open_issues: 0
            },
            Event::Done
        ]
    );
}

#[test]
fn no_issues() {
    let parameters = QueryLimits {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(100).unwrap(),
        label_page_size: NonZeroUsize::new(10).unwrap(),
    };
    let record = EventRecorder::new();
    let mut machine = ReposAndIssues::new(vec!["octocat".into(), "achtkatze".into()], parameters)
        .with_subscriber(Rc::clone(&record));

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
                        name
                        owner {
                            login
                        }
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                        primaryLanguage {
                            name
                        }
                        repositoryTopics(first: 20) {
                            nodes {
                                topic {
                                    name
                                }
                            }
                        }
                        url
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
                        name
                        owner {
                            login
                        }
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                        primaryLanguage {
                            name
                        }
                        repositoryTopics(first: 20) {
                            nodes {
                                topic {
                                    name
                                }
                            }
                        }
                        url
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

    assert!(machine.get_output().is_empty());
    assert_eq!(
        record.get_new_events(),
        [Event::Start, Event::StartFetchRepos]
    );

    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
                "repositories": {
                    "nodes": [
                        {
                            "id": "r1",
                            "owner": {"login": "octocat"},
                            "name": "repo1",
                            "nameWithOwner": "octocat/repo1",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "Rust"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "example"}},
                                    {"topic": {"name": "testing"}},
                                ],
                            },
                            "url": "https://example.github/octocat/repo1",
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
                            "owner": {"login": "achtkatze"},
                            "name": "repo2",
                            "nameWithOwner": "achtkatze/repo2",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "Python"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "stuff"}}
                                ],
                            },
                            "url": "https://example.github/achtkatze/repo2",
                        },
                        {
                            "id": "r3",
                            "owner": {"login": "achtkatze"},
                            "name": "repo3",
                            "nameWithOwner": "achtkatze/repo3",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "Python"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "stuff"}},
                                    {"topic": {"name": "issues"}},
                                ],
                            },
                            "url": "https://example.github/achtkatze/repo3",
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

    assert_eq!(
        machine.get_output(),
        [
            Output::Repository(Repository {
                id: "r1".into(),
                name: "repo1".into(),
                owner: "octocat".into(),
                fullname: "octocat/repo1".into(),
                open_issues: 0,
                language: Some("Rust".into()),
                topics: vec!["example".into(), "testing".into()],
                url: "https://example.github/octocat/repo1".into(),
            }),
            Output::Repository(Repository {
                id: "r2".into(),
                name: "repo2".into(),
                owner: "achtkatze".into(),
                fullname: "achtkatze/repo2".into(),
                open_issues: 0,
                language: Some("Python".into()),
                topics: vec!["stuff".into()],
                url: "https://example.github/achtkatze/repo2".into(),
            }),
            Output::Repository(Repository {
                id: "r3".into(),
                name: "repo3".into(),
                owner: "achtkatze".into(),
                fullname: "achtkatze/repo3".into(),
                open_issues: 0,
                language: Some("Python".into()),
                topics: vec!["stuff".into(), "issues".into()],
                url: "https://example.github/achtkatze/repo3".into(),
            }),
        ]
    );

    assert!(record.get_new_events().is_empty());

    assert_eq!(machine.get_next_query(), None);

    assert!(machine.get_output().is_empty());
    assert_eq!(
        record.get_new_events(),
        [
            Event::EndFetchRepos {
                repositories: 3,
                repos_with_open_issues: 0
            },
            Event::Done
        ]
    );
}

#[test]
fn issues() {
    let parameters = QueryLimits {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(100).unwrap(),
        label_page_size: NonZeroUsize::new(10).unwrap(),
    };
    let record = EventRecorder::new();
    let mut machine = ReposAndIssues::new(vec!["octocat".into(), "achtkatze".into()], parameters)
        .with_subscriber(Rc::clone(&record));

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
                        name
                        owner {
                            login
                        }
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                        primaryLanguage {
                            name
                        }
                        repositoryTopics(first: 20) {
                            nodes {
                                topic {
                                    name
                                }
                            }
                        }
                        url
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
                        name
                        owner {
                            login
                        }
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                        primaryLanguage {
                            name
                        }
                        repositoryTopics(first: 20) {
                            nodes {
                                topic {
                                    name
                                }
                            }
                        }
                        url
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

    assert!(machine.get_output().is_empty());
    assert_eq!(
        record.get_new_events(),
        [Event::Start, Event::StartFetchRepos]
    );

    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
                "repositories": {
                    "nodes": [
                        {
                            "id": "r1",
                            "owner": {"login": "octocat"},
                            "name": "repo1",
                            "nameWithOwner": "octocat/repo1",
                            "issues": {
                                "totalCount": 3,
                            },
                            "primaryLanguage": {"name": "Rust"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "example"}},
                                    {"topic": {"name": "testing"}},
                                ],
                            },
                            "url": "https://example.github/octocat/repo1",
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
                            "owner": {"login": "achtkatze"},
                            "name": "repo2",
                            "nameWithOwner": "achtkatze/repo2",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "Python"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "stuff"}}
                                ],
                            },
                            "url": "https://example.github/achtkatze/repo2",
                        },
                        {
                            "id": "r3",
                            "owner": {"login": "achtkatze"},
                            "name": "repo3",
                            "nameWithOwner": "achtkatze/repo3",
                            "issues": {
                                "totalCount": 2,
                            },
                            "primaryLanguage": {"name": "Python"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "stuff"}},
                                    {"topic": {"name": "issues"}},
                                ],
                            },
                            "url": "https://example.github/achtkatze/repo3",
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

    assert_eq!(
        machine.get_output(),
        [
            Output::Repository(Repository {
                id: "r1".into(),
                name: "repo1".into(),
                owner: "octocat".into(),
                fullname: "octocat/repo1".into(),
                open_issues: 3,
                language: Some("Rust".into()),
                topics: vec!["example".into(), "testing".into()],
                url: "https://example.github/octocat/repo1".into(),
            }),
            Output::Repository(Repository {
                id: "r2".into(),
                name: "repo2".into(),
                owner: "achtkatze".into(),
                fullname: "achtkatze/repo2".into(),
                open_issues: 0,
                language: Some("Python".into()),
                topics: vec!["stuff".into()],
                url: "https://example.github/achtkatze/repo2".into(),
            }),
            Output::Repository(Repository {
                id: "r3".into(),
                name: "repo3".into(),
                owner: "achtkatze".into(),
                fullname: "achtkatze/repo3".into(),
                open_issues: 2,
                language: Some("Python".into()),
                topics: vec!["stuff".into(), "issues".into()],
                url: "https://example.github/achtkatze/repo3".into(),
            }),
        ]
    );

    assert!(record.get_new_events().is_empty());

    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_repo_id: ID!, $q0_cursor: String, $q1_repo_id: ID!, $q1_cursor: String) {
            q0: node(id: $q0_repo_id) {
                ... on Repository {
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

    assert!(machine.get_output().is_empty());
    assert_eq!(
        record.get_new_events(),
        [
            Event::EndFetchRepos {
                repositories: 3,
                repos_with_open_issues: 2
            },
            Event::StartFetchIssues
        ]
    );

    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
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

    assert_eq!(
        machine.get_output(),
        [
            Output::Issue(Issue {
                repo_id: "r1".into(),
                number: 1,
                title: "I found a bug!".into(),
                labels: vec!["bug".into()],
                url: "https://example.github/octocat/repo1/issues/1".into(),
                created: "2020-01-01T00:00:00Z".into(),
                updated: "2020-01-01T00:00:00Z".into(),
            }),
            Output::Issue(Issue {
                repo_id: "r1".into(),
                number: 17,
                title: "How do I use this code?".into(),
                labels: vec!["question".into(), "PEBKAC".into()],
                url: "https://example.github/octocat/repo1/issues/17".into(),
                created: "2020-02-01T00:00:00Z".into(),
                updated: "2021-01-01T00:00:00Z".into(),
            }),
            Output::Issue(Issue {
                repo_id: "r1".into(),
                number: 42,
                title: "Idea to make code better".into(),
                labels: vec!["enhancement".into()],
                url: "https://example.github/octocat/repo1/issues/42".into(),
                created: "2021-01-01T00:00:00Z".into(),
                updated: "2022-01-01T00:00:00Z".into(),
            }),
            Output::Issue(Issue {
                repo_id: "r3".into(),
                number: 23,
                title: "Why are we speaking German?".into(),
                labels: vec!["german".into(), "language".into(), "question".into()],
                url: "https://example.github/achtkatze/repo3/issues/23".into(),
                created: "2020-06-15T12:34:56Z".into(),
                updated: "2020-06-15T12:34:56Z".into(),
            }),
            Output::Issue(Issue {
                repo_id: "r3".into(),
                number: 24,
                title: "Wenn ist das Nunstück git und Slotermeyer?".into(),
                labels: vec!["german".into(), "funny".into()],
                url: "https://example.github/achtkatze/repo3/issues/24".into(),
                created: "2020-06-16T00:00:00Z".into(),
                updated: "2020-06-16T00:00:00Z".into(),
            }),
        ]
    );
    assert!(record.get_new_events().is_empty());

    assert_eq!(machine.get_next_query(), None);

    assert!(machine.get_output().is_empty());
    assert_eq!(
        record.get_new_events(),
        [Event::EndFetchIssues { open_issues: 5 }, Event::Done,]
    );
}

#[test]
fn extra_labels() {
    let parameters = QueryLimits {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(100).unwrap(),
        label_page_size: NonZeroUsize::new(5).unwrap(),
    };
    let record = EventRecorder::new();
    let mut machine =
        ReposAndIssues::new(vec!["monocat".into()], parameters).with_subscriber(Rc::clone(&record));

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
                        name
                        owner {
                            login
                        }
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                        primaryLanguage {
                            name
                        }
                        repositoryTopics(first: 20) {
                            nodes {
                                topic {
                                    name
                                }
                            }
                        }
                        url
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

    assert!(machine.get_output().is_empty());
    assert_eq!(
        record.get_new_events(),
        [Event::Start, Event::StartFetchRepos]
    );

    let response = JsonMap::from_iter([(
        "q0".into(),
        serde_json::json!({
            "repositories": {
                "nodes": [
                    {
                        "id": "r1",
                        "owner": {"login": "monocat"},
                        "name": "rainbow",
                        "nameWithOwner": "monocat/rainbow",
                        "issues": {
                            "totalCount": 1,
                        },
                        "primaryLanguage": {"name": "Perl"},
                        "repositoryTopics": {
                            "nodes": [
                                {"topic": {"name": "colors"}},
                                {"topic": {"name": "rainbow"}},
                                {"topic": {"name": "pretty"}},
                            ],
                        },
                        "url": "https://example.github/monocat/rainbow",
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

    assert_eq!(
        machine.get_output(),
        [Output::Repository(Repository {
            id: "r1".into(),
            owner: "monocat".into(),
            name: "rainbow".into(),
            fullname: "monocat/rainbow".into(),
            open_issues: 1,
            language: Some("Perl".into()),
            topics: vec!["colors".into(), "rainbow".into(), "pretty".into()],
            url: "https://example.github/monocat/rainbow".into(),
        })]
    );

    assert!(record.get_new_events().is_empty());

    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_repo_id: ID!, $q0_cursor: String) {
            q0: node(id: $q0_repo_id) {
                ... on Repository {
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
            ("q0_repo_id".into(), "r1".into()),
            ("q0_cursor".into(), serde_json::Value::Null),
        ])
    );

    assert!(machine.get_output().is_empty());
    assert_eq!(
        record.get_new_events(),
        [
            Event::EndFetchRepos {
                repositories: 1,
                repos_with_open_issues: 1
            },
            Event::StartFetchIssues
        ]
    );

    let response = JsonMap::from_iter([(
        "q0".into(),
        serde_json::json!({
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
        }),
    )]);
    assert!(machine.handle_response(response).is_ok());

    assert!(machine.get_output().is_empty());
    assert!(record.get_new_events().is_empty());

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

    assert!(machine.get_output().is_empty());
    assert_eq!(
        record.get_new_events(),
        [
            Event::EndFetchIssues { open_issues: 1 },
            Event::StartFetchLabels {
                issues_with_extra_labels: 1
            },
        ]
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
    assert!(record.get_new_events().is_empty());

    assert_eq!(machine.get_next_query(), None);

    assert_eq!(
        machine.get_output(),
        [Output::Issue(Issue {
            repo_id: "r1".into(),
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
        })]
    );

    assert_eq!(
        record.get_new_events(),
        [Event::EndFetchLabels { extra_labels: 3 }, Event::Done]
    );
}

#[test]
fn multiple_pages() {
    let parameters = QueryLimits {
        batch_size: DEFAULT_BATCH_SIZE,
        page_size: NonZeroUsize::new(5).unwrap(),
        label_page_size: NonZeroUsize::new(5).unwrap(),
    };
    let record = EventRecorder::new();
    let mut machine = ReposAndIssues::new(vec!["quadcat".into(), "ochocat".into()], parameters)
        .with_subscriber(Rc::clone(&record));

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
                        name
                        owner {
                            login
                        }
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                        primaryLanguage {
                            name
                        }
                        repositoryTopics(first: 20) {
                            nodes {
                                topic {
                                    name
                                }
                            }
                        }
                        url
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
                        name
                        owner {
                            login
                        }
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                        primaryLanguage {
                            name
                        }
                        repositoryTopics(first: 20) {
                            nodes {
                                topic {
                                    name
                                }
                            }
                        }
                        url
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

    assert!(machine.get_output().is_empty());
    assert_eq!(
        record.get_new_events(),
        [Event::Start, Event::StartFetchRepos]
    );

    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
                "repositories": {
                    "nodes": [
                        {
                            "id": "r1",
                            "owner": {"login": "quadcat"},
                            "name": "code-only",
                            "nameWithOwner": "quadcat/code-only",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "C"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "just-some-code"}},
                                    {"topic": {"name": "no-issues"}},
                                ],
                            },
                            "url": "https://example.github/quadcat/code-only",
                        },
                        {
                            "id": "r2",
                            "owner": {"login": "quadcat"},
                            "name": "empty",
                            "nameWithOwner": "quadcat/empty",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": null,
                            "repositoryTopics": {
                                "nodes": [],
                            },
                            "url": "https://example.github/quadcat/empty",
                        },
                        {
                            "id": "r3",
                            "owner": {"login": "quadcat"},
                            "name": "example",
                            "nameWithOwner": "quadcat/example",
                            "issues": {
                                "totalCount": 6,
                            },
                            "primaryLanguage": {"name": "Rust"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "example"}},
                                    {"topic": {"name": "demo"}},
                                    {"topic": {"name": "sample"}},
                                ],
                            },
                            "url": "https://example.github/quadcat/example",
                        },
                        {
                            "id": "r4",
                            "owner": {"login": "quadcat"},
                            "name": "nil",
                            "nameWithOwner": "quadcat/nil",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": null,
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "nil"}},
                                    {"topic": {"name": "nada"}},
                                    {"topic": {"name": "nothing"}},
                                    {"topic": {"name": "ignore-this"}},
                                ],
                            },
                            "url": "https://example.github/quadcat/nil",
                        },
                        {
                            "id": "r5",
                            "owner": {"login": "quadcat"},
                            "name": "no-issues",
                            "nameWithOwner": "quadcat/no-issues",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "Scheme"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topics": {"name": "scheming"}},
                                    {"topics": {"name": "planning"}},
                                    {"topics": {"name": "secret"}},
                                ],
                            },
                            "url": "https://example.github/quadcat/no-issues",
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
                            "owner": {"login": "ochocat"},
                            "name": "black",
                            "nameWithOwner": "ochocat/black",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "C"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "colors"}},
                                    {"topic": {"name": "black"}},
                                    {"topic": {"name": "hex000000"}},
                                ],
                            },
                            "url": "https://example.github/ochocat/black",
                        },
                        {
                            "id": "r7",
                            "owner": {"login": "ochocat"},
                            "name": "blue",
                            "nameWithOwner": "ochocat/blue",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "C"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "colors"}},
                                    {"topic": {"name": "blue"}},
                                    {"topic": {"name": "hex0000ff"}},
                                ],
                            },
                            "url": "https://example.github/ochocat/blue",
                        },
                        {
                            "id": "r8",
                            "owner": {"login": "ochocat"},
                            "name": "cyan",
                            "nameWithOwner": "ochocat/cyan",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "C"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "colors"}},
                                    {"topic": {"name": "cyan"}},
                                    {"topic": {"name": "hex00ffff"}},
                                ],
                            },
                            "url": "https://example.github/ochocat/cyan",
                        },
                        {
                            "id": "r9",
                            "owner": {"login": "ochocat"},
                            "name": "fuchsia",
                            "nameWithOwner": "ochocat/fuchsia",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "C"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "colors"}},
                                    {"topic": {"name": "fuchsia"}},
                                    {"topic": {"name": "hexff00ff"}},
                                ],
                            },
                            "url": "https://example.github/ochocat/fuchsia",
                        },
                        {
                            "id": "r10",
                            "owner": {"login": "ochocat"},
                            "name": "green",
                            "nameWithOwner": "ochocat/green",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "C"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "colors"}},
                                    {"topic": {"name": "green"}},
                                    {"topic": {"name": "hex00ff00"}},
                                ],
                            },
                            "url": "https://example.github/ochocat/green",
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
    assert!(record.get_new_events().is_empty());

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
                        name
                        owner {
                            login
                        }
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                        primaryLanguage {
                            name
                        }
                        repositoryTopics(first: 20) {
                            nodes {
                                topic {
                                    name
                                }
                            }
                        }
                        url
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
                        name
                        owner {
                            login
                        }
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                        primaryLanguage {
                            name
                        }
                        repositoryTopics(first: 20) {
                            nodes {
                                topic {
                                    name
                                }
                            }
                        }
                        url
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
    assert!(record.get_new_events().is_empty());

    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
                "repositories": {
                    "nodes": [
                        {
                            "id": "r11",
                            "owner": {"login": "quadcat"},
                            "name": "nothing",
                            "nameWithOwner": "quadcat/nothing",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "Rust"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topics": {"name": "void"}},
                                ],
                            },
                            "url": "https://example.github/quadcat/nothing",
                        },
                        {
                            "id": "r12",
                            "owner": {"login": "quadcat"},
                            "name": "test",
                            "nameWithOwner": "quadcat/test",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "Rust"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topics": {"name": "test"}},
                                    {"topics": {"name": "experimentation"}},
                                    {"topics": {"name": "futzing"}},
                                ],
                            },
                            "url": "https://example.github/quadcat/test",
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
                            "owner": {"login": "ochocat"},
                            "name": "grey",
                            "nameWithOwner": "ochocat/grey",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "C"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "colors"}},
                                    {"topic": {"name": "grey"}},
                                    {"topic": {"name": "hex808080"}},
                                ],
                            },
                            "url": "https://example.github/ochocat/grey",
                        },
                        {
                            "id": "r14",
                            "owner": {"login": "ochocat"},
                            "name": "indigo",
                            "nameWithOwner": "ochocat/indigo",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "C"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "colors"}},
                                    {"topic": {"name": "indigo"}},
                                    {"topic": {"name": "hex4b0082"}},
                                ],
                            },
                            "url": "https://example.github/ochocat/indigo",
                        },
                        {
                            "id": "r15",
                            "owner": {"login": "ochocat"},
                            "name": "magenta",
                            "nameWithOwner": "ochocat/magenta",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "C"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "colors"}},
                                    {"topic": {"name": "magenta"}},
                                    {"topic": {"name": "hexff00ff"}},
                                ],
                            },
                            "url": "https://example.github/ochocat/magenta",
                        },
                        {
                            "id": "r16",
                            "owner": {"login": "ochocat"},
                            "name": "orange",
                            "nameWithOwner": "ochocat/orange",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "C"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "colors"}},
                                    {"topic": {"name": "orange"}},
                                    {"topic": {"name": "hexffa500"}},
                                ],
                            },
                            "url": "https://example.github/ochocat/orange",
                        },
                        {
                            "id": "r17",
                            "owner": {"login": "ochocat"},
                            "name": "purple",
                            "nameWithOwner": "ochocat/purple",
                            "issues": {
                                "totalCount": 0,
                            },
                            "primaryLanguage": {"name": "C"},
                            "repositoryTopics": {
                                "nodes": [
                                    {"topic": {"name": "colors"}},
                                    {"topic": {"name": "purple"}},
                                    {"topic": {"name": "hex800080"}},
                                ],
                            },
                            "url": "https://example.github/ochocat/purple",
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

    assert_eq!(
        machine.get_output(),
        [
            Output::Repository(Repository {
                id: "r1".into(),
                owner: "quadcat".into(),
                name: "code-only".into(),
                fullname: "quadcat/code-only".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["just-some-code".into(), "no-issues".into()],
                url: "https://example.github/quadcat/code-only".into(),
            }),
            Output::Repository(Repository {
                id: "r2".into(),
                owner: "quadcat".into(),
                name: "empty".into(),
                fullname: "quadcat/empty".into(),
                open_issues: 0,
                language: None,
                topics: Vec::new(),
                url: "https://example.github/quadcat/empty".into(),
            }),
            Output::Repository(Repository {
                id: "r3".into(),
                owner: "quadcat".into(),
                name: "example".into(),
                fullname: "quadcat/example".into(),
                open_issues: 6,
                language: Some("Rust".into()),
                topics: vec!["example".into(), "demo".into(), "sample".into()],
                url: "https://example.github/quadcat/example".into(),
            }),
            Output::Repository(Repository {
                id: "r4".into(),
                owner: "quadcat".into(),
                name: "nil".into(),
                fullname: "quadcat/nil".into(),
                open_issues: 0,
                language: None,
                topics: vec![
                    "nil".into(),
                    "nada".into(),
                    "nothing".into(),
                    "ignore-this".into()
                ],
                url: "https://example.github/quadcat/nil".into(),
            }),
            Output::Repository(Repository {
                id: "r5".into(),
                owner: "quadcat".into(),
                name: "no-issues".into(),
                fullname: "quadcat/no-issues".into(),
                open_issues: 0,
                language: Some("Scheme".into()),
                topics: vec!["scheming".into(), "planning".into(), "secret".into()],
                url: "https://example.github/quadcat/no-issues".into(),
            }),
            Output::Repository(Repository {
                id: "r11".into(),
                owner: "quadcat".into(),
                name: "nothing".into(),
                fullname: "quadcat/nothing".into(),
                open_issues: 0,
                language: Some("Rust".into()),
                topics: vec!["void".into()],
                url: "https://example.github/quadcat/nothing".into(),
            }),
            Output::Repository(Repository {
                id: "r12".into(),
                owner: "quadcat".into(),
                name: "test".into(),
                fullname: "quadcat/test".into(),
                open_issues: 0,
                language: Some("Rust".into()),
                topics: vec!["test".into(), "experimentation".into(), "futzing".into()],
                url: "https://example.github/quadcat/test".into(),
            }),
        ]
    );

    assert!(record.get_new_events().is_empty());

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
                        name
                        owner {
                            login
                        }
                        nameWithOwner
                        issues(states: [OPEN]) {
                            totalCount
                        }
                        primaryLanguage {
                            name
                        }
                        repositoryTopics(first: 20) {
                            nodes {
                                topic {
                                    name
                                }
                            }
                        }
                        url
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
    assert!(record.get_new_events().is_empty());

    let response = JsonMap::from_iter([(
        "q0".into(),
        serde_json::json!({
            "repositories": {
                "nodes": [
                    {
                        "id": "r18",
                        "owner": {"login": "ochocat"},
                        "name": "rainbow",
                        "nameWithOwner": "ochocat/rainbow",
                        "issues": {
                            "totalCount": 11,
                        },
                        "primaryLanguage": {"name": "C"},
                        "repositoryTopics": {
                            "nodes": [
                                {"topic": {"name": "colors"}},
                                {"topic": {"name": "rainbow"}},
                                {"topic": {"name": "panchromatic"}},
                            ],
                        },
                        "url": "https://example.github/ochocat/rainbow",
                    },
                    {
                        "id": "r19",
                        "owner": {"login": "ochocat"},
                        "name": "red",
                        "nameWithOwner": "ochocat/red",
                        "issues": {
                            "totalCount": 0,
                        },
                        "primaryLanguage": {"name": "C"},
                        "repositoryTopics": {
                            "nodes": [
                                {"topic": {"name": "colors"}},
                                {"topic": {"name": "red"}},
                                {"topic": {"name": "hexff0000"}},
                            ],
                        },
                        "url": "https://example.github/ochocat/red",
                    },
                    {
                        "id": "r20",
                        "owner": {"login": "ochocat"},
                        "name": "violet",
                        "nameWithOwner": "ochocat/violet",
                        "issues": {
                            "totalCount": 0,
                        },
                        "primaryLanguage": {"name": "C"},
                        "repositoryTopics": {
                            "nodes": [
                                {"topic": {"name": "colors"}},
                                {"topic": {"name": "violet"}},
                                {"topic": {"name": "hexee82ee"}},
                            ],
                        },
                        "url": "https://example.github/ochocat/violet",
                    },
                    {
                        "id": "r21",
                        "owner": {"login": "ochocat"},
                        "name": "white",
                        "nameWithOwner": "ochocat/white",
                        "issues": {
                            "totalCount": 0,
                        },
                        "primaryLanguage": {"name": "C"},
                        "repositoryTopics": {
                            "nodes": [
                                {"topic": {"name": "colors"}},
                                {"topic": {"name": "white"}},
                                {"topic": {"name": "hexffffff"}},
                            ],
                        },
                        "url": "https://example.github/ochocat/white",
                    },
                    {
                        "id": "r22",
                        "owner": {"login": "ochocat"},
                        "name": "yellow",
                        "nameWithOwner": "ochocat/yellow",
                        "issues": {
                            "totalCount": 0,
                        },
                        "primaryLanguage": {"name": "C"},
                        "repositoryTopics": {
                            "nodes": [
                                {"topic": {"name": "colors"}},
                                {"topic": {"name": "yellow"}},
                                {"topic": {"name": "hexffff00"}},
                            ],
                        },
                        "url": "https://example.github/ochocat/yellow",
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

    assert_eq!(
        machine.get_output(),
        [
            Output::Repository(Repository {
                id: "r6".into(),
                owner: "ochocat".into(),
                name: "black".into(),
                fullname: "ochocat/black".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["colors".into(), "black".into(), "hex000000".into()],
                url: "https://example.github/ochocat/black".into(),
            }),
            Output::Repository(Repository {
                id: "r7".into(),
                owner: "ochocat".into(),
                name: "blue".into(),
                fullname: "ochocat/blue".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["colors".into(), "blue".into(), "hex0000ff".into()],
                url: "https://example.github/ochocat/blue".into(),
            }),
            Output::Repository(Repository {
                id: "r8".into(),
                owner: "ochocat".into(),
                name: "cyan".into(),
                fullname: "ochocat/cyan".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["colors".into(), "cyan".into(), "hex00ffff".into()],
                url: "https://example.github/ochocat/cyan".into(),
            }),
            Output::Repository(Repository {
                id: "r9".into(),
                owner: "ochocat".into(),
                name: "fuchsia".into(),
                fullname: "ochocat/fuchsia".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["colors".into(), "fuchsia".into(), "hexff00ff".into()],
                url: "https://example.github/ochocat/fuchsia".into(),
            }),
            Output::Repository(Repository {
                id: "r10".into(),
                owner: "ochocat".into(),
                name: "green".into(),
                fullname: "ochocat/green".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["colors".into(), "green".into(), "hex00ff00".into()],
                url: "https://example.github/ochocat/green".into(),
            }),
            Output::Repository(Repository {
                id: "r13".into(),
                owner: "ochocat".into(),
                name: "grey".into(),
                fullname: "ochocat/grey".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["colors".into(), "grey".into(), "hex808080".into()],
                url: "https://example.github/ochocat/grey".into(),
            }),
            Output::Repository(Repository {
                id: "r14".into(),
                owner: "ochocat".into(),
                name: "indigo".into(),
                fullname: "ochocat/indigo".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["colors".into(), "indigo".into(), "hex4b0082".into()],
                url: "https://example.github/ochocat/indigo".into(),
            }),
            Output::Repository(Repository {
                id: "r15".into(),
                owner: "ochocat".into(),
                name: "magenta".into(),
                fullname: "ochocat/magenta".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["colors".into(), "magenta".into(), "hexff00ff".into()],
                url: "https://example.github/ochocat/magenta".into(),
            }),
            Output::Repository(Repository {
                id: "r16".into(),
                owner: "ochocat".into(),
                name: "orange".into(),
                fullname: "ochocat/orange".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["colors".into(), "orange".into(), "hexffa500".into()],
                url: "https://example.github/ochocat/orange".into(),
            }),
            Output::Repository(Repository {
                id: "r17".into(),
                owner: "ochocat".into(),
                name: "purple".into(),
                fullname: "ochocat/purple".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["colors".into(), "purple".into(), "hex800080".into()],
                url: "https://example.github/ochocat/purple".into(),
            }),
            Output::Repository(Repository {
                id: "r18".into(),
                owner: "ochocat".into(),
                name: "rainbow".into(),
                fullname: "ochocat/rainbow".into(),
                open_issues: 11,
                language: Some("C".into()),
                topics: vec!["colors".into(), "rainbow".into(), "panchromatic".into()],
                url: "https://example.github/ochocat/rainbow".into(),
            }),
            Output::Repository(Repository {
                id: "r19".into(),
                owner: "ochocat".into(),
                name: "red".into(),
                fullname: "ochocat/red".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["colors".into(), "red".into(), "hexff0000".into()],
                url: "https://example.github/ochocat/red".into(),
            }),
            Output::Repository(Repository {
                id: "r20".into(),
                owner: "ochocat".into(),
                name: "violet".into(),
                fullname: "ochocat/violet".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["colors".into(), "violet".into(), "hexee82ee".into()],
                url: "https://example.github/ochocat/violet".into(),
            }),
            Output::Repository(Repository {
                id: "r21".into(),
                owner: "ochocat".into(),
                name: "white".into(),
                fullname: "ochocat/white".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["colors".into(), "white".into(), "hexffffff".into()],
                url: "https://example.github/ochocat/white".into(),
            }),
            Output::Repository(Repository {
                id: "r22".into(),
                owner: "ochocat".into(),
                name: "yellow".into(),
                fullname: "ochocat/yellow".into(),
                open_issues: 0,
                language: Some("C".into()),
                topics: vec!["colors".into(), "yellow".into(), "hexffff00".into()],
                url: "https://example.github/ochocat/yellow".into(),
            }),
        ]
    );

    assert!(record.get_new_events().is_empty());

    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_repo_id: ID!, $q0_cursor: String, $q1_repo_id: ID!, $q1_cursor: String) {
            q0: node(id: $q0_repo_id) {
                ... on Repository {
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
            ("q0_cursor".into(), serde_json::Value::Null),
            ("q1_repo_id".into(), "r18".into()),
            ("q1_cursor".into(), serde_json::Value::Null),
        ])
    );

    assert!(machine.get_output().is_empty());
    assert_eq!(
        record.get_new_events(),
        [
            Event::EndFetchRepos {
                repositories: 22,
                repos_with_open_issues: 2
            },
            Event::StartFetchIssues
        ]
    );

    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
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
                    }
                }
            }),
        ),
        (
            "q1".into(),
            serde_json::json!({
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
                    }
                }
            }),
        ),
    ]);
    assert!(machine.handle_response(response).is_ok());

    assert!(machine.get_output().is_empty());
    assert!(record.get_new_events().is_empty());

    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_repo_id: ID!, $q0_cursor: String, $q1_repo_id: ID!, $q1_cursor: String) {
            q0: node(id: $q0_repo_id) {
                ... on Repository {
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

    assert!(machine.get_output().is_empty());
    assert!(record.get_new_events().is_empty());

    let response = JsonMap::from_iter([
        (
            "q0".into(),
            serde_json::json!({
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

    assert_eq!(
        machine.get_output(),
        [
            Output::Issue(Issue {
                repo_id: "r3".into(),
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
            }),
            Output::Issue(Issue {
                repo_id: "r3".into(),
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
            }),
            Output::Issue(Issue {
                repo_id: "r3".into(),
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
            }),
            Output::Issue(Issue {
                repo_id: "r3".into(),
                number: 5,
                title: "My Fifth Issue™".into(),
                labels: vec!["example".into(), "test".into(), "so bored now".into(),],
                url: "https://example.github/quadcat/example/issues/5".into(),
                created: "2022-11-22T19:10:13Z".into(),
                updated: "2022-11-22T19:10:13Z".into(),
            }),
            Output::Issue(Issue {
                repo_id: "r3".into(),
                number: 6,
                title: "My Last Issue™".into(),
                labels: vec!["example".into(), "test".into(), "never again".into(),],
                url: "https://example.github/quadcat/example/issues/6".into(),
                created: "2023-12-23T05:52:34Z".into(),
                updated: "2023-12-23T05:52:34Z".into(),
            }),
        ]
    );
    assert!(record.get_new_events().is_empty());

    let payload = machine.get_next_query().unwrap();
    assert_eq!(
        payload.query,
        indoc! {"
        query ($q0_repo_id: ID!, $q0_cursor: String) {
            q0: node(id: $q0_repo_id) {
                ... on Repository {
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
    assert!(record.get_new_events().is_empty());

    let response = JsonMap::from_iter([(
        "q0".into(),
        serde_json::json!({
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

    assert_eq!(
        machine.get_output(),
        [
            Output::Issue(Issue {
                repo_id: "r18".into(),
                number: 1,
                title: "The red isn't red enough".into(),
                labels: vec!["colors".into(), "red".into(),],
                url: "https://example.github/ochocat/rainbow/issues/1".into(),
                created: "2020-01-01T00:00:00Z".into(),
                updated: "2020-01-01T00:00:00Z".into(),
            }),
            Output::Issue(Issue {
                repo_id: "r18".into(),
                number: 2,
                title: "The orange isn't orange enough".into(),
                labels: vec!["colors".into(), "orange".into(),],
                url: "https://example.github/ochocat/rainbow/issues/2".into(),
                created: "2020-01-01T00:01:00Z".into(),
                updated: "2020-01-01T00:01:00Z".into(),
            }),
            Output::Issue(Issue {
                repo_id: "r18".into(),
                number: 3,
                title: "The yellow isn't yellow enough".into(),
                labels: vec!["colors".into(), "yellow".into(),],
                url: "https://example.github/ochocat/rainbow/issues/3".into(),
                created: "2020-01-01T00:02:00Z".into(),
                updated: "2020-01-01T00:02:00Z".into(),
            }),
            Output::Issue(Issue {
                repo_id: "r18".into(),
                number: 4,
                title: "The green isn't green enough".into(),
                labels: vec!["colors".into(), "green".into(),],
                url: "https://example.github/ochocat/rainbow/issues/4".into(),
                created: "2020-01-01T00:03:00Z".into(),
                updated: "2020-01-01T00:03:00Z".into(),
            }),
            Output::Issue(Issue {
                repo_id: "r18".into(),
                number: 5,
                title: "The blue isn't blue enough".into(),
                labels: vec!["colors".into(), "blue".into(),],
                url: "https://example.github/ochocat/rainbow/issues/5".into(),
                created: "2020-01-01T00:04:00Z".into(),
                updated: "2020-01-01T00:04:00Z".into(),
            }),
            Output::Issue(Issue {
                repo_id: "r18".into(),
                number: 6,
                title: "The indigo isn't indigo enough".into(),
                labels: vec!["colors".into(), "indigo".into(),],
                url: "https://example.github/ochocat/rainbow/issues/6".into(),
                created: "2020-01-01T00:05:00Z".into(),
                updated: "2020-01-01T00:05:00Z".into(),
            }),
            Output::Issue(Issue {
                repo_id: "r18".into(),
                number: 7,
                title: "The violet isn't violet enough".into(),
                labels: vec!["colors".into(), "violet".into(),],
                url: "https://example.github/ochocat/rainbow/issues/7".into(),
                created: "2020-01-01T00:06:00Z".into(),
                updated: "2020-01-01T00:06:00Z".into(),
            }),
            Output::Issue(Issue {
                repo_id: "r18".into(),
                number: 8,
                title: "The magenta isn't magenta enough".into(),
                labels: vec!["colors".into(), "magenta".into(),],
                url: "https://example.github/ochocat/rainbow/issues/8".into(),
                created: "2020-01-01T00:07:00Z".into(),
                updated: "2020-01-01T00:07:00Z".into(),
            }),
            Output::Issue(Issue {
                repo_id: "r18".into(),
                number: 9,
                title: "The cyan isn't cyan enough".into(),
                labels: vec!["colors".into(), "cyan".into(),],
                url: "https://example.github/ochocat/rainbow/issues/9".into(),
                created: "2020-01-01T00:08:00Z".into(),
                updated: "2020-01-01T00:08:00Z".into(),
            }),
            Output::Issue(Issue {
                repo_id: "r18".into(),
                number: 10,
                title: "The fuchsia isn't fuchsia enough".into(),
                labels: vec!["colors".into(), "fuchsia".into(),],
                url: "https://example.github/ochocat/rainbow/issues/10".into(),
                created: "2020-01-01T00:09:00Z".into(),
                updated: "2020-01-01T00:09:00Z".into(),
            }),
        ]
    );
    assert!(record.get_new_events().is_empty());

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

    assert!(machine.get_output().is_empty());
    assert_eq!(
        record.get_new_events(),
        [
            Event::EndFetchIssues { open_issues: 17 },
            Event::StartFetchLabels {
                issues_with_extra_labels: 2
            },
        ]
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
    assert!(record.get_new_events().is_empty());

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
    assert!(record.get_new_events().is_empty());

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
    assert!(record.get_new_events().is_empty());

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
    assert!(record.get_new_events().is_empty());

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
    assert!(record.get_new_events().is_empty());

    assert_eq!(machine.get_next_query(), None);

    let mut outputs = machine.get_output();
    outputs.sort_unstable_by_key(|p| match p {
        Output::Repository(r) => (0, r.id.clone(), 0),
        Output::Issue(ish) => (1, ish.repo_id.clone(), ish.number),
    });
    assert_eq!(
        outputs,
        [
            Output::Issue(Issue {
                repo_id: "r18".into(),
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
            }),
            Output::Issue(Issue {
                repo_id: "r3".into(),
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
            }),
        ]
    );

    assert_eq!(
        record.get_new_events(),
        [Event::EndFetchLabels { extra_labels: 18 }, Event::Done]
    );
}
