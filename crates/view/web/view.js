// The page: loads /api/sessions, selects a session (the one the address's fragment names, or the
// first), loads it, follows /events, and keeps the live marker (live and the time of the last
// update, or reconnecting), and renders the detail panel. The selection is written into the
// fragment, #session=<id>&at=<sequence>, or #session=<id>&at=<sequence>&event=<number> while an
// event is selected (the event entry's number across all sessions), with history.replaceState,
// and read at load: never the query, which the route table answers 404. An event named at load
// whose entry the named session holds is selected and the playhead put on its action; one not
// held is ignored and `at` applies. When /events or an /api/ route answers 401 or 403 after the
// page loaded, the page stops: no stream, no request, "offline" and the refused token's text. A
// lost connection and any other status are retried.

import { address, attribute, byId, text } from "./dom.js";
import * as state from "./state.js";
import { api, follow, refusal, takeToken } from "./stream.js";
import { renderSessions, tick } from "./sessions.js";
import { renderPage } from "./page.js";
import { renderControls, renderTimeline, resetTimeline } from "./timeline.js";
import { renderDetail } from "./detail.js";

const nodes = {
  live: byId("live"),
  liveWord: byId("live-word"),
  liveTime: byId("live-time"),
  list: byId("session-list"),
  heading: byId("session-heading"),
  facts: byId("session-facts"),
  page: byId("page-line"),
  timeline: byId("actions"),
  latest: byId("latest"),
  newer: byId("newer"),
  message: byId("message"),
  selection: byId("selection"),
  requests: byId("requests"),
};

let token = null;
// The selection the address named at load, {session, at, event}; applied by the first load only.
let wanted = null;
// The fragment last written.
let written = null;
// The stream followed, once the page loaded.
let stream = null;
// Whether the view refused the token after the page loaded: nothing is asked again.
let refused = false;

function clock(ms) {
  return new Date(ms).toLocaleTimeString([], { hour12: false });
}

function setLive(kind, at) {
  if (refused && kind !== "offline") {
    return;
  }
  attribute(nodes.live, "class", "live is-" + kind);
  text(nodes.liveWord, kind === "live" ? "live" : kind);
  text(nodes.liveTime, at ? clock(at) : "");
}

function updated() {
  setLive("live", Date.now());
}

function say(message) {
  text(nodes.message, message);
  attribute(nodes.message, "hidden", message ? null : "");
}

// GET of an /api/ route with the token; refused without a request once the view refused it.
async function ask(path) {
  if (refused) {
    throw new Error("the view refused the token");
  }
  return api(token, path);
}

// `#session=<id>&at=<sequence>&event=<number>` read from `hash`; null when it names no session.
function readSelection(hash) {
  const result = { session: null, at: null, event: null };
  for (const pair of hash.replace(/^#/, "").split("&")) {
    const equals = pair.indexOf("=");
    const name = equals < 0 ? pair : pair.slice(0, equals);
    const value = equals < 0 ? "" : pair.slice(equals + 1);
    if (name === "session" && /^[0-9a-f-]{36}$/.test(value)) {
      result.session = value;
    } else if (name === "at" && /^[0-9]{1,15}$/.test(value)) {
      result.at = Number(value);
    } else if (name === "event" && /^[0-9]{1,15}$/.test(value)) {
      result.event = Number(value);
    }
  }
  return result.session === null ? null : result;
}

// The selection into the fragment: `at` only while Latest is off and the selected action has a
// sequence; `event` only while an event is selected and its entry is held.
function writeSelection() {
  const id = state.selected();
  let fragment = "";
  if (id) {
    fragment = "#session=" + id;
    if (!state.latest()) {
      const list = state.actions(id);
      const at = state.playheadIndex(list);
      const sequence = at >= 0 ? list[at].action.sequence : null;
      if (typeof sequence === "number") {
        fragment += "&at=" + sequence;
      }
      const event = state.eventEntry(list);
      if (event) {
        fragment += "&event=" + event.number;
      }
    }
  }
  if (fragment !== written) {
    written = fragment;
    window.history.replaceState(null, "", window.location.pathname + fragment);
  }
}

function renderList() {
  renderSessions(nodes.list, state.sessions(), state.selected(), choose);
}

function renderSelected() {
  const id = state.selected();
  const summary = id ? state.session(id) : null;
  if (!summary) {
    text(nodes.heading, "No session selected");
    text(nodes.facts, "");
    renderPage(nodes.page, null);
    resetTimeline(nodes.timeline);
    renderControls(nodes.latest, nodes.newer, true, 0);
    renderDetail(nodes.selection, nodes.requests, null, null, []);
    writeSelection();
    return;
  }
  if (summary.title !== null && summary.title !== undefined && summary.title !== "") {
    text(nodes.heading, summary.title);
  } else if (summary.address) {
    address(nodes.heading, summary.address);
  } else {
    text(nodes.heading, "session " + String(summary.id).slice(0, 8));
  }
  const facts = [summary.state, "session " + summary.id];
  if (summary.opened_at !== null && summary.opened_at !== undefined) {
    facts.push("opened " + clock(summary.opened_at));
  }
  if (summary.ended_at !== null && summary.ended_at !== undefined) {
    facts.push("ended " + clock(summary.ended_at));
  }
  text(nodes.facts, facts.join(" · "));
  const list = state.actions(id);
  const at = state.playheadIndex(list);
  const event = state.eventEntry(list);
  renderPage(nodes.page, state.pageAt(id));
  renderTimeline(nodes.timeline, list, at, state.drops(id), state.latest(), moveTo, event ? event.number : null, pick);
  renderControls(nodes.latest, nodes.newer, state.latest(), at < 0 ? 0 : list.length - 1 - at);
  renderDetail(nodes.selection, nodes.requests, at < 0 ? null : list[at], event, state.entries(id));
  writeSelection();
}

// The playhead onto the action whose action entry has `number`; Latest off.
function moveTo(number) {
  state.setPlayhead(number);
  renderSelected();
}

// Selects the event entry `number` of the selected session and moves the playhead to its
// action; Latest off.
function pick(number) {
  const id = state.selected();
  const owner = id ? state.actions(id).find((action) => action.events.some((entry) => entry.number === number)) : null;
  if (owner) {
    state.selectEvent(number, owner.action.number);
    renderSelected();
  }
}

function onLatest() {
  state.followLatest();
  renderSelected();
}

// ArrowLeft and ArrowRight without a modifier move the playhead one action; the page has no
// text input, so they are handled on the document.
function onKey(event) {
  if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) {
    return;
  }
  const step = event.key === "ArrowLeft" ? -1 : event.key === "ArrowRight" ? 1 : 0;
  const id = state.selected();
  if (step === 0 || !id) {
    return;
  }
  event.preventDefault();
  if (state.stepPlayhead(state.actions(id), step)) {
    renderSelected();
  }
}

