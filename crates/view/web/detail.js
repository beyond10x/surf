// The right column: the selected action (the one at the playhead) or the selected event with its
// fields, and the selected session's requests, one row per request_id with its address and its
// status, media type and size, or its failure and reason, or pending. Built only through dom.js;
// every value is text. Nothing here throws on a null member of docs/view-stream.md: a null
// sequence or command is written "?", a null field value "no value", a null event name
// "unreadable event".

import { clear, element } from "./dom.js";

const NO_VALUE = "no value";

function clock(ms) {
  return new Date(ms).toLocaleTimeString([], { hour12: false });
}

function absent(value) {
  return value === null || value === undefined;
}

// A value of an entry that may be null: "?", as on the timeline.
function written(value) {
  return absent(value) ? "?" : String(value);
}

// The fields of an event entry as an object; {} for anything else.
function fieldsOf(entry) {
  const fields = entry.fields;
  return fields !== null && typeof fields === "object" && !Array.isArray(fields) ? fields : {};
}

// A node `tag` of class `className` holding `value`: as an address when `isAddress`, as text
// otherwise, or the words "no value" for a null value.
function value(tag, className, held, isAddress) {
  if (absent(held)) {
    return element(tag, { className: className + " absent", text: NO_VALUE });
  }
  return element(tag, isAddress ? { className, address: held } : { className, text: held });
}

function pair(name, node) {
  return element("div", { className: "part" }, [element("dt", { text: name }), node]);
}

// The action at the playhead: `action` as state.js actions() groups it.
function actionView(action) {
  const entry = action.action;
  const parts = [
    pair("sequence", element("dd", { className: "sequence", text: written(entry.sequence) })),
    pair("command", element("dd", { className: "command", text: written(entry.command) })),
  ];
  if (!absent(entry.argument)) {
    parts.push(pair("argument", element("dd", { className: "argument", address: entry.argument })));
  }
  parts.push(pair("kept", element("dd", { className: "kept", text: absent(entry.kept_at) ? "?" : clock(entry.kept_at) })));
  parts.push(pair("events", element("dd", { className: "events", text: action.events.length })));
  return [element("h3", { className: "selection-kind", text: "Action" }), element("dl", { className: "summary" }, parts)];
}

// The selected event entry: its qualified name and every field in the entry's order.
function eventView(entry) {
  const name = typeof entry.name === "string" ? entry.name : "unreadable event";
  const nodes = [
    element("h3", { className: "selection-kind", text: "Event" }),
    element("p", { className: "name" + (typeof entry.name === "string" ? "" : " absent"), text: name }),
  ];
  if (entry.unmapped === true) {
    nodes.push(element("p", { className: "unmapped", text: "unmapped: no fields" }));
    return nodes;
  }
  const parts = Object.entries(fieldsOf(entry)).map(([field, held]) =>
    pair(field, value("dd", "value", held, field === "url")),
  );
  nodes.push(element("dl", { className: "fields" }, parts));
  return nodes;
}

// The requests among `entries` (the held entries of one session): one per request_id of a
// surf.net event, in the order of the first entry held for it.
function requestsOf(entries) {
  const rows = new Map();
  for (const entry of entries) {
    if (entry.kind !== "event" || typeof entry.name !== "string" || !entry.name.startsWith("surf.net.")) {
      continue;
    }
    const fields = fieldsOf(entry);
    const id = fields.request_id;
    if (typeof id !== "string") {
      continue;
    }
    let row = rows.get(id);
    if (!row) {
      row = { address: fields.url, started: false, response: null, failure: null };
      rows.set(id, row);
    }
    if (entry.name === "surf.net.RequestStarted" && !row.started) {
      row.started = true;
      row.address = fields.url;
    } else if (entry.name === "surf.net.ResponseReceived") {
      row.response = fields;
    } else if (entry.name === "surf.net.FetchFailed") {
      row.failure = fields;
    }
  }
  return [...rows.values()];
}

function requestRow(row) {
  const parts = [value("span", "address", row.address, true)];
  let state = "";
  if (row.response) {
    parts.push(
      value("span", "status", row.response.status),
      value("span", "media-type", row.response.media_type),
      value("span", "size", row.response.body),
    );
  } else if (row.failure) {
    state = " is-failed";
    parts.push(element("span", { className: "failed", text: "failed" }), value("span", "reason", row.failure.reason));
  } else {
    state = " is-pending";
    parts.push(element("span", { className: "pending", text: "pending" }));
  }
  return element("li", { className: "request" + state }, parts);
}

// `action` the action at the playhead as state.js actions() groups it, or null; `event` the
// selected event entry, or null; `entries` every entry held of the selected session.
export function renderDetail(selectionNode, requestsNode, action, event, entries) {
  clear(selectionNode);
  if (event) {
    selectionNode.append(...eventView(event));
  } else if (action) {
    selectionNode.append(...actionView(action));
  } else {
    selectionNode.append(element("p", { className: "empty", text: "no action" }));
  }
  clear(requestsNode);
  for (const row of requestsOf(entries)) {
    requestsNode.append(requestRow(row));
  }
}
