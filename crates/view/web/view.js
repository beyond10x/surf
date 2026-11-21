// The page: loads /api/sessions, selects the first session, loads it, follows /events, and keeps
// the live marker (live and the time of the last update, or reconnecting).

import { attribute, byId, text } from "./dom.js";
import * as state from "./state.js";
import { api, follow, Refused, takeToken } from "./stream.js";
import { renderSessions, tick } from "./sessions.js";
import { renderPage } from "./page.js";
import { renderTimeline, resetTimeline } from "./timeline.js";

const nodes = {
  live: byId("live"),
  liveWord: byId("live-word"),
  liveTime: byId("live-time"),
  list: byId("session-list"),
  heading: byId("session-heading"),
  facts: byId("session-facts"),
  page: byId("page-line"),
  timeline: byId("actions"),
  message: byId("message"),
};

let token = null;

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
  attribute(nodes.message, "hidden", message ? null : "");
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
    return;
  }
  text(nodes.heading, summary.title || summary.address || "session " + summary.id.slice(0, 8));
  const facts = [summary.state, "session " + summary.id];
  if (summary.opened_at !== null) {
    facts.push("opened " + clock(summary.opened_at));
  }
  if (summary.ended_at !== null) {
    facts.push("ended " + clock(summary.ended_at));
  }
  text(nodes.facts, facts.join(" · "));
  renderPage(nodes.page, state.pageLine(id));
  renderTimeline(nodes.timeline, id, state.actions(id));
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
  if (state.selected() === null && list.sessions.length > 0) {
    state.select(list.sessions[0].id);
  }
  await loadSelected();
  renderList();
  renderSelected();
  return list.newest;
}

async function choose(id) {
  state.select(id);
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
