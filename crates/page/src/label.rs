//! The label of each actionable element: its accessible name, computed over the tree by
//! Accessible Name and Description Computation 1.2 (4.3.2 "Computation steps") with HTML-AAM 4.1
//! "Accessible Name Computations By HTML Element". The crate documentation says which steps are
//! taken and where this deviates; the two bounds are [`LABEL_BYTES_MAX`] and [`LABEL_WORK_MAX`].
//!
//! What the tree does not offer is gathered by the one walk of `actionable_elements`
//! ([`Facts`]): a dense index of the nodes with the hidden flag of each, the first element per
//! ID, and each `label` element with its labeled control. The names are computed after that
//! walk, each with its own stack: no function recurses over the depth of the tree, and none
//! calls `is_hidden`.

use std::collections::HashMap;

use surf_html::{Namespace, NodeData, NodeId, Tree};

use crate::{ActionableElement, Element, Kind, State, element};

/// The most bytes a label holds. A longer name is cut at the last UTF-8 character boundary at or
/// below this many bytes (a space the cut would leave at the end is dropped) and the row's
/// `label_cut` is set; the walk for that name stops there. A deliberate deviation: the
/// standards place no bound on a name.
pub const LABEL_BYTES_MAX: usize = 1_024;

/// The work for all labels of one call of `actionable_elements`: one unit per node read (each
/// node pushed onto a name's stack, each element named and each `label` read), one unit per byte
/// read from a text node or an attribute value into a name, and the bytes of each
/// `aria-labelledby` value split into IDREFs. Once it is spent, the name being computed and each
/// later one are taken from the element's own attributes only (`aria-label`; an `input`'s
/// `value`, or its `Submit` or `Reset`; `alt`; `title`; `placeholder`; `aria-placeholder`, in
/// the order of the element's sources), without `aria-labelledby`, `label` elements or the
/// subtree, and the row's `label_cut` is set. A deliberate deviation: the standards place no
/// bound on the work.
pub const LABEL_WORK_MAX: usize = 16_777_216;

/// What the walk of `actionable_elements` records for the labels.
#[derive(Default)]
pub(crate) struct Facts<'a> {
    /// The dense index of each node the walk reached.
    index: HashMap<NodeId, usize>,
    /// The hidden flag of each node, by its index: the flag the walk carried down.
    hidden: Vec<bool>,
    /// The first element in tree order with each ID (the `id` attribute, not empty).
    by_id: HashMap<&'a str, NodeId>,
    /// Each `label` element in tree order.
    labels: Vec<Label<'a>>,
    /// The `label` elements without `for` around the node being walked: each link names one
    /// and the link of the next `label` out.
    chain: Vec<Link>,
}

struct Label<'a> {
    node: NodeId,
    /// The value of the `for` attribute, if it has one.
    target: Option<&'a str>,
    /// The labeled control of a `label` without `for`: its first labelable descendant.
    control: Option<NodeId>,
}

struct Link {
    label: usize,
    outer: Option<usize>,
}

/// A link into [`Facts`]'s list of the `label` elements without `for` that enclose a node.
pub(crate) type Labels = Option<usize>;

impl<'a> Facts<'a> {
    /// Records `node`, reached by the walk with the hidden flag `hidden`, inside the `label`
    /// elements `labels`. Returns the `labels` for the children of `node`.
    pub(crate) fn record(
        &mut self,
        node: NodeId,
        hidden: bool,
        this: Option<&Element<'a>>,
        labels: Labels,
    ) -> Labels {
        self.index.insert(node, self.hidden.len());
        self.hidden.push(hidden);
        let Some(this) = this else {
            return labels;
        };
        if let Some(id) = this.attribute("id").filter(|id| !id.is_empty()) {
            self.by_id.entry(id).or_insert(node);
        }
        if is_labelable(this) {
            // `node` is the first labelable descendant of every enclosing `label` without `for`
            // that has none yet; those are the nearest ones, so the walk up stops at the first
            // that has one.
            let mut link = labels;
            while let Some(at) = link {
                let label = &mut self.labels[self.chain[at].label];
                if label.control.is_some() {
                    break;
                }
                label.control = Some(node);
                link = self.chain[at].outer;
            }
        }
        if this.is_html("label") {
            let target = this.attribute("for");
            self.labels.push(Label {
                node,
                target,
                control: None,
            });
            if target.is_none() {
                self.chain.push(Link {
                    label: self.labels.len() - 1,
                    outer: labels,
                });
                return Some(self.chain.len() - 1);
            }
        }
        labels
    }
}

