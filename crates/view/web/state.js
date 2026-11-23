// What the page holds: the sessions in list order, the entries of the sessions it loaded, and
// the selection: the selected session, the playhead over its actions and the selected event; and
// what the page line and the panel "Now" read at the playhead. No DOM here.

const state = {
  newest: 0,
  order: [],
  sessions: new Map(),
  entries: new Map(),
  numbers: new Map(),
  // Per loaded session, what its last detail said of dropped entries: {dropped, log_dropped}.
  drops: new Map(),
  // Per session whose detail has been asked for and has not answered: {asks, entries}, the
  // stream entries read meanwhile, which setDetail merges. Only a selected session's detail is
  // asked for, so nothing is held for a session nobody selected.
  awaited: new Map(),
  selected: null,
  // Latest on: the playhead follows the newest action. Off: it stays on the action entry whose
  // number is `playhead`.
  latest: true,
  playhead: null,
  // The selected event, by its entry's number (D23), or null. While one is selected the
  // playhead is on its action and Latest is off.
  event: null,
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
  for (const id of [...state.awaited.keys()]) {
    if (!state.sessions.has(id)) {
      state.awaited.delete(id);
    }
  }
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

// Selects a session; its playhead starts with Latest on, and no event is selected.
export function select(id) {
  state.selected = id;
  state.latest = true;
  state.playhead = null;
  state.event = null;
}

export function selected() {
  return state.selected;
}

export function latest() {
  return state.latest;
}

// Latest on: the playhead moves to the newest action and follows it; no event is selected.
export function followLatest() {
  state.latest = true;
  state.playhead = null;
  state.event = null;
}

// The playhead on the action whose action entry has number `number`; Latest off, no event
// selected.
export function setPlayhead(number) {
  state.latest = false;
  state.playhead = number;
  state.event = null;
}

// Selects the event entry `number` of the action whose action entry has number `action`: the
// playhead moves to that action, Latest off.
export function selectEvent(number, action) {
  state.latest = false;
  state.playhead = action;
  state.event = number;
}

// The number of the selected event entry, or null.
export function selectedEvent() {
  return state.event;
}

// The selected event's entry among `list` (actions() of the selected session), or null when no
// event is selected or its entry is not held there.
export function eventEntry(list) {
  if (state.event === null) {
    return null;
  }
  for (const action of list) {
    const found = action.events.find((entry) => entry.number === state.event);
    if (found) {
      return found;
    }
  }
  return null;
}

// The index of the playhead in `list` (actions() of the selected session), or -1 for none.
// With Latest on, the newest action. With Latest off, when the action at the playhead is no
// longer held, the playhead moves once to the oldest action that is held (the nearest to where
// it was) and Latest stays off; with no action held at all there is no playhead.
export function playheadIndex(list) {
  if (list.length === 0) {
    return -1;
  }
  if (state.latest) {
    return list.length - 1;
  }
  const at = list.findIndex((action) => action.action.number === state.playhead);
  if (at >= 0) {
    return at;
  }
  state.playhead = list[0].action.number;
  return 0;
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

// The detail of the session `id` is asked for: until it answers, the stream entries of that
// session are held (addEntry). Returns the ask, for giveUp.
export function expectDetail(id) {
  let ask = state.awaited.get(id);
  if (!ask) {
    ask = { asks: 0, entries: [] };
    state.awaited.set(id, ask);
  }
  ask.asks += 1;
  return { id, ask };
}

// A detail asked for by expectDetail did not answer: what was held for it is given up once no
// other ask of that session is waiting.
export function giveUp(asked) {
  asked.ask.asks -= 1;
  if (asked.ask.asks <= 0 && state.awaited.get(asked.id) === asked.ask) {
    state.awaited.delete(asked.id);
  }
}

// The detail of /api/sessions/<id>: its summary and its entries. Entries already held above the
// detail's newest are kept, so that a stream entry read while the detail loaded is not lost:
// those of a loaded session and those held while its detail was asked for, each number once.
export function setDetail(detail) {
  const id = detail.session.id;
  putSession(detail.session);
  const held = (state.entries.get(id) || []).concat(state.awaited.has(id) ? state.awaited.get(id).entries : []);
  state.awaited.delete(id);
  const fresh = detail.entries.slice();
  const last = fresh.length ? fresh[fresh.length - 1].number : 0;
  const numbers = new Set(fresh.map((entry) => entry.number));
  let sort = false;
  for (const entry of held) {
    if (entry.number > last && !numbers.has(entry.number)) {
      sort = sort || (fresh.length > 0 && fresh[fresh.length - 1].number > entry.number);
      numbers.add(entry.number);
      fresh.push(entry);
    }
  }
  if (sort) {
    fresh.sort((a, b) => a.number - b.number);
  }
  state.entries.set(id, fresh);
  state.numbers.set(id, numbers);
  state.drops.set(id, { dropped: detail.dropped || 0, log_dropped: detail.log_dropped === true });
  for (const entry of fresh) {
    setNewest(entry.number);
  }
  // A selected event whose entry this load no longer holds is cleared.
  if (state.event !== null && id === state.selected && !numbers.has(state.event)) {
    state.event = null;
  }
}

// What the last detail of the session said of dropped entries.
export function drops(id) {
  return state.drops.get(id) || { dropped: 0, log_dropped: false };
}

export function loaded(id) {
  return state.entries.has(id);
}

// A stream entry. False when its session is not loaded or its number is held already; an entry
// of a session whose detail is asked for and has not answered is held for setDetail.
export function addEntry(entry) {
  setNewest(entry.number);
  const numbers = state.numbers.get(entry.session);
  if (!numbers) {
    const ask = state.awaited.get(entry.session);
    if (ask) {
      ask.entries.push(entry);
    }
    return false;
  }
  if (numbers.has(entry.number)) {
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

// Everything loaded is dropped, and what was held for a detail not yet answered; the list and
// the selected session are loaded again after it.
export function forget() {
  state.entries = new Map();
  state.numbers = new Map();
  state.drops = new Map();
  state.awaited = new Map();
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

// The length a `fill` typed into each control of the document at action `at` of `list`, by
// handle: the `value` of the newest surf.agent.FillRequested entry of that handle ("12 bytes",
// docs/view-stream.md: the event holds the length, never the text), from `at` back to the
// action whose answer's `changed` member says it loaded that document. `before` leaves out the
// events of the action at `at` itself: the lengths before it.
export function fillLengths(list, at, before = false) {
  const lengths = new Map();
  for (let index = at; index >= 0; index -= 1) {
    const held = list[index];
    if (!(before && index === at)) {
      for (let event = held.events.length - 1; event >= 0; event -= 1) {
        const entry = held.events[event];
        const fields = entry.fields;
        if (entry.name !== "surf.agent.FillRequested" || !fields || typeof fields.handle !== "string") {
          continue;
        }
        if (!lengths.has(fields.handle) && typeof fields.value === "string") {
          lengths.set(fields.handle, fields.value);
        }
      }
    }
    const changed = held.answer ? held.answer.changed : null;
    if (changed && changed.kind === "page") {
      break;
    }
  }
  return lengths;
}

// What can be done at the playhead of the session: {now, lengths} with the `now` member of its
// action's answer entry (null when the answer has none) and fillLengths() there, the word
// "running" when that answer is not held, or null when the session has no action held.
export function nowAt(id) {
  const list = actions(id);
  const at = playheadIndex(list);
  if (at < 0) {
    return null;
  }
  const answer = list[at].answer;
  if (!answer) {
    return "running";
  }
  return { now: answer.now === undefined ? null : answer.now, lengths: fillLengths(list, at) };
}

// What the action at the playhead changed: {command, handle, changed, lengths, before} with its
// command, the handle its path line names (or null), the `changed` member of its answer entry
// (null when the answer has none), and fillLengths() after and before it; the word "running"
// when that answer is not held, or null when the session has no action held.
export function changedAt(id) {
  const list = actions(id);
  const at = playheadIndex(list);
  if (at < 0) {
    return null;
  }
  const held = list[at];
  if (!held.answer) {
    return "running";
  }
  const path = held.answer.path;
  const step = path && Array.isArray(path.actions) ? path.actions.find((one) => one && one.sequence === held.action.sequence) : null;
  return {
    command: typeof held.action.command === "string" ? held.action.command : null,
    handle: step && typeof step.handle === "string" ? step.handle : null,
    changed: held.answer.changed === undefined ? null : held.answer.changed,
    lengths: fillLengths(list, at),
    before: fillLengths(list, at, true),
  };
}
