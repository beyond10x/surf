//! The runner (`docs/design/scripts.md`, Q4 to Q7 and Q16): one swell agent and one realm,
//! created on a thread of the runner's own and dropped on it, the limits of Q4, the stack of Q5,
//! the hooks of Q6 and the ends of Q7.

use std::fmt;
use std::io;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use swell::{
    Agent, AgentState, CreateError, ErrorKind, HookCx, Host, InterruptHandle, Job, Limits,
    LoadRequest, ObjectRef, Outcome, Parsed, Realm, Root, Seeds, Source, Termination,
};

/// The agent's limits in a session (`docs/design/scripts.md`, Q4).
pub const SESSION_ENGINE_LIMITS: Limits = Limits {
    steps_per_entry: 10_000_000,
    memory_bytes: 67_108_864,
    frame_depth: 10_000,
    reentry_depth: 256,
    parser_nesting: 1_000,
    string_length: 16_777_216,
    bigint_bits: 1_048_576,
};

/// The stack margin for surf's own frames per re-entry (Q5): as much again as swell's own room
/// per re-entry, `STACK_BYTES_PER_REENTRY` (`crates/swell/src/host/limits.rs:46 @ 0.8.0`).
const STACK_MARGIN_PER_REENTRY: usize = 14_336;

/// The stack margin for the frames below the first entry (Q5): 1 MiB.
const STACK_MARGIN_FIXED: usize = 1_048_576;

/// The most characters of a message a [`ScriptError`] keeps (Q16).
pub const MESSAGE_CHARS: usize = 1_024;

/// What the test clock answers first (Q6): 2026-01-01T00:00:00Z, in nanoseconds since the Unix
/// epoch.
pub const TEST_CLOCK_START: i128 = 1_767_225_600_000_000_000;

/// How far the test clock advances on each call (Q6): 1 ms, so that a busy wait on the clock
/// ends.
pub const TEST_CLOCK_STEP: i128 = 1_000_000;

/// The limits of one runner: the agent's, fixed when it is created, and the runner's own, held
/// between two scripts (Q4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunnerLimits {
    /// The agent's seven limits.
    pub engine: Limits,
    /// Steps of all the runner's scripts together, summed from `last_entry_steps()`.
    pub steps_per_runner: u64,
    /// Scripts one runner runs.
    pub scripts_per_runner: u32,
    /// Bytes of one script's source, UTF-8.
    pub source_bytes: usize,
    /// The wall clock from the first script's start, through the agent's interrupt handle.
    pub wall_clock: Duration,
}

impl RunnerLimits {
    /// The values of a session (Q4).
    pub const SESSION: RunnerLimits = RunnerLimits {
        engine: SESSION_ENGINE_LIMITS,
        steps_per_runner: 50_000_000,
        scripts_per_runner: 1_000,
        source_bytes: 524_288,
        wall_clock: Duration::from_secs(5),
    };

    /// The margin for surf's frames beside swell's (Q5): (`reentry_depth` + 1) × 14,336 bytes
    /// plus 1 MiB, a starting value that `story:scripts-parse-run` measures.
    pub const fn stack_margin(&self) -> usize {
        (self.engine.reentry_depth as usize)
            .saturating_add(1)
            .saturating_mul(STACK_MARGIN_PER_REENTRY)
            .saturating_add(STACK_MARGIN_FIXED)
    }

    /// The stack of the runner's thread: `min_stack_bytes()` of the agent's limits plus
    /// [`RunnerLimits::stack_margin`] (Q5); 8,739,584 bytes for [`RunnerLimits::SESSION`].
    pub const fn stack_bytes(&self) -> usize {
        self.engine
            .min_stack_bytes()
            .saturating_add(self.stack_margin())
    }
}

/// Where the runner runs: a session, with the system clock and seeds from the operating
/// system, or a test, with a clock and seeds fixed (Q6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Session,
    Test,
}

impl Mode {
    /// The seeds of a new agent (Q6): in a session 48 bytes drawn through
    /// `surf_secret::fill_random`, 16 for `hash_key` and 32 for `random_seed`; in a test
    /// swell's own test values, `[7; 16]` and `[11; 32]`.
    pub fn seeds(self) -> io::Result<Seeds> {
        match self {
            Mode::Session => {
                let mut drawn = [0u8; 48];
                surf_secret::fill_random(&mut drawn)?;
                let (hash_key, random_seed) = drawn.split_at(16);
                let mut seeds = Seeds {
                    hash_key: [0; 16],
                    random_seed: [0; 32],
                };
                seeds.hash_key.copy_from_slice(hash_key);
                seeds.random_seed.copy_from_slice(random_seed);
                Ok(seeds)
            }
            Mode::Test => Ok(Seeds {
                hash_key: [7; 16],
                random_seed: [11; 32],
            }),
        }
    }
}

