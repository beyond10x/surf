// The page: loads /api/sessions, selects a session (the one the address's fragment names, or the
// first), loads it, follows /events, and keeps the live marker (live and the time of the last
// update, or reconnecting). The selection is written into the fragment,
// #session=<id>&at=<sequence>, with history.replaceState, and read at load: never the query,
// which the route table answers 404.

import { address, byId, text } from "./dom.js";
import * as state from "./state.js";
import { api, follow, Refused, takeToken } from "./stream.js";
import { renderSessions, tick } from "./sessions.js";
import { renderPage } from "./page.js";
import { renderControls, renderTimeline, resetTimeline } from "./timeline.js";

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
};

let token = null;
// The selection the address named at load, {session, at}; applied by the first load only.
let wanted = null;
// The fragment last written.
let written = null;

function clock(ms) {
  return new Date(ms).toLocaleTimeString([], { hour12: false });
}

function setLive(kind, at) {
  nodes.live.className = "live is-" + kind;
  text(nodes.liveWord, kind === "live" ? "live" : kind);
  text(nodes.liveTime, at ? clock(at) : "");
}

function updated() {
  setLive("live", Date.now());
}

function say(message) {
  text(nodes.message, message);
  if (message) {
    nodes.message.removeAttribute("hidden");
  } else {
    nodes.message.setAttribute("hidden", "");
  }
}

// `#session=<id>&at=<sequence>` read from `hash`; null when it names no session.
function readSelection(hash) {
  const result = { session: null, at: null };
  for (const pair of hash.replace(/^#/, "").split("&")) {
    const equals = pair.indexOf("=");
    const name = equals < 0 ? pair : pair.slice(0, equals);
    const value = equals < 0 ? "" : pair.slice(equals + 1);
    if (name === "session" && /^[0-9a-f-]{36}$/.test(value)) {
      result.session = value;
    } else if (name === "at" && /^[0-9]{1,15}$/.test(value)) {
      result.at = Number(value);
    }
  }
  return result.session === null ? null : result;
}

// The selection into the fragment: `at` only while Latest is off and the selected action has a
// sequence.
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
  renderPage(nodes.page, state.pageAt(id));
  renderTimeline(nodes.timeline, list, at, state.drops(id), state.latest(), moveTo);
  renderControls(nodes.latest, nodes.newer, state.latest(), at < 0 ? 0 : list.length - 1 - at);
  writeSelection();
}

// The playhead onto the action whose action entry has `number`; Latest off.
function moveTo(number) {
  state.setPlayhead(number);
  renderSelected();
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
  const detail = await api(token, "/api/sessions/" + id);
  state.setDetail(detail);
}

async function loadAll() {
  const list = await api(token, "/api/sessions");
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
  if (named && named.at !== null && state.selected() === named.session) {
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
    fail(error);
  }
}

function fail(error) {
  if (error instanceof Refused && error.status === 401) {
    say("The view refused the token. Open the address the program printed, with its #token.");
  } else {
    say("The view did not answer: " + error.message);
  }
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
    loadAll().catch(fail);
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
  follow(token, newest, {
    open() {
      say("");
      updated();
      loadAll().catch(fail);
    },
    alive() {
      updated();
    },
    event: onEvent,
    closed() {
      setLive("reconnecting", null);
    },
  });
  setInterval(() => tick(nodes.list), 1000);
}

start();
