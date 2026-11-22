//! `story:agent-history-spec`: the navigator's two obligations, `CommitNavigation` (a push's and a
//! traversal's effects on the tab's entries) and `TraverseHistory` (the entry one step back or
//! forward), through the crate's public surface.

use surf_ports::NavigatorPorts;
use surf_types::behaviour::NavigationStorage;
use surf_types::navigation::obligations::{CommitNavigationBehavior, TraverseHistoryBehavior};
use surf_types::navigation::{
    CommitNavigation, CommitNavigationOutcome, HistoryHandling, NavigationData, NavigationNotFound,
    NavigationSnapshot, NavigationState, NavigationStateConflict, NoHistoryEntry, Standing,
    TraversalStarted, TraversalUnsupported, TraverseHistory, TraverseHistoryOutcome,
};
use surf_types::net::{MediaType, Method, RequestId, Status};
use surf_types::primitives::Uuid;
use surf_types::shell::TabId;
use surf_types::url::Url;

fn id(n: u64) -> RequestId {
    RequestId(Uuid(format!("00000000-0000-4000-8000-{n:012x}")))
}

fn tab(n: u64) -> TabId {
    TabId(Uuid(format!("00000000-0000-4000-9000-{n:012x}")))
}

fn url(path: &str) -> Url {
    Url(format!("http://example.org/{path}"))
}

/// A `Started` navigation of tab 1 to `/n`, by `Get` unless `method` says otherwise.
fn started(ports: &mut NavigatorPorts, n: u64, method: Method) {
    ports.put(NavigationSnapshot {
        state: NavigationState::Started,
        data: NavigationData {
            navigation_id: id(n),
            tab_id: tab(1),
            url: url(&n.to_string()),
            method,
            handling: HistoryHandling::Push,
            standing: None,
            next: None,
            entry_id: None,
            entry_next: None,
        },
    });
}

/// A `Started` traversal of tab 1 to the entry `entry`, whose next entry is `entry_next`.
fn traversal(ports: &mut NavigatorPorts, n: u64, entry: u64, entry_next: Option<u64>) {
    ports.put(NavigationSnapshot {
        state: NavigationState::Started,
        data: NavigationData {
            navigation_id: id(n),
            tab_id: tab(1),
            url: url(&entry.to_string()),
            method: Method::Get,
            handling: HistoryHandling::Traverse,
            standing: None,
            next: None,
            entry_id: Some(id(entry)),
            entry_next: entry_next.map(id),
        },
    });
}

fn commit(ports: &mut NavigatorPorts, n: u64) -> CommitNavigationOutcome {
    ports
        .commit_navigation(CommitNavigation {
            navigation_id: id(n),
            url: url(&n.to_string()),
            status: Status(200),
            media_type: MediaType("text/plain".to_owned()),
            body: b"hi".to_vec(),
        })
        .expect("CommitNavigation is met")
}

/// Starts and commits a push of `/n` in tab 1.
fn push(ports: &mut NavigatorPorts, n: u64, method: Method) {
    started(ports, n, method);
    assert!(matches!(
        commit(ports, n),
        CommitNavigationOutcome::Committed { .. }
    ));
}

fn standing(ports: &NavigatorPorts, n: u64) -> Option<Standing> {
    ports.get(&id(n)).expect("the row is held").data.standing
}

fn next(ports: &NavigatorPorts, n: u64) -> Option<RequestId> {
    ports.get(&id(n)).expect("the row is held").data.next
}

fn traverse(ports: &mut NavigatorPorts, step: Standing) -> TraverseHistoryOutcome {
    ports
        .traverse_history(TraverseHistory {
            tab_id: tab(1),
            step,
        })
        .expect("TraverseHistory is met")
}

#[test]
fn a_push_makes_its_entry_current_and_the_one_before_it_back_with_it_next() {
    let mut ports = NavigatorPorts::new();
    push(&mut ports, 1, Method::Get);
    assert_eq!(standing(&ports, 1), Some(Standing::Current));
    push(&mut ports, 2, Method::Get);
    push(&mut ports, 3, Method::Get);
    assert_eq!(standing(&ports, 1), Some(Standing::Earlier));
    assert_eq!(standing(&ports, 2), Some(Standing::Back));
    assert_eq!(standing(&ports, 3), Some(Standing::Current));
    assert_eq!(next(&ports, 1), Some(id(2)));
    assert_eq!(next(&ports, 2), Some(id(3)));
    assert_eq!(next(&ports, 3), None);
}

#[test]
fn a_push_leaves_the_entries_of_another_tab_alone() {
    let mut ports = NavigatorPorts::new();
    push(&mut ports, 1, Method::Get);
    let mut other = ports.get(&id(1)).expect("held");
    other.data.navigation_id = id(9);
    other.data.tab_id = tab(2);
    ports.put(other);
    push(&mut ports, 2, Method::Get);
    assert_eq!(standing(&ports, 9), Some(Standing::Current));
    assert_eq!(next(&ports, 9), None);
}