/// The host value of the runner's agent: the hooks of Q6.
pub struct ScriptHost {
    clock: Clock,
}

enum Clock {
    System,
    Test { next: i128 },
}

impl ScriptHost {
    /// The host of `mode`.
    pub fn new(mode: Mode) -> Self {
        let clock = match mode {
            Mode::Session => Clock::System,
            Mode::Test => Clock::Test {
                next: TEST_CLOCK_START,
            },
        };
        Self { clock }
    }

    /// What `host_system_utc_epoch_nanoseconds` answers now (Q6).
    fn now(&mut self) -> i128 {
        match &mut self.clock {
            Clock::System => match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(after) => i128::try_from(after.as_nanos()).unwrap_or(i128::MAX),
                Err(before) => {
                    i128::try_from(before.duration().as_nanos()).map_or(i128::MIN, |n| -n)
                }
            },
            Clock::Test { next } => {
                let now = *next;
                *next = next.saturating_add(TEST_CLOCK_STEP);
                now
            }
        }
    }
}

impl Host for ScriptHost {
    // No engine path makes a job or a load request before swell's slices 10 and 11
    // (`crates/swell/src/host/hooks.rs:7-9 @ 0.8.0`): promises and modules are refused at the
    // tag. A job or a request handed to the host is dropped.
    fn host_enqueue_generic_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}

    fn host_enqueue_timeout_job(_cx: &mut HookCx<'_, Self>, _job: Job, _milliseconds: f64) {}

    fn host_enqueue_finalization_registry_cleanup_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}

    fn host_load_imported_module(_cx: &mut HookCx<'_, Self>, _request: LoadRequest) {}

    /// HTML's HostSystemUTCEpochNanoseconds: the clock of the host's mode (Q6).
    fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        cx.host_mut().now()
    }

    // `host_ensure_can_compile_strings`, `system_time_zone_identifier` and
    // `get_named_time_zone_offset_nanoseconds` keep swell's defaults: `eval` and `Function`
    // are allowed (Q4), the time zone is "UTC" with the offset 0 (Q6).
}

/// The kind of what a script threw (Q7): one of swell's seven `ErrorKind` variants, or `Value`
/// for a thrown value that is no error object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScriptErrorKind {
    Error,
    EvalError,
    RangeError,
    ReferenceError,
    SyntaxError,
    TypeError,
    UriError,
    Value,
}

impl ScriptErrorKind {
    /// The word of the kind: the constructor's name, or `value`.
    pub fn word(self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::EvalError => "EvalError",
            Self::RangeError => "RangeError",
            Self::ReferenceError => "ReferenceError",
            Self::SyntaxError => "SyntaxError",
            Self::TypeError => "TypeError",
            Self::UriError => "URIError",
            Self::Value => "value",
        }
    }
}

/// Why a script was not run, or which limit ended it: the words of Q1's line `reasons:` and of
/// Q4's column `outcome when reached` (Q7's `surf.document.ScriptReason`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reason {
    Empty,
    Src,
    Type,
    Module,
    Importmap,
    Speculationrules,
    Disconnected,
    Nomodule,
    Event,
    Svg,
    Inserted,
    Eof,
    StepsPerScript,
    Memory,
    SourceBytes,
    WallClock,
    Nodes,
    Depth,
    Attributes,
    StoredBytes,
    ScriptsPerDocument,
    StepsPerDocument,
    Faulted,
}

impl Reason {
    /// The reason's word.
    pub fn word(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Src => "src",
            Self::Type => "type",
            Self::Module => "module",
            Self::Importmap => "importmap",
            Self::Speculationrules => "speculationrules",
            Self::Disconnected => "disconnected",
            Self::Nomodule => "nomodule",
            Self::Event => "event",
            Self::Svg => "svg",
            Self::Inserted => "inserted",
            Self::Eof => "eof",
            Self::StepsPerScript => "steps-per-script",
            Self::Memory => "memory",
            Self::SourceBytes => "source-bytes",
            Self::WallClock => "wall-clock",
            Self::Nodes => "nodes",
            Self::Depth => "depth",
            Self::Attributes => "attributes",
            Self::StoredBytes => "stored-bytes",
            Self::ScriptsPerDocument => "scripts-per-document",
            Self::StepsPerDocument => "steps-per-document",
            Self::Faulted => "faulted",
        }
    }
}

