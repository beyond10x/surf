//! What a step of the slice `attributes` buys (`story:scripts-dom-attributes`, acceptance 11):
//! for each member that reads or writes more than its own node, the worst shape the parser
//! admits for it (4,096 names of 1,000 bytes that differ in their last four, 16,384 short
//! names, values at and over `string_length`, 1 MiB names), one call on its own runner, its
//! steps, its time and the time a step against the plain walk's (`getElementById('none')` over
//! 20,000 elements without attributes). The lines are printed; the tests assert a floor of
//! steps, the sum of the charges of decision 7 for that shape, never a time. One run a shape,
//! no loop to steady a number. And the source test of acceptance 10: the members touch the
//! tree and the charges only through the files that own them.

use std::time::{Duration, Instant};

use surf_html::limits::Limits as ParserLimits;
use surf_html::{NodeData, ParseOptions, Tree, parse_document};
use surf_script::dom::{self, DocumentInfo};
use surf_script::{Document, Mode, Runner, RunnerLimits, ScriptEnd, run_document};
use swell::Limits;

fn limits() -> RunnerLimits {
    RunnerLimits {
        engine: Limits {
            steps_per_entry: 50_000_000,
            memory_bytes: 256 * 1024 * 1024,
            frame_depth: 1_000,
            reentry_depth: 64,
            parser_nesting: 1_000,
            string_length: 1 << 22,
            bigint_bits: 1 << 10,
        },
        steps_per_runner: 1_000_000_000,
        scripts_per_runner: 1_000,
        source_bytes: 1 << 20,
        wall_clock: Duration::from_secs(50),
    }
}

/// The parser's bounds this module raises, in one place: 20,000 attributes an element (the
/// shapes hold 16,384, and the writes add the 16,385th), 22 MiB of input (the page of the
/// values holds 20,971,522 bytes of them) and 256 MiB of stored bytes.
fn raised() -> ParserLimits {
    ParserLimits {
        max_attributes: 20_000,
        max_input_bytes: 23_068_672,
        max_stored_bytes: 256 * 1024 * 1024,
        ..ParserLimits::default()
    }
}

fn parsed(page: &str) -> Tree {
    parse_document(
        page,
        &ParseOptions {
            limits: raised(),
            ..ParseOptions::default()
        },
    )
}

fn with_tree<R: Send>(
    tree: impl FnOnce() -> Tree + Send,
    body: impl FnOnce(&mut Runner) -> R + Send,
) -> R {
    run_document(
        limits(),
        Mode::Test,
        move || Document {
            tree: tree(),
            limits: raised(),
            slices: dom::slices(DocumentInfo::default()),
        },
        body,
    )
    .expect("a runner with the bindings")
}

/// One measured call: what it is, its steps, its time, and the floor of its steps.
struct Call {
    what: String,
    steps: u64,
    took: Duration,
    floor: u64,
}

/// Runs `source` once and measures it; the script must run to its end.
fn call(runner: &mut Runner, what: &str, source: &str, floor: u64) -> Call {
    let before = runner.steps();
    let at = Instant::now();
    let end = runner.evaluate(source);
    let took = at.elapsed();
    assert_eq!(end, ScriptEnd::Ran, "{what}: {end}");
    Call {
        what: what.to_string(),
        steps: runner.steps() - before,
        took,
        floor,
    }
}

/// Runs `setup` and then measures `source` on a runner of its own over `page`.
fn call_on(page: &str, setup: &str, what: &str, source: &str, floor: u64) -> Call {
    let page = page.to_string();
    with_tree(
        move || parsed(&page),
        |runner| {
            let end = runner.evaluate(setup);
            assert_eq!(end, ScriptEnd::Ran, "{what}: the setup: {end}");
            call(runner, what, source, floor)
        },
    )
}

/// The plain walk's time a step, in nanoseconds: one call of `getElementById('none')` over
/// 20,000 elements without attributes.
fn plain() -> f64 {
    let page = format!("<!DOCTYPE html><div>{}</div>", "<p></p>".repeat(20_000));
    let plain = call_on(
        &page,
        "",
        "plain",
        "document.getElementById('none');",
        20_000,
    );
    per_step(&plain)
}

fn per_step(call: &Call) -> f64 {
    call.took.as_nanos() as f64 / call.steps.max(1) as f64
}

