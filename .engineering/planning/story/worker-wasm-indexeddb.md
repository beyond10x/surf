---
format: aep.planning-md/3
id: story:worker-wasm-indexeddb
kind: story
status: draft
title: A Service Worker host, WebAssembly and IndexedDB
summary: Needed only by a page that runs its backend inside a Service Worker as a WebAssembly module with state in IndexedDB; not on the path of a consumer whose backend is an HTTP server.
relations:
- decomposes: epic:app-journeys-in-surf
- depends_on: story:scripts-fetch-from-script
revision: 1
---
## Context

The consumer's study names these three for one of its options only (a backend compiled to
WebAssembly that answers the application's `fetch` calls from inside a Service Worker, with
state in IndexedDB); the consumer chose a backend that is an HTTP server, so these are off its
path. surf has none of the three. swell's nineteen slices hold no WebAssembly
(`docs/design/engine.md:3011-3029 @ 0.9.0`).

## What it holds

- A worker global and a second agent per registration, the `fetch` event between a page's
  request and the network, the registration's life.
- WebAssembly: an engine of its own beside swell, under the dependency rule of `AGENTS.md`
  ("Goals").
- IndexedDB: a store per session and origin.

## Waits for

`story:scripts-fetch-from-script`; an architecture decision of the owner. Nothing orders it.