/// Where an error arose (Q16): line and column from 1, the column in UTF-16 code units, and
/// whether they count in `eval` or `Function` code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Site {
    pub line: u32,
    pub column: u32,
    pub dynamic: bool,
}

/// What the runner reads of a thrown value or a parse error, without running code (Q16).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ScriptError {
    pub kind: ScriptErrorKind,
    /// The own `message`, decoded from UTF-16 with unpaired surrogates replaced and cut to its
    /// first [`MESSAGE_CHARS`] characters; `None` for a value that is no error object or an
    /// error without a String `message`.
    pub message: Option<String>,
    /// `None` when swell names no site: a throw raised outside any ECMAScript frame.
    pub site: Option<Site>,
}

/// How one script ended: Q7's closed list.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScriptEnd {
    /// The script ran to its end.
    Ran,
    /// The script threw and did not catch it, or its source did not parse.
    Threw(ScriptError),
    /// swell refused a construct of a later delivery slice; the kind is `SyntaxError`.
    Refused(ScriptError),
    /// A limit ended the script.
    Limit(Reason),
    /// The engine found its own invariant broken; the agent is faulted for good.
    Faulted,
    /// The script was not run.
    NotRun(Reason),
}

impl fmt::Display for ScriptError {
    /// ` "<message>" <line>:<column>`, each part only when present, `eval <line>:<column>` for
    /// a site in `eval` or `Function` code.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(message) = &self.message {
            write!(f, " {message:?}")?;
        }
        if let Some(site) = self.site {
            let eval = if site.dynamic { "eval " } else { "" };
            write!(f, " {eval}{}:{}", site.line, site.column)?;
        }
        Ok(())
    }
}

impl fmt::Display for ScriptEnd {
    /// The end in Q7's words: `ran`, `threw <kind> "<message>" <line>:<column>`,
    /// `refused "<message>" <line>:<column>`, `limit: <word>`, `faulted`, `not run: <word>`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ran => f.write_str("ran"),
            Self::Threw(error) => write!(f, "threw {}{error}", error.kind.word()),
            Self::Refused(error) => write!(f, "refused{error}"),
            Self::Limit(reason) => write!(f, "limit: {}", reason.word()),
            Self::Faulted => f.write_str("faulted"),
            Self::NotRun(reason) => write!(f, "not run: {}", reason.word()),
        }
    }
}

/// Why no runner was made.
#[derive(Debug)]
pub enum RunnerError {
    /// The runner's thread or its clock's thread could not be started.
    Thread(io::Error),
    /// The seeds could not be drawn from the operating system's random source.
    Seeds(io::Error),
    /// swell made no agent.
    Agent(CreateError),
    /// The realm was not created: swell's answer.
    Realm(String),
}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Thread(error) => write!(f, "the runner's thread: {error}"),
            Self::Seeds(error) => write!(f, "the agent's seeds: {error}"),
            Self::Agent(error) => write!(f, "the agent: {error:?}"),
            Self::Realm(answer) => write!(f, "the realm: {answer}"),
        }
    }
}

impl std::error::Error for RunnerError {}

