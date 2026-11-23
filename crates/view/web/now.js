// The centre panel "Now": what can be done on the page at the playhead, the `now` member of the
// answer entry of the action at the playhead (docs/view-stream.md, "The now member"), grouped by
// kind: links, buttons, and fields for every control; a kind not named is its own group. A link
// with its handle, visibility word, label and target; a control with its handle, visibility
// word, kind, label, value and flags; a button with what it submits (form, method, action). A
// password shows "(empty)", or the length a fill typed ("12 bytes", from the FillRequested
// entry, never the value), or "(set)" when no fill of this document typed it. A hidden element
// is marked; the visibility note is shown when the answer has it. A line with neither target
// nor state is shown as its text. Built only through dom.js; every value is text: a label, a
// target, a value and a line come from the page, and no target or action is a link. Nothing
// here throws on a null member.

import { clear, element } from "./dom.js";

// The heading of the group of a kind; a control of any other kind is a field, and a kind not
// named here without a state is its own heading.
const GROUPS = new Map([
  ["link", "links"],
  ["button", "buttons"],
]);

// What each visibility word of the answer says (docs/agent-answer.md, "Now").
const VISIBILITY = new Map([
  ["vis", "visible"],
  ["hid", "hidden attribute"],
  ["?", "not computed"],
]);

// The flags of a control's state, in the order a line writes them.
const FLAGS = ["disabled", "checked", "required", "readonly"];

function absent(value) {
  return value === null || value === undefined;
}

function written(value) {
  return absent(value) ? "?" : String(value);
}

function cut(isCut) {
  return isCut === true ? element("span", { className: "cut", text: "…" }) : null;
}

function isObject(value) {
  return value !== null && typeof value === "object";
}

// The text of a control's value as the answer prints it: a quoted value in quotes, with "..."
// after a cut one; a password "(empty)", its length from `lengths` ("12 bytes"), or "(set)".
// Shared with changes.js.
export function valueText(value, handle, lengths) {
  if (!isObject(value)) {
    return "?";
  }
  if (value.kind === "text") {
    return '"' + written(value.text) + '"' + (value.cut === true ? "..." : "");
  }
  if (value.kind === "empty") {
    return "(empty)";
  }
  if (value.kind === "set") {
    const length = lengths instanceof Map ? lengths.get(handle) : undefined;
    return typeof length === "string" ? length : "(set)";
  }
  return "?";
}

function heading(held) {
  const kind = written(held && held.kind);
  if (GROUPS.has(kind)) {
    return GROUPS.get(kind);
  }
  return isObject(held.state) ? "fields" : kind;
}

function label(held) {
  return element("span", { className: "label" + (absent(held.label) ? " absent" : ""), text: absent(held.label) ? "no label" : held.label });
}

// The parts of a control's or a button's state: value, flags, and what a button submits.
function stateParts(held, lengths) {
  const state = held.state;
  const parts = [];
  if (isObject(state.value)) {
    const password = state.value.kind === "set" || state.value.kind === "empty";
    parts.push(element("span", { className: "value" + (password ? " password" : ""), text: valueText(state.value, held.handle, lengths) }));
  }
  for (const flag of FLAGS) {
    if (state[flag] === true) {
      parts.push(element("span", { className: "flag", text: flag }));
    }
  }
  const submits = state.submits;
  if (isObject(submits)) {
    parts.push(
      element("span", { className: "submits" }, [
        element("span", { className: "form", text: written(submits.form) }),
        element("span", { className: "method", text: written(submits.method) }),
        absent(submits.action)
          ? element("span", { className: "unresolved", text: "unresolved" })
          : element("span", { className: "action", address: submits.action }),
        cut(submits.action_cut),
      ]),
    );
  }
  return parts;
}

function item(held, lengths) {
  const visibility = written(held.visibility);
  const parts = [
    element("span", { className: "handle", text: written(held.handle) }),
    element("span", {
      className: "visibility",
      text: visibility,
      attributes: { title: VISIBILITY.get(visibility) || null },
    }),
  ];
  if (isObject(held.state)) {
    parts.push(element("span", { className: "kind", text: written(held.kind) }), label(held), cut(held.label_cut), ...stateParts(held, lengths));
  } else if (typeof held.text === "string") {
    parts.push(element("span", { className: "text", text: held.text }));
  } else {
    parts.push(
      label(held),
      cut(held.label_cut),
      element("span", { className: "arrow", text: "→" }),
      element("span", { className: "target", address: absent(held.target) ? "" : held.target }),
      cut(held.target_cut),
      held.unresolved === true ? element("span", { className: "unresolved", text: "unresolved" }) : null,
    );
  }
  return element("li", { className: "now-element" + (visibility === "hid" ? " is-hidden" : "") }, parts);
}

// The elements of `now` in groups, each group where its first element is.
function groups(elements, lengths) {
  const found = new Map();
  for (const held of elements) {
    const name = heading(held);
    if (!found.has(name)) {
      found.set(name, []);
    }
    found.get(name).push(held);
  }
  return [...found].map(([name, members]) =>
    element("section", { className: "now-group" }, [
      element("h4", { text: name }),
      element(
        "ol",
        { className: "now-list" },
        members.map((held) => item(held, lengths)),
      ),
    ]),
  );
}

// `at` is what state.js nowAt() gives: null with no action, the word "running" while the
// action's answer is not held, or {now, lengths} with the answer's now member (null when it has
// none) and the lengths fills typed.
export function renderNow(node, at) {
  clear(node);
  if (at === null || at === undefined) {
    node.append(element("p", { className: "now-empty", text: "no action" }));
    return;
  }
  if (at === "running") {
    node.append(element("p", { className: "now-empty running", text: "running" }));
    return;
  }
  const now = at.now;
  const elements = now && Array.isArray(now.elements) ? now.elements.filter((held) => held && typeof held === "object") : [];
  if (elements.length === 0) {
    node.append(element("p", { className: "now-empty", text: "nothing to act on" }));
    return;
  }
  node.append(element("p", { className: "now-count", text: written(now.shown) + " of " + written(now.total) }));
  node.append(...groups(elements, at.lengths));
  if (typeof now.visibility_note === "string") {
    node.append(element("p", { className: "now-note", text: "visibility: " + now.visibility_note }));
  }
}
