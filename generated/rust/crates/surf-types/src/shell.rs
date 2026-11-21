// generated from surf v1
// contract digest 19964b50f9197185811d923fbd6c3eec7ec2855a2cc03c2c60d6e4a2149578f0
// do not edit: regenerate with `ess synthesize`

//! Shell — `surf.shell`.
//!
//! The windows a user has open and the tabs inside them.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// The states of `surf.shell.Tab`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Tab<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabState {
    /// `Closed`.
    Closed,
    /// `Open`.
    Open,
}

/// TabId — `surf.shell.TabId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabId(pub crate::primitives::Uuid);

/// The states of `surf.shell.Window`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Window<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    /// `Open`.
    Open,
}

/// WindowId — `surf.shell.WindowId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowId(pub crate::primitives::Uuid);

/// What Tab — `surf.shell.Tab` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Tab<S>`], and at a boundary by [`TabSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabData {
    /// The identity: `tab_id` — `surf.shell.TabId`.
    pub tab_id: TabId,
    /// `window_id` — `surf.shell.WindowId`.
    ///
    /// Carries `tabs`: `surf.shell.Window` owns many `surf.shell.Tab`.
    pub window_id: WindowId,
}

/// The states of `surf.shell.Tab`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](tab_state::Marker), so [`Tab<S>`](Tab) can only ever rest in a real state.
pub mod tab_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Closed {}
        impl Sealed for super::Open {}
    }

    /// A declared state of `Tab`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::TabState;
    }

    /// `Closed`. Terminal: an instance may rest here forever.
    pub struct Closed;

    impl Marker for Closed {
        const STATE: super::TabState = super::TabState::Closed;
    }

    /// `Open`. Where a new instance starts.
    pub struct Open;

    impl Marker for Open {
        const STATE: super::TabState = super::TabState::Open;
    }
}

/// Tab — `surf.shell.Tab` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Open`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`TabSnapshot`]
/// and [`TabSnapshot::refine`].
pub struct Tab<S: tab_state::Marker> {
    data: TabData,
    state: core::marker::PhantomData<S>,
}

impl<S: tab_state::Marker> Tab<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> TabState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &TabData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> TabData {
        self.data
    }
}

impl Tab<tab_state::Open> {
    /// A new instance, resting in `Open` — the only state the lifecycle starts one in.
    pub fn new(data: TabData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Tab<tab_state::Open> {
    /// `close` — `Open` → `Closed`. Taken by the `closed` outcome of `surf.shell.CloseTab`.
    pub fn close(self) -> Tab<tab_state::Closed> {
        Tab {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `surf.shell.Tab` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`TabSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: TabState,
    /// What it holds.
    pub data: TabData,
}

/// An `Tab` in whichever declared state it was found.
pub enum AnyTab {
    /// Resting in `Closed`.
    Closed(Tab<tab_state::Closed>),
    /// Resting in `Open`.
    Open(Tab<tab_state::Open>),
}

impl TabSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `TabState` cannot spell one.
    pub fn refine(self) -> AnyTab {
        match self.state {
            TabState::Closed => AnyTab::Closed(Tab {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            TabState::Open => AnyTab::Open(Tab {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyTab {
    /// The state, as the runtime value.
    pub fn state(&self) -> TabState {
        match self {
            Self::Closed(_) => TabState::Closed,
            Self::Open(_) => TabState::Open,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> TabSnapshot {
        match self {
            Self::Closed(instance) => TabSnapshot {
                state: TabState::Closed,
                data: instance.into_data(),
            },
            Self::Open(instance) => TabSnapshot {
                state: TabState::Open,
                data: instance.into_data(),
            },
        }
    }
}

/// What Window — `surf.shell.Window` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Window<S>`], and at a boundary by [`WindowSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowData {
    /// The identity: `window_id` — `surf.shell.WindowId`.
    pub window_id: WindowId,
}

/// The states of `surf.shell.Window`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](window_state::Marker), so [`Window<S>`](Window) can only ever rest in a real state.
pub mod window_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Open {}
    }

    /// A declared state of `Window`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::WindowState;
    }

    /// `Open`. Where a new instance starts.
    pub struct Open;

    impl Marker for Open {
        const STATE: super::WindowState = super::WindowState::Open;
    }
}

/// Window — `surf.shell.Window` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Open`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`WindowSnapshot`]
/// and [`WindowSnapshot::refine`].
pub struct Window<S: window_state::Marker> {
    data: WindowData,
    state: core::marker::PhantomData<S>,
}

impl<S: window_state::Marker> Window<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> WindowState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &WindowData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> WindowData {
        self.data
    }
}

impl Window<window_state::Open> {
    /// A new instance, resting in `Open` — the only state the lifecycle starts one in.
    pub fn new(data: WindowData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `surf.shell.Window` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`WindowSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: WindowState,
    /// What it holds.
    pub data: WindowData,
}

/// An `Window` in whichever declared state it was found.
pub enum AnyWindow {
    /// Resting in `Open`.
    Open(Window<window_state::Open>),
}

impl WindowSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `WindowState` cannot spell one.
    pub fn refine(self) -> AnyWindow {
        match self.state {
            WindowState::Open => AnyWindow::Open(Window {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyWindow {
    /// The state, as the runtime value.
    pub fn state(&self) -> WindowState {
        match self {
            Self::Open(_) => WindowState::Open,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> WindowSnapshot {
        match self {
            Self::Open(instance) => WindowSnapshot {
                state: WindowState::Open,
                data: instance.into_data(),
            },
        }
    }
}

/// Close a tab — the input of `surf.shell.CloseTab`.
///
/// Everything it can result in is [`CloseTabOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseTab {
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: TabId,
}

/// Everything `surf.shell.CloseTab` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseTabOutcome {
    /// `no-such-tab` — for an identity no record carries.
    NoSuchTab {
        /// Why it was refused: `surf.shell.TabNotFound`.
        error: TabNotFound,
    },
    /// `closed` — otherwise.
    ///
    /// The tab is Closed.
    Closed {
        /// The `surf.shell.TabClosed` this outcome publishes.
        tab_closed: TabClosed,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The tab was already Closed.
    WrongState {
        /// Why it was refused: `surf.shell.TabStateConflict`.
        error: TabStateConflict,
    },
}

/// Open a tab — the input of `surf.shell.OpenTab`.
///
/// Open an empty tab in a window.
///
/// Everything it can result in is [`OpenTabOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenTab {
    /// `window_id` — `surf.shell.WindowId`.
    pub window_id: WindowId,
}

/// Everything `surf.shell.OpenTab` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenTabOutcome {
    /// `no-such-window` — when no `surf.shell.Window` carries the identity `input.window_id` names.
    ///
    /// The window does not exist, so no tab was opened.
    NoSuchWindow {
        /// Why it was refused: `surf.shell.WindowNotFound`.
        error: WindowNotFound,
    },
    /// `opened` — otherwise.
    ///
    /// The tab is Open and shows nothing.
    Opened {
        /// The `surf.shell.TabOpened` this outcome publishes.
        tab_opened: TabOpened,
    },
}

/// Open a window — the input of `surf.shell.OpenWindow`.
///
/// Open a window that holds no tabs yet.
///
/// Everything it can result in is [`OpenWindowOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenWindow {
}

/// Everything `surf.shell.OpenWindow` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenWindowOutcome {
    /// `opened` — otherwise.
    ///
    /// The window exists and holds no tabs.
    Opened {
        /// The `surf.shell.WindowOpened` this outcome publishes.
        window_opened: WindowOpened,
    },
}

/// TabClosed — the event `surf.shell.TabClosed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabClosed {
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: TabId,
    /// `window_id` — `surf.shell.WindowId`.
    pub window_id: WindowId,
}

