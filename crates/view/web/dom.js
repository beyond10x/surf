// The only module that creates elements. Every value is written with textContent, never as
// markup, and no attribute that starts with "on" is ever set.

// options.text is written with textContent; options.address the same way, in pieces that wrap
// at the address's boundaries (see address below).
export function element(tag, options = {}, children = []) {
  const node = document.createElement(tag);
  if (options.className) {
    node.className = options.className;
  }
  if (options.text !== undefined && options.text !== null) {
    node.textContent = String(options.text);
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
// span written with textContent, a <wbr> between them. Copying it gives the value unchanged.
export function address(node, value) {
  node.replaceChildren();
  const text = value === null || value === undefined ? "" : String(value);
  pieces(text).forEach((piece, at) => {
    if (at > 0) {
      node.append(document.createElement("wbr"));
    }
    const span = document.createElement("span");
    span.textContent = piece;
    node.append(span);
  });
  return node;
}

export function attribute(node, name, value) {
  if (/^on/i.test(name)) {
    throw new Error("an event attribute is never set");
  }
  if (value === null || value === undefined || value === false) {
    node.removeAttribute(name);
  } else {
    node.setAttribute(name, String(value));
  }
}

export function text(node, value) {
  node.textContent = value === null || value === undefined ? "" : String(value);
}

export function clear(node) {
  node.replaceChildren();
}

export function byId(id) {
  return document.getElementById(id);
}