/// Prints a line a call, against the plain walk, and asserts each call's floor of steps.
fn report(name: &str, calls: &[Call]) {
    let plain = plain();
    let lines: Vec<String> = calls
        .iter()
        .map(|call| {
            format!(
                "{}: {} steps (floor {}), {:?}, {:.1} ns a step, {:.1} x the plain walk's {:.1} ns",
                call.what,
                call.steps,
                call.floor,
                call.took,
                per_step(call),
                per_step(call) / plain,
                plain
            )
        })
        .collect();
    eprintln!("{name}:\n{}", lines.join("\n"));
    for call in calls {
        assert!(call.steps >= call.floor, "{}: {lines:#?}", call.what);
    }
}

/// `n` attributes `n0` .. `n<n-1>` with empty values, as markup.
/// The number of attributes of the first element named `name` in the runner's tree, and the
/// byte length of the local name of its last attribute.
fn last_attribute(runner: &mut Runner, name: &str) -> (usize, usize) {
    let tree = runner
        .agent_and_realm()
        .0
        .host()
        .document()
        .expect("the bindings")
        .tree();
    let attributes = tree
        .descendants(tree.root())
        .into_iter()
        .find_map(|node| match tree.data(node) {
            NodeData::Element {
                local_name,
                attributes,
                ..
            } if local_name == name => Some(attributes),
            _ => None,
        })
        .expect("the element");
    let last = attributes
        .last()
        .map_or(0, |attribute| attribute.local_name.len());
    (attributes.len(), last)
}

fn numbered(n: usize) -> String {
    (0..n).map(|i| format!(" n{i}=\"\"")).collect()
}

/// 11a, 11b and 11c: an element of 4,096 attributes whose names are 1,000 bytes, 996 letters
/// and a number of four digits, and one of 16,384 short names `n0` to `n16383`.
#[test]
fn scans_of_long_names_and_of_many_names() {
    let long: String = (0..4_096)
        .map(|i| format!(" {}{i:04}=\"\"", "a".repeat(996)))
        .collect();
    let page = format!("<!DOCTYPE html><i{long}></i><b{}></b>", numbered(16_384));
    let wanted = format!("{}zzzz", "a".repeat(996));
    let i = "var e = document.getElementsByTagName('i')[0]; var w = '".to_string() + &wanted + "';";
    let b = "var e = document.getElementsByTagName('b')[0];";
    // 4,096 / 4 + 4,096 x 1,000 / 16: every name has the length of the wanted one, and each
    // compare reads the whole name.
    let long_floor = 4_096 / 4 + 4_096 * 1_000 / 16;
    assert_eq!(long_floor, 257_024);
    // The names of 16,384: 10 x 2 + 90 x 3 + 900 x 4 + 9,000 x 5 + 6,384 x 6 bytes.
    let names: u64 = 10 * 2 + 90 * 3 + 900 * 4 + 9_000 * 5 + 6_384 * 6;
    assert_eq!(names, 87_194);
    let calls = vec![
        call_on(
            &page,
            &i,
            "getAttribute of an absent name over 4,096 names of 1,000 bytes",
            "e.getAttribute(w);",
            long_floor,
        ),
        call_on(
            &page,
            &i,
            "hasAttribute of an absent name over 4,096 names of 1,000 bytes",
            "e.hasAttribute(w);",
            long_floor,
        ),
        call_on(
            &page,
            b,
            "getAttributeNS(null, 'x') over 16,384 names",
            "e.getAttributeNS(null, 'x');",
            16_384 / 4,
        ),
        call_on(
            &page,
            b,
            "hasAttributeNS(null, 'x') over 16,384 names",
            "e.hasAttributeNS(null, 'x');",
            16_384 / 4,
        ),
        // `k` is 10 (`n0` to `n9` have the length of `id`): 16,384 / 4 + 10 x 2 / 16.
        call_on(&page, b, "id over 16,384 names", "e.id;", 16_384 / 4 + 1),
        // One step a name, the names' bytes over 16, and 8 a String the glue answers.
        call_on(
            &page,
            b,
            "getAttributeNames over 16,384 names",
            "e.getAttributeNames();",
            16_384 + names / 16 + 8 * 16_384,
        ),
    ];
    assert_eq!(calls[5].floor, 152_905);
    report("scans of long names and of many names", &calls);
}

