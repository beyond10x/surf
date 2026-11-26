//! Fourth review of `story:webidl-bindings`, on its fix round 3: the record of an uncaught
//! `DOMException` read as a prefix of at most 2,048 code units of its name and of its message
//! (`exception_text` and `prefix_units` of the prelude: a length and own index properties of a
//! String object, each an engine entry of its own), and the runner's sum of steps, which takes
//! the script's count before those entries (`Runner::error`, `thrown_steps`).
//!
//! The harness is the third review's (`adversary_5`): a document runner whose slices answer
//! nothing.

use surf_script::{Reason, Runner, ScriptEnd, ScriptErrorKind};

use super::adversary_5::{collect, evaluate, limits, ran, with};

/// What a script that threw left in the record, or the end it had.
fn record(end: ScriptEnd) -> Result<Option<String>, String> {
    match end {
        ScriptEnd::Threw(error) if error.kind == ScriptErrorKind::Error => Ok(error.message),
        other => Err(other.to_string()),
    }
}

/// Accessors of the page on every key the record's reads could reach past a String object's
/// own properties: the index keys at both ends of a read of 2,048 code units and `length` on
/// `Object.prototype`, the same index keys on `String.prototype` (an empty String object, so
/// none of them is its own), `toString`, `valueOf` and `Symbol.toPrimitive` of
/// `String.prototype`. Each adds its key to `G.ran`.
const TRAPS: &str = r#"
G.ran = "";
function trap(o, k) {
  Object.defineProperty(o, k, { get: function () { G.ran += String(k) + ","; return "Z"; }, configurable: true });
}
["0", "1", "2046", "2047", "2048", "2049", "length"].forEach(function (k) { trap(Object.prototype, k); });
["0", "1", "2046", "2047", "2048", "2049"].forEach(function (k) { trap(String.prototype, k); });
String.prototype.toString = function () { G.ran += "toString,"; return "Z"; };
String.prototype.valueOf = function () { G.ran += "valueOf,"; return "Z"; };
if (typeof Symbol === "function" && Symbol.toPrimitive) {
  String.prototype[Symbol.toPrimitive] = function () { G.ran += "toPrimitive,"; return "Z"; };
}
G.calls = 0;
G.long = { toString: function () { G.calls++; return "x".repeat(3000); } };
"#;

