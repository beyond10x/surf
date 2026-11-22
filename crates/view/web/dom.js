// The only module that creates elements and the only one that sets an attribute or a property of
// an element. A value from a session is set only as textContent or as the value of an attribute
// of ATTRIBUTES below, never as markup; attribute() refuses any other name with an error, so
// href, src, style, srcdoc and on... cannot be set from anywhere. The one other property set
// here is scrollLeft (scroll), a number.
//
// Every text goes through written() below: a bidirectional control (U+202A to U+202E, U+2066 to
// U+2069) in a value is shown as a mark that names it, "U+202E" in a span of class bidi-mark,
// so a value cannot reorder what a person reads. In a title attribute the same name stands in
// its place, as text. Other format characters (U+200E, U+200F, joiners, a soft hyphen) stay.

// Every attribute name the page sets.
const ATTRIBUTES = new Set([
  "class",
  "title",
  "type",
  "hidden",
  "aria-current",
  "aria-pressed",
  "data-at",
  "data-domain",
  "data-number",
]);

// Whether the code point `code` is one of the nine bidirectional embedding, override and
// isolate controls.
function bidiControl(code) {
  return (code >= 0x202a && code <= 0x202e) || (code >= 0x2066 && code <= 0x2069);
}

// The name of a bidirectional control: "U+202E".
function bidiName(code) {
  return "U+" + code.toString(16).toUpperCase();
}

// Writes `value` into `node` as its only content: text nodes, and for each bidirectional control
// a span of class bidi-mark whose textContent names it. A value without one is set as
// textContent, as before.
function written(node, value) {
  const held = value === null || value === undefined ? "" : String(value);
  let run = "";
  const parts = [];
  for (const character of held) {
    const code = character.codePointAt(0);
    if (bidiControl(code)) {
      if (run !== "") {
        parts.push(document.createTextNode(run));
        run = "";
      }
      const mark = document.createElement("span");
      attribute(mark, "class", "bidi-mark");
      mark.textContent = bidiName(code);
      parts.push(mark);
    } else {
      run += character;
    }
  }
  if (parts.length === 0) {
    node.textContent = held;
    return;
  }
  if (run !== "") {
    parts.push(document.createTextNode(run));
  }
  node.replaceChildren(...parts);
}

// options.className is the class attribute; options.text is written as text (written above);
// options.address the same way, in pieces that wrap at the address's boundaries (see address
// below); options.attributes through attribute().
export function element(tag, options = {}, children = []) {
  const node = document.createElement(tag);
  if (options.className) {
    attribute(node, "class", options.className);
  }
  if (options.text !== undefined && options.text !== null) {
    written(node, options.text);
  } else if (options.address !== undefined && options.address !== null) {
    address(node, options.address);
  }
  for (const [name, value] of Object.entries(options.attributes || {})) {
    attribute(node, name, value);
  }
  for (const child of children) {
    if (child) {
      node.append(child);
    }
  }
  return node;
}

// The pieces of an address that a line may break between: after each "/" whose next character
// is not "/" (so the double slash after the scheme stays whole), and before "?", "#" and "&".
function pieces(value) {
  const result = [];
  let current = "";
  for (let at = 0; at < value.length; at += 1) {
    const c = value[at];
    if ((c === "?" || c === "#" || c === "&") && current !== "") {
      result.push(current);
      current = "";
    }
    current += c;
    if (c === "/" && value[at + 1] !== "/") {
      result.push(current);
      current = "";
    }
  }
  if (current !== "") {
    result.push(current);
  }
  return result;
}

// Writes `value` into `node` as text that wraps at the address's own boundaries: each piece a
// span written as text (written above), a <wbr> between them. Copying it gives the value
// unchanged, but for a bidirectional control, which copies as its name.
export function address(node, value) {
  node.replaceChildren();
  const text = value === null || value === undefined ? "" : String(value);
  pieces(text).forEach((piece, at) => {
    if (at > 0) {
      node.append(document.createElement("wbr"));
    }
    const span = document.createElement("span");
    written(span, piece);
    node.append(span);
  });
  return node;
}

// Sets the attribute `name`, one of ATTRIBUTES, to `value` as text; null, undefined or false
// removes it. Any other name is refused with an error.
export function attribute(node, name, value) {
  if (!ATTRIBUTES.has(name)) {
    throw new Error("the page sets no attribute " + JSON.stringify(String(name)));
  }
  if (value === null || value === undefined || value === false) {
    node.removeAttribute(name);
  } else if (name === "title") {
    node.setAttribute(name, titled(String(value)));
  } else {
    node.setAttribute(name, String(value));
  }
}

// `value` with each bidirectional control replaced by its name, for a title attribute.
function titled(value) {
  let result = "";
  for (const character of value) {
    const code = character.codePointAt(0);
    result += bidiControl(code) ? bidiName(code) : character;
  }
  return result;
}

// Sets the content of `node` to `value` as text (written above).
export function text(node, value) {
  written(node, value);
}

// Scrolls `node` sideways to `left` pixels.
export function scroll(node, left) {
  node.scrollLeft = Number(left);
}

export function clear(node) {
  node.replaceChildren();
}

export function byId(id) {
  return document.getElementById(id);
}