/// 11d: a value of `string_length` bytes (1 << 22) is read and charged twice, once by the
/// slice and once by the glue; one byte longer it is in the band where its units must be
/// counted, which is charged before the count; over three times `string_length` it is the
/// `RangeError` with no count. One runner and one parse: the reads change nothing.
#[test]
fn values_at_and_over_string_length() {
    let at = 1_usize << 22;
    let page = format!(
        "<!DOCTYPE html><i a=\"{}\"></i><i a=\"{}\"></i><i a=\"{}\"></i>",
        "x".repeat(at),
        "x".repeat(at + 1),
        "x".repeat(3 * at + 1)
    );
    let calls = with_tree(
        move || parsed(&page),
        |runner| {
            let end = runner.evaluate("var G = this; G.i = document.getElementsByTagName('i');");
            assert_eq!(end, ScriptEnd::Ran, "{end}");
            let range = |n: usize| {
                format!(
                    "try {{ i[{n}].getAttribute('a'); throw new Error('no RangeError'); }} \
                     catch (e) {{ if (!(e instanceof RangeError)) throw e; }}"
                )
            };
            vec![
                // The slice's charge and the glue's, 2 x 4,194,304 / 16; no count.
                call(
                    runner,
                    "getAttribute of 4,194,304 bytes",
                    "i[0].getAttribute('a');",
                    2 * 4_194_304 / 16,
                ),
                // 4,194,305 / 16 for the value, and again for the count before it runs.
                call(
                    runner,
                    "getAttribute of 4,194,305 bytes, the RangeError",
                    &range(1),
                    2 * (4_194_305 / 16),
                ),
                // 12,582,913 / 16, once: no count.
                call(
                    runner,
                    "getAttribute of 12,582,913 bytes, the RangeError",
                    &range(2),
                    12_582_913 / 16,
                ),
            ]
        },
    );
    assert_eq!(calls[1].floor, 524_288);
    assert_eq!(calls[2].floor, 786_432);
    assert!(
        calls[2].steps < 1_572_864,
        "a value over 3 x string_length is not counted: {}",
        calls[2].steps
    );
    report("values at and over string_length", &calls);
}

