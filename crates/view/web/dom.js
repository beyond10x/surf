// The only module that creates elements. Every value is written with textContent, never as
// markup, and no attribute that starts with "on" is ever set.

export function element(tag, options = {}, children = []) {
  const node = document.createElement(tag);
  if (options.className) {
    node.className = options.className;
  }
  if (options.text !== undefined && options.text !== null) {
    node.textContent = String(options.text);
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
