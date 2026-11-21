// The selected session's actions in order: each a segment labelled with its sequence, command
// and argument, under it one mark per event of that action, grouped by domain. One segment
// carries the playhead (aria-current="step"); a click on a segment moves the playhead there. Each
// mark holds a button that selects its event (aria-pressed for the selected one).
// A gap in front of the first segment counts the entries dropped. Nothing here throws on a
// null member of docs/view-stream.md: a null sequence or command is written "?", a null event
// name is a mark in the group "other".

import { attribute, clear, element, scroll, text } from "./dom.js";

// The groups of marks, in this order, then OTHER for any other prefix and for a null name.
export const DOMAINS = ["surf.shell", "surf.agent", "surf.navigation", "surf.net", "surf.document", "surf.render"];
const OTHER = "other";

function domainOf(name) {
  if (typeof name === "string") {
    for (const domain of DOMAINS) {
      if (name.startsWith(domain + ".")) {
        return domain;
      }
    }
  }
  return OTHER;
}

function shortName(name) {
  if (typeof name !== "string") {
    return "?";
  }
  const dot = name.lastIndexOf(".");
  return dot < 0 ? name : name.slice(dot + 1);
}

// A value of the timeline that may be null: "?".
function written(value) {
  return value === null || value === undefined ? "?" : String(value);
}

// A mark: the li keeps its class and title; its button, with the short name as its text,
// selects the event. The click stops there, so the segment's handler does not clear it.
function mark(event, selected, onEvent) {
  const fields = Object.entries(event.fields || {})
    .map(([name, value]) => name + " " + written(value))
    .join("\n");
  const name = typeof event.name === "string" ? event.name : "unreadable event";
  const button = element("button", {
    className: "mark-select",
    text: shortName(event.name),
    attributes: {
      type: "button",
      "data-number": event.number,
      "aria-pressed": selected !== null && event.number === selected ? "true" : "false",
    },
  });
  button.addEventListener("click", (click) => {
    click.stopPropagation();
    onEvent(event.number);
  });
  return element(
    "li",
    {
      className: "mark" + (event.unmapped ? " unmapped" : ""),
      attributes: { title: name + (fields ? "\n" + fields : "") },
    },
    [button],
  );
}

function groups(events, selected, onEvent) {
  const byDomain = new Map();
  for (const event of events) {
    const domain = domainOf(event.name);
    if (!byDomain.has(domain)) {
      byDomain.set(domain, []);
    }
    byDomain.get(domain).push(event);
  }
  const shown = [...DOMAINS, OTHER].filter((domain) => byDomain.has(domain));
  return element(
    "div",
    { className: "groups" },
    shown.map((domain) =>
      element("div", { className: "group", attributes: { "data-domain": domain } }, [
        element("p", { className: "group-label", text: domain }),
        element(
          "ol",
          { className: "marks" },
          byDomain.get(domain).map((event) => mark(event, selected, onEvent)),
        ),
      ]),
    ),
  );
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
    return "failed " + written(page.reason);
  }
  return written(page.status);
}

function segment(action, current, onSelect, selected, onEvent) {
  const entry = action.action;
  const button = element(
    "button",
    {
      className: "select",
      attributes: { type: "button", "data-number": entry.number, "aria-pressed": current ? "true" : "false" },
    },
    [
      element("span", { className: "label" }, [
        element("span", { className: "sequence", text: written(entry.sequence) }),
        element("span", { className: "command", text: written(entry.command) }),
        entry.argument === null || entry.argument === undefined
          ? null
          : element("span", { className: "argument", address: entry.argument }),
      ]),
    ],
  );
  const end = element("p", {
    className: "outcome" + (action.answer ? "" : " running"),
    text: outcome(action.answer),
  });
  const node = element("li", { className: "segment" }, [button, groups(action.events, selected, onEvent), end]);
  attribute(node, "aria-current", current ? "step" : null);
  node.addEventListener("click", () => onSelect(entry.number));
  return node;
}

function gap(words) {
  return element("li", { className: "gap", text: words });
}

// `actions` as state.js groups them, `at` the index of the playhead (-1 for none), `drops` the
// session detail's {dropped, log_dropped}, `follow` whether the playhead follows the newest,
// `onSelect` called with a segment's action entry number, `selected` the number of the selected
// event entry (or null), `onEvent` called with a mark's event entry number.
export function renderTimeline(list, actions, at, drops, follow, onSelect, selected, onEvent) {
  const track = list.parentElement;
  const focused = document.activeElement;
  const focusedNumber =
    focused && list.contains(focused) ? focused.getAttribute("data-number") : null;
  clear(list);
  if (drops.log_dropped) {
    list.append(gap("log dropped"));
    return;
  }
  if (drops.dropped > 0) {
    list.append(gap(drops.dropped + " dropped"));
  } else if (actions.length === 0) {
    list.append(element("li", { className: "empty", text: "no actions" }));
    return;
  }
  let current = null;
  actions.forEach((action, index) => {
    const node = segment(action, index === at, onSelect, selected, onEvent);
    if (index === at) {
      current = node;
    }
    list.append(node);
  });
  if (focusedNumber !== null) {
    const again = list.querySelector('button[data-number="' + focusedNumber + '"]');
    if (again) {
      again.focus({ preventScroll: true });
    }
  }
  if (!track || !current) {
    return;
  }
  if (follow) {
    scroll(track, track.scrollWidth);
  } else if (
    current.offsetLeft < track.scrollLeft ||
    current.offsetLeft + current.offsetWidth > track.scrollLeft + track.clientWidth
  ) {
    scroll(track, Math.max(0, current.offsetLeft - (track.clientWidth - current.offsetWidth) / 2));
  }
}

export function resetTimeline(list) {
  clear(list);
}

// The "Latest" control and the marker of actions newer than the playhead.
export function renderControls(latestButton, newerNode, follow, newer) {
  attribute(latestButton, "aria-pressed", follow ? "true" : "false");
  const words = !follow && newer > 0 ? newer + " newer" : "";
  text(newerNode, words);
  attribute(newerNode, "hidden", words ? null : "");
}
