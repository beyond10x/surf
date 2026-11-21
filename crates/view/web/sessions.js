// The session list: state, address, title, time since the last action; a click selects.

import { attribute, clear, element, text } from "./dom.js";

export function since(then, now) {
  if (then === null || then === undefined) {
    return "no action";
  }
  const seconds = Math.max(0, Math.round((now - then) / 1000));
  if (seconds < 60) {
    return seconds + " s ago";
  }
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return minutes + " min ago";
  }
  const hours = Math.floor(minutes / 60);
  if (hours < 48) {
    return hours + " h ago";
  }
  return Math.floor(hours / 24) + " d ago";
}

function item(summary, selectedId, now, onSelect) {
  const ended = summary.state !== "open";
  const button = element(
    "button",
    {
      className: "session",
      attributes: { type: "button", "aria-pressed": summary.id === selectedId ? "true" : "false" },
    },
    [
      element("span", { className: "state state-" + summary.state, text: summary.state }),
      summary.address
        ? element("span", { className: "address", address: summary.address })
        : element("span", { className: "address absent", text: "no address" }),
      summary.title !== null && summary.title !== undefined
        ? element("span", { className: "title", text: summary.title })
        : null,
      element("span", {
        className: "since",
        text: since(summary.last_action_at, now),
        attributes: { "data-at": summary.last_action_at === null ? "" : summary.last_action_at },
      }),
    ],
  );
  button.addEventListener("click", () => onSelect(summary.id));
  const li = element("li", { className: ended ? "ended" : "" }, [button]);
  attribute(li, "aria-current", summary.id === selectedId ? "true" : null);
  return li;
}

export function renderSessions(list, summaries, selectedId, onSelect, now = Date.now()) {
  clear(list);
  if (summaries.length === 0) {
    list.append(element("li", { className: "empty", text: "no sessions" }));
    return;
  }
  for (const summary of summaries) {
    list.append(item(summary, selectedId, now, onSelect));
  }
}

// Brings each "time since" up to date without rebuilding the list.
export function tick(list, now = Date.now()) {
  for (const node of list.querySelectorAll(".since")) {
    const at = node.getAttribute("data-at");
    text(node, since(at === "" ? null : Number(at), now));
  }
}
