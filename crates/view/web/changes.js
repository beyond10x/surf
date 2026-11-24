// The centre panel "Changed by <action>": what the action at the playhead changed, the `changed`
// member of its answer entry (docs/view-stream.md, "The changed member"). Within one page, a
// line per control, `f1 value "" -> "ada"`, `f3 checked true -> false`, `f4 selected "Reader"
// -> "Editor"`; a password's two sides as its lengths from the FillRequested entries before
// and after the action ("12 bytes", never the value), or "(set)" and "(empty)". For another
// page, `page "<old>" -> "<new>"` and a line `+ text "<text>"` per new text, `+ text more <n>`
// for those not shown. Built only through dom.js; every value is text, set as textContent, never
// as markup. Nothing here throws on a null member.

import { clear, element, text } from "./dom.js";
import { valueText } from "./now.js";

function absent(value) {
  return value === null || value === undefined;
}

function written(value) {
  return absent(value) ? "?" : String(value);
}

function isObject(value) {
  return value !== null && typeof value === "object";
}

// An address of a `page` line: quoted, or the word none.
function place(address) {
  return typeof address === "string" ? '"' + address + '"' : "none";
}

function side(what, value, handle, lengths) {
  if (what === "checked") {
    return value === true ? "true" : value === false ? "false" : "?";
  }
  return valueText(value, handle, lengths);
}

// The line of one control change.
function controlLine(line, at) {
  const what = written(line.what);
  const handle = written(line.handle);
  return [handle, what, side(what, line.before, line.handle, at.before), "->", side(what, line.after, line.handle, at.lengths)].join(" ");
}

function lines(at) {
  const changed = at.changed;
  if (changed.kind === "controls") {
    const held = Array.isArray(changed.lines) ? changed.lines.filter(isObject) : [];
    return held.map((line) => controlLine(line, at));
  }
  if (changed.kind === "page") {
    const result = ["page " + place(changed.from) + " -> " + place(changed.to)];
    for (const one of Array.isArray(changed.texts) ? changed.texts.filter(isObject) : []) {
      result.push('+ text "' + written(one.text) + '"' + (one.cut === true ? "..." : ""));
    }
    if (typeof changed.more === "number" && changed.more > 0) {
      result.push("+ text more " + changed.more);
    }
    return result;
  }
  return [];
}

// `at` is what state.js changedAt() gives: null with no action, the word "running" while the
// action's answer is not held, or {command, handle, changed, lengths, before}. `heading` is the
// panel's heading, "Changed" or "Changed by <command> [<handle>]".
export function renderChanges(heading, node, at) {
  clear(node);
  if (at === null || at === undefined) {
    text(heading, "Changed");
    node.append(element("p", { className: "changes-empty", text: "no action" }));
    return;
  }
  if (at === "running") {
    text(heading, "Changed");
    node.append(element("p", { className: "changes-empty running", text: "running" }));
    return;
  }
  text(heading, "Changed by " + written(at.command) + (typeof at.handle === "string" ? " " + at.handle : ""));
  if (!isObject(at.changed)) {
    node.append(element("p", { className: "changes-empty", text: "no changed block" }));
    return;
  }
  const found = lines(at);
  if (found.length === 0) {
    node.append(element("p", { className: "changes-empty", text: "nothing changed" }));
    return;
  }
  node.append(
    element(
      "ol",
      { className: "changes-list" },
      found.map((line) => element("li", { className: "change", text: line })),
    ),
  );
}
