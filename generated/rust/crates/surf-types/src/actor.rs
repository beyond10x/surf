// generated from surf v1
// contract digest 3228537f9d6e7cdd446cb9ccbf75b058addda67c10b212a848f9115f7eba7e7b
// do not edit: regenerate with `ess synthesize`

//! Every actor the specification declares, and the commands each may invoke — as data.
//!
//! A grant is checked against a caller identity, which these types do not read from anywhere:
//! whatever authenticates a request builds a [`Caller`], and a served surface checks it against
//! [`may`] before the command runs. The `PLAN.md` beside this workspace says, per actor,
//! whether a generated surface enforces the grant or the caller does.

/// An actor the specification declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Actor {
    /// `surf.agent.Opener`.
    AgentOpener,
    /// `surf.agent.SessionHolder`.
    AgentSessionHolder,
    /// `surf.navigation.User`.
    NavigationUser,
    /// `surf.shell.User`.
    ShellUser,
}

impl Actor {
    /// Every declared actor, ordered by qualified name.
    pub const ALL: &'static [Actor] = &[
        Actor::AgentOpener,
        Actor::AgentSessionHolder,
        Actor::NavigationUser,
        Actor::ShellUser,
    ];

    /// The actor's qualified name, as the specification spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Actor::AgentOpener => "surf.agent.Opener",
            Actor::AgentSessionHolder => "surf.agent.SessionHolder",
            Actor::NavigationUser => "surf.navigation.User",
            Actor::ShellUser => "surf.shell.User",
        }
    }
}

/// The qualified names of the commands `actor` may invoke, ordered by name; empty for an
/// actor that only observes.
pub fn may(actor: Actor) -> &'static [&'static str] {
    match actor {
        Actor::AgentOpener => &[
            "surf.agent.OpenSession",
        ],
        Actor::AgentSessionHolder => &[
            "surf.agent.CloseSession",
            "surf.agent.Fill",
            "surf.agent.Follow",
            "surf.agent.Go",
            "surf.agent.GoBack",
            "surf.agent.GoForward",
            "surf.agent.Pick",
            "surf.agent.Press",
        ],
        Actor::NavigationUser => &[
            "surf.navigation.CancelNavigation",
            "surf.navigation.Navigate",
        ],
        Actor::ShellUser => &[
            "surf.shell.CloseTab",
            "surf.shell.OpenTab",
            "surf.shell.OpenWindow",
        ],
    }
}

/// Who a request was authenticated as.
///
/// Built by whatever authenticates the request — a session, a token, a certificate — and
/// handed to the served surface's `dispatch` and `handle`, which check its grant
/// before the command runs. Never derived from the request itself: a client can write
/// anything into a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Caller {
    /// The declared actor.
    pub actor: Actor,
}

impl Caller {
    /// `true` when this caller may invoke `command`, named by its qualified name.
    pub fn may(&self, command: &str) -> bool {
        may(self.actor).contains(&command)
    }
}

/// The qualified names of the views `actor` may read, ordered by name: the views its grant
/// names. A view no actor's grant names is open to every caller and listed for none.
pub fn may_read(actor: Actor) -> &'static [&'static str] {
    match actor {
        Actor::AgentOpener => &[
            "surf.agent.Sessions",
        ],
        Actor::AgentSessionHolder => &[
            "surf.agent.SessionLog",
            "surf.agent.SessionState",
        ],
        Actor::NavigationUser => &[],
        Actor::ShellUser => &[],
    }
}

impl Caller {
    /// `true` when this caller may read `view`, a view some actor's grant names, named by
    /// its qualified name.
    pub fn may_read(&self, view: &str) -> bool {
        may_read(self.actor).contains(&view)
    }
}