/// Item 2 of the fourth pass. The page's accessors on `Object.prototype` and
/// `String.prototype`, its `toString`, `valueOf` and `Symbol.toPrimitive` of Strings: none runs
/// while the record of an uncaught `DOMException` is read, whether its name and message are
/// read whole or by index; a name given as an object is converted once, by the constructor,
/// before the throw. The records are the exceptions' own texts.
#[test]
fn the_record_read_by_index_runs_no_accessor_of_the_page() {
    let ends = with(limits(16 << 20, 1 << 20), |runner| {
        ran(runner, TRAPS);
        let mut ends = vec![
            evaluate(
                runner,
                r#"throw new DOMException("m".repeat(3000), G.long);"#,
            ),
            evaluate(runner, r#"throw new DOMException("m".repeat(3000), "N");"#),
            evaluate(runner, r#"throw new DOMException("m", "N");"#),
        ];
        ends.push(evaluate(
            runner,
            r#"check(G.ran === "" && G.calls === 1, "ran: " + G.ran + "; calls: " + G.calls);"#,
        ));
        ends
    });
    let records: Vec<_> = ends.into_iter().map(record).collect();
    assert_eq!(
        records,
        vec![
            Ok(Some("x".repeat(1024))),
            Ok(Some(format!("N: {}", "m".repeat(1021)))),
            Ok(Some("N: m".to_string())),
            Err("ran".to_string()),
        ]
    );
}

/// Item 2 of the fourth pass, the cut. A name of 2,048 code units is copied whole and one of
/// 2,049 is read by index: both records are its first 1,024 characters. A character outside
/// the BMP split by the cut at 2,048 code units, in the name or in the message, leaves no
/// U+FFFD in the record; a lone surrogate of the text itself is written U+FFFD, as before.
#[test]
fn the_cut_at_2048_code_units_keeps_whole_characters() {
    let emoji = "\u{1F600}";
    let cases = [
        (r#""m", "x".repeat(2048)"#, "x".repeat(1024)),
        (r#""m", "x".repeat(2049)"#, "x".repeat(1024)),
        (r#""m", "x".repeat(2047) + "😀""#, "x".repeat(1024)),
        (
            r#""m", "a" + "😀".repeat(1100)"#,
            format!("a{}", emoji.repeat(1023)),
        ),
        (
            r#""b" + "😀".repeat(1100), "N""#,
            format!("N: b{}", emoji.repeat(1020)),
        ),
        (
            r#""m", "\uD800" + "x".repeat(3000)"#,
            format!("\u{FFFD}{}", "x".repeat(1023)),
        ),
        (
            r#""\uDC00".repeat(3000), "N""#,
            format!("N: {}", "\u{FFFD}".repeat(1021)),
        ),
    ];
    let records = with(limits(16 << 20, 1 << 20), |runner| {
        cases
            .iter()
            .map(|(arguments, _)| {
                record(evaluate(
                    runner,
                    &format!("throw new DOMException({arguments});"),
                ))
            })
            .collect::<Vec<_>>()
    });
    let wrong: Vec<String> = cases
        .iter()
        .zip(records)
        .filter(|((_, expected), got)| {
            got.as_ref().ok().cloned().flatten().as_deref() != Some(expected.as_str())
        })
        .map(|((arguments, expected), got)| {
            let got = match got {
                Ok(Some(text)) => format!(
                    "{} chars, starts {:?}, ends {:?}",
                    text.chars().count(),
                    text.chars().take(4).collect::<String>(),
                    text.chars().rev().take(4).collect::<String>()
                ),
                other => format!("{other:?}"),
            };
            format!(
                "{arguments}: expected {} chars, got {got}",
                expected.chars().count()
            )
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Item 2 of the fourth pass, the record after the memory limit. The memory budget is spent by
/// Strings the page keeps (the script ends `limit: memory`, and the collection after it frees
/// little), and then a script throws a `DOMException` made before, whose name and message are
/// read by index, each read making Strings on swell's heap: the record is whole (the read's
/// `with_cx` is a safe point), the agent is not faulted, and later scripts run.
#[test]
fn a_record_read_after_the_memory_limit_is_whole() {
    let (hoard, thrown, after, again) = with(limits(4 << 20, 1 << 20), |runner| {
        ran(
            runner,
            r#"G.e = new DOMException("m".repeat(3000), "x".repeat(3000)); G.keep = [];"#,
        );
        let hoard = evaluate(
            runner,
            r#"for (var i = 0; ; i++) { G.keep.push("k".repeat(16384) + i); }"#,
        );
        let thrown = runner.evaluate("throw G.e;");
        let after = runner.evaluate("G.keep = null;");
        collect(runner);
        let again = evaluate(runner, "throw G.e;");
        (hoard, thrown, after, again)
    });
    assert_eq!(hoard, ScriptEnd::Limit(Reason::Memory), "{hoard}");
    assert_eq!(record(thrown), Ok(Some("x".repeat(1024))));
    assert_eq!(after, ScriptEnd::Ran, "{after}");
    assert_eq!(record(again), Ok(Some("x".repeat(1024))));
}

/// Item 2 of the fourth pass, as swell `0.10.0` reads it. The slice sets the agent's interrupt
/// and then throws a `DOMException`. Since swell `0.9.1` the engine reads the flag after a host
/// function that answered by a throw too (HOST-216): the run ends in `Terminated(Interrupted)`,
/// so the script ends `limit: wall-clock` and no record of the throw is read; swell clears the
/// flag when it reads it. The next script runs and its record is whole.
#[test]
fn interrupt_set_before_a_throw_ends_at_wall_clock_and_the_next_record_is_whole() {
    let (first, flag, next) = with(limits(16 << 20, 1 << 20), |runner| {
        let first = evaluate(runner, r#"document.getElementById("interrupt");"#);
        let flag = runner.agent_and_realm().0.interrupt_handle().is_set();
        let next = evaluate(runner, r#"throw new DOMException("m", "N");"#);
        (first, flag, next)
    });
    assert_eq!(first, ScriptEnd::Limit(Reason::WallClock), "{first}");
    assert!(!flag, "the interrupt flag after the script");
    assert_eq!(record(next), Ok(Some("N: m".to_string())));
}

/// The steps the runner adds for `source`.
fn steps_of(runner: &mut Runner, source: &str) -> (u64, ScriptEnd) {
    let before = runner.steps();
    let end = evaluate(runner, source);
    (runner.steps() - before, end)
}

/// Item 3 of the fourth pass. The runner adds the script entry's steps once, whatever the
/// script's end: a loop of 20,000 rounds ended by nothing, by an uncaught `DOMException` read
/// by index (4,000 entries or more of the record's reads after the script), by an uncaught
/// `Error`, by a throw of a number and of a plain object that inherits from
/// `DOMException.prototype`, and with a read of `code` as its last expression; each adds the
/// steps of the loop and fewer than 1,000 more, never twice the loop, and never the record's.
/// A script ended by the step limit adds the limit; a parse error adds none; the same loop
/// afterwards adds what it added first (no count is left over from an earlier script).
#[test]
fn the_steps_of_a_script_are_its_entrys_whatever_its_end() {
    let mut small = limits(16 << 20, 1 << 20);
    small.engine.steps_per_entry = 1_000_000;
    let lp = "for (var i = 0; i < 20000; i++) {}";
    let (base, rows, limit, parse, again) = with(small, |runner| {
        ran(runner, r#"G.m = "m".repeat(3000); G.n = "x".repeat(3000);"#);
        let base = steps_of(runner, lp);
        let rows: Vec<(&str, u64, ScriptEnd)> = [
            "throw new DOMException(G.m, G.n);",
            r#"throw new DOMException("m", "N");"#,
            r#"throw new Error("e");"#,
            "throw 1;",
            "throw Object.create(DOMException.prototype);",
            r#"var e = new DOMException("m", "N"); e.code;"#,
            r#"var e = new DOMException("m", G.n); e.code;"#,
        ]
        .into_iter()
        .map(|tail| {
            let (steps, end) = steps_of(runner, &format!("{lp} {tail}"));
            (tail, steps, end)
        })
        .collect();
        let limit = steps_of(runner, "for (;;) {}");
        let parse = steps_of(runner, &format!("{lp} ("));
        let again = steps_of(runner, lp);
        (base, rows, limit, parse, again)
    });
    assert_eq!(base.1, ScriptEnd::Ran, "{}", base.1);
    let loop_steps = base.0;
    assert!(loop_steps >= 20_000, "the loop counted {loop_steps} steps");
    let wrong: Vec<String> = rows
        .iter()
        .filter(|(_, steps, _)| *steps < loop_steps || *steps >= loop_steps + 1_000)
        .map(|(tail, steps, end)| format!("{tail}: {steps} steps (loop {loop_steps}), {end}"))
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
    assert!(
        matches!(rows[0].2, ScriptEnd::Threw(ref error) if error.message == Some("x".repeat(1024))),
        "{}",
        rows[0].2
    );
    assert_eq!(
        limit.1,
        ScriptEnd::Limit(Reason::StepsPerScript),
        "{}",
        limit.1
    );
    assert!(
        (999_000..=1_000_000).contains(&limit.0),
        "the step limit added {} steps",
        limit.0
    );
    assert!(matches!(parse.1, ScriptEnd::Threw(_)), "{}", parse.1);
    assert_eq!(parse.0, 0, "a parse error added {} steps", parse.0);
    assert_eq!(again, base, "the loop after the others");
}