/// TabOpened — the event `surf.shell.TabOpened`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabOpened {
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: TabId,
    /// `window_id` — `surf.shell.WindowId`.
    pub window_id: WindowId,
}

/// WindowOpened — the event `surf.shell.WindowOpened`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowOpened {
    /// `window_id` — `surf.shell.WindowId`.
    pub window_id: WindowId,
}

/// The declared error `surf.shell.TabNotFound`.
///
/// No tab has this identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabNotFound {
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: TabId,
}

/// The declared error `surf.shell.TabStateConflict`.
///
/// The tab is not in a state this command acts from, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabStateConflict {
    /// `state` — `surf.shell.Tab.State`.
    pub state: TabState,
}

/// The declared error `surf.shell.WindowNotFound`.
///
/// No window has this identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowNotFound {
    /// `window_id` — `surf.shell.WindowId`.
    pub window_id: WindowId,
}

/// Tabs — one row of the view `surf.shell.Tabs`.
///
/// Projects `surf.shell.Tab` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tabs {
    /// `tab_id` — `surf.shell.TabId`.
    pub tab_id: TabId,
    /// `window_id` — `surf.shell.WindowId`.
    pub window_id: WindowId,
    /// `state` — `surf.shell.Tab.State`.
    pub state: TabState,
}

/// Windows — one row of the view `surf.shell.Windows`.
///
/// Projects `surf.shell.Window` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Windows {
    /// `window_id` — `surf.shell.WindowId`.
    pub window_id: WindowId,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
pub mod obligations {
    /// The behaviour `surf.shell.CloseTab` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait CloseTabBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.shell.CloseTab`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn close_tab(&mut self, input: super::CloseTab) -> Result<super::CloseTabOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.shell.OpenTab` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait OpenTabBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.shell.OpenTab`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn open_tab(&mut self, input: super::OpenTab) -> Result<super::OpenTabOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `surf.shell.OpenWindow` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait OpenWindowBehavior {
        /// Decides and enacts exactly one declared outcome of `surf.shell.OpenWindow`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn open_window(&mut self, input: super::OpenWindow) -> Result<super::OpenWindowOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `surf.shell.Tabs` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait TabsQuery {
        /// Serves `surf.shell.Tabs` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn tabs(&self) -> Result<Vec<super::Tabs>, crate::obligation::UnmetObligation>;
    }

    /// The query `surf.shell.Windows` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait WindowsQuery {
        /// Serves `surf.shell.Windows` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn windows(&self) -> Result<Vec<super::Windows>, crate::obligation::UnmetObligation>;
    }

}
