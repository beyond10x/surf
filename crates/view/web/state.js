// What the page holds: the sessions in list order, the entries of the sessions it loaded, and
// the selection: the selected session and the playhead over its actions. No DOM here.

const state = {
  newest: 0,
  order: [],
  sessions: new Map(),
  entries: new Map(),
  numbers: new Map(),
  // Per loaded session, what its last detail said of dropped entries: {dropped, log_dropped}.
  drops: new Map(),
  selected: null,
  // Latest on: the playhead follows the newest action. Off: it stays on the action entry whose
  // number is `playhead`.
  latest: true,
  playhead: null,
};

export function newest() {
  return state.newest;
}

export function setNewest(number) {
  if (number > state.newest) {
    state.newest = number;
  }
}

// Whether `held` is later than `incoming`: a summary has no number of its own, so a list read
// before a session event must not undo that event. Later means a later last action, or ended
// where the other is not.
function later(held, incoming) {
  if (!held) {
    return false;
  }
  if ((held.last_action_at || 0) !== (incoming.last_action_at || 0)) {
    return (held.last_action_at || 0) > (incoming.last_action_at || 0);
  }
  return held.ended_at !== null && incoming.ended_at === null;
}

// The session list of /api/sessions: replaces the list, keeps the selection when it is listed.
export function setSessions(list) {
  const held = state.sessions;
  state.order = list.sessions.map((summary) => summary.id);
  state.sessions = new Map(
    list.sessions.map((summary) => {
      const kept = held.get(summary.id);
      return [summary.id, later(kept, summary) ? kept : summary];
    }),
  );
  setNewest(list.newest);
  if (state.selected !== null && !state.sessions.has(state.selected)) {
    select(null);
  }
}

// A session summary from an event or a detail: replaces the one held unless that one is later,
// or lists a new session last.
export function putSession(summary) {
  const held = state.sessions.get(summary.id);
  if (!held) {
    state.order.push(summary.id);
  }
  if (!later(held, summary)) {
    state.sessions.set(summary.id, summary);
  }
}

export function sessions() {
  return state.order.map((id) => state.sessions.get(id));
}

export function session(id) {
  return state.sessions.get(id) || null;
}

// Selects a session; its playhead starts with Latest on.
export function select(id) {
  state.selected = id;
  state.latest = true;
  state.playhead = null;
}

export function selected() {
  return state.selected;
}

export function latest() {
  return state.latest;
}

// Latest on: the playhead moves to the newest action and follows it.
export function followLatest() {
  state.latest = true;
  state.playhead = null;
}

// The playhead on the action whose action entry has number `number`; Latest off.
export function setPlayhead(number) {
  state.latest = false;
  state.playhead = number;
}

// The index of the playhead in `list` (actions() of the selected session), or -1 for none.
export function playheadIndex(list) {
  if (list.length === 0) {
    return -1;
  }
  if (!state.latest) {
    const at = list.findIndex((action) => action.action.number === state.playhead);
    if (at >= 0) {
      return at;
    }
  }
  return list.length - 1;
}

// Moves the playhead `step` actions (-1 or 1) and turns Latest off; at either end it does
// nothing. Whether it moved.
export function stepPlayhead(list, step) {
  const at = playheadIndex(list);
  const to = at + step;
  if (at < 0 || to < 0 || to >= list.length) {
    return false;
  }
  setPlayhead(list[to].action.number);
  return true;
}

// The detail of /api/sessions/<id>: its summary and its entries. Entries already held above the
// detail's newest are kept, so that a stream entry read while the detail loaded is not lost.
export function setDetail(detail) {
  const id = detail.session.id;
  putSession(detail.session);
  const held = state.entries.get(id) || [];
  const fresh = detail.entries.slice();
  const last = fresh.length ? fresh[fresh.length - 1].number : 0;
  for (const entry of held) {
    if (entry.number > last) {
      fresh.push(entry);
    }
  }
  state.entries.set(id, fresh);
  state.numbers.set(id, new Set(fresh.map((entry) => entry.number)));
  state.drops.set(id, { dropped: detail.dropped || 0, log_dropped: detail.log_dropped === true });
  for (const entry of fresh) {
    setNewest(entry.number);
  }
}

// What the last detail of the session said of dropped entries.
export function drops(id) {
  return state.drops.get(id) || { dropped: 0, log_dropped: false };
}

export function loaded(id) {
  return state.entries.has(id);
}

// A stream entry. False when its session is not loaded or its number is held already.
export function addEntry(entry) {
  setNewest(entry.number);
  const numbers = state.numbers.get(entry.session);
  if (!numbers || numbers.has(entry.number)) {
    return false;
  }
  numbers.add(entry.number);
  const list = state.entries.get(entry.session);
  list.push(entry);
  if (list.length > 1 && list[list.length - 2].number > entry.number) {
    list.sort((a, b) => a.number - b.number);
  }
  return true;
}

export function entries(id) {
  return state.entries.get(id) || [];
}

// Everything loaded is dropped; the list and the selected session are loaded again after it.
export function forget() {
  state.entries = new Map();
  state.numbers = new Map();
  state.drops = new Map();
}

// The entries of a session as actions: each action entry with the events after it and the
// answer entry that ends it (decision D19).
export function actions(id) {
  const result = [];
  let current = null;
  for (const entry of entries(id)) {
    if (entry.kind === "action") {
      current = { action: entry, events: [], answer: null };
      result.push(current);
    } else if (current && entry.kind === "event") {
      current.events.push(entry);
    } else if (current && entry.kind === "answer") {
      current.answer = entry;
      current = null;
    }
  }
  return result;
}

// The page line at the playhead of the session: its action's answer entry's page line, the
// word "running" when that answer is not held, or null when the session has no action held.
export function pageAt(id) {
  const list = actions(id);
  const at = playheadIndex(list);
  if (at < 0) {
    return null;
  }
  const answer = list[at].answer;
  return answer ? answer.page : "running";
}
