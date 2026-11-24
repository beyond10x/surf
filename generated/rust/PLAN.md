<!--
  generated from surf v1
  contract digest 815b5846cd605d5ad5bf889b9e1b2b924cfbeb7d6931ec05ff659e3ea3fc4b79
  do not edit: regenerate with `ess synthesize`
-->
# Synthesis plan — surf v1

Scope: `component-skeletons`, planned by `ess-synth`. Regenerate with `ess synthesize`.

278 capabilities: **265 generated**, **9 obligations**, **4 refused**. An obligation is yours to implement against its contract; a refusal is a fact about this synthesis scope, not about the specification.

## Generated

| capability | source |
| --- | --- |
| domain type | `surf.agent.Action.State` |
| domain type | `surf.agent.ActionCommand` |
| domain type | `surf.agent.ActionId` |
| domain type | `surf.agent.FillCall` |
| domain type | `surf.agent.FollowCall` |
| domain type | `surf.agent.GoCall` |
| domain type | `surf.agent.LogCall` |
| domain type | `surf.agent.LookCall` |
| domain type | `surf.agent.OpenCall` |
| domain type | `surf.agent.OpenedSession` |
| domain type | `surf.agent.PageForm` |
| domain type | `surf.agent.PickCall` |
| domain type | `surf.agent.PressCall` |
| domain type | `surf.agent.Session.State` |
| domain type | `surf.agent.SessionCall` |
| domain type | `surf.agent.SessionId` |
| domain type | `surf.agent.SessionReadout` |
| domain type | `surf.agent.SessionToken` |
| domain type | `surf.document.Document.State` |
| domain type | `surf.document.DocumentId` |
| domain type | `surf.document.Element.State` |
| domain type | `surf.document.ElementId` |
| domain type | `surf.document.ElementKind` |
| domain type | `surf.document.EncodingName` |
| domain type | `surf.document.Form.State` |
| domain type | `surf.document.FormId` |
| domain type | `surf.document.Option.State` |
| domain type | `surf.document.OptionId` |
| domain type | `surf.document.ParseLimit` |
| domain type | `surf.document.Script.State` |
| domain type | `surf.document.ScriptEnd` |
| domain type | `surf.document.ScriptErrorKind` |
| domain type | `surf.document.ScriptId` |
| domain type | `surf.document.ScriptReason` |
| domain type | `surf.document.Submission.State` |
| domain type | `surf.document.SubmissionEndReason` |
| domain type | `surf.document.SubmissionId` |
| domain type | `surf.navigation.HistoryHandling` |
| domain type | `surf.navigation.Navigation.State` |
| domain type | `surf.navigation.Standing` |
| domain type | `surf.net.FailureReason` |
| domain type | `surf.net.MediaType` |
| domain type | `surf.net.Method` |
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
| entity lifecycle | `surf.agent.Action` |
| entity lifecycle | `surf.agent.Session` |
| entity lifecycle | `surf.document.Document` |
| entity lifecycle | `surf.document.Element` |
| entity lifecycle | `surf.document.Form` |
| entity lifecycle | `surf.document.Option` |
| entity lifecycle | `surf.document.Script` |
| entity lifecycle | `surf.document.Submission` |
| entity lifecycle | `surf.navigation.Navigation` |
| entity lifecycle | `surf.net.Request` |
| entity lifecycle | `surf.render.Frame` |
| entity lifecycle | `surf.shell.Tab` |
| entity lifecycle | `surf.shell.Window` |
| command contract | `surf.agent.CloseSession` |
| command behaviour | `surf.agent.CloseSession` |
| command contract | `surf.agent.Fill` |
| command behaviour | `surf.agent.Fill` |
| command contract | `surf.agent.Follow` |
| command behaviour | `surf.agent.Follow` |
| command contract | `surf.agent.Go` |
| command behaviour | `surf.agent.Go` |
| command contract | `surf.agent.GoBack` |
| command behaviour | `surf.agent.GoBack` |
| command contract | `surf.agent.GoForward` |
| command behaviour | `surf.agent.GoForward` |
| command contract | `surf.agent.NoteNavigationCommitted` |
| command contract | `surf.agent.NoteNavigationEnded` |
| command contract | `surf.agent.NoteNavigationFailed` |
| command contract | `surf.agent.NoteNavigationStarted` |
| command contract | `surf.agent.OpenSession` |
| command behaviour | `surf.agent.OpenSession` |
| command contract | `surf.agent.Pick` |
| command behaviour | `surf.agent.Pick` |
| command contract | `surf.agent.Press` |
| command behaviour | `surf.agent.Press` |
| command contract | `surf.agent.RecordAction` |
| command behaviour | `surf.agent.RecordAction` |
| command contract | `surf.document.AbandonSubmission` |
| command behaviour | `surf.document.AbandonSubmission` |
| command contract | `surf.document.AddElement` |
| command behaviour | `surf.document.AddElement` |
| command contract | `surf.document.AddForm` |
| command behaviour | `surf.document.AddForm` |
| command contract | `surf.document.AddOption` |
| command behaviour | `surf.document.AddOption` |
| command contract | `surf.document.AddScriptEnd` |
| command contract | `surf.document.Check` |
| command contract | `surf.document.FinishParsing` |
| command behaviour | `surf.document.FinishParsing` |
| command contract | `surf.document.ParseDocument` |
| command behaviour | `surf.document.ParseDocument` |
| command contract | `surf.document.SelectOption` |
| command contract | `surf.document.SetValue` |
| command behaviour | `surf.document.SetValue` |
| command contract | `surf.document.StartSubmission` |
| command behaviour | `surf.document.StartSubmission` |
| command contract | `surf.document.SubmitForm` |
| command behaviour | `surf.document.SubmitForm` |
| command contract | `surf.navigation.CancelNavigation` |
| command behaviour | `surf.navigation.CancelNavigation` |
| command contract | `surf.navigation.CommitNavigation` |
| command contract | `surf.navigation.FailNavigation` |
| command behaviour | `surf.navigation.FailNavigation` |
| command contract | `surf.navigation.Navigate` |
| command behaviour | `surf.navigation.Navigate` |
| command contract | `surf.navigation.StartTraversal` |
| command behaviour | `surf.navigation.StartTraversal` |
| command contract | `surf.navigation.TraverseHistory` |
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
| event type | `surf.agent.ActionRecorded` |
| event type | `surf.agent.EventsLeftOut` |
| event type | `surf.agent.FillRequested` |
| event type | `surf.agent.NavigationRequested` |
| event type | `surf.agent.PickRequested` |
| event type | `surf.agent.PressRequested` |
| event type | `surf.agent.SessionClosed` |
| event type | `surf.agent.SessionNavigationChanged` |
| event type | `surf.agent.SessionOpened` |
| event type | `surf.agent.TraversalRequested` |
| event type | `surf.document.CheckednessSet` |
| event type | `surf.document.DocumentParsed` |
| event type | `surf.document.ElementAdded` |
| event type | `surf.document.FormAdded` |
| event type | `surf.document.FormSubmitted` |
| event type | `surf.document.OptionAdded` |
| event type | `surf.document.OptionSelected` |
| event type | `surf.document.ParsingStarted` |
| event type | `surf.document.ScriptEndAdded` |
| event type | `surf.document.SubmissionAbandoned` |
| event type | `surf.document.SubmissionStarted` |
| event type | `surf.document.ValueSet` |
| event type | `surf.navigation.NavigationCancelled` |
| event type | `surf.navigation.NavigationCommitted` |
| event type | `surf.navigation.NavigationFailed` |
| event type | `surf.navigation.NavigationStarted` |
| event type | `surf.navigation.TraversalStarted` |
| event type | `surf.net.FetchFailed` |
| event type | `surf.net.RequestStarted` |
| event type | `surf.net.ResponseReceived` |
| event type | `surf.render.FramePainted` |
| event type | `surf.shell.TabClosed` |
| event type | `surf.shell.TabOpened` |
| event type | `surf.shell.WindowOpened` |
| error type | `surf.agent.NotSessionHolder` |
| error type | `surf.agent.SessionExists` |
| error type | `surf.agent.SessionNotFound` |
| error type | `surf.agent.SessionStateConflict` |
| error type | `surf.document.ControlDisabled` |
| error type | `surf.document.ControlHidden` |
| error type | `surf.document.ControlKindConflict` |
| error type | `surf.document.ControlReadonly` |
| error type | `surf.document.DocumentNotFound` |
| error type | `surf.document.DocumentStateConflict` |
| error type | `surf.document.ElementNotFound` |
| error type | `surf.document.InvalidScriptEnd` |
| error type | `surf.document.InvalidScriptPosition` |
| error type | `surf.document.OptionNotFound` |
| error type | `surf.document.SubmissionNotFound` |
| error type | `surf.document.SubmissionStateConflict` |
| error type | `surf.navigation.NavigationNotFound` |
| error type | `surf.navigation.NavigationStateConflict` |
| error type | `surf.navigation.NoHistoryEntry` |
| error type | `surf.navigation.TraversalUnsupported` |
| error type | `surf.net.RequestNotFound` |
| error type | `surf.net.RequestStateConflict` |
| error type | `surf.render.InvalidViewport` |
| error type | `surf.shell.TabNotFound` |
| error type | `surf.shell.TabStateConflict` |
| error type | `surf.shell.WindowNotFound` |
| view type | `surf.agent.SessionLog` |
| view query | `surf.agent.SessionLog` |
| view type | `surf.agent.SessionState` |
| view query | `surf.agent.SessionState` |
| view type | `surf.agent.Sessions` |
| view query | `surf.agent.Sessions` |
| view type | `surf.document.Controls` |
| view query | `surf.document.Controls` |
| view type | `surf.document.Documents` |
| view query | `surf.document.Documents` |
| view type | `surf.document.Elements` |
| view query | `surf.document.Elements` |
| view type | `surf.document.Forms` |
| view query | `surf.document.Forms` |
| view type | `surf.document.Options` |
| view query | `surf.document.Options` |
| view type | `surf.document.Scripts` |
| view query | `surf.document.Scripts` |
| view type | `surf.document.Submissions` |
| view query | `surf.document.Submissions` |
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
| binding transformation | `check-on-pick-requested` |
| binding delivery | `check-on-pick-requested` |
| binding transformation | `close-tab-on-session-closed` |
| binding delivery | `close-tab-on-session-closed` |
| binding transformation | `commit-on-response` |
| binding delivery | `commit-on-response` |
| binding transformation | `fail-on-fetch-failed` |
| binding delivery | `fail-on-fetch-failed` |
| binding transformation | `fetch-on-navigation-started` |
| binding delivery | `fetch-on-navigation-started` |
| binding transformation | `navigate-on-form-submitted` |
| binding delivery | `navigate-on-form-submitted` |
| binding transformation | `navigate-on-navigation-requested` |
| binding delivery | `navigate-on-navigation-requested` |
| binding transformation | `note-on-navigation-cancelled` |
| binding delivery | `note-on-navigation-cancelled` |
| binding transformation | `note-on-navigation-committed` |
| binding delivery | `note-on-navigation-committed` |
| binding transformation | `note-on-navigation-failed` |
| binding delivery | `note-on-navigation-failed` |
| binding transformation | `note-on-navigation-started` |
| binding delivery | `note-on-navigation-started` |
| binding transformation | `parse-on-navigation-committed` |
| binding delivery | `parse-on-navigation-committed` |
| binding transformation | `select-on-pick-requested` |
| binding delivery | `select-on-pick-requested` |
| binding transformation | `set-value-on-fill-requested` |
| binding delivery | `set-value-on-fill-requested` |
| binding transformation | `start-on-traversal-started` |
| binding delivery | `start-on-traversal-started` |
| binding transformation | `submit-on-press-requested` |
| binding delivery | `submit-on-press-requested` |
| binding transformation | `traverse-on-traversal-requested` |
| binding delivery | `traverse-on-traversal-requested` |
| component port | `agent` |
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
| command behaviour | `surf.agent.NoteNavigationCommitted` | kept an obligation by a set outcome over filtered rows (`instances:`), in `noted` | given `surf.agent.NoteNavigationCommitted` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `noted` otherwise, changes every `surf.agent.Session` where `navigation_id == input.navigation_id`, setting `url`, `navigation_state`, `status`, `media_type`, emits `surf.agent.SessionNavigationChanged` |
| command behaviour | `surf.agent.NoteNavigationEnded` | kept an obligation by a set outcome over filtered rows (`instances:`), in `noted` | given `surf.agent.NoteNavigationEnded` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `noted` otherwise, changes every `surf.agent.Session` where `navigation_id == input.navigation_id`, setting `navigation_state`, emits `surf.agent.SessionNavigationChanged` |
| command behaviour | `surf.agent.NoteNavigationFailed` | kept an obligation by a set outcome over filtered rows (`instances:`), in `noted` | given `surf.agent.NoteNavigationFailed` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `noted` otherwise, changes every `surf.agent.Session` where `navigation_id == input.navigation_id`, setting `navigation_state`, `failure_reason`, emits `surf.agent.SessionNavigationChanged` |
| command behaviour | `surf.agent.NoteNavigationStarted` | kept an obligation by a set outcome over filtered rows (`instances:`), in `noted` | given `surf.agent.NoteNavigationStarted` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `noted` otherwise, changes every `surf.agent.Session` where `tab_id == input.tab_id`, setting `navigation_id`, `url`, `navigation_state`, `status`, `media_type`, `failure_reason`, emits `surf.agent.SessionNavigationChanged` |
| command behaviour | `surf.document.AddScriptEnd` | kept an obligation by the guard path `message.count`, in `invalid-end` | given `surf.document.AddScriptEnd` input, decide and enact exactly one outcome. Selection precedence: on commands with `when_related:`, check `existing_instance` then `exists: false` before input-guarded refusals; choose the first declared input refusal whose guard holds; then check addressed-row existence (`unknown_instance`, and `existing_instance` on commands without `when_related:`); then the held state (`when_subject_state` and `when_subject`), with `wrong_state` only if the selected branch moves from a state the row does not hold; then choose the present `when_related:` predicate refusal whose predicate and optional input guard hold; then accepting and external branches in declaration order. An accepting branch that moves nothing answers in every state. Related-presence predicates do not precede input-guarded refusals. Declared outcomes (declaration order, not selection precedence): `invalid-end` when `(((end == Ran or end == Faulted) and (defined(reason) or defined(error_kind) or defined(message) or defined(line) or defined(column) or defined(dynamic))) or ((end == Limit or end == NotRun) and (not (defined(reason)) or defined(error_kind) or defined(message) or defined(line) or defined(column) or defined(dynamic))) or ((end == Threw or end == Refused) and (defined(reason) or not (defined(error_kind)))) or (end == Limit and defined(reason) and reason != StepsPerScript and reason != Memory and reason != SourceBytes and reason != WallClock and reason != Nodes and reason != Depth and reason != Attributes and reason != StoredBytes) or (end == NotRun and defined(reason) and (reason == StepsPerScript or reason == SourceBytes or reason == Nodes or reason == Depth or reason == Attributes or reason == StoredBytes)) or (end == Threw and defined(error_kind) and error_kind == Value and defined(message)) or (end == Refused and defined(error_kind) and error_kind != SyntaxError) or (defined(message) and message.count > 1024))`, error `surf.document.InvalidScriptEnd`; `invalid-position` when `(position < 1 or (defined(line) and line < 1) or (defined(line) and line >= 4294967296) or (defined(column) and column < 1) or (defined(column) and column >= 4294967296))`, error `surf.document.InvalidScriptPosition`; `no-such-document` when no `surf.document.Document` carries the identity `input.document_id` names, error `surf.document.DocumentNotFound`; `already-parsed` when the `surf.document.Document` that `input.document_id` names satisfies `state == Parsed`, error `surf.document.DocumentStateConflict`; `added` otherwise, creates `surf.document.Script`, emits `surf.document.ScriptEndAdded` |
| command behaviour | `surf.document.Check` | kept an obligation by effects on filtered rows (`affects:`), in `radio-checked` | given `surf.document.Check` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `no-such-element` for an identity no record carries, error `surf.document.ElementNotFound`; `not-checkable` when the existing subject's stored fields satisfy `(kind != checkbox and kind != radio)`, error `surf.document.ControlKindConflict`; `disabled` when the existing subject's stored fields satisfy `disabled == true`, error `surf.document.ControlDisabled`; `hidden` when the existing subject's stored fields satisfy `hidden == true`, error `surf.document.ControlHidden`; `radio-checked` when the existing subject's stored fields satisfy `kind == radio`, updates `surf.document.Element`, changes, beside its subject, every `surf.document.Element` where `(kind == radio and document_id == subject.document_id and radio_group == subject.radio_group)`, setting `checked`, emits `surf.document.CheckednessSet`; `unchecked` when the existing subject's stored fields satisfy `checked == true`, updates `surf.document.Element`, emits `surf.document.CheckednessSet`; `checked` otherwise, updates `surf.document.Element`, emits `surf.document.CheckednessSet` |
| command behaviour | `surf.document.SelectOption` | kept an obligation by effects on filtered rows (`affects:`), in `selected` | given `surf.document.SelectOption` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `no-such-option` for an identity no record carries, error `surf.document.OptionNotFound`; `disabled` when the existing subject's stored fields satisfy `disabled == true`, error `surf.document.ControlDisabled`; `hidden` when the existing subject's stored fields satisfy `hidden == true`, error `surf.document.ControlHidden`; `selected` otherwise, updates `surf.document.Option`, changes, beside its subject, every `surf.document.Option` where `select_id == subject.select_id`, setting `selected`, emits `surf.document.OptionSelected` |
| command behaviour | `surf.navigation.CommitNavigation` | kept an obligation by effects on filtered rows (`affects:`), in `traversed` | given `surf.navigation.CommitNavigation` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `no-such-navigation` for an identity no record carries, error `surf.navigation.NavigationNotFound`; `traversed` when the existing subject's stored fields satisfy `handling == traverse`, takes `commit` of `surf.navigation.Navigation`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and standing == back)`, setting `standing`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and standing == forward)`, setting `standing`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and standing == current)`, setting `standing`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and next == subject.entry_id)`, setting `standing`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and navigation_id == subject.entry_next)`, setting `standing`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and navigation_id == subject.entry_id)`, setting `standing`, emits `surf.navigation.NavigationCommitted`; `committed` otherwise, takes `commit` of `surf.navigation.Navigation`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and (standing == forward or standing == later))`, setting `standing`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and standing == back)`, setting `standing`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and standing == current)`, setting `standing`, `next`, changes, beside its subject, every `surf.navigation.Navigation` where `(tab_id == subject.tab_id and handling == traverse and state == Started)`, moving it along `cancel` (a row resting outside its `from` states is left as it is), emits `surf.navigation.NavigationCommitted`; `wrong-state` from a state no declared move starts in, error `surf.navigation.NavigationStateConflict` |
| command behaviour | `surf.navigation.TraverseHistory` | kept an obligation by a guard over the rows a selector selects (`when_related: {entity, where, exists | count | forall}`), in `no-entry` | given `surf.navigation.TraverseHistory` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `unsupported` when `(step != back and step != forward)`, error `surf.navigation.TraversalUnsupported`; `no-entry` when, of the `surf.navigation.Navigation` rows satisfying `(tab_id == input.tab_id and defined(standing) and standing == input.step and method == Get)`, no row is selected, error `surf.navigation.NoHistoryEntry`; `requested` otherwise, emits `surf.navigation.TraversalStarted` |

## Refused — not represented by this synthesis

| capability | source | stage | why |
| --- | --- | --- | --- |
| actor grants | `surf.agent.Opener` | planning | may invoke `surf.agent.OpenSession`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `surf.agent.SessionHolder` | planning | may invoke `surf.agent.CloseSession`, `surf.agent.Fill`, `surf.agent.Follow`, `surf.agent.Go`, `surf.agent.GoBack`, `surf.agent.GoForward`, `surf.agent.Pick`, `surf.agent.Press`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `surf.navigation.User` | planning | may invoke `surf.navigation.CancelNavigation`, `surf.navigation.Navigate`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `surf.shell.User` | planning | may invoke `surf.shell.CloseTab`, `surf.shell.OpenTab`, `surf.shell.OpenWindow`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