/// HTML 4.10.2, labelable elements: `button`, `input` not in the Hidden state, `meter`,
/// `output`, `progress`, `select`, `textarea` (form-associated custom elements: surf has none).
fn is_labelable(this: &Element<'_>) -> bool {
    this.namespace == Namespace::Html
        && match this.local_name {
            "button" | "meter" | "output" | "progress" | "select" | "textarea" => true,
            "input" => State::of(this.attribute("type")) != State::Hidden,
            _ => false,
        }
}

/// Fills `label` and `label_cut` of each row, in order, from what the walk recorded.
pub(crate) fn fill(tree: &Tree, facts: Facts<'_>, rows: &mut [ActionableElement]) {
    let mut labels_of: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for label in &facts.labels {
        let control = match label.target {
            // HTML 4.10.4: the first element in tree order whose ID is the value, if it is
            // labelable; nothing else.
            Some(id) => facts
                .by_id
                .get(id)
                .copied()
                .filter(|&c| element(tree, c).is_some_and(|e| is_labelable(&e))),
            None => label.control,
        };
        if let Some(control) = control {
            labels_of.entry(control).or_default().push(label.node);
        }
    }
    let mut namer = Namer {
        tree,
        stamp: vec![0; facts.hidden.len()],
        facts,
        labels_of,
        generation: 0,
        work: 0,
        spent: false,
        charging: true,
        root: tree.root(),
        name: Name::default(),
        tasks: Vec::new(),
    };
    for row in rows {
        let (label, cut) = namer.name_of(row);
        row.label = label;
        row.label_cut = cut;
    }
}

/// Why a name stopped before its sources were read.
enum Stop {
    /// The name reached [`LABEL_BYTES_MAX`].
    Full,
    /// [`LABEL_WORK_MAX`] is spent.
    Spent,
}

/// How a node is read.
#[derive(Clone, Copy)]
struct Mode {
    /// Inside an `aria-labelledby` traversal: a node's own `aria-labelledby` is not followed
    /// (2B).
    labelledby: bool,
    /// The node that started this traversal (the node referenced, the `label`, the hidden
    /// element named) was hidden: hidden nodes are read (2A).
    include_hidden: bool,
    /// Inside a `label` of the element named: the element itself gives nothing (HTML-AAM 4.1.1
    /// to 4.1.4, a control encapsulated by its label).
    in_label: bool,
}

/// One step of a name's walk, on its stack.
enum Task {
    /// Compute the text alternative of the node (4.3.2, the steps from 2A).
    Visit(NodeId, Mode),
    /// After the IDREFs of an element's `aria-labelledby`: when they gave nothing, the element
    /// goes on at 2C.
    Resume(NodeId, Mode, Mark),
    /// After an element's children: when they gave nothing, its `title` (2I).
    Title(NodeId, Mark),
    /// The next result is joined with a space.
    Space,
}

/// A name being built, stripped and collapsed as it grows: ASCII whitespace before the first
/// character is dropped, a run of it between characters becomes one space, and a run at the end
/// is never written.
#[derive(Default)]
struct Name {
    text: String,
    /// A space is due before the next character.
    space: bool,
    cut: bool,
}

#[derive(Clone, Copy)]
struct Mark {
    len: usize,
    space: bool,
}

impl Name {
    fn mark(&self) -> Mark {
        Mark {
            len: self.text.len(),
            space: self.space,
        }
    }

    /// Whether a character was written since `mark`.
    fn grew(&self, mark: Mark) -> bool {
        self.text.len() > mark.len
    }

    /// Back to `mark`, when nothing was written since.
    fn reset(&mut self, mark: Mark) {
        self.space = mark.space;
    }

    fn join(&mut self) {
        if !self.text.is_empty() {
            self.space = true;
        }
    }

