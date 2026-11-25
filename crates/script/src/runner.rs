//! The runner (`docs/design/scripts.md`, Q4 to Q7 and Q16): one swell agent and one realm,
//! created on a thread of the runner's own and dropped on it, the limits of Q4, the stack of Q5,
//! the hooks of Q6 and the ends of Q7.

use std::fmt;
use std::io;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use surf_html::Tree;
use surf_html::limits::Limit;
use swell::{
    Abrupt, Agent, AgentState, Completion, CreateError, ErrorKind, GlobalObjects, GlobalSetup,
    HookCx, Host, HostError, InterruptHandle, Job, Limits, LoadRequest, ObjectRef, Outcome, Parsed,
    Realm, RealmRef, Root, Seeds, Source, Termination, Value,
};

use crate::bindings::{self, Dom, DomState, Slices};

/// The host errors that are no fault of the engine (swell's `HostError` at `0.10.0` but
/// `Faulted`, seven variants): a stale or foreign handle, a value of the wrong kind, the module
/// errors, an evaluation while another runs, and `MemoryBudget`. Each is a defect of surf's own
/// calls that a page cannot cause. It is read as `HostError::Faulted` is: a parse or a run ends
/// `faulted`, a read of the site or the steps answers nothing, and a read of a thrown value
/// answers kind `value` with no message. `MemoryBudget` is among them because at `0.10.0` only
/// `queue_microtask` answers it (`crates/swell/src/host/jobs.rs:207 @ 0.10.0`), and surf calls
/// no `queue_microtask`; it leaves this macro with surf's first call of `queue_microtask`, where
/// surf decides its reading at the call. Every variant is named, so a variant a later tag adds
/// stops the build where surf decides its word.
macro_rules! surf_defect {
    () => {
        HostError::StaleHandle
            | HostError::ForeignHandle
            | HostError::WrongKind { .. }
            | HostError::ModuleStatus { .. }
            | HostError::ModuleTerminated
            | HostError::EvaluationInProgress
            | HostError::MemoryBudget
    };
}

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
/// per re-entry, `STACK_BYTES_PER_REENTRY` (`crates/swell/src/host/limits.rs:46 @ 0.10.0`).
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
    /// Steps of all the runner's scripts and their checkpoints together, summed from
    /// `last_entry_steps()`. Held and counted here ([`Runner::steps`]); the caller enforces it
    /// (`story:scripts-parse-run`).
    pub steps_per_runner: u64,
    /// Scripts one runner runs. Held and counted here ([`Runner::scripts`]); the caller
    /// enforces it (`story:scripts-parse-run`).
    pub scripts_per_runner: u32,
    /// Bytes of one script's source, UTF-8. Held here; the caller measures and enforces it
    /// (`story:scripts-parse-run`).
    pub source_bytes: usize,
    /// The wall clock from the first script's start, through the agent's interrupt handle; a
    /// script started after it ran out is `not run: wall-clock`. The first script's start is
    /// the clock's start, so a bound of 0 ends the first script `limit: wall-clock`: a clock
    /// that ran out when a script ends is that script's end ([`Runner::evaluate`]).
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
    /// The DOM bindings of a document's runner ([`run_document`]); `None` for a runner
    /// without a document.
    pub(crate) dom: Option<Box<Dom>>,
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
        Self { clock, dom: None }
    }

    /// The document of a runner with the DOM bindings, `None` for one without.
    pub fn document(&self) -> Option<&DomState> {
        self.dom.as_deref().map(|dom| &dom.state)
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
    // Generic, timeout and cleanup jobs are pending at swell's tag: HOST-85 and HOST-86
    // `pending slice 14`, HOST-87 `pending slice 16`
    // (`conformance/host/requirements.txt:85-87 @ 0.10.0`), and modules are refused. A job or a
    // load request handed to the host is dropped. Promise jobs are not handed here: swell
    // queues them on the agent, and the runner runs them in the checkpoint after each script
    // (`Runner::evaluate`).
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

/// The kind of what a script threw (Q7): one of swell's eight `ErrorKind` variants at `0.10.0`,
/// `AggregateError` among them, or `Value` for a thrown value that is no error object. A kind
/// swell adds later is read as `Error` until surf names it ([`error_kind`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScriptErrorKind {
    Error,
    EvalError,
    RangeError,
    ReferenceError,
    SyntaxError,
    TypeError,
    UriError,
    AggregateError,
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
            Self::AggregateError => "AggregateError",
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
    /// first [`MESSAGE_CHARS`] characters; for a `DOMException`, `<name>: <message>` from its
    /// data, cut the same; `None` for a value that is no error object or an error without a
    /// String `message`, and for a `DOMException` whose read the memory budget ended. A read
    /// that an interrupt ends makes no record: the script ends `limit: wall-clock`
    /// ([`Runner::evaluate`]).
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
    run_on(limits, mode, None::<fn() -> Document>, body)
}