/// 11e and 11f: on an element of 16,384 attributes `n0` to `n16383`, the writes that add the
/// 16,385th attribute with a value of 4 Mi bytes or a name of 1 MiB, and the removals of the
/// last attribute.
#[test]
fn writes_and_removals_over_16_384_attributes() {
    let attributes = numbered(16_384);
    let page = format!("<!DOCTYPE html><b{attributes}></b>");
    // The arguments come from the parser's markup, read by the setup with `getAttribute`,
    // which copies a parsed value at once where `'x'.repeat(..)` builds it by steps of the
    // engine: the measured call and its floor are the same.
    let value_page = format!(
        "<!DOCTYPE html><b{attributes}></b><i v=\"{}\"></i>",
        "x".repeat(4_194_304)
    );
    let name_page = format!(
        "<!DOCTYPE html><b{attributes}></b><i n=\"{}\" q=\"x:{}\"></i>",
        "y".repeat(1_048_576),
        "y".repeat(1_048_574)
    );
    let b = "var e = document.getElementsByTagName('b')[0];";
    let i = "var i = document.getElementsByTagName('i')[0];";
    let value = format!("{b} {i} var v = i.getAttribute('v');");
    let local = format!("{b} {i} var n = i.getAttribute('n');");
    let qualified = format!("{b} {i} var q = i.getAttribute('q');");
    // Two charged scans of 16,384 attributes, three passes: 2 x 16,384 / 4.
    let scans = 2 * 16_384 / 4;
    // The glue's copy of the value, 4,194,304 x 2 / 16, and the tree's, (1 + 4,194,304) / 16.
    let value_floor = 524_288 + 262_144 + scans;
    assert_eq!(value_floor, 794_624);
    // The glue's copy of a name of 1 MiB, its validation and the tree's copy of it.
    let name_floor = 131_072 + 65_536 + 65_536 + scans;
    // The glue (and 3 for the URL), validate and extract over the name, and the tree's copy
    // without the colon, (1 + 1,048,574) / 16.
    let ns_floor = 131_075 + 65_536 + 65_535 + scans;
    assert_eq!(ns_floor, 270_338);
    // `k` is 6,384 (`n10000` to `n16383` have 6 bytes): each scan 16,384 / 4 + 6,384 x 6 / 16.
    let removal_floor = 2 * (16_384 / 4 + 6_384 * 6 / 16);
    assert_eq!(removal_floor, 12_980);
    // Each write adds the 16,385th attribute, read from the tree after the measured call: the
    // count of `b`'s attributes, and the local name of the last one by its length.
    let added = |what: &str, page: &str, setup: &str, source: &str, local: usize, floor: u64| {
        let page = page.to_string();
        let (setup, source) = (setup.to_string(), source.to_string());
        with_tree(
            move || parsed(&page),
            |runner| {
                let end = runner.evaluate(&setup);
                assert_eq!(end, ScriptEnd::Ran, "{what}: the setup: {end}");
                let measured = call(runner, what, &source, floor);
                let (count, last) = last_attribute(runner, "b");
                assert_eq!(count, 16_385, "{what}: the 16,385th attribute");
                assert_eq!(last, local, "{what}: the local name of the added attribute");
                measured
            },
        )
    };
    let calls = vec![
        added(
            "setAttribute('a', 4 Mi bytes)",
            &value_page,
            &value,
            "e.setAttribute('a', v);",
            1,
            value_floor,
        ),
        added(
            "id = 4 Mi bytes",
            &value_page,
            &value,
            "e.id = v;",
            2,
            value_floor,
        ),
        added(
            "setAttributeNS(SVG, a name of 1 MiB, '')",
            &name_page,
            &qualified,
            "e.setAttributeNS('http://www.w3.org/2000/svg', q, '');",
            1_048_574,
            ns_floor,
        ),
        added(
            "toggleAttribute(a name of 1 MiB), adding",
            &name_page,
            &local,
            "e.toggleAttribute(n);",
            1_048_576,
            name_floor,
        ),
        added(
            "setAttribute(a name of 1 MiB, '')",
            &name_page,
            &local,
            "e.setAttribute(n, '');",
            1_048_576,
            name_floor,
        ),
        call_on(
            &page,
            b,
            "removeAttribute('n16383')",
            "e.removeAttribute('n16383');",
            removal_floor,
        ),
        call_on(
            &page,
            b,
            "removeAttributeNS(null, 'n16383')",
            "e.removeAttributeNS(null, 'n16383');",
            removal_floor,
        ),
        call_on(
            &page,
            b,
            "toggleAttribute('n16383'), removing",
            "e.toggleAttribute('n16383');",
            removal_floor,
        ),
    ];
    report("writes and removals over 16,384 attributes", &calls);
}

/// 11g: the quadratic shape, 4,096 new names on one element: the write of index `i` finds
/// `i` attributes and two scans each charge `i / 4`, at least
/// 2 x 4 x (0 + 1 + ... + 1,023) = 4,190,208 steps.
#[test]
fn four_thousand_new_names_on_one_element() {
    let floor: u64 = (0..4_096_u64).map(|i| 2 * (i / 4)).sum();
    assert_eq!(floor, 4_190_208);
    let calls = vec![call_on(
        "<!DOCTYPE html><p></p>",
        "var e = document.documentElement; var i;",
        "4,096 setAttribute of new names on one element",
        "for (i = 0; i < 4096; i++) e.setAttribute('n' + i, 'v');",
        floor,
    )];
    report("4,096 new names on one element", &calls);
}

/// `source` with every line comment, the rest of a line after `//`, and each line left blank
/// cut.
fn code_lines(source: &str) -> String {
    source
        .lines()
        .map(|line| line.find("//").map_or(line, |at| &line[..at]))
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The text of the function `head` names, from its line to the next line that starts a
/// function after its blanks.
fn function_text<'a>(code: &'a str, head: &str) -> (usize, &'a str) {
    let start = code
        .find(head)
        .unwrap_or_else(|| panic!("no `{head}` in the source"));
    let start = code[..start].rfind('\n').map_or(0, |at| at + 1);
    let mut end = code.len();
    let mut offset = start
        + code[start..]
            .find('\n')
            .map_or(code.len() - start, |n| n + 1);
    while offset < code.len() {
        let line_end = code[offset..]
            .find('\n')
            .map_or(code.len(), |n| offset + n + 1);
        let line = code[offset..line_end].trim_start();
        if line.starts_with("fn ")
            || line.starts_with("pub fn ")
            || line.starts_with("pub(super) fn ")
        {
            end = offset;
            break;
        }
        offset = line_end;
    }
    (start, &code[start..end])
}

