// The centre panel "Now": what can be done on the page at the playhead, the `now` member of the
// answer entry of the action at the playhead (docs/view-stream.md, "The now member"), grouped by
// kind, each element with its handle, visibility word, label and target; a hidden one marked;
// the visibility note when the answer has it. A line of a kind the view does not know is shown
// as its text. Built only through dom.js; every value is text: a label, a target and a line come
// from the page, and no target is a link. Nothing here throws on a null member.

import { clear, element } from "./dom.js";

// The heading of the group of a kind; a kind not named here is its own heading.
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

function absent(value) {
  return value === null || value === undefined;
}

function written(value) {
  return absent(value) ? "?" : String(value);
}

function cut(isCut) {
  return isCut === true ? element("span", { className: "cut", text: "…" }) : null;
}

function item(held) {
  const visibility = written(held.visibility);
  const parts = [
    element("span", { className: "handle", text: written(held.handle) }),
    element("span", {
      className: "visibility",
      text: visibility,
      attributes: { title: VISIBILITY.get(visibility) || null },
    }),
  ];
  if (typeof held.text === "string") {
    parts.push(element("span", { className: "text", text: held.text }));
  } else {
    parts.push(
      element("span", { className: "label" + (absent(held.label) ? " absent" : ""), text: absent(held.label) ? "no label" : held.label }),
      cut(held.label_cut),
      element("span", { className: "arrow", text: "→" }),
      element("span", { className: "target", address: absent(held.target) ? "" : held.target }),
      cut(held.target_cut),
      held.unresolved === true ? element("span", { className: "unresolved", text: "unresolved" }) : null,
    );
  }
  return element("li", { className: "now-element" + (visibility === "hid" ? " is-hidden" : "") }, parts);
}

// The elements of `now` in groups by kind, each group where its first element is.
function groups(elements) {
  const found = new Map();
  for (const held of elements) {
    const kind = written(held && held.kind);
    if (!found.has(kind)) {
      found.set(kind, []);
    }
    found.get(kind).push(held);
  }
  return [...found].map(([kind, members]) =>
    element("section", { className: "now-group" }, [
      element("h4", { text: GROUPS.get(kind) || kind }),
      element("ol", { className: "now-list" }, members.map(item)),
    ]),
  );
}

// `at` is what state.js nowAt() gives: null with no action, the word "running" while the
// action's answer is not held, or {now} with the answer's now member (null when it has none).
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
  node.append(...groups(elements));
  if (typeof now.visibility_note === "string") {
    node.append(element("p", { className: "now-note", text: "visibility: " + now.visibility_note }));
  }
}