/// Runs `body` with a new runner on a thread of [`RunnerLimits::stack_bytes`] made for it, and
/// answers what `body` returned. The agent is created on that thread and dropped on it when
/// `body` returns (Q5). A panic of `body` is resumed on the caller's thread.
pub fn run<R, F>(limits: RunnerLimits, mode: Mode, body: F) -> Result<R, RunnerError>
where
    R: Send,
    F: FnOnce(&mut Runner) -> R + Send,
{
    let stack_bytes = limits.stack_bytes();
    thread::scope(|scope| {
        let thread = thread::Builder::new()
            .name("surf-script".to_string())
            .stack_size(stack_bytes)
            .spawn_scoped(scope, move || {
                let mut runner = Runner::new(limits, mode, stack_bytes)?;
                Ok(body(&mut runner))
            })
            .map_err(RunnerError::Thread)?;
        match thread.join() {
            Ok(answer) => answer,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    })
}

/// The clock of the wall-clock limit (Q4): a thread that sets the agent's interrupt when the
/// bound has passed since the first script's start, and ends when the runner is dropped.
struct WallClock {
    /// Hands the deadline to the thread at the first script's start; dropped, it ends the
    /// thread.
    start: Option<Sender<Instant>>,
    /// The deadline, once the first script started; `None` before, and for a bound too large
    /// for an `Instant`.
    deadline: Option<Instant>,
    started: bool,
    thread: Option<JoinHandle<()>>,
}

impl WallClock {
    fn new(interrupt: InterruptHandle) -> io::Result<Self> {
        let (start, starts) = mpsc::channel::<Instant>();
        let thread = thread::Builder::new()
            .name("surf-script-clock".to_string())
            .spawn(move || {
                let Ok(deadline) = starts.recv() else {
                    return;
                };
                loop {
                    let left = deadline.saturating_duration_since(Instant::now());
                    match starts.recv_timeout(left) {
                        Err(RecvTimeoutError::Timeout) => {
                            interrupt.interrupt();
                            return;
                        }
                        Err(RecvTimeoutError::Disconnected) => return,
                        Ok(_) => {}
                    }
                }
            })?;
        Ok(Self {
            start: Some(start),
            deadline: None,
            started: false,
            thread: Some(thread),
        })
    }

    /// Before each script: at the first, start the clock with `bound`; at a later one, set the
    /// interrupt when the deadline has passed and the thread has not set it yet, so that the
    /// script ends `limit: wall-clock` at its start.
    fn before_script(&mut self, bound: Duration, interrupt: &InterruptHandle) {
        if !self.started {
            self.started = true;
            self.deadline = Instant::now().checked_add(bound);
            if let (Some(deadline), Some(start)) = (self.deadline, &self.start) {
                let _ = start.send(deadline);
            }
            return;
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            interrupt.interrupt();
        }
    }
}

impl Drop for WallClock {
    fn drop(&mut self) {
        drop(self.start.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// One agent with one realm, on the thread [`run`] made, and the totals of its scripts.
pub struct Runner {
    // Dropped in this order: the clock's thread ends first, the agent last.
    clock: WallClock,
    realm: Realm,
    agent: Agent<ScriptHost>,
    limits: RunnerLimits,
    stack_bytes: usize,
    steps: u64,
    scripts: u32,
    collections: u32,
    memory_ends: u32,
    faulted: bool,
}

impl Runner {
    fn new(limits: RunnerLimits, mode: Mode, stack_bytes: usize) -> Result<Self, RunnerError> {
        let seeds = mode.seeds().map_err(RunnerError::Seeds)?;
        let mut agent =
            Agent::new(ScriptHost::new(mode), limits.engine, seeds).map_err(RunnerError::Agent)?;
        let realm = match agent.create_realm(None, None) {
            Ok(Outcome::Normal(realm)) => realm,
            Ok(other) => return Err(RunnerError::Realm(format!("{other:?}"))),
            Err(error) => return Err(RunnerError::Realm(format!("{error:?}"))),
        };
        let clock = WallClock::new(agent.interrupt_handle()).map_err(RunnerError::Thread)?;
        Ok(Self {
            clock,
            realm,
            agent,
            limits,
            stack_bytes,
            steps: 0,
            scripts: 0,
            collections: 0,
            memory_ends: 0,
            faulted: false,
        })
    }

    /// Parses `source` as a classic script in the runner's realm and runs it to its end.
    ///
    /// After `faulted`, each later script ends `not run: faulted`; after the first
    /// `limit: memory` the runner calls `collect()` once, and after a second each later script
    /// ends `not run: memory` (Q4). The steps the script spent are added to [`Runner::steps`]
    /// and the script to [`Runner::scripts`].
    pub fn evaluate(&mut self, source: &str) -> ScriptEnd {
        if self.faulted || matches!(self.agent.state(), AgentState::Faulted) {
            self.faulted = true;
            return ScriptEnd::NotRun(Reason::Faulted);
        }
        if self.memory_ends >= 2 {
            return ScriptEnd::NotRun(Reason::Memory);
        }
        self.clock
            .before_script(self.limits.wall_clock, &self.agent.interrupt_handle());
        self.scripts = self.scripts.saturating_add(1);
        let end = self.run_script(source);
        if let Ok(steps) = self.agent.last_entry_steps() {
            self.steps = self.steps.saturating_add(steps);
        }
        match end {
            ScriptEnd::Limit(Reason::Memory) => {
                self.memory_ends += 1;
                if self.memory_ends == 1 {
                    self.collect();
                }
            }
            ScriptEnd::Faulted => self.faulted = true,
            _ => {}
        }
        end
    }

    /// The parse and the run of one script, and its end.
    fn run_script(&mut self, source: &str) -> ScriptEnd {
        let refusals = self.agent.refusals();
        let parsed = match self
            .agent
            .parse_script(&self.realm, Source::Utf8(source), None)
        {
            Ok(parsed) => parsed,
            Err(_) => return ScriptEnd::Faulted,
        };
        match parsed {
            Parsed::Parsed(script) => match self.agent.evaluate_script(&script) {
                Ok(Outcome::Normal(_)) => ScriptEnd::Ran,
                Ok(Outcome::Throw(thrown)) => ScriptEnd::Threw(self.error(&thrown)),
                Ok(Outcome::Terminated(termination)) => terminated(termination),
                Err(_) => ScriptEnd::Faulted,
            },
            // The site is the first error's, and the error is the first of the list (Q16).
            Parsed::Errors(errors) => {
                let error = match errors.first() {
                    Some(first) => self.error(first),
                    None => ScriptError {
                        kind: ScriptErrorKind::SyntaxError,
                        message: None,
                        site: self.site(),
                    },
                };
                if self.agent.refusals() > refusals {
                    ScriptEnd::Refused(error)
                } else {
                    ScriptEnd::Threw(error)
                }
            }
            Parsed::Terminated(termination) => terminated(termination),
        }
    }

    /// What a thrown value or a parse error holds, read without running code (Q16).
    fn error(&self, value: &Root) -> ScriptError {
        let (kind, message) = match self.agent.error_summary(value) {
            Ok(Some(summary)) => (
                error_kind(summary.kind),
                summary.message.map(|units| message(&units)),
            ),
            _ => (ScriptErrorKind::Value, None),
        };
        ScriptError {
            kind,
            message,
            site: self.site(),
        }
    }

    /// Where the error the last entry answered arose.
    fn site(&self) -> Option<Site> {
        match self.agent.last_entry_error_site() {
            Ok(Some(site)) => Some(Site {
                line: site.line,
                column: site.column,
                dynamic: site.dynamic,
            }),
            _ => None,
        }
    }

    /// The one collection after the first `limit: memory` (HOST-188 to HOST-190).
    fn collect(&mut self) {
        self.collections = self.collections.saturating_add(1);
        match self.agent.collect() {
            Ok(Outcome::Terminated(Termination::EngineFault)) | Err(_) => self.faulted = true,
            Ok(_) => {}
        }
    }

    /// The steps of the runner's scripts, summed from `last_entry_steps()` after each.
    pub fn steps(&self) -> u64 {
        self.steps
    }

    /// The scripts the runner ran, whatever their end; a script not run is not counted.
    pub fn scripts(&self) -> u32 {
        self.scripts
    }

    /// The collections the runner made after `limit: memory` (Q4): at most one.
    pub fn collections(&self) -> u32 {
        self.collections
    }

    /// The stack, in bytes, of the thread the runner runs on.
    pub fn stack_bytes(&self) -> usize {
        self.stack_bytes
    }

    /// The runner's limits.
    pub fn limits(&self) -> RunnerLimits {
        self.limits
    }

    /// The agent and its realm, for a caller that defines what the realm holds.
    pub fn agent_and_realm(&mut self) -> (&mut Agent<ScriptHost>, &Realm) {
        (&mut self.agent, &self.realm)
    }
}

/// The end of an entry swell terminated (Q4).
fn terminated(termination: Termination) -> ScriptEnd {
    match termination {
        Termination::StepBudget => ScriptEnd::Limit(Reason::StepsPerScript),
        Termination::MemoryBudget => ScriptEnd::Limit(Reason::Memory),
        Termination::Interrupted => ScriptEnd::Limit(Reason::WallClock),
        Termination::EngineFault => ScriptEnd::Faulted,
    }
}

/// The kind of a swell error. `Error`, and a kind a later tag of swell adds (HOST-201 announces
/// `AggregateError` and `#[non_exhaustive]`), read as `Error` (Q7).
fn error_kind(kind: ErrorKind) -> ScriptErrorKind {
    match kind {
        ErrorKind::EvalError => ScriptErrorKind::EvalError,
        ErrorKind::RangeError => ScriptErrorKind::RangeError,
        ErrorKind::ReferenceError => ScriptErrorKind::ReferenceError,
        ErrorKind::SyntaxError => ScriptErrorKind::SyntaxError,
        ErrorKind::TypeError => ScriptErrorKind::TypeError,
        ErrorKind::UriError => ScriptErrorKind::UriError,
        _ => ScriptErrorKind::Error,
    }
}

/// A message's UTF-16 units decoded, unpaired surrogates replaced, cut to its first
/// [`MESSAGE_CHARS`] characters (Q16).
fn message(units: &[u16]) -> String {
    char::decode_utf16(units.iter().copied())
        .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
        .take(MESSAGE_CHARS)
        .collect()
}
