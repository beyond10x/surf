//! Adversary cases for `story:agent-history-spec`: the navigator's `CommitNavigation` and
//! `TraverseHistory` held to a list model of a tab's session history (entries in order, an index
//! of the current one, the entries a push removed) over a fixed-seed sequence of pushes by GET
//! and POST, steps back and forward that commit or never do, in two tabs; and a step in a tab
//! with no history beside a tab that has one.

use surf_ports::NavigatorPorts;
use surf_types::behaviour::NavigationStorage;
use surf_types::navigation::obligations::{CommitNavigationBehavior, TraverseHistoryBehavior};
use surf_types::navigation::{
    CommitNavigation, CommitNavigationOutcome, HistoryHandling, NavigationData,
    NavigationSnapshot, NavigationState, NoHistoryEntry, Standing, TraverseHistory,
    TraverseHistoryOutcome,
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

fn url(n: u64) -> Url {
    Url(format!("http://example.org/{n}"))
}

fn start(
    ports: &mut NavigatorPorts,
    n: u64,
    t: u64,
    method: Method,
    handling: HistoryHandling,
    entry: Option<RequestId>,
    entry_next: Option<RequestId>,
) {
    ports.put(NavigationSnapshot {
        state: NavigationState::Started,
        data: NavigationData {
            navigation_id: id(n),
            tab_id: tab(t),
            url: url(n),
            method,
            handling,
            standing: None,
            next: None,
            entry_id: entry,
            entry_next,
        },
    });
}

fn commit(ports: &mut NavigatorPorts, n: u64) -> CommitNavigationOutcome {
    ports
        .commit_navigation(CommitNavigation {
            navigation_id: id(n),
            url: url(n),
            status: Status(200),
            media_type: MediaType("text/plain".to_owned()),
            body: Vec::new(),
        })
        .expect("CommitNavigation is met")
}

fn traverse(ports: &mut NavigatorPorts, t: u64, step: Standing) -> TraverseHistoryOutcome {
    ports
        .traverse_history(TraverseHistory {
            tab_id: tab(t),
            step,
        })
        .expect("TraverseHistory is met")
}

/// One tab's history as the HTML Standard lists it: entries in order and the current index.
#[derive(Default)]
struct Model {
    entries: Vec<(u64, Method)>,
    current: usize,
    dropped: Vec<u64>,
}

impl Model {
    fn push(&mut self, n: u64, method: Method) {
        if !self.entries.is_empty() {
            for (d, _) in self.entries.drain(self.current + 1..) {
                self.dropped.push(d);
            }
        }
        self.entries.push((n, method));
        self.current = self.entries.len() - 1;
    }

    /// The entry a step goes to, if one made by GET is there, and the entry after that one.
    fn target(&self, step: Standing) -> Option<(usize, u64, Option<u64>)> {
        let at = match step {
            Standing::Back if self.current > 0 && !self.entries.is_empty() => self.current - 1,
            Standing::Forward if self.current + 1 < self.entries.len() => self.current + 1,
            _ => return None,
        };
        let (n, method) = self.entries[at];
        (method == Method::Get).then(|| (at, n, self.entries.get(at + 1).map(|e| e.0)))
    }

    fn standing(&self, i: usize) -> Standing {
        match i {
            i if i + 1 < self.current => Standing::Earlier,
            i if i + 1 == self.current => Standing::Back,
            i if i == self.current => Standing::Current,
            i if i == self.current + 1 => Standing::Forward,
            _ => Standing::Later,
        }
    }
}

thread_local! {
    /// Every standing a check saw on some row during the walk.
    static SEEN: std::cell::RefCell<Vec<Standing>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// A fixed-seed generator (a 64-bit linear congruential one), so that every run walks the same
/// sequence.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % n
    }
}