    fn push(&mut self, c: char) -> Result<(), Stop> {
        if c.is_ascii_whitespace() {
            self.join();
            return Ok(());
        }
        let space = usize::from(self.space);
        if self.text.len() + space + c.len_utf8() > LABEL_BYTES_MAX {
            self.cut = true;
            return Err(Stop::Full);
        }
        if self.space {
            self.text.push(' ');
            self.space = false;
        }
        self.text.push(c);
        Ok(())
    }
}

struct Namer<'t> {
    tree: &'t Tree,
    facts: Facts<'t>,
    labels_of: HashMap<NodeId, Vec<NodeId>>,
    /// The name during which each node, by its index, was last read.
    stamp: Vec<usize>,
    /// The number of the name being computed.
    generation: usize,
    /// The work spent by the names so far.
    work: usize,
    spent: bool,
    /// Work is counted: false while a name is taken from attributes only.
    charging: bool,
    /// The element being named.
    root: NodeId,
    name: Name,
    tasks: Vec<Task>,
}

impl<'t> Namer<'t> {
    fn name_of(&mut self, row: &ActionableElement) -> (String, bool) {
        self.generation += 1;
        self.root = row.node;
        self.name = Name::default();
        let this = element(self.tree, row.node).expect("an actionable element is an element");
        let full = if self.spent {
            Err(Stop::Spent)
        } else {
            self.charging = true;
            self.element_name(row, &this, false)
        };
        if let Err(Stop::Spent) = full {
            self.name = Name::default();
            self.charging = false;
            // Attributes only: nothing is charged, so only the byte bound stops it.
            let _ = self.element_name(row, &this, true);
            self.name.cut = true;
        }
        let mut name = std::mem::take(&mut self.name);
        name.text.shrink_to_fit();
        (name.text, name.cut)
    }

    /// The element's sources in order, first non-blank wins (HTML-AAM 4.1); with
    /// `attributes_only`, its own attributes alone.
    fn element_name(
        &mut self,
        row: &ActionableElement,
        this: &Element<'t>,
        attributes_only: bool,
    ) -> Result<(), Stop> {
        if !attributes_only {
            self.charge(1)?;
            // 2B, LabelledBy.
            if let Some(ids) = this.attribute("aria-labelledby") {
                let mark = self.name.mark();
                self.charge(ids.len())?;
                for token in ids.split_ascii_whitespace() {
                    if let Some(&target) = self.facts.by_id.get(token) {
                        self.name.join();
                        let mode = Mode {
                            labelledby: true,
                            include_hidden: self.hidden(target),
                            in_label: false,
                        };
                        self.walk(target, mode)?;
                    }
                }
                if self.name.grew(mark) {
                    return Ok(());
                }
                self.name.reset(mark);
            }
        }
        // 2D, AriaLabel.
        if self.source(this.attribute("aria-label"))? {
            return Ok(());
        }
        // 2E, Host Language Label: the control's `label` elements, joined by a space.
        if !attributes_only && let Some(labels) = self.labels_of.remove(&row.node) {
            let mark = self.name.mark();
            for label in labels {
                self.name.join();
                let mode = Mode {
                    labelledby: false,
                    include_hidden: self.hidden(label),
                    in_label: true,
                };
                self.walk(label, mode)?;
            }
            if self.name.grew(mark) {
                return Ok(());
            }
            self.name.reset(mark);
        }
        match row.kind {
            Kind::Link if this.local_name == "area" => {
                let _ =
                    self.source(this.attribute("alt"))? || self.source(this.attribute("title"))?;
            }
            Kind::Link | Kind::Button if this.local_name != "input" => {
                if !attributes_only && self.subtree(row)? {
                    return Ok(());
                }
                self.source(this.attribute("title"))?;
            }
            Kind::Textarea => self.text_control(this)?,
            Kind::Select => {
                self.source(this.attribute("title"))?;
            }
            _ => match State::of(this.attribute("type")) {
                state @ (State::SubmitButton | State::ResetButton | State::Button) => {
                    let named = match this.attribute("value") {
                        Some(value) => self.source(Some(value))?,
                        None => self.default_name(state)?,
                    };
                    if !named {
                        self.source(this.attribute("title"))?;
                    }
                }
                State::ImageButton => {
                    let _ = self.source(this.attribute("alt"))?
                        || self.source(this.attribute("title"))?
                        || self.default_name(State::ImageButton)?;
                }
                State::Text
                | State::Search
                | State::Telephone
                | State::Url
                | State::Email
                | State::Password
                | State::Number => self.text_control(this)?,
                _ => {
                    self.source(this.attribute("title"))?;
                }
            },
        }
        Ok(())
    }

