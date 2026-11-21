// The page line at the playhead: the page line of the selected action's answer entry (status,
// media type, address, title; "failed <reason> <address>"; or "none"), or "running" while that
// action's answer is not held.

import { clear, element } from "./dom.js";

function part(name, value, className, isAddress) {
  return element("div", { className: "part " + (className || "") }, [
    element("dt", { text: name }),
    element("dd", isAddress ? { address: value } : { text: value }),
  ]);
}

// `page` is a page line of docs/view-stream.md, the word "running", or null.
export function renderPage(node, page) {
  clear(node);
  if (page === "running") {
    node.append(element("p", { className: "line running", text: "running" }));
    return;
  }
  if (!page || page.kind === "none") {
    node.append(element("p", { className: "line none", text: "none" }));
    return;
  }
  if (page.kind === "failed") {
    node.append(
      element("p", { className: "line failed" }, [
        element("span", { className: "word", text: "failed" }),
        element("span", { className: "reason", text: page.reason }),
        element("span", { className: "address", address: page.address }),
      ]),
    );
    return;
  }
  node.append(
    element("dl", { className: "line loaded" }, [
      part("status", page.status === null || page.status === undefined ? "?" : String(page.status), "status"),
      part("media type", page.media_type),
      part("address", page.address, "address", true),
      part("title", page.title === null ? "no title" : page.title, page.title === null ? "absent" : ""),
    ]),
  );
}
