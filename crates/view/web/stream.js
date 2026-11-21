// The token, the API requests and the event stream of docs/view-stream.md.
//
// The token arrives in the address's fragment, is kept in sessionStorage and removed from the
// address. /events is read with fetch and a stream reader, because EventSource sends no
// Authorization header.

const KEY = "surf-view-token";
const FIRST_DELAY = 500;
const LAST_DELAY = 5000;

// Moves a token from the fragment into sessionStorage and removes the fragment; returns the
// token kept, or null.
export function takeToken() {
  const fragment = window.location.hash.replace(/^#/, "");
  if (/^[0-9a-f]{64}$/.test(fragment)) {
    window.sessionStorage.setItem(KEY, fragment);
  }
  if (window.location.hash) {
    window.history.replaceState(null, "", window.location.pathname);
  }
  return window.sessionStorage.getItem(KEY);
}

export class Refused extends Error {
  constructor(status) {
    super("the view answered " + status);
    this.status = status;
  }
}

// GET of a route under /api/, with the token; the parsed JSON body.
export async function api(token, path) {
  const answer = await fetch(path, {
    headers: { Authorization: "Bearer " + token },
    cache: "no-store",
    credentials: "omit",
  });
  if (answer.status !== 200) {
    throw new Refused(answer.status);
  }
  return answer.json();
}

// Reads one block of the event stream: the fields of each line until the empty line.
function parseBlock(lines) {
  const event = { id: null, name: "message", data: null, comment: false };
  for (const line of lines) {
    if (line.startsWith(":")) {
      event.comment = true;
      continue;
    }
    const colon = line.indexOf(":");
    const field = colon < 0 ? line : line.slice(0, colon);
    let value = colon < 0 ? "" : line.slice(colon + 1);
    if (value.startsWith(" ")) {
      value = value.slice(1);
    }
    if (field === "id") {
      event.id = value;
    } else if (field === "event") {
      event.name = value;
    } else if (field === "data") {
      event.data = event.data === null ? value : event.data + "\n" + value;
    }
  }
  return event;
}

// Follows /events from `from` until stopped. Calls handlers.open() when a stream answers (the
// page loads the session list and its selected session again then: session events are live
// only, so one sent before this stream opened is not replayed), handlers.event({id, name,
// data}) for each entry, session and reset event, handlers.alive() on a comment line, and
// handlers.closed() when a stream ends. A new stream follows after a delay that grows to 5 s,
// with Last-Event-ID set to the last id read.
export function follow(token, from, handlers) {
  let last = from;
  let delay = FIRST_DELAY;
  let stopped = false;
  let controller = null;

  async function once() {
    controller = new AbortController();
    const answer = await fetch("/events", {
      headers: { Authorization: "Bearer " + token, "Last-Event-ID": String(last) },
      cache: "no-store",
      credentials: "omit",
      signal: controller.signal,
    });
    if (answer.status !== 200 || !answer.body) {
      throw new Refused(answer.status);
    }
    handlers.open();
    const reader = answer.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";
    let lines = [];
    for (;;) {
      const { value, done } = await reader.read();
      if (done) {
        return;
      }
      buffer += decoder.decode(value, { stream: true });
      let end = buffer.indexOf("\n");
      while (end >= 0) {
        const line = buffer.slice(0, end).replace(/\r$/, "");
        buffer = buffer.slice(end + 1);
        end = buffer.indexOf("\n");
        if (line !== "") {
          lines.push(line);
          continue;
        }
        const block = parseBlock(lines);
        lines = [];
        if (block.comment && block.data === null) {
          handlers.alive();
          continue;
        }
        if (block.data === null) {
          continue;
        }
        if (block.id !== null && /^[0-9]+$/.test(block.id)) {
          last = Number(block.id);
        }
        delay = FIRST_DELAY;
        handlers.event({ id: last, name: block.name, data: JSON.parse(block.data) });
      }
    }
  }

  async function run() {
    let first = true;
    while (!stopped) {
      if (!first) {
        await new Promise((resolve) => setTimeout(resolve, delay));
        delay = Math.min(delay * 2, LAST_DELAY);
        if (stopped) {
          return;
        }
      }
      first = false;
      try {
        await once();
        handlers.closed(null);
      } catch (error) {
        handlers.closed(error);
      }
    }
  }

  run();
  return {
    stop() {
      stopped = true;
      if (controller) {
        controller.abort();
      }
    },
    last() {
      return last;
    },
  };
}
