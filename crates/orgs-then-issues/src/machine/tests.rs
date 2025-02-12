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

// repos, but no issues
// multiple pages of repos
// multiple pages of issues
// issues with extra labels
// issues with multiple pages of labels