/// The text from the first `{` at or after `from` to the `}` that closes it, by counted braces:
/// its start and its end (after the `}`).
fn braced(text: &str, from: usize) -> Option<(usize, usize)> {
    let start = from + text[from..].find('{')?;
    let mut depth = 0_usize;
    for (at, c) in text[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((start, start + at + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// The members of `attributes` touch the tree and the charges only through `charged_write`:
/// it charges for memory once and settles twice, each settle the statement
/// `cx.settle_stored(..)?;`, one in each arm of the `match` of the tree's answer, and nothing
/// leaves the function between the tree call and the settle of each arm.
#[test]
fn the_members_touch_the_tree_only_through_the_files() {
    // Which file names the tree and the charges, in any spelling, is held by
    // `adversary_dom_attributes::the_members_file_names_no_tree_and_no_charge_in_any_spelling`;
    // this test holds the order inside `charged_write`.
    let writes = code_lines(include_str!("../src/dom/attribute_writes.rs"));
    let (_, body) = function_text(&writes, "fn charged_write");
    assert_eq!(body.matches("charge_stored(").count(), 1, "{body}");
    assert_eq!(body.matches("settle_stored(").count(), 2, "{body}");
    assert_eq!(body.matches("settle_stored(0, 0)").count(), 1, "{body}");
    // Each settle is one statement `cx.settle_stored(..)?;`.
    let settles: Vec<(usize, usize)> = body
        .match_indices("cx.settle_stored(")
        .map(|(at, _)| {
            let end = at + body[at..].find(';').expect("the end of a settle") + 1;
            assert!(
                body[at..end].ends_with(")?;"),
                "a settle is not `cx.settle_stored(..)?;`: {}",
                &body[at..end]
            );
            (at, end)
        })
        .collect();
    assert_eq!(settles.len(), 2, "{body}");
    let call = "write(cx.tree_mut())";
    assert_eq!(
        body.matches(call).count(),
        1,
        "charged_write holds `{call}` once:\n{body}"
    );
    let call_at = body.find(call).expect("the tree call");
    let line_of_call = body[..call_at].rfind('\n').map_or(0, |at| at + 1);
    let match_at = line_of_call
        + body[line_of_call..]
            .find("match ")
            .unwrap_or_else(|| panic!("no `match` of the tree's answer:\n{body}"));
    let mut arms = Vec::new();
    let mut from = match_at;
    for _ in 0..2 {
        let arrow = from
            + body[from..]
                .find("=>")
                .unwrap_or_else(|| panic!("an arm is missing:\n{body}"));
        let head_start = body[..arrow].rfind('\n').map_or(0, |at| at + 1);
        let head = body[head_start..arrow].trim().to_string();
        let after = body[arrow + 2..].trim_start();
        assert!(
            after.starts_with('{'),
            "the arm `{head}` has no `{{` after its `=>`:\n{body}"
        );
        let (open, close) = braced(body, arrow).expect("the arm's braces");
        arms.push((head, open, close));
        from = close;
    }
    assert!(
        arms.iter().any(|(head, _, _)| head.starts_with("Err(")),
        "no arm for the `Err`: {arms:?}"
    );
    assert!(
        arms.iter().any(|(head, _, _)| head.starts_with("Ok(")),
        "no arm for the `Ok`: {arms:?}"
    );
    let mut cut: Vec<(usize, usize)> = Vec::new();
    for (head, open, close) in &arms {
        let inside: Vec<&(usize, usize)> = settles
            .iter()
            .filter(|(at, _)| at > open && at < close)
            .collect();
        assert_eq!(
            inside.len(),
            1,
            "the arm `{head}` holds one settle:\n{body}"
        );
        // The settle and the rest of the arm after it.
        cut.push((inside[0].0, *close));
    }
    let last = settles.iter().map(|(_, end)| *end).max().expect("a settle");
    let mut gap = String::new();
    let mut at = call_at;
    cut.sort_unstable();
    for (from, to) in cut {
        if from > at {
            gap.push_str(&body[at..from.min(last)]);
        }
        at = at.max(to);
    }
    if at < last {
        gap.push_str(&body[at..last]);
    }
    for token in ["?", "return", "unwrap", "expect("] {
        assert!(
            !gap.contains(token),
            "`{token}` between the tree call and a settle:\n{gap}"
        );
    }
}
