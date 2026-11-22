# The `surf` command

`surf <url>` loads one address: it opens a window and a tab, navigates the tab to the address,
fetches it over `http:` and runs the page load to its end. Each release has a pre-built binary
for Linux and macOS (see the installation steps). A run on 2026-11-20 against a
local server:

```console
$ surf http://127.0.0.1:41977/index.html
navigation f6eef140-16d8-458c-bdfb-8999c52d7b31 Committed http://127.0.0.1:41977/index.html
request f6eef140-16d8-458c-bdfb-8999c52d7b31 Responded http://127.0.0.1:41977/index.html
document 9ed741bc-a98b-428b-9585-c90c379a8e4d http://127.0.0.1:41977/index.html
$ surf http://127.0.0.1:1/
surf: http://127.0.0.1:1/: unreachable: connecting to 127.0.0.1:1 failed: Connection refused (os error 111)
navigation 40d80baa-c7b8-4c3e-b9b9-ef8938cf6882 Failed http://127.0.0.1:1/
request 40d80baa-c7b8-4c3e-b9b9-ef8938cf6882 Failed http://127.0.0.1:1/
fetch-failed 40d80baa-c7b8-4c3e-b9b9-ef8938cf6882 Unreachable http://127.0.0.1:1/
```

## Output

The `surf:` line went to standard error. Standard output holds only these lines, one per row:

```text
navigation <navigation-id> <state> <address>
request <request-id> <state> <address>
document <document-id> <address>
fetch-failed <request-id> <reason> <address>
```

- Every navigation of the `Navigations` view comes first, then every request of the `Requests`
  view, every document of the `Documents` view, and one line per published `surf.net.FetchFailed`.
  Each view's lines are in the order the view returns them.
- Fields are separated by one space. An identity is a UUID in lower-case 8-4-4-4-12 form. A
  navigation and its request have the same identity.
- `<state>` is `Started`, `Committed`, `Failed` or `Cancelled` for a navigation and `Pending`,
  `Responded` or `Failed` for a request. `<reason>` is `Unreachable`, `UnsupportedScheme`,
  `MalformedResponse`, `TooManyRedirects` or `ForbiddenDestination`. A document has one
  state, and its line has no state field.
- After redirects, the `navigation` and `document` lines hold the address of the last response,
  and the `request` and `fetch-failed` lines the address asked for.
- `<address>` is the rest of the line. Printable ASCII (U+0020 to U+007E) other than the
  backslash is written as itself. The backslash and every other character, control characters,
  line feeds, Unicode line and paragraph separators and bidirectional controls included, are
  written as `\x` and two lower-case hexadecimal digits per byte of their UTF-8 encoding. An
  address on a `surf:` line on standard error is written the same way.

A document is parsed for every response with the media type `text/html`, whatever its status.

## The page as text: `--as`

`surf <url> --as tree|markdown` (also `--as=<form>`, before or after the address) prints, after
the rows, the page as the outline of its tree or as Markdown, in the `text` block of the agent
answer (`docs/agent-answer.md`, section "Text"):

```text
note text: no-css
text <form> <n> of <n> lines from 1, <b> of <b> bytes
> <line>
>
```

- `note text: no-css`: the text is computed from the tree without CSS, so text a style sheet
  hides is shown. Then the header and all `<n>` lines, each `>` alone for an empty line or `> `
  and the line; `<b>` counts them with their prefix and line feed. There is no bound and no
  `more` line: `surf` runs once, and the tree is bounded by the parser's limits.
- Every line of page text starts with `>`, and no row does, so a line a page wrote cannot pass
  for a row. A body line holds printable ASCII and UTF-8 characters; control, invisible,
  bidirectional and private-use characters are written as `\x` and two hexadecimal digits per
  byte (rule 2 of `docs/design/page-as-text.md`, section 3).
- With no HTML document (a failed load, a response that is not `text/html`) the last line is
  `note text: no-document` and there is no block. The exit status is the one without `--as`.

The page of `docs/design/page-as-text.md` section 1, `surf http://shop.example/ --as markdown`,
after its rows:

```text
note text: no-css
text markdown 14 of 14 lines from 1, 263 of 263 bytes
> # Fruit shop
>
> Fresh fruit, *picked today*. See [our offers](</offers>) [@l1].
>
> - Apples
> - Pears
>
> | Fruit | Price | Stock |
> | --- | --- | --- |
> | Apple | 0.50 | 120 |
>
> ![A basket of apples]
>
> E-mail [@f1 text: E-mail] [@b1 button: Subscribe]
```

## Exit statuses

| exit status | when |
|---|---|
| 0 | the navigation is `Committed` |
| 1 | the navigation is not `Committed`: `Failed` |
| 2 | usage error: not exactly one address, an address that is not UTF-8, or `--as` without `tree` or `markdown` or given twice; a usage line on standard error |
| 3 | the run stopped: an unmet obligation of the generated code, or `OpenTab` refused the window just opened; the reason on standard error and nothing on standard output. Also when writing to standard output reports an error, such as a full disk or a reader that has gone, whatever the navigation's state: the reason on standard error, and the rows may be partly written |

A standard error that cannot be written changes none of these.

A closed standard output (`surf <url> >&-`) is not an error surf sees. The Rust runtime opens
`/dev/null` on a closed descriptor 1 before surf's own code runs, so a closed standard output behaves like
one redirected to `/dev/null`: exit 0 or 1 by the navigation's state, and the rows are lost.
Telling the two apart would take unsafe code.

## Limits

The fetch takes its limits from `surf-fetch`'s defaults: 10 s to resolve the host and connect,
30 s to send the request and receive the whole response, so at most 40 s for one fetch; 64 KiB of
header and 16 MiB of body ([`crates/fetch`](../crates/fetch/src/lib.rs)). It follows at most 20
redirects (301, 302, 303, 307, 308 with a `Location`) inside the one fetch, within the same 40 s.
When the first request went to a public address, a redirect to a loopback, private,
link-local, unique-local, shared (100.64.0.0/10) or unspecified address, or to an IPv6 address
that embeds one (IPv4-mapped, IPv4-compatible, NAT64 or 6to4), also one reached through a name,
fails the load with `ForbiddenDestination` before any connection to it. A first
request to such an address, and the redirects of a chain that starts there, load as before.
