<!--
  generated from surf v1
  contract digest 19964b50f9197185811d923fbd6c3eec7ec2855a2cc03c2c60d6e4a2149578f0
  do not edit: regenerate with `ess synthesize`
-->
# Synthesis plan — surf v1

Scope: `component-skeletons`, planned by `ess-synth`. Regenerate with `ess synthesize`.

99 capabilities: **97 generated**, **0 obligations**, **2 refused**. An obligation is yours to implement against its contract; a refusal is a fact about this synthesis scope, not about the specification.

## Generated

| capability | source |
| --- | --- |
| domain type | `surf.agent.GoCall` |
| domain type | `surf.agent.OpenCall` |
| domain type | `surf.agent.OpenedSession` |
| domain type | `surf.agent.SessionCall` |
| domain type | `surf.agent.SessionId` |
| domain type | `surf.agent.SessionReadout` |
| domain type | `surf.agent.SessionToken` |
| domain type | `surf.document.Document.State` |
| domain type | `surf.document.DocumentId` |
| domain type | `surf.navigation.Navigation.State` |
| domain type | `surf.net.FailureReason` |
| domain type | `surf.net.MediaType` |
| domain type | `surf.net.Request.State` |
| domain type | `surf.net.RequestId` |
| domain type | `surf.net.Status` |
| domain type | `surf.render.Frame.State` |
| domain type | `surf.render.FrameId` |
| domain type | `surf.shell.Tab.State` |
| domain type | `surf.shell.TabId` |
| domain type | `surf.shell.Window.State` |
| domain type | `surf.shell.WindowId` |
| domain type | `surf.url.Url` |
| entity lifecycle | `surf.document.Document` |
| entity lifecycle | `surf.navigation.Navigation` |
| entity lifecycle | `surf.net.Request` |
| entity lifecycle | `surf.render.Frame` |
| entity lifecycle | `surf.shell.Tab` |
| entity lifecycle | `surf.shell.Window` |
| command contract | `surf.document.ParseDocument` |
| command behaviour | `surf.document.ParseDocument` |
| command contract | `surf.navigation.CancelNavigation` |
| command behaviour | `surf.navigation.CancelNavigation` |
| command contract | `surf.navigation.CommitNavigation` |
| command behaviour | `surf.navigation.CommitNavigation` |
| command contract | `surf.navigation.FailNavigation` |
| command behaviour | `surf.navigation.FailNavigation` |
| command contract | `surf.navigation.Navigate` |
| command behaviour | `surf.navigation.Navigate` |
| command contract | `surf.net.DeliverResponse` |
| command behaviour | `surf.net.DeliverResponse` |
| command contract | `surf.net.FailRequest` |
| command behaviour | `surf.net.FailRequest` |
| command contract | `surf.net.Fetch` |
| command behaviour | `surf.net.Fetch` |
| command contract | `surf.render.PaintFrame` |
| command behaviour | `surf.render.PaintFrame` |
| command contract | `surf.shell.CloseTab` |
| command behaviour | `surf.shell.CloseTab` |
| command contract | `surf.shell.OpenTab` |
| command behaviour | `surf.shell.OpenTab` |
| command contract | `surf.shell.OpenWindow` |
| command behaviour | `surf.shell.OpenWindow` |
| event type | `surf.document.DocumentParsed` |
| event type | `surf.navigation.NavigationCancelled` |
| event type | `surf.navigation.NavigationCommitted` |
| event type | `surf.navigation.NavigationFailed` |
| event type | `surf.navigation.NavigationStarted` |
| event type | `surf.net.FetchFailed` |
| event type | `surf.net.RequestStarted` |
| event type | `surf.net.ResponseReceived` |
| event type | `surf.render.FramePainted` |
| event type | `surf.shell.TabClosed` |
| event type | `surf.shell.TabOpened` |
| event type | `surf.shell.WindowOpened` |
| error type | `surf.navigation.NavigationNotFound` |
| error type | `surf.navigation.NavigationStateConflict` |
| error type | `surf.net.RequestNotFound` |
| error type | `surf.net.RequestStateConflict` |
| error type | `surf.render.InvalidViewport` |
| error type | `surf.shell.TabNotFound` |
| error type | `surf.shell.TabStateConflict` |
| error type | `surf.shell.WindowNotFound` |
| view type | `surf.document.Documents` |
| view query | `surf.document.Documents` |
| view type | `surf.navigation.Navigations` |
| view query | `surf.navigation.Navigations` |
| view type | `surf.net.Requests` |
| view query | `surf.net.Requests` |
| view type | `surf.render.Frames` |
| view query | `surf.render.Frames` |
| view type | `surf.shell.Tabs` |
| view query | `surf.shell.Tabs` |
| view type | `surf.shell.Windows` |
| view query | `surf.shell.Windows` |
| binding transformation | `commit-on-response` |
| binding delivery | `commit-on-response` |
| binding transformation | `fail-on-fetch-failed` |
| binding delivery | `fail-on-fetch-failed` |
| binding transformation | `fetch-on-navigation-started` |
| binding delivery | `fetch-on-navigation-started` |
| binding transformation | `parse-on-navigation-committed` |
| binding delivery | `parse-on-navigation-committed` |
| component port | `document` |
| component port | `navigator` |
| component port | `net` |
| component port | `renderer` |
| component port | `shell` |

## Ports — yours to provide

What the specification fully determines is generated; what it cannot determine is an obligation. A generated command behaviour or view query reads and writes through the ports below, and they are yours to provide: synthesis generates each port's contract and never an implementation of one, so where instances live stays your decision.

| port | what it answers |
| --- | --- |
| storage | one per entity a generated behaviour or query reads or writes: the instance stored under an identity; storing, replacing and removing one; and every stored instance, in the order the store keeps them |
| context | where a generated behaviour asks it: the caller's attributes, every identity and value the specification says the implementation assigns, and whether each `external:` branch is taken |

## Obligations — yours to implement

| capability | source | why not generated | contract |
| --- | --- | --- | --- |

## Refused — not represented by this synthesis

| capability | source | stage | why |
| --- | --- | --- | --- |
| actor grants | `surf.navigation.User` | planning | may invoke `surf.navigation.CancelNavigation`, `surf.navigation.Navigate`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `surf.shell.User` | planning | may invoke `surf.shell.CloseTab`, `surf.shell.OpenTab`, `surf.shell.OpenWindow`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
