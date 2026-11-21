// What the page holds: the sessions in list order, the entries of the sessions it loaded, and
// the selection. No DOM here.

const state = {
  newest: 0,
  order: [],
  sessions: new Map(),
  entries: new Map(),
  numbers: new Map(),
  selected: null,
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
    state.selected = null;
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

export function select(id) {
  state.selected = id;
}

export function selected() {
  return state.selected;
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
  for (const entry of fresh) {
    setNewest(entry.number);
  }
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

// The page line of the session's newest answer entry, or null when none is held.
export function pageLine(id) {
  const list = entries(id);
  for (let at = list.length - 1; at >= 0; at -= 1) {
    if (list[at].kind === "answer") {
      return list[at].page;
    }
  }
  return null;
}
