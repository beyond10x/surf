// generated from surf v1
// contract digest 19964b50f9197185811d923fbd6c3eec7ec2855a2cc03c2c60d6e4a2149578f0
// do not edit: regenerate with `ess synthesize`

//! Agent — `surf.agent`.
//!
//! A browser session an AI agent drives from a command line: opened once, acted on with a token, read back as text, closed. This slice holds only the types the command line `cli/surf-agent.yaml` names; the session itself, its commands and its views come with the session model. The `net` component holds this domain until the `agent` component has commands (`story:agent-session-spec`).
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// GoCall — `surf.agent.GoCall`.
///
/// The address to load into a session, and the token that proves the caller holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoCall {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `token` — `surf.agent.SessionToken`.
    pub token: SessionToken,
    /// `url` — `surf.url.Url`.
    pub url: crate::url::Url,
}

/// OpenCall — `surf.agent.OpenCall`.
///
/// The input of `open`, the address to load first if one is given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCall {
    /// `url` — `Optional<surf.url.Url>`.
    pub url: Option<crate::url::Url>,
}

/// OpenedSession — `surf.agent.OpenedSession`.
///
/// The answer of `open`; the only answer that carries the token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedSession {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `token` — `surf.agent.SessionToken`.
    pub token: SessionToken,
}

/// SessionCall — `surf.agent.SessionCall`.
///
/// Which session a call acts on, and the token that proves the caller holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCall {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `token` — `surf.agent.SessionToken`.
    pub token: SessionToken,
}

/// SessionId — `surf.agent.SessionId`: a distinct wrapper around `Uuid`.
///
/// The identity of a session, drawn by `open`; not secret, it appears in listings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(pub crate::primitives::Uuid);

/// SessionReadout — `surf.agent.SessionReadout`.
///
/// The state of a session, as every call but `open` answers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionReadout {
    /// `session_id` — `String`.
    pub session_id: String,
    /// `state` — `String`.
    pub state: String,
    /// `url` — `Optional<surf.url.Url>`.
    pub url: Option<crate::url::Url>,
    /// `navigation_state` — `Optional<surf.navigation.Navigation.State>`.
    pub navigation_state: Option<crate::navigation::NavigationState>,
}

/// SessionToken — `surf.agent.SessionToken`: a distinct wrapper around `String`.
///
/// The secret an agent presents to act on its session, 64 lower-case hexadecimal digits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionToken(pub String);