    /// HTML-AAM 4.1.1 after the label elements: `title`, `placeholder`, `aria-placeholder`.
    fn text_control(&mut self, this: &Element<'t>) -> Result<(), Stop> {
        let _ = self.source(this.attribute("title"))?
            || self.source(this.attribute("placeholder"))?
            || self.source(this.attribute("aria-placeholder"))?;
        Ok(())
    }

    /// HTML-AAM 4.1.2 and 4.1.3: the implementation-defined string of a submit or reset button
    /// without a value.
    fn default_name(&mut self, state: State) -> Result<bool, Stop> {
        match state {
            State::SubmitButton | State::ImageButton => self.source(Some("Submit")),
            State::ResetButton => self.source(Some("Reset")),
            _ => Ok(false),
        }
    }

    /// The name from the children of the element named (2F), hidden ones left out unless the
    /// element is hidden itself (the crate's deviation). Whether it gave a character.
    fn subtree(&mut self, row: &ActionableElement) -> Result<bool, Stop> {
        let mark = self.name.mark();
        let children = self.tree.children(row.node);
        self.charge(children.len())?;
        self.tasks.clear();
        let mode = Mode {
            labelledby: false,
            include_hidden: row.hidden,
            in_label: false,
        };
        self.tasks
            .extend(children.iter().rev().map(|&c| Task::Visit(c, mode)));
        self.run()?;
        if self.name.grew(mark) {
            return Ok(true);
        }
        self.name.reset(mark);
        Ok(false)
    }

    /// Computes the text alternative of `start` read by `mode`, appended to the name.
    fn walk(&mut self, start: NodeId, mode: Mode) -> Result<(), Stop> {
        self.charge(1)?;
        self.tasks.clear();
        self.tasks.push(Task::Visit(start, mode));
        self.run()
    }

    fn run(&mut self) -> Result<(), Stop> {
        while let Some(task) = self.tasks.pop() {
            match task {
                Task::Space => self.name.join(),
                Task::Visit(node, mode) => self.visit(node, mode)?,
                Task::Resume(node, mode, mark) => {
                    if !self.name.grew(mark) {
                        self.name.reset(mark);
                        let this = element(self.tree, node).expect("resumed at an element");
                        self.at_embedded_control(node, &this, mode)?;
                    }
                }
                Task::Title(node, mark) => {
                    if !self.name.grew(mark) {
                        self.name.reset(mark);
                        let this = element(self.tree, node).expect("a title of an element");
                        self.source(this.attribute("title"))?;
                    }
                }
            }
        }
        Ok(())
    }

    /// 4.3.2 from 2A for one node.
    fn visit(&mut self, node: NodeId, mode: Mode) -> Result<(), Stop> {
        let at = self.facts.index[&node];
        // Each node is consulted only once per name (the note under 2F).
        if self.stamp[at] == self.generation {
            return Ok(());
        }
        self.stamp[at] = self.generation;
        // 2A, Hidden Not Referenced.
        if self.facts.hidden[at] && !mode.include_hidden {
            return Ok(());
        }
        let tree = self.tree;
        match tree.data(node) {
            // 2G, Text Node.
            NodeData::Text(text) => self.append(text),
            NodeData::Element { .. } => {
                if mode.in_label && node == self.root {
                    return Ok(());
                }
                let this = element(tree, node).expect("an element");
                // 2B, LabelledBy, unless already in such a traversal.
                if !mode.labelledby
                    && let Some(ids) = this.attribute("aria-labelledby")
                {
                    self.charge(ids.len())?;
                    let targets: Vec<NodeId> = ids
                        .split_ascii_whitespace()
                        .filter_map(|token| self.facts.by_id.get(token).copied())
                        .collect();
                    if !targets.is_empty() {
                        self.charge(targets.len())?;
                        self.tasks.push(Task::Resume(node, mode, self.name.mark()));
                        for &target in targets.iter().rev() {
                            let traversal = Mode {
                                labelledby: true,
                                include_hidden: self.hidden(target),
                                in_label: false,
                            };
                            self.tasks.push(Task::Visit(target, traversal));
                            self.tasks.push(Task::Space);
                        }
                        return Ok(());
                    }
                }
                self.at_embedded_control(node, &this, mode)
            }
            _ => Ok(()),
        }
    }

