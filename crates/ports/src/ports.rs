//! One ports type per component, each holding that component's rows and nothing else.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use surf_types::agent::obligations::{
    NoteNavigationCommittedBehavior, NoteNavigationEndedBehavior, NoteNavigationFailedBehavior,
    NoteNavigationStartedBehavior,
};
use surf_types::agent::{
    ActionId, ActionSnapshot, NoteNavigationCommitted, NoteNavigationCommittedOutcome,
    NoteNavigationEnded, NoteNavigationEndedOutcome, NoteNavigationFailed,
    NoteNavigationFailedOutcome, NoteNavigationStarted, NoteNavigationStartedOutcome, SessionData,
    SessionId, SessionNavigationChanged, SessionSnapshot,
};
use surf_types::behaviour::{
    ActionStorage, DocumentStorage, ElementStorage, FormStorage, FrameStorage, ListOptionStorage,
    NavigationStorage, RequestStorage, ScriptStorage, SessionStorage, SubmissionStorage,
    TabStorage, TryContext, WindowStorage, unmet_context,
};
use surf_types::document::obligations::{CheckBehavior, SelectOptionBehavior};
use surf_types::document::{
    Check, CheckOutcome, CheckednessSet, ControlDisabled, ControlHidden, ControlKindConflict,
    DocumentId, DocumentSnapshot, ElementData, ElementId, ElementKind, ElementNotFound,
    ElementSnapshot, FormId, FormSnapshot, ListOptionData, ListOptionSnapshot, OptionId,
    OptionNotFound, OptionSelected, ScriptId, ScriptSnapshot, SelectOption, SelectOptionOutcome,
    SubmissionId, SubmissionSnapshot,
};
use surf_types::navigation::obligations::{CommitNavigationBehavior, TraverseHistoryBehavior};
use surf_types::navigation::{
    CommitNavigation, CommitNavigationOutcome, HistoryHandling, NavigationCommitted,
    NavigationData, NavigationNotFound, NavigationSnapshot, NavigationState,
    NavigationStateConflict, NoHistoryEntry, Standing, TraversalStarted, TraversalUnsupported,
    TraverseHistory, TraverseHistoryOutcome,
};
use surf_types::net::{MediaType, Method, RequestId, RequestSnapshot};
use surf_types::obligation::UnmetObligation;
use surf_types::render::{FrameId, FrameSnapshot};
use surf_types::shell::{TabId, TabSnapshot, WindowId, WindowSnapshot};

use crate::random::{OsRandom, RandomSource, assign};
use crate::store::{Keyed, Rows};

macro_rules! keyed {
    ($snapshot:ty, $key:ty, $field:ident) => {
        impl Keyed for $snapshot {
            type Key = $key;

            fn key(&self) -> &$key {
                &self.data.$field
            }

            fn text(key: &$key) -> &str {
                &key.0.0
            }
        }
    };
}

keyed!(WindowSnapshot, WindowId, window_id);
keyed!(TabSnapshot, TabId, tab_id);
keyed!(NavigationSnapshot, RequestId, navigation_id);
keyed!(RequestSnapshot, RequestId, request_id);
keyed!(DocumentSnapshot, DocumentId, document_id);
keyed!(ElementSnapshot, ElementId, element_id);
keyed!(FormSnapshot, FormId, form_id);
keyed!(ListOptionSnapshot, OptionId, option_id);
keyed!(SubmissionSnapshot, SubmissionId, submission_id);
keyed!(ScriptSnapshot, ScriptId, script_id);
keyed!(FrameSnapshot, FrameId, frame_id);
keyed!(SessionSnapshot, SessionId, session_id);
keyed!(ActionSnapshot, ActionId, action_id);

/// What a context answer about the caller names when it is unmet.
const CALLER: &str = "the caller's session_id";

/// The identity of an action, as a context answer names it.
const ACTION_ID: &str = "surf.agent.ActionId";

/// The `Optional<String>` a `{generated: true}` payload field asks for, as a context answer
/// names it: the `handle` and `label` of the `NavigationRequested` a `Go` emits.
const OPTIONAL_STRING: &str = "Optional<String>";

/// The `Optional<Bytes>` and `Optional<surf.net.MediaType>` a `{generated: true}` payload field
/// asks for, as a context answer names them: the body and its media type of the
/// `NavigationStarted` a `StartTraversal` emits.
const OPTIONAL_BYTES: &str = "Optional<Bytes>";
const OPTIONAL_MEDIA_TYPE: &str = "Optional<surf.net.MediaType>";