/// The document of a runner with the DOM bindings: its tree, the limits of the parser it was
/// parsed under, which every write of a script is held to, and the slices.
pub struct Document {
    pub tree: Tree,
    pub limits: surf_html::limits::Limits,
    pub slices: Slices,
}

/// Runs `body` as [`run`] does, with a runner whose realm has the DOM bindings of the document
/// `document` makes on the runner's thread: the global object is a `Window`, and the realm's
/// setup defines the interface objects and the global's members in a `with_cx` right after
/// `create_realm`, outside the runner's totals and before the first script.
pub fn run_document<R, D, F>(
    limits: RunnerLimits,
    mode: Mode,
    document: D,
    body: F,
) -> Result<R, RunnerError>
where
    R: Send,
    D: FnOnce() -> Document + Send,
    F: FnOnce(&mut Runner) -> R + Send,
{
    run_on(limits, mode, Some(document), body)
}

fn run_on<R, D, F>(
    limits: RunnerLimits,
    mode: Mode,
    document: Option<D>,
    body: F,
) -> Result<R, RunnerError>
where
    R: Send,
    D: FnOnce() -> Document + Send,
    F: FnOnce(&mut Runner) -> R + Send,
{
    let stack_bytes = limits.stack_bytes();
    thread::scope(|scope| {
        let thread = thread::Builder::new()
            .name("surf-script".to_string())
            .stack_size(stack_bytes)
            .spawn_scoped(scope, move || {
                let document = document.map(|make| make());
                let mut runner = Runner::new(limits, mode, stack_bytes, document)?;
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

    /// Whether the clock started and its deadline has passed: a script now is not run.
    fn ran_out(&self) -> bool {
        self.started
            && self
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
    }

    /// Before each script that runs: at the first, start the clock with `bound`.
    fn before_script(&mut self, bound: Duration) {
        if self.started {
            return;
        }
        self.started = true;
        self.deadline = Instant::now().checked_add(bound);
        if let (Some(deadline), Some(start)) = (self.deadline, &self.start) {
            let _ = start.send(deadline);
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

/// The answer of a read of a thrown value that the run's end ended: an interrupt took it.
struct RunEnded;

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
    /// The steps of the script's entry, read in [`Runner::error`] before the read of a
    /// `DOMException`'s data, whose engine calls begin entries of their own.
    thrown_steps: Option<u64>,
}

impl Runner {
    fn new(
        limits: RunnerLimits,
        mode: Mode,
        stack_bytes: usize,
        document: Option<Document>,
    ) -> Result<Self, RunnerError> {
        let seeds = mode.seeds().map_err(RunnerError::Seeds)?;
        let mut agent =
            Agent::new(ScriptHost::new(mode), limits.engine, seeds).map_err(RunnerError::Agent)?;
        let mut window = |cx: &mut HookCx<'_, ScriptHost>, realm: RealmRef| {
            let global = cx
                .new_host_object(bindings::window_spec(realm))
                .map_err(|_| Abrupt::Terminated(Termination::EngineFault))??;
            Completion::Ok(GlobalObjects { global, this: None })
        };
        let setup: Option<GlobalSetup<'_, ScriptHost>> = document
            .as_ref()
            .map(|_| &mut window as GlobalSetup<'_, ScriptHost>);
        let realm = match agent.create_realm(setup, None) {
            Ok(Outcome::Normal(realm)) => realm,
            Ok(other @ (Outcome::Throw(_) | Outcome::Terminated(_))) => {
                return Err(RunnerError::Realm(format!("{other:?}")));
            }
            Err(error @ (HostError::Faulted | surf_defect!())) => {
                return Err(RunnerError::Realm(format!("{error:?}")));
            }
        };
        if let Some(document) = document {
            install(&mut agent, &realm, document)?;
        }
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
            thrown_steps: None,
        })
    }

    /// Parses `source` as a classic script in the runner's realm, runs it to its end and then
    /// performs HTML's microtask checkpoint ("clean up after running script").
    ///
    /// An interrupt that is set when a script ends is the script's end. When the run answered,
    /// whatever it answered, and before a thrown value is read and before the checkpoint, the
    /// runner asks whether the wall clock ran out or the interrupt is set
    /// ([`Runner::run_ended`]); if so, the script ends `limit: wall-clock`, or with the limit the
    /// bindings recorded ([`Runner::document_end`]), no text of a thrown value is read, no
    /// checkpoint runs and the interrupt is withdrawn. An interrupt that ends the read of a
    /// thrown value all the same (set after the question, taken by the read's first engine
    /// call) ends the script the same way, not with a record without a message. After a
    /// checkpoint that emptied the queue the runner asks again: a script whose run and jobs
    /// ended well while the clock ran out or the interrupt was set before the runner returned
    /// ends `limit: wall-clock` too.
    ///
    /// Two of these checks are a second line: at swell `0.10.0` they never decide an end
    /// alone. The question before the read of a thrown value ([`Runner::thrown`]): without it
    /// the read's first engine call or the question before the checkpoint gives the same end;
    /// it keeps the rule that no text is read after an interrupt. The flag half of the question
    /// before the checkpoint: the checkpoint's start takes a set flag and ends at the wall
    /// clock the same way. Both decide if a later tag changes where the engine reads the flag.
    ///
    /// The checkpoint follows a script that ended `ran`, `threw` or `refused`; after a limit or
    /// `faulted` there is none. It is one entry of swell's with a step budget of its own,
    /// `steps_per_entry` (HOST-193), and its end is the script's end: a limit or a fault of the
    /// checkpoint replaces the script's own end by the rule of [`Runner::document_end`], and a
    /// checkpoint that emptied the queue leaves it. After any end `limit: <word>`, of the script
    /// or of its checkpoint, the microtask queue is cleared, so that jobs a script queued before
    /// its limit do not spend the next script's budget. A job's throw and a rejection no
    /// handler took are not recorded: the script ends as it ended.
    ///
    /// After `faulted`, each later script ends `not run: faulted`; after the first
    /// `limit: memory` the runner clears the queue and then calls `collect()` once, and after a
    /// second each later script ends `not run: memory`; a script started after the wall clock
    /// ran out ends `not run: wall-clock` (Q4). A script not run is not counted. The steps a
    /// script that ran spent and the steps of its checkpoint are added to [`Runner::steps`] and
    /// the script to [`Runner::scripts`]; the checkpoint is no script of its own.
    ///
    /// No script inherits an interrupt: after every end, whatever its word (a limit, `faulted`,
    /// `not run`, a parse error), the runner withdraws a set interrupt here, in this one place,
    /// before it returns; [`Runner::script_end`] has every way out of a script, and none of them
    /// returns around the withdrawal. The end word stays what swell or the bindings answered. A
    /// clock that ran out is known by the clock itself, not by the flag, so the next script is
    /// `not run: wall-clock` all the same.
    pub fn evaluate(&mut self, source: &str) -> ScriptEnd {
        let end = self.script_end(source);
        if let ScriptEnd::Limit(_) = end {
            self.clear_microtasks();
        }
        self.agent.interrupt_handle().clear();
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

    /// The end of one script as [`Runner::evaluate`] reads it, before the clear of the queue,
    /// the withdrawal of the interrupt and the collection.
    fn script_end(&mut self, source: &str) -> ScriptEnd {
        let faulted = match self.agent.state() {
            AgentState::Faulted => true,
            AgentState::Alive => false,
        };
        if self.faulted || faulted {
            self.faulted = true;
            return ScriptEnd::NotRun(Reason::Faulted);
        }
        if self.memory_ends >= 2 {
            return ScriptEnd::NotRun(Reason::Memory);
        }
        if self.clock.ran_out() {
            return ScriptEnd::NotRun(Reason::WallClock);
        }
        self.clock.before_script(self.limits.wall_clock);
        self.scripts = self.scripts.saturating_add(1);
        let end = self.run_script(source);
        // The clock half decides here; the flag half is a second line at swell 0.10.0, where
        // the checkpoint's start takes a set flag and ends at the wall clock the same way.
        let end = self.at_run_end(end);
        let end = self.document_end(end);
        // The script's steps are read before the checkpoint, which begins an entry of its own
        // and so changes what `last_entry_steps()` and `last_entry_error_site()` answer.
        let steps = match self.thrown_steps.take() {
            Some(steps) => Ok(steps),
            None => self.agent.last_entry_steps(),
        };
        self.add_steps(steps);
        match end {
            ScriptEnd::Ran | ScriptEnd::Threw(_) | ScriptEnd::Refused(_) => self.checkpoint(end),
            ScriptEnd::Limit(_) | ScriptEnd::Faulted | ScriptEnd::NotRun(_) => end,
        }
    }

    /// Decision 10 of `story:webidl-bindings`, as ruled in its fix round 1: when the bindings
    /// recorded a limit during the entry (`bindings::end_with_limit`), that limit is the
    /// script's end, in the words of Q4's column `outcome when reached`, whatever swell
    /// answered: `Terminated(Interrupted)` (the answer `end_with_limit` makes), a throw, a
    /// normal end, or `StepBudget`. Two ends keep their own word: `limit: memory`, whose
    /// collection and count the runner needs (Q4's row "after `Terminated(MemoryBudget)`"), and
    /// `faulted`, after which the agent answers nothing. The record is cleared after every
    /// script that recorded one, and the interrupt is withdrawn after every script
    /// ([`Runner::evaluate`]), so the next script inherits neither.
    fn document_end(&mut self, end: ScriptEnd) -> ScriptEnd {
        let Some(dom) = self.agent.host_mut().dom.as_deref_mut() else {
            return end;
        };
        let Some(limit) = dom.state.take_limit() else {
            return end;
        };
        match end {
            ScriptEnd::Limit(Reason::Memory) | ScriptEnd::Faulted => return end,
            ScriptEnd::Ran
            | ScriptEnd::Threw(_)
            | ScriptEnd::Refused(_)
            | ScriptEnd::Limit(_)
            | ScriptEnd::NotRun(_) => {}
        }
        match limit {
            Limit::Nodes => ScriptEnd::Limit(Reason::Nodes),
            Limit::Depth => ScriptEnd::Limit(Reason::Depth),
            Limit::Attributes => ScriptEnd::Limit(Reason::Attributes),
            Limit::StoredBytes => ScriptEnd::Limit(Reason::StoredBytes),
            // The limits of the parser's input and of its steps: no member of
            // `story:webidl-bindings` records these two, and Q4 gives them no word for a
            // script, so the end is the interrupt's. The first story with a member that writes
            // into the tree gives each a word of Q4 or shows that no write reaches it.
            Limit::InputBytes | Limit::Steps => ScriptEnd::Limit(Reason::WallClock),
        }
    }

    /// Whether the run's end has come: the wall clock ran out, or the interrupt is set and not
    /// yet used. `is_set` reads the flag without taking it
    /// (`crates/swell/src/host/interrupt.rs:28-31 @ 0.10.0`); what takes it is the start of an
    /// outermost entry, a charge of steps and the interpreter's own checks, each of which clears
    /// it and ends the run in `Terminated(Interrupted)` (`host/agent.rs:965-993 @ 0.10.0`,
    /// `interp/budget.rs:34-41 @ 0.10.0`).
    fn run_ended(&self) -> bool {
        self.clock.ran_out() || self.agent.interrupt_handle().is_set()
    }

    /// The end of a script whose run's end came: `limit: wall-clock`. The interrupt is
    /// withdrawn by [`Runner::evaluate`] before it returns, as after every script.
    fn run_end(&self) -> ScriptEnd {
        ScriptEnd::Limit(Reason::WallClock)
    }

    /// `end`, or the run's end when it came ([`Runner::run_ended`]) and `end` is `ran`, `threw`
    /// or `refused`: a limit, a fault and a script not run keep their word.
    fn at_run_end(&mut self, end: ScriptEnd) -> ScriptEnd {
        match end {
            ScriptEnd::Ran | ScriptEnd::Threw(_) | ScriptEnd::Refused(_) if self.run_ended() => {
                self.run_end()
            }
            ScriptEnd::Ran
            | ScriptEnd::Threw(_)
            | ScriptEnd::Refused(_)
            | ScriptEnd::Limit(_)
            | ScriptEnd::Faulted
            | ScriptEnd::NotRun(_) => end,
        }
    }

    /// Adds the steps of an entry to [`Runner::steps`].
    fn add_steps(&mut self, steps: Result<u64, HostError>) {
        match steps {
            Ok(steps) => self.steps = self.steps.saturating_add(steps),
            // A faulted agent answers no count; the script's end says `faulted`.
            Err(HostError::Faulted | surf_defect!()) => {}
        }
    }

    /// HTML's microtask checkpoint after a script that ended `end` (`ran`, `threw` or
    /// `refused`), and the script's end after it. Its steps, which `last_entry_steps()` reads
    /// for the whole checkpoint also after a termination (HOST-197), are added to the total.
    /// `Normal` leaves `end`, unless the wall clock ran out or the interrupt is set when it
    /// answered, and the end is then the run's ([`Runner::at_run_end`]). The checkpoint reads
    /// the flag at its start and before it takes each job off (`host/agent.rs:747`, `:760`,
    /// `:965-973`, `host/jobs.rs:295-298 @ 0.10.0`); after a job's function answered it reads
    /// it when that function is a script function, and not when it is a host function
    /// (`ops/promise.rs:436-500`, `host/jobs.rs:387-411 @ 0.10.0`). So a last job whose
    /// function is a host function, or the wall clock's thread after the last read, leaves the
    /// flag set when the checkpoint answers `Normal`. A termination is read as a script's is ([`terminated`]); the
    /// answer is then passed through [`Runner::document_end`], so a DOM limit a job recorded
    /// ends the script with that limit's word. The checkpoint never answers `Throw` (a throw
    /// there faults the agent, `crates/swell/src/host/agent.rs:765-769 @ 0.10.0`), and on a
    /// faulted agent it answers `Err(HostError::Faulted)` (`agent.rs:752 @ 0.10.0`): both are
    /// read as `faulted`.
    fn checkpoint(&mut self, end: ScriptEnd) -> ScriptEnd {
        let end = match self.agent.perform_microtask_checkpoint() {
            Ok(Outcome::Normal(_)) => self.at_run_end(end),
            Ok(Outcome::Terminated(termination)) => terminated(termination),
            Ok(Outcome::Throw(_)) => ScriptEnd::Faulted,
            Err(HostError::Faulted | surf_defect!()) => ScriptEnd::Faulted,
        };
        let steps = self.agent.last_entry_steps();
        self.add_steps(steps);
        self.document_end(end)
    }

    /// Drops every job left in the microtask queue unrun, after a script or its checkpoint
    /// ended at a limit (HOST-200). HTML and swell's note keep the queue after a script
    /// stopped; surf clears it, so that a page that floods the queue cannot spend the next
    /// scripts' budgets.
    fn clear_microtasks(&mut self) {
        match self.agent.clear_microtasks() {
            Ok(_) => {}
            // A faulted agent runs no job again.
            Err(HostError::Faulted | surf_defect!()) => {}
        }
    }

    /// The parse and the run of one script, and its end.
    fn run_script(&mut self, source: &str) -> ScriptEnd {
        let refusals = self.agent.refusals();
        let parsed = match self
            .agent
            .parse_script(&self.realm, Source::Utf8(source), None)
        {
            Ok(parsed) => parsed,
            Err(HostError::Faulted | surf_defect!()) => return ScriptEnd::Faulted,
        };
        match parsed {
            Parsed::Parsed(script) => match self.agent.evaluate_script(&script) {
                Ok(Outcome::Normal(_)) => ScriptEnd::Ran,
                Ok(Outcome::Throw(thrown)) => self.thrown(&thrown, ScriptEnd::Threw),
                Ok(Outcome::Terminated(termination)) => terminated(termination),
                Err(HostError::Faulted | surf_defect!()) => ScriptEnd::Faulted,
            },
            // The site is the first error's, and the error is the first of the list (Q16).
            Parsed::Errors(errors) => {
                let end = if self.agent.refusals() > refusals {
                    ScriptEnd::Refused
                } else {
                    ScriptEnd::Threw
                };
                match errors.first() {
                    Some(first) => self.thrown(first, end),
                    None => end(ScriptError {
                        kind: ScriptErrorKind::SyntaxError,
                        message: None,
                        site: self.site(),
                    }),
                }
            }
            Parsed::Terminated(termination) => terminated(termination),
        }
    }

    /// The end of a script whose run answered the thrown value or parse error `value`: `end`
    /// with its record, or the run's end ([`Runner::run_end`]) when it came before the read or
    /// ended the read, in which case no record is made.
    fn thrown(&mut self, value: &Root, end: fn(ScriptError) -> ScriptEnd) -> ScriptEnd {
        // A second line at swell 0.10.0: without it the read's first engine call or the
        // question before the checkpoint gives the same end. It keeps the rule that no text is
        // read after an interrupt, and decides if a later tag reads the flag elsewhere.
        if self.run_ended() {
            return self.run_end();
        }
        match self.error(value) {
            Ok(error) => end(error),
            Err(RunEnded) => self.run_end(),
        }
    }

    /// What a thrown value or a parse error holds, read without running code (Q16). In a
    /// document's runner a `DOMException`'s message is `<name>: <message>` from the exception's
    /// own data ([`Runner::exception_text`]); `Err(RunEnded)` when an interrupt ended that read.
    fn error(&mut self, value: &Root) -> Result<ScriptError, RunEnded> {
        let (kind, message) = match self.agent.error_summary(value) {
            Ok(Some(summary)) => (
                error_kind(summary.kind),
                summary.message.map(|units| message(&units)),
            ),
            Ok(None) => (ScriptErrorKind::Value, None),
            // Not reachable from a live agent: the value comes from the entry that just answered
            // it, of this agent and not released, and a fault ends that entry `Terminated`, not
            // `Throw`. Read as a thrown value with nothing known of it.
            Err(HostError::Faulted | surf_defect!()) => (ScriptErrorKind::Value, None),
        };
        // The site first: the read of an exception's data below is an entry of its own, after
        // which `last_entry_error_site` answers nothing.
        let site = self.site();
        let message = match kind {
            ScriptErrorKind::Error => {
                // The steps first too: that read's engine calls begin entries of their own.
                if let Ok(steps) = self.agent.last_entry_steps() {
                    self.thrown_steps = Some(steps);
                }
                self.exception_text(value)?.or(message)
            }
            ScriptErrorKind::EvalError
            | ScriptErrorKind::RangeError
            | ScriptErrorKind::ReferenceError
            | ScriptErrorKind::SyntaxError
            | ScriptErrorKind::TypeError
            | ScriptErrorKind::UriError
            | ScriptErrorKind::AggregateError
            | ScriptErrorKind::Value => message,
        };
        Ok(ScriptError {
            kind,
            message,
            site,
        })
    }

    /// The `<name>: <message>` of a `DOMException` a script threw and did not catch, cut to
    /// [`MESSAGE_CHARS`] characters (Q16, as ruled in fix round 1 of `story:webidl-bindings`; no
    /// longer prefix is copied), read from its data in a `with_cx` that runs no code
    /// (`bindings::exception_text`). `Ok(None)` when the value has no text: a runner without a
    /// document, any other value, and a read the memory budget ended (the record is then an
    /// `Error` without a message). `Err(RunEnded)` when the read was ended by the run's end:
    /// an interrupt set after [`Runner::run_ended`] answered, which the read's first engine
    /// call takes. `bindings::exception_text` answers `None` for both; a termination of a call
    /// is sticky until the `with_cx` returns (`crates/swell/src/host/mod.rs:171-172 @ 0.10.0`),
    /// so one more engine call, a ToNumber of `undefined`, answers it.
    fn exception_text(&mut self, value: &Root) -> Result<Option<String>, RunEnded> {
        if self.agent.host().dom.is_none() {
            return Ok(None);
        }
        let read = self.agent.with_cx(&self.realm, |cx| {
            let Ok(value) = cx.get(value) else {
                return Ok(None);
            };
            if let Some(text) = bindings::exception_text(cx, value, MESSAGE_CHARS) {
                return Ok(Some(text));
            }
            match cx.to_number(Value::Undefined) {
                Ok(Err(Abrupt::Terminated(Termination::Interrupted))) => Err(RunEnded),
                Ok(Ok(_) | Err(Abrupt::Throw(_) | Abrupt::Terminated(_)))
                | Err(HostError::Faulted | surf_defect!()) => Ok(None),
            }
        });
        match read {
            Ok(text) => Ok(text?.map(|text| text.chars().take(MESSAGE_CHARS).collect())),
            Err(HostError::Faulted | surf_defect!()) => Ok(None),
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
            Ok(None) => None,
            Err(HostError::Faulted | surf_defect!()) => None,
        }
    }

    /// The one collection after the first `limit: memory` (HOST-188 to HOST-190).
    fn collect(&mut self) {
        self.collections = self.collections.saturating_add(1);
        match self.agent.collect() {
            Ok(Outcome::Normal(_) | Outcome::Throw(_)) => {}
            Ok(Outcome::Terminated(
                Termination::StepBudget | Termination::MemoryBudget | Termination::Interrupted,
            )) => {}
            Ok(Outcome::Terminated(Termination::EngineFault)) => self.faulted = true,
            Err(HostError::Faulted | surf_defect!()) => self.faulted = true,
        }
    }

    /// The steps of the runner's scripts and their checkpoints, summed from
    /// `last_entry_steps()` after each.
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

    /// The agent and its realm, for the bindings that install their functions in the realm.
    /// An entry run through it is outside the runner's totals ([`Runner::steps`],
    /// [`Runner::scripts`]) and outside the wall clock; only the bindings' setup before the
    /// first script may use it.
    pub fn agent_and_realm(&mut self) -> (&mut Agent<ScriptHost>, &Realm) {
        (&mut self.agent, &self.realm)
    }
}

/// The realm's setup of a document's runner: the interface objects and the global's members,
/// defined in one `with_cx` right after `create_realm` (`bindings::install`), the bindings then
/// held by the host.
fn install(
    agent: &mut Agent<ScriptHost>,
    realm: &Realm,
    document: Document,
) -> Result<(), RunnerError> {
    let failed = |what: String| RunnerError::Realm(format!("the DOM bindings: {what}"));
    let interrupt = agent.interrupt_handle();
    let global = agent
        .global_object(realm)
        .map_err(|error| failed(format!("{error:?}")))?;
    let dom = Dom::new(
        DomState::new(document.tree, document.limits, interrupt),
        document.slices,
    );
    let answer = agent.with_cx(realm, |cx| {
        let realm_ref = cx.realm_ref(realm).map_err(|error| format!("{error:?}"))?;
        let global = match cx.get(&global) {
            Ok(Value::Object(global)) => global,
            other => return Err(format!("the global object: {other:?}")),
        };
        bindings::install(cx, realm_ref, global, dom).map_err(|abrupt| format!("{abrupt:?}"))
    });
    match answer {
        Ok(Ok(())) => Ok(()),
        Ok(Err(what)) => Err(failed(what)),
        Err(error) => Err(failed(format!("{error:?}"))),
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

/// The kind of a swell error (Q7): one arm per kind swell names at `0.10.0`, and one wildcard
/// arm, because `ErrorKind` is `#[non_exhaustive]` and a `match` outside swell needs one. A
/// kind a later tag adds does not stop the build: it reads `Error` until surf names it.
fn error_kind(kind: ErrorKind) -> ScriptErrorKind {
    match kind {
        ErrorKind::Error => ScriptErrorKind::Error,
        ErrorKind::EvalError => ScriptErrorKind::EvalError,
        ErrorKind::RangeError => ScriptErrorKind::RangeError,
        ErrorKind::ReferenceError => ScriptErrorKind::ReferenceError,
        ErrorKind::SyntaxError => ScriptErrorKind::SyntaxError,
        ErrorKind::TypeError => ScriptErrorKind::TypeError,
        ErrorKind::UriError => ScriptErrorKind::UriError,
        ErrorKind::AggregateError => ScriptErrorKind::AggregateError,
        // Swallows every kind swell adds after `AggregateError`; there is none at `0.10.0`
        // (`crates/swell/src/host/mod.rs:39-55 @ 0.10.0`). The test
        // `engine::every_error_constructor_keeps_its_word` notices such a kind: it throws
        // `new <name>([])` for every global constructor whose name ends in `Error` and fails
        // when one reads another word. What it cannot notice: a kind with no global
        // constructor whose name ends in `Error` reaches this arm unnoticed.
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