#[test]
fn a_traversal_back_and_forward_moves_the_entries_and_adds_none() {
    let mut ports = NavigatorPorts::new();
    for n in 1..=4 {
        push(&mut ports, n, Method::Get);
    }
    // Back from 4 to 3: 3 is current, 2 back, 4 forward, 1 earlier.
    traversal(&mut ports, 10, 3, Some(4));
    assert!(matches!(
        commit(&mut ports, 10),
        CommitNavigationOutcome::Traversed { .. }
    ));
    let at = |ports: &NavigatorPorts| [1, 2, 3, 4].map(|n| standing(ports, n));
    use Standing::{Back, Current, Earlier, Forward, Later};
    assert_eq!(
        at(&ports),
        [Some(Earlier), Some(Back), Some(Current), Some(Forward)]
    );
    assert_eq!(standing(&ports, 10), None);
    // Back again, to 2: 4 is later.
    traversal(&mut ports, 11, 2, Some(3));
    commit(&mut ports, 11);
    assert_eq!(
        at(&ports),
        [Some(Back), Some(Current), Some(Forward), Some(Later)]
    );
    // Forward to 3: 2 back, 4 forward again.
    traversal(&mut ports, 12, 3, Some(4));
    commit(&mut ports, 12);
    assert_eq!(
        at(&ports),
        [Some(Earlier), Some(Back), Some(Current), Some(Forward)]
    );
}

#[test]
fn a_push_after_going_back_drops_the_entries_after_the_current_one() {
    let mut ports = NavigatorPorts::new();
    for n in 1..=3 {
        push(&mut ports, n, Method::Get);
    }
    traversal(&mut ports, 10, 2, Some(3));
    commit(&mut ports, 10);
    traversal(&mut ports, 11, 1, Some(2));
    commit(&mut ports, 11);
    push(&mut ports, 4, Method::Get);
    use Standing::{Back, Current, Dropped};
    assert_eq!(
        [1, 2, 3, 4].map(|n| standing(&ports, n)),
        [Some(Back), Some(Dropped), Some(Dropped), Some(Current)]
    );
    assert_eq!(next(&ports, 1), Some(id(4)));
}

#[test]
fn a_commit_names_its_tab_and_refuses_an_unknown_or_ended_navigation() {
    let mut ports = NavigatorPorts::new();
    assert_eq!(
        commit(&mut ports, 1),
        CommitNavigationOutcome::NoSuchNavigation {
            error: NavigationNotFound {
                navigation_id: id(1)
            }
        }
    );
    push(&mut ports, 1, Method::Get);
    assert_eq!(
        commit(&mut ports, 1),
        CommitNavigationOutcome::WrongState {
            error: NavigationStateConflict {
                state: NavigationState::Committed
            }
        }
    );
    let row = ports.get(&id(1)).expect("held");
    assert_eq!(row.data.url, url("1"));
    assert_eq!(row.state, NavigationState::Committed);
}

#[test]
fn traverse_refuses_another_step_a_missing_entry_and_an_entry_made_by_post() {
    let mut ports = NavigatorPorts::new();
    assert_eq!(
        traverse(&mut ports, Standing::Current),
        TraverseHistoryOutcome::Unsupported {
            error: TraversalUnsupported {
                step: Standing::Current
            }
        }
    );
    let no_entry = |step| TraverseHistoryOutcome::NoEntry {
        error: NoHistoryEntry {
            tab_id: tab(1),
            step,
        },
    };
    assert_eq!(
        traverse(&mut ports, Standing::Back),
        no_entry(Standing::Back)
    );
    push(&mut ports, 1, Method::Post);
    push(&mut ports, 2, Method::Get);
    assert_eq!(
        traverse(&mut ports, Standing::Back),
        no_entry(Standing::Back)
    );
    assert_eq!(
        traverse(&mut ports, Standing::Forward),
        no_entry(Standing::Forward)
    );
    assert_eq!(ports.list().len(), 2);
}

#[test]
fn traverse_names_the_entry_its_address_and_the_entry_after_it() {
    let mut ports = NavigatorPorts::new();
    for n in 1..=3 {
        push(&mut ports, n, Method::Get);
    }
    assert_eq!(
        traverse(&mut ports, Standing::Back),
        TraverseHistoryOutcome::Requested {
            traversal_started: TraversalStarted {
                tab_id: tab(1),
                url: url("2"),
                entry_id: id(2),
                entry_next: Some(id(3)),
            }
        }
    );
    traversal(&mut ports, 10, 2, Some(3));
    commit(&mut ports, 10);
    assert_eq!(
        traverse(&mut ports, Standing::Forward),
        TraverseHistoryOutcome::Requested {
            traversal_started: TraversalStarted {
                tab_id: tab(1),
                url: url("3"),
                entry_id: id(3),
                entry_next: None,
            }
        }
    );
    assert_eq!(ports.list().len(), 4);
}