fn check(ports: &NavigatorPorts, models: &[Model; 2], step: usize, what: &str) {
    SEEN.with(|seen| {
        let mut seen = seen.borrow_mut();
        for row in ports.list() {
            match row.data.standing {
                Some(s) if !seen.contains(&s) => seen.push(s),
                _ => {}
            }
        }
    });
    for (t, model) in models.iter().enumerate() {
        for (i, (n, _)) in model.entries.iter().enumerate() {
            let row = ports.get(&id(*n)).expect("an entry's row is held").data;
            assert_eq!(
                row.standing,
                Some(model.standing(i)),
                "step {step} ({what}): tab {t}, entry {i} of {} (current {})",
                model.entries.len(),
                model.current
            );
            let next = model.entries.get(i + 1).map(|e| id(e.0));
            assert_eq!(row.next, next, "step {step} ({what}): tab {t}, next of entry {i}");
        }
        for n in &model.dropped {
            let row = ports.get(&id(*n)).expect("a dropped entry's row is held").data;
            assert_eq!(row.standing, Some(Standing::Dropped), "step {step} ({what}): tab {t}");
        }
        let currents = ports
            .list()
            .into_iter()
            .filter(|s| s.data.tab_id == tab(t as u64) && s.data.standing == Some(Standing::Current))
            .count();
        assert_eq!(
            currents,
            usize::from(!model.entries.is_empty()),
            "step {step} ({what}): tab {t} has one current entry once it has history"
        );
    }
}

#[test]
fn a_seeded_walk_of_pushes_and_steps_in_two_tabs_matches_the_list_model() {
    let mut ports = NavigatorPorts::new();
    let mut models = [Model::default(), Model::default()];
    let mut rng = Lcg(0x5eed_2026_1122);
    let mut n = 0;
    for step in 0..400 {
        n += 1;
        let t = rng.below(2) as usize;
        let model = &mut models[t];
        match rng.below(10) {
            0..=2 => {
                let method = if rng.below(5) == 0 { Method::Post } else { Method::Get };
                start(&mut ports, n, t as u64, method, HistoryHandling::Push, None, None);
                if rng.below(6) == 0 {
                    // A push that never commits (it fails or is cancelled): nothing moves.
                    check(&ports, &models, step, "uncommitted push");
                    continue;
                }
                assert!(matches!(commit(&mut ports, n), CommitNavigationOutcome::Committed { .. }));
                model.push(n, method);
                check(&ports, &models, step, "push");
            }
            r => {
                let direction = if r < 6 { Standing::Back } else { Standing::Forward };
                let outcome = traverse(&mut ports, t as u64, direction);
                match model.target(direction) {
                    None => {
                        assert_eq!(
                            outcome,
                            TraverseHistoryOutcome::NoEntry {
                                error: NoHistoryEntry { tab_id: tab(t as u64), step: direction }
                            },
                            "step {step}: tab {t} {direction:?}"
                        );
                        check(&ports, &models, step, "refused step");
                    }
                    Some((at, entry, entry_next)) => {
                        let TraverseHistoryOutcome::Requested { traversal_started } = outcome else {
                            panic!("step {step}: tab {t} {direction:?} refused: {outcome:?}");
                        };
                        assert_eq!(traversal_started.entry_id, id(entry), "step {step}");
                        assert_eq!(traversal_started.entry_next, entry_next.map(id), "step {step}");
                        assert_eq!(traversal_started.url, url(entry), "step {step}");
                        start(
                            &mut ports,
                            n,
                            t as u64,
                            Method::Get,
                            HistoryHandling::Traverse,
                            Some(traversal_started.entry_id),
                            traversal_started.entry_next,
                        );
                        if rng.below(5) == 0 {
                            check(&ports, &models, step, "uncommitted step");
                            continue;
                        }
                        assert!(matches!(
                            commit(&mut ports, n),
                            CommitNavigationOutcome::Traversed { .. }
                        ));
                        model.current = at;
                        check(&ports, &models, step, "committed step");
                    }
                }
            }
        }
    }
    // The walk reached every standing.
    let seen = |s| SEEN.with(|seen| seen.borrow().contains(&s));
    for s in [
        Standing::Earlier,
        Standing::Back,
        Standing::Current,
        Standing::Forward,
        Standing::Later,
        Standing::Dropped,
    ] {
        assert!(seen(s), "{s:?} reached");
    }
}

#[test]
fn a_step_in_a_tab_without_history_is_refused_beside_a_tab_with_one() {
    let mut ports = NavigatorPorts::new();
    for n in 1..=3 {
        start(&mut ports, n, 1, Method::Get, HistoryHandling::Push, None, None);
        commit(&mut ports, n);
    }
    // Tab 1 now has an entry back; going back from it leaves one forward too.
    start(&mut ports, 4, 1, Method::Get, HistoryHandling::Traverse, Some(id(2)), Some(id(3)));
    commit(&mut ports, 4);
    for step in [Standing::Back, Standing::Forward] {
        assert_eq!(
            traverse(&mut ports, 2, step),
            TraverseHistoryOutcome::NoEntry {
                error: NoHistoryEntry {
                    tab_id: tab(2),
                    step
                }
            }
        );
    }
}