/// Implements one storage trait for a ports type over one of its private tables.
macro_rules! storage {
    ($storage:ident for $ports:ty, $rows:ident: $snapshot:ty, $key:ty $(, $param:ident)?) => {
        impl$(<$param>)? $storage for $ports {
            fn get(&self, identity: &$key) -> Option<$snapshot> {
                self.$rows.get(identity)
            }

            fn put(&mut self, snapshot: $snapshot) {
                self.$rows.put(snapshot);
            }

            fn delete(&mut self, identity: &$key) {
                self.$rows.delete(identity);
            }

            fn list(&self) -> Vec<$snapshot> {
                self.$rows.list()
            }
        }
    };
    // A storage trait without `list`: the entity has no view, so the generation asks for none.
    ($storage:ident for $ports:ty, $rows:ident: $snapshot:ty, $key:ty, $param:ident, unlisted) => {
        impl<$param> $storage for $ports {
            fn get(&self, identity: &$key) -> Option<$snapshot> {
                self.$rows.get(identity)
            }

            fn put(&mut self, snapshot: $snapshot) {
                self.$rows.put(snapshot);
            }

            fn delete(&mut self, identity: &$key) {
                self.$rows.delete(identity);
            }
        }
    };
}

/// Implements `TryContext` for a ports type over its random source.
///
/// `TryContext` asks for all eleven kinds of identity. These ports assign each of the ten that
/// belong to the page load, a document's elements, forms, options, submissions and script ends
/// included; the generated behaviours of a component ask only for their own.
///
/// It also asks for the caller's `session_id` and an action's identity, which only the `agent`
/// component's commands read. These ports know no caller and record no action, so they answer
/// both as unmet rather than as a caller with none or an identity for no action.
///
/// And it asks for the `Optional<Bytes>`, `Optional<surf.net.MediaType>` and `Optional<String>`
/// of the `NavigationStarted` a `StartTraversal` emits, a traversal's request having no body and
/// no initiator (`story:agent-cookie-jar`). The navigator's ports answer the three absent
/// (`context!(NavigatorPorts, absent_body)`); the others start no traversal and answer them as
/// unmet (the `Optional<String>` of a `Go`'s request is the agent's ports' own).
macro_rules! context {
    ($ports:ident) => {
        context!($ports, {
            fn try_generate_optional_bytes(&mut self) -> Result<Option<Vec<u8>>, UnmetObligation> {
                Err(unmet_context(OPTIONAL_BYTES))
            }

            fn try_generate_optional_surf_net_media_type(
                &mut self,
            ) -> Result<Option<MediaType>, UnmetObligation> {
                Err(unmet_context(OPTIONAL_MEDIA_TYPE))
            }

            fn try_generate_optional_string(&mut self) -> Result<Option<String>, UnmetObligation> {
                Err(unmet_context(OPTIONAL_STRING))
            }
        });
    };
    ($ports:ident, absent_body) => {
        context!($ports, {
            fn try_generate_optional_bytes(&mut self) -> Result<Option<Vec<u8>>, UnmetObligation> {
                Ok(None)
            }

            fn try_generate_optional_surf_net_media_type(
                &mut self,
            ) -> Result<Option<MediaType>, UnmetObligation> {
                Ok(None)
            }

            fn try_generate_optional_string(&mut self) -> Result<Option<String>, UnmetObligation> {
                Ok(None)
            }
        });
    };
    ($ports:ident, { $($body:item)* }) => {
        impl<R: RandomSource> TryContext for $ports<R> {
            $($body)*

            fn try_caller_session_id(&self) -> Result<Option<SessionId>, UnmetObligation> {
                Err(unmet_context(CALLER))
            }

            fn try_generate_surf_agent_action_id(&mut self) -> Result<ActionId, UnmetObligation> {
                Err(unmet_context(ACTION_ID))
            }

            fn try_generate_surf_document_document_id(
                &mut self,
            ) -> Result<DocumentId, UnmetObligation> {
                assign(&mut self.random, "surf.document.DocumentId").map(DocumentId)
            }

            fn try_generate_surf_document_element_id(
                &mut self,
            ) -> Result<ElementId, UnmetObligation> {
                assign(&mut self.random, "surf.document.ElementId").map(ElementId)
            }

            fn try_generate_surf_document_form_id(&mut self) -> Result<FormId, UnmetObligation> {
                assign(&mut self.random, "surf.document.FormId").map(FormId)
            }

            fn try_generate_surf_document_option_id(
                &mut self,
            ) -> Result<OptionId, UnmetObligation> {
                assign(&mut self.random, "surf.document.OptionId").map(OptionId)
            }

            fn try_generate_surf_document_script_id(
                &mut self,
            ) -> Result<ScriptId, UnmetObligation> {
                assign(&mut self.random, "surf.document.ScriptId").map(ScriptId)
            }

            fn try_generate_surf_document_submission_id(
                &mut self,
            ) -> Result<SubmissionId, UnmetObligation> {
                assign(&mut self.random, "surf.document.SubmissionId").map(SubmissionId)
            }

            fn try_generate_surf_net_request_id(&mut self) -> Result<RequestId, UnmetObligation> {
                assign(&mut self.random, "surf.net.RequestId").map(RequestId)
            }

            fn try_generate_surf_render_frame_id(&mut self) -> Result<FrameId, UnmetObligation> {
                assign(&mut self.random, "surf.render.FrameId").map(FrameId)
            }

            fn try_generate_surf_shell_tab_id(&mut self) -> Result<TabId, UnmetObligation> {
                assign(&mut self.random, "surf.shell.TabId").map(TabId)
            }

            fn try_generate_surf_shell_window_id(&mut self) -> Result<WindowId, UnmetObligation> {
                assign(&mut self.random, "surf.shell.WindowId").map(WindowId)
            }
        }

        impl $ports<OsRandom> {
            /// Empty tables, with identities drawn from the operating system's random source.
            pub fn new() -> Self {
                Self::with_source(OsRandom::new())
            }
        }

        impl Default for $ports<OsRandom> {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

/// The `shell` component's ports: its windows and tabs, and the random source its identities
/// come from.
#[derive(Debug)]
pub struct ShellPorts<R = OsRandom> {
    windows: Rows<WindowSnapshot>,
    tabs: Rows<TabSnapshot>,
    random: R,
}

impl<R: RandomSource> ShellPorts<R> {
    /// Empty tables, with identities drawn from `source`.
    pub fn with_source(source: R) -> Self {
        Self {
            windows: Rows::default(),
            tabs: Rows::default(),
            random: source,
        }
    }
}

storage!(WindowStorage for ShellPorts<R>, windows: WindowSnapshot, WindowId, R);
storage!(TabStorage for ShellPorts<R>, tabs: TabSnapshot, TabId, R);
context!(ShellPorts);

/// The `navigator` component's ports: its navigations, which are also its tabs' session history
/// entries, and the random source its identities come from.
///
/// It also meets the two obligations the generation leaves to it: `CommitNavigation`, which
/// moves the tab's other entries beside its subject (`affects:`), and `TraverseHistory`, which
/// reads the entry one step from the current one (a row set).
#[derive(Debug)]
pub struct NavigatorPorts<R = OsRandom> {
    navigations: Rows<NavigationSnapshot>,
    random: R,
}

impl<R: RandomSource> NavigatorPorts<R> {
    /// An empty table, with identities drawn from `source`.
    pub fn with_source(source: R) -> Self {
        Self {
            navigations: Rows::default(),
            random: source,
        }
    }
}

storage!(NavigationStorage for NavigatorPorts<R>, navigations: NavigationSnapshot, RequestId, R);
context!(NavigatorPorts, absent_body);

/// Changes every other navigation of `tab` in `rows` that `selects` picks: one `affects:` entry
/// of `CommitNavigation`.
fn stand<S: NavigationStorage + ?Sized>(
    rows: &mut S,
    subject: &RequestId,
    tab: &TabId,
    selects: impl Fn(&NavigationData) -> bool,
    change: impl Fn(&mut NavigationData),
) {
    for mut snapshot in rows.list() {
        let row = &snapshot.data;
        if &row.navigation_id != subject && &row.tab_id == tab && selects(row) {
            change(&mut snapshot.data);
            rows.put(snapshot);
        }
    }
}

/// The tab's entry at `step` in `rows`, made by `Get`: the row set `TraverseHistory` reads. A
/// row with no standing is no entry.
fn entry_at<S: NavigationStorage + ?Sized>(
    rows: &S,
    tab: &TabId,
    step: Standing,
) -> Option<NavigationData> {
    rows.list()
        .into_iter()
        .map(|snapshot| snapshot.data)
        .find(|row| &row.tab_id == tab && row.standing == Some(step) && row.method == Method::Get)
}

/// A response's arrival, its outcomes chosen in the order `spec/` declares them: a navigation no
/// row holds is refused, and one that already ended (`wrong-state`). A traversal's commit makes
/// the entry it went to current and adds none; a push's makes the navigation its tab's current
/// entry, after the one that was current, with the method of its committing request, drops the
/// entries after that one, and cancels every traversal of the tab still `Started`, so that one
/// answers `wrong-state` and commits nothing. The effects on the tab's other rows apply in the
/// order `spec/` writes them.
///
/// It reads and writes `rows` only, so a ports type that wraps a [`NavigationStorage`] meets the
/// obligation over its own storage.
///
/// # Errors
///
/// None: the obligation is met for every input. The `Result` is the obligation's signature.
pub fn commit_navigation<S: NavigationStorage + ?Sized>(
    rows: &mut S,
    input: CommitNavigation,
) -> Result<CommitNavigationOutcome, UnmetObligation> {
    let Some(mut subject) = rows.get(&input.navigation_id) else {
        return Ok(CommitNavigationOutcome::NoSuchNavigation {
            error: NavigationNotFound {
                navigation_id: input.navigation_id,
            },
        });
    };
    if subject.state != NavigationState::Started {
        return Ok(CommitNavigationOutcome::WrongState {
            error: NavigationStateConflict {
                state: subject.state,
            },
        });
    }
    let before = subject.data.clone();
    let id = &input.navigation_id;
    let tab = &before.tab_id;
    let navigation_committed = NavigationCommitted {
        navigation_id: input.navigation_id.clone(),
        tab_id: before.tab_id.clone(),
        url: input.url.clone(),
        status: input.status,
        media_type: input.media_type,
        body: input.body,
    };
    subject.state = NavigationState::Committed;
    subject.data.url = input.url;
    if before.handling == HistoryHandling::Traverse {
        rows.put(subject);
        let at = |standing| move |row: &NavigationData| row.standing == Some(standing);
        let to = |standing| move |row: &mut NavigationData| row.standing = Some(standing);
        stand(rows, id, tab, at(Standing::Back), to(Standing::Earlier));
        stand(rows, id, tab, at(Standing::Forward), to(Standing::Later));
        stand(rows, id, tab, at(Standing::Current), to(Standing::Earlier));
        let entry = before.entry_id.clone();
        let entry_next = before.entry_next.clone();
        stand(
            rows,
            id,
            tab,
            |row| entry.is_some() && row.next == entry,
            to(Standing::Back),
        );
        stand(
            rows,
            id,
            tab,
            |row| entry_next.as_ref() == Some(&row.navigation_id),
            to(Standing::Forward),
        );
        stand(
            rows,
            id,
            tab,
            |row| entry.as_ref() == Some(&row.navigation_id),
            to(Standing::Current),
        );
        return Ok(CommitNavigationOutcome::Traversed {
            navigation_committed,
        });
    }
    subject.data.standing = Some(Standing::Current);
    subject.data.method = input.method;
    rows.put(subject);
    stand(
        rows,
        id,
        tab,
        |row| matches!(row.standing, Some(Standing::Forward | Standing::Later)),
        |row| row.standing = Some(Standing::Dropped),
    );
    stand(
        rows,
        id,
        tab,
        |row| row.standing == Some(Standing::Back),
        |row| row.standing = Some(Standing::Earlier),
    );
    stand(
        rows,
        id,
        tab,
        |row| row.standing == Some(Standing::Current),
        |row| {
            row.standing = Some(Standing::Back);
            row.next = Some(id.clone());
        },
    );
    for mut snapshot in rows.list() {
        let row = &snapshot.data;
        if &row.navigation_id != id
            && &row.tab_id == tab
            && row.handling == HistoryHandling::Traverse
            && snapshot.state == NavigationState::Started
        {
            snapshot.state = NavigationState::Cancelled;
            rows.put(snapshot);
        }
    }
    Ok(CommitNavigationOutcome::Committed {
        navigation_committed,
    })
}

impl<R> CommitNavigationBehavior for NavigatorPorts<R> {
    fn commit_navigation(
        &mut self,
        input: CommitNavigation,
    ) -> Result<CommitNavigationOutcome, UnmetObligation> {
        commit_navigation(self, input)
    }
}

/// One step back or forward, its outcomes chosen in the order `spec/` declares them: a step that
/// is neither is refused before any row is read; then a tab with no entry made by `Get` at that
/// step is refused; otherwise the entry's address, the entry and the entry after it go to
/// `StartTraversal`. It changes no row.
///
/// It reads `rows` only, as [`commit_navigation`] does.
///
/// # Errors
///
/// None: the obligation is met for every input. The `Result` is the obligation's signature.
pub fn traverse_history<S: NavigationStorage + ?Sized>(
    rows: &S,
    input: TraverseHistory,
) -> Result<TraverseHistoryOutcome, UnmetObligation> {
    if input.step != Standing::Back && input.step != Standing::Forward {
        return Ok(TraverseHistoryOutcome::Unsupported {
            error: TraversalUnsupported { step: input.step },
        });
    }
    let Some(entry) = entry_at(rows, &input.tab_id, input.step) else {
        return Ok(TraverseHistoryOutcome::NoEntry {
            error: NoHistoryEntry {
                tab_id: input.tab_id,
                step: input.step,
            },
        });
    };
    Ok(TraverseHistoryOutcome::Requested {
        traversal_started: TraversalStarted {
            tab_id: input.tab_id,
            url: entry.url,
            entry_id: entry.navigation_id,
            entry_next: entry.next,
        },
    })
}

impl<R> TraverseHistoryBehavior for NavigatorPorts<R> {
    fn traverse_history(
        &mut self,
        input: TraverseHistory,
    ) -> Result<TraverseHistoryOutcome, UnmetObligation> {
        traverse_history(self, input)
    }
}

/// The `net` component's ports: its requests.
///
/// No random source and no `TryContext`: the `net` behaviours assign no identity, a request
/// carries the one its `Fetch` names.
#[derive(Debug, Default)]
pub struct NetPorts {
    requests: Rows<RequestSnapshot>,
}

impl NetPorts {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }
}

storage!(RequestStorage for NetPorts, requests: RequestSnapshot, RequestId);

/// The `document` component's ports: its documents, their forms, elements and options, the
/// submissions of their forms, the records of their scripts' ends, and the random source their
/// identities come from.
///
/// It also meets the two obligations the generation leaves to it, `Check` and `SelectOption`,
/// which change other rows beside their subject (`affects:`). `AddScriptEnd` is generated whole:
/// these ports store the record it writes, as they store a form's.
#[derive(Debug)]
pub struct DocumentPorts<R = OsRandom> {
    documents: Rows<DocumentSnapshot>,
    forms: Rows<FormSnapshot>,
    elements: Rows<ElementSnapshot>,
    options: Rows<ListOptionSnapshot>,
    submissions: Rows<SubmissionSnapshot>,
    scripts: Rows<ScriptSnapshot>,
    random: R,
}

impl<R: RandomSource> DocumentPorts<R> {
    /// Empty tables, with identities drawn from `source`.
    pub fn with_source(source: R) -> Self {
        Self {
            documents: Rows::default(),
            forms: Rows::default(),
            elements: Rows::default(),
            options: Rows::default(),
            submissions: Rows::default(),
            scripts: Rows::default(),
            random: source,
        }
    }
}

impl<R> DocumentPorts<R> {
    /// Writes `change` into every stored element `selects` picks.
    fn change_elements(
        &mut self,
        selects: impl Fn(&ElementData) -> bool,
        change: impl Fn(&mut ElementData),
    ) {
        for mut snapshot in self.elements.list() {
            if selects(&snapshot.data) {
                change(&mut snapshot.data);
                self.elements.put(snapshot);
            }
        }
    }

    /// Writes `change` into every stored option `selects` picks.
    fn change_options(
        &mut self,
        selects: impl Fn(&ListOptionData) -> bool,
        change: impl Fn(&mut ListOptionData),
    ) {
        for mut snapshot in self.options.list() {
            if selects(&snapshot.data) {
                change(&mut snapshot.data);
                self.options.put(snapshot);
            }
        }
    }
}

storage!(DocumentStorage for DocumentPorts<R>, documents: DocumentSnapshot, DocumentId, R);
storage!(FormStorage for DocumentPorts<R>, forms: FormSnapshot, FormId, R);
storage!(ElementStorage for DocumentPorts<R>, elements: ElementSnapshot, ElementId, R);
storage!(ListOptionStorage for DocumentPorts<R>, options: ListOptionSnapshot, OptionId, R);
storage!(SubmissionStorage for DocumentPorts<R>, submissions: SubmissionSnapshot, SubmissionId, R);
storage!(ScriptStorage for DocumentPorts<R>, scripts: ScriptSnapshot, ScriptId, R, unlisted);
context!(DocumentPorts);

/// A checkbox's or a radio button's activation, its outcomes chosen in the order `spec/` declares
/// them: an element no row holds, one neither a checkbox nor a radio button, a disabled one and
/// a hidden one are refused; a radio button is checked and every other radio button of its group
/// (same document, same `radio_group` key: the same form owner and name, or no form owner and
/// the same name) unchecked, where a radio button with no key (no name) selects no other; a
/// checked checkbox is unchecked, and an unchecked one checked.
impl<R> CheckBehavior for DocumentPorts<R> {
    fn check(&mut self, input: Check) -> Result<CheckOutcome, UnmetObligation> {
        let Some(mut subject) = self.elements.get(&input.element_id) else {
            return Ok(CheckOutcome::NoSuchElement {
                error: ElementNotFound {
                    element_id: input.element_id,
                },
            });
        };
        let element = &subject.data;
        if element.kind != ElementKind::Checkbox && element.kind != ElementKind::Radio {
            return Ok(CheckOutcome::NotCheckable {
                error: ControlKindConflict {
                    element_id: input.element_id,
                },
            });
        }
        if element.disabled {
            return Ok(CheckOutcome::Disabled {
                error: ControlDisabled {
                    element_id: input.element_id,
                },
            });
        }
        if element.hidden {
            return Ok(CheckOutcome::Hidden {
                error: ControlHidden {
                    element_id: input.element_id,
                },
            });
        }
        if element.kind == ElementKind::Radio {
            let group = element.clone();
            subject.data.checked = true;
            self.elements.put(subject);
            self.change_elements(
                |other| {
                    other.element_id != group.element_id
                        && other.kind == ElementKind::Radio
                        && other.document_id == group.document_id
                        && other.radio_group.is_some()
                        && other.radio_group == group.radio_group
                },
                |other| other.checked = false,
            );
            return Ok(CheckOutcome::RadioChecked {
                checkedness_set: CheckednessSet {
                    element_id: input.element_id,
                    checked: true,
                },
            });
        }
        let checked = !element.checked;
        subject.data.checked = checked;
        self.elements.put(subject);
        let checkedness_set = CheckednessSet {
            element_id: input.element_id,
            checked,
        };
        Ok(if checked {
            CheckOutcome::Checked { checkedness_set }
        } else {
            CheckOutcome::Unchecked { checkedness_set }
        })
    }
}

/// An option's selection, its outcomes chosen in the order `spec/` declares them: an option no
/// row holds, a disabled one and a hidden one are refused, naming its select; otherwise it is
/// selected and every other option of its select unselected.
impl<R> SelectOptionBehavior for DocumentPorts<R> {
    fn select_option(
        &mut self,
        input: SelectOption,
    ) -> Result<SelectOptionOutcome, UnmetObligation> {
        let Some(mut subject) = self.options.get(&input.option_id) else {
            return Ok(SelectOptionOutcome::NoSuchOption {
                error: OptionNotFound {
                    option_id: input.option_id,
                },
            });
        };
        let select_id = subject.data.select_id.clone();
        if subject.data.disabled {
            return Ok(SelectOptionOutcome::Disabled {
                error: ControlDisabled {
                    element_id: select_id,
                },
            });
        }
        if subject.data.hidden {
            return Ok(SelectOptionOutcome::Hidden {
                error: ControlHidden {
                    element_id: select_id,
                },
            });
        }
        subject.data.selected = true;
        self.options.put(subject);
        self.change_options(
            |other| other.option_id != input.option_id && other.select_id == select_id,
            |other| other.selected = false,
        );
        Ok(SelectOptionOutcome::Selected {
            option_selected: OptionSelected {
                option_id: input.option_id,
                select_id,
            },
        })
    }
}

/// The `renderer` component's ports: its frames, and the random source its identities come
/// from.
#[derive(Debug)]
pub struct RendererPorts<R = OsRandom> {
    frames: Rows<FrameSnapshot>,
    random: R,
}

impl<R: RandomSource> RendererPorts<R> {
    /// An empty table, with identities drawn from `source`.
    pub fn with_source(source: R) -> Self {
        Self {
            frames: Rows::default(),
            random: source,
        }
    }
}

storage!(FrameStorage for RendererPorts<R>, frames: FrameSnapshot, FrameId, R);
context!(RendererPorts);

/// Who the `agent` component's commands are sent as: the `session_id` the caller's credential
/// carries, or none.
///
/// A handle: every clone reads and writes the same answer. [`AgentPorts`] keeps one and answers
/// the generated holder check from it; whoever authenticates a call keeps another and sets it
/// before issuing the command. The specification leaves the authentication to the session
/// process (`spec/domains/agent.yaml`, the marker at the token's type).
#[derive(Debug, Clone, Default)]
pub struct Caller(Arc<Mutex<Option<SessionId>>>);

impl Caller {
    /// A handle that names no caller yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sends every later command as a caller carrying `session_id`, or as one carrying none.
    pub fn set(&self, session_id: Option<SessionId>) {
        *self.held() = session_id;
    }

    /// The `session_id` the caller carries, if any.
    pub fn get(&self) -> Option<SessionId> {
        self.held().clone()
    }

    fn held(&self) -> MutexGuard<'_, Option<SessionId>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The `agent` component's ports: its sessions and their actions, the [`Caller`] its commands are
/// sent as, and the random source an action's identity comes from.
///
/// A session carries the identity its `OpenSession` names; an action gets one assigned when it is
/// recorded. It also meets the four obligations the generation leaves to it,
/// `NoteNavigationStarted`, `NoteNavigationCommitted`, `NoteNavigationFailed` and
/// `NoteNavigationEnded`, which change every session their filter selects.
#[derive(Debug)]
pub struct AgentPorts<R = OsRandom> {
    sessions: Rows<SessionSnapshot>,
    actions: Rows<ActionSnapshot>,
    caller: Caller,
    random: R,
}

impl AgentPorts<OsRandom> {
    /// Empty tables, with a caller of its own that names no one, and identities drawn from the
    /// operating system's random source.
    pub fn new() -> Self {
        Self::with_caller(Caller::new())
    }

    /// Empty tables, its commands sent as whoever `caller` names when each runs, and identities
    /// drawn from the operating system's random source.
    pub fn with_caller(caller: Caller) -> Self {
        Self::with_caller_and_source(caller, OsRandom::new())
    }
}

impl Default for AgentPorts<OsRandom> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: RandomSource> AgentPorts<R> {
    /// Empty tables, with a caller of its own that names no one, and identities drawn from
    /// `source`.
    pub fn with_source(source: R) -> Self {
        Self::with_caller_and_source(Caller::new(), source)
    }

    /// Empty tables, its commands sent as whoever `caller` names when each runs, and identities
    /// drawn from `source`.
    pub fn with_caller_and_source(caller: Caller, source: R) -> Self {
        Self {
            sessions: Rows::default(),
            actions: Rows::default(),
            caller,
            random: source,
        }
    }
}

impl<R> AgentPorts<R> {
    /// A handle to the caller these ports answer with.
    pub fn caller(&self) -> Caller {
        self.caller.clone()
    }

    /// Writes `change` into every stored session `selects` picks, and answers how many it changed.
    fn change_every(
        &mut self,
        selects: impl Fn(&SessionData) -> bool,
        change: impl Fn(&mut SessionData),
    ) -> i64 {
        let mut changed = 0;
        for mut snapshot in self.sessions.list() {
            if selects(&snapshot.data) {
                change(&mut snapshot.data);
                self.sessions.put(snapshot);
                changed += 1;
            }
        }
        changed
    }
}

storage!(SessionStorage for AgentPorts<R>, sessions: SessionSnapshot, SessionId, R);
storage!(ActionStorage for AgentPorts<R>, actions: ActionSnapshot, ActionId, R);

impl<R: RandomSource> TryContext for AgentPorts<R> {
    fn try_caller_session_id(&self) -> Result<Option<SessionId>, UnmetObligation> {
        Ok(self.caller.get())
    }

    /// The `handle` and `label` of the `NavigationRequested` a `Go` emits: a `go` names no link,
    /// so both are absent (`spec/domains/agent.yaml`, `surf.agent.NavigationRequested`).
    fn try_generate_optional_string(&mut self) -> Result<Option<String>, UnmetObligation> {
        Ok(None)
    }

    fn try_generate_optional_bytes(&mut self) -> Result<Option<Vec<u8>>, UnmetObligation> {
        Err(unmet_context(OPTIONAL_BYTES))
    }

    fn try_generate_optional_surf_net_media_type(
        &mut self,
    ) -> Result<Option<MediaType>, UnmetObligation> {
        Err(unmet_context(OPTIONAL_MEDIA_TYPE))
    }

    fn try_generate_surf_agent_action_id(&mut self) -> Result<ActionId, UnmetObligation> {
        assign(&mut self.random, ACTION_ID).map(ActionId)
    }

    fn try_generate_surf_document_document_id(&mut self) -> Result<DocumentId, UnmetObligation> {
        Err(unmet_context("surf.document.DocumentId"))
    }

    fn try_generate_surf_document_element_id(&mut self) -> Result<ElementId, UnmetObligation> {
        Err(unmet_context("surf.document.ElementId"))
    }

    fn try_generate_surf_document_form_id(&mut self) -> Result<FormId, UnmetObligation> {
        Err(unmet_context("surf.document.FormId"))
    }

    fn try_generate_surf_document_option_id(&mut self) -> Result<OptionId, UnmetObligation> {
        Err(unmet_context("surf.document.OptionId"))
    }

    fn try_generate_surf_document_script_id(&mut self) -> Result<ScriptId, UnmetObligation> {
        Err(unmet_context("surf.document.ScriptId"))
    }

    fn try_generate_surf_document_submission_id(
        &mut self,
    ) -> Result<SubmissionId, UnmetObligation> {
        Err(unmet_context("surf.document.SubmissionId"))
    }

    fn try_generate_surf_net_request_id(&mut self) -> Result<RequestId, UnmetObligation> {
        Err(unmet_context("surf.net.RequestId"))
    }

    fn try_generate_surf_render_frame_id(&mut self) -> Result<FrameId, UnmetObligation> {
        Err(unmet_context("surf.render.FrameId"))
    }

    fn try_generate_surf_shell_tab_id(&mut self) -> Result<TabId, UnmetObligation> {
        Err(unmet_context("surf.shell.TabId"))
    }

    fn try_generate_surf_shell_window_id(&mut self) -> Result<WindowId, UnmetObligation> {
        Err(unmet_context("surf.shell.WindowId"))
    }
}

/// `noted`: every session holding the tab shows the navigation as `Started`, at its address, and
/// nothing of the page before it: no status, media type or failure reason.
impl<R> NoteNavigationStartedBehavior for AgentPorts<R> {
    fn note_navigation_started(
        &mut self,
        input: NoteNavigationStarted,
    ) -> Result<NoteNavigationStartedOutcome, UnmetObligation> {
        let sessions = self.change_every(
            |session| session.tab_id == input.tab_id,
            |session| {
                session.navigation_id = Some(input.navigation_id.clone());
                session.url = Some(input.url.clone());
                session.navigation_state = Some(NavigationState::Started);
                session.status = None;
                session.media_type = None;
                session.failure_reason = None;
            },
        );
        Ok(NoteNavigationStartedOutcome::Noted {
            session_navigation_changed: SessionNavigationChanged {
                navigation_id: input.navigation_id,
                state: NavigationState::Started,
                sessions,
            },
        })
    }
}

/// `noted`: every session showing the navigation shows it `Committed` at the response's address
/// (the one after the last redirect), with its status and media type.
impl<R> NoteNavigationCommittedBehavior for AgentPorts<R> {
    fn note_navigation_committed(
        &mut self,
        input: NoteNavigationCommitted,
    ) -> Result<NoteNavigationCommittedOutcome, UnmetObligation> {
        let sessions = self.change_every(
            |session| session.navigation_id.as_ref() == Some(&input.navigation_id),
            |session| {
                session.url = Some(input.url.clone());
                session.navigation_state = Some(NavigationState::Committed);
                session.status = Some(input.status.clone());
                session.media_type = Some(input.media_type.clone());
            },
        );
        Ok(NoteNavigationCommittedOutcome::Noted {
            session_navigation_changed: SessionNavigationChanged {
                navigation_id: input.navigation_id,
                state: NavigationState::Committed,
                sessions,
            },
        })
    }
}

/// `noted`: every session showing the navigation shows it `Failed`, with its reason.
impl<R> NoteNavigationFailedBehavior for AgentPorts<R> {
    fn note_navigation_failed(
        &mut self,
        input: NoteNavigationFailed,
    ) -> Result<NoteNavigationFailedOutcome, UnmetObligation> {
        let sessions = self.change_every(
            |session| session.navigation_id.as_ref() == Some(&input.navigation_id),
            |session| {
                session.navigation_state = Some(NavigationState::Failed);
                session.failure_reason = Some(input.reason);
            },
        );
        Ok(NoteNavigationFailedOutcome::Noted {
            session_navigation_changed: SessionNavigationChanged {
                navigation_id: input.navigation_id,
                state: NavigationState::Failed,
                sessions,
            },
        })
    }
}

/// `noted`: every session showing the navigation shows the state it ended in.
impl<R> NoteNavigationEndedBehavior for AgentPorts<R> {
    fn note_navigation_ended(
        &mut self,
        input: NoteNavigationEnded,
    ) -> Result<NoteNavigationEndedOutcome, UnmetObligation> {
        let sessions = self.change_every(
            |session| session.navigation_id.as_ref() == Some(&input.navigation_id),
            |session| session.navigation_state = Some(input.state),
        );
        Ok(NoteNavigationEndedOutcome::Noted {
            session_navigation_changed: SessionNavigationChanged {
                navigation_id: input.navigation_id,
                state: input.state,
                sessions,
            },
        })
    }
}