async function loadSelected() {
  const id = state.selected();
  if (!id) {
    return;
  }
  const asked = state.expectDetail(id);
  let detail;
  try {
    detail = await ask("/api/sessions/" + id);
  } catch (error) {
    state.giveUp(asked);
    throw error;
  }
  state.setDetail(detail);
}

async function loadAll() {
  const list = await ask("/api/sessions");
  state.setSessions(list);
  const named = wanted;
  wanted = null;
  if (named) {
    // A session not listed falls back to the first session, Latest on.
    state.select(state.session(named.session) ? named.session : null);
  }
  if (state.selected() === null && list.sessions.length > 0) {
    state.select(list.sessions[0].id);
  }
  await loadSelected();
  const owner =
    named && named.event !== null && state.selected() === named.session
      ? state.actions(named.session).find((action) => action.events.some((entry) => entry.number === named.event))
      : null;
  if (owner) {
    // An event held in the named session: selected, the playhead on its action.
    state.selectEvent(named.event, owner.action.number);
  } else if (named && named.at !== null && state.selected() === named.session) {
    const action = state.actions(named.session).find((held) => held.action.sequence === named.at);
    if (action) {
      state.setPlayhead(action.action.number);
    } else {
      // A sequence not held falls back to the first session, Latest on.
      state.select(list.sessions[0].id);
      if (list.sessions[0].id !== named.session) {
        await loadSelected();
      }
    }
  }
  renderList();
  renderSelected();
  return list.newest;
}

async function choose(id) {
  if (id !== state.selected()) {
    state.select(id);
  }
  renderList();
  renderSelected();
  try {
    await loadSelected();
    if (state.selected() === id) {
      renderSelected();
      renderList();
    }
  } catch (error) {
    failed(error);
  }
}

function fail(error) {
  if (refusal(error)) {
    say("The view refused the token. Open the address the program printed, with its #token.");
  } else {
    say("The view did not answer: " + error.message);
  }
}

// A request after the page loaded failed. A refusal (401 or 403) stops the page: the stream
// stops, nothing is asked again, the live marker reads offline. After it, nothing more is said.
function failed(error) {
  if (refused) {
    return;
  }
  if (refusal(error)) {
    if (stream) {
      stream.stop();
    }
    setLive("offline", null);
    refused = true;
  }
  fail(error);
}

function onEvent(event) {
  updated();
  if (event.name === "entry") {
    const entry = event.data;
    if (state.addEntry(entry) && entry.session === state.selected()) {
      renderSelected();
    }
  } else if (event.name === "session") {
    state.putSession(event.data);
    renderList();
    if (event.data.id === state.selected()) {
      renderSelected();
    }
  } else if (event.name === "reset") {
    state.forget();
    loadAll().catch(failed);
  }
}

async function start() {
  token = takeToken();
  wanted = readSelection(window.location.hash);
  nodes.latest.addEventListener("click", onLatest);
  document.addEventListener("keydown", onKey);
  if (!token) {
    setLive("offline", null);
    say("No view token. Open the address the program printed, with its #token.");
    return;
  }
  let newest;
  try {
    newest = await loadAll();
  } catch (error) {
    setLive("offline", null);
    fail(error);
    return;
  }
  say("");
  updated();
  stream = follow(token, newest, {
    open() {
      say("");
      updated();
      loadAll().catch(failed);
    },
    alive() {
      updated();
    },
    event: onEvent,
    closed() {
      setLive("reconnecting", null);
    },
    refused: failed,
  });
  setInterval(() => tick(nodes.list), 1000);
}

start();
