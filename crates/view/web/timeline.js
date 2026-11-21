// The selected session's actions in order: each a segment labelled with its command and
// argument, under it one mark per event of that action, named by its event name. The newest
// action is marked; new actions and events are appended as they arrive.

import { attribute, clear, element, text } from "./dom.js";

const shown = { session: null, segments: [] };

function shortName(name) {
  const dot = name.lastIndexOf(".");
  return dot < 0 ? name : name.slice(dot + 1);
}

function mark(event) {
  const fields = Object.entries(event.fields || {})
    .map(([name, value]) => name + " " + value)
    .join("\n");
  return element("li", {
    className: "mark" + (event.unmapped ? " unmapped" : ""),
    text: shortName(event.name),
    attributes: { title: event.name + (fields ? "\n" + fields : "") },
  });
}

function outcome(answer) {
  if (!answer) {
    return "running";
  }
  const page = answer.page;
  if (!page || page.kind === "none") {
    return "page none";
  }
  if (page.kind === "failed") {
    return "failed " + page.reason;
  }
  return String(page.status);
}

function segment(action) {
  const label = element("p", { className: "label" }, [
    element("span", { className: "sequence", text: action.action.sequence }),
    element("span", { className: "command", text: action.action.command }),
    action.action.argument === null
      ? null
      : element("span", { className: "argument", address: action.action.argument }),
  ]);
  const marks = element("ol", { className: "marks" });
  const end = element("p", { className: "outcome" });
  const node = element("li", { className: "segment" }, [label, marks, end]);
  const rendered = { node, marks, end, events: 0, answered: false };
  update(rendered, action);
  return rendered;
}

function update(rendered, action) {
  for (const event of action.events.slice(rendered.events)) {
    rendered.marks.append(mark(event));
  }
  rendered.events = action.events.length;
  const answered = action.answer !== null;
  if (rendered.answered !== answered || rendered.end.textContent === "") {
    text(rendered.end, outcome(action.answer));
    rendered.end.className = "outcome" + (answered ? "" : " running");
    rendered.answered = answered;
  }
}

// `actions` as state.js groups them. Only what changed since the last call is written.
export function renderTimeline(list, sessionId, actions) {
  const scroller = list.parentElement;
  const atEnd = !scroller || scroller.scrollLeft + scroller.clientWidth >= scroller.scrollWidth - 8;
  if (shown.session !== sessionId || actions.length < shown.segments.length) {
    clear(list);
    shown.session = sessionId;
    shown.segments = [];
  }
  if (actions.length === 0) {
    clear(list);
    list.append(element("li", { className: "empty", text: "no actions" }));
    shown.segments = [];
    return;
  }
  const empty = list.querySelector(".empty");
  if (empty) {
    empty.remove();
  }
  actions.forEach((action, at) => {
    const rendered = shown.segments[at];
    if (rendered) {
      update(rendered, action);
    } else {
      const fresh = segment(action);
      shown.segments.push(fresh);
      list.append(fresh.node);
    }
  });
  shown.segments.forEach((rendered, at) => {
    const newest = at === shown.segments.length - 1;
    rendered.node.classList.toggle("newest", newest);
    attribute(rendered.node, "aria-current", newest ? "step" : null);
  });
  if (scroller && atEnd) {
    scroller.scrollLeft = scroller.scrollWidth;
  }
}

export function resetTimeline(list) {
  clear(list);
  shown.session = null;
  shown.segments = [];
}