    /// 4.3.2 from 2C for an element.
    fn at_embedded_control(
        &mut self,
        node: NodeId,
        this: &Element<'t>,
        mode: Mode,
    ) -> Result<(), Stop> {
        let html = this.namespace == Namespace::Html;
        // 2C, Embedded Control: a control inside the name of another element gives its value.
        if html && node != self.root {
            match this.local_name {
                "input" => match State::of(this.attribute("type")) {
                    state @ (State::Text
                    | State::Search
                    | State::Telephone
                    | State::Url
                    | State::Email
                    | State::Password) => {
                        let raw = this.attribute("value").unwrap_or("");
                        self.charge(raw.len())?;
                        let value = state.value(this).unwrap_or_default();
                        return self.append(&value);
                    }
                    State::Number | State::Range => {
                        let value = this
                            .attribute("aria-valuetext")
                            .or_else(|| this.attribute("aria-valuenow"))
                            .or_else(|| this.attribute("value"))
                            .unwrap_or("");
                        return self.append(value);
                    }
                    _ => {}
                },
                "textarea" => {
                    let tree = self.tree;
                    let children = tree.children(node);
                    self.charge(children.len())?;
                    for &child in children {
                        if let NodeData::Text(text) = tree.data(child) {
                            self.append(text)?;
                        }
                    }
                    return Ok(());
                }
                // The chosen option is `story:agent-form-port`'s: the empty string.
                "select" => return Ok(()),
                _ => {}
            }
        }
        // 2D, AriaLabel.
        if self.source(this.attribute("aria-label"))? {
            return Ok(());
        }
        // 2E, Host Language Label: the element's own attributes (HTML-AAM 4.1); a control's
        // `label` elements are not followed inside a walk.
        if html {
            match this.local_name {
                // HTML-AAM 4.1.10: `alt`, even when blank; `title` only without `alt` (2I).
                "img" => {
                    if let Some(alt) = this.attribute("alt") {
                        return self.append(alt);
                    }
                }
                "area" => {
                    if self.source(this.attribute("alt"))? {
                        return Ok(());
                    }
                }
                "input" => match State::of(this.attribute("type")) {
                    State::ImageButton => {
                        let _ = self.source(this.attribute("alt"))?
                            || self.source(this.attribute("title"))?
                            || self.default_name(State::ImageButton)?;
                        return Ok(());
                    }
                    state @ (State::SubmitButton | State::ResetButton | State::Button) => {
                        let named = match this.attribute("value") {
                            Some(value) => self.source(Some(value))?,
                            None => self.default_name(state)?,
                        };
                        if named {
                            return Ok(());
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        // 2F and 2H, the children joined without a space; then 2I, Tooltip.
        self.tasks.push(Task::Title(node, self.name.mark()));
        let children = self.tree.children(node);
        if !children.is_empty() {
            self.charge(children.len())?;
            self.tasks
                .extend(children.iter().rev().map(|&c| Task::Visit(c, mode)));
        }
        Ok(())
    }

    /// Appends `value` when it holds a character that is not ASCII whitespace; whether it did.
    fn source(&mut self, value: Option<&str>) -> Result<bool, Stop> {
        let Some(value) = value else {
            return Ok(false);
        };
        let mark = self.name.mark();
        self.append(value)?;
        if self.name.grew(mark) {
            return Ok(true);
        }
        self.name.reset(mark);
        Ok(false)
    }

    /// Appends `text` character by character, each byte read charged.
    fn append(&mut self, text: &str) -> Result<(), Stop> {
        for c in text.chars() {
            self.charge(c.len_utf8())?;
            self.name.push(c)?;
        }
        Ok(())
    }

    fn charge(&mut self, units: usize) -> Result<(), Stop> {
        if !self.charging {
            return Ok(());
        }
        self.work = self.work.saturating_add(units);
        if self.work > LABEL_WORK_MAX {
            self.spent = true;
            return Err(Stop::Spent);
        }
        Ok(())
    }

    fn hidden(&self, node: NodeId) -> bool {
        self.facts.hidden[self.facts.index[&node]]
    }
}
