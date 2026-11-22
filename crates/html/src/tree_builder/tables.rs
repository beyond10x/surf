//! The table insertion modes of 13.2.6.4 (in table, in table text, in caption, in column group,
//! in table body, in row, in cell), each in its own function named after the mode, its rules in
//! the standard's order, and the steps they share: clearing the stack back to a table, table
//! body or row context, closing a cell, and the "has an element in table scope" scan. Written by
//! `story:html-tree-tables`; foster parenting itself is in the appropriate place for inserting a
//! node ([`TreeBuilder::appropriate_place`]), which reads the flag the "anything else" rule of
//! in table sets.
//!
//! In table text keeps only the ASCII whitespace that comes before the first other character:
//! once a character that is not whitespace arrives, every pending character and every later one
//! up to the next token that is not a character is processed by the "anything else" rule of in
//! table at once, in order. The standard keeps them all and decides at that token; the tree is
//! the same, because no other token comes between them, and the parse holds at most the
//! whitespace run of the input instead of every character of it.

use super::{Flow, Mode, QuirksMode, Scope, TreeBuilder, is_whitespace};
use crate::tokenizer::Token;

/// Whether `token` is a start tag named one of `names`.
fn is_start(token: &Token, names: &[&str]) -> bool {
    matches!(token, Token::StartTag { name, .. } if names.contains(&name.as_str()))
}

/// Whether `token` is an end tag named one of `names`.
fn is_end(token: &Token, names: &[&str]) -> bool {
    matches!(token, Token::EndTag { name } if names.contains(&name.as_str()))
}

/// The elements of "clear the stack back to a table context".
const TABLE_CONTEXT: &[&str] = &["table", "template", "html"];

/// The elements of "clear the stack back to a table body context".
const TABLE_BODY_CONTEXT: &[&str] = &["tbody", "tfoot", "thead", "template", "html"];

/// The elements of "clear the stack back to a table row context".
const TABLE_ROW_CONTEXT: &[&str] = &["tr", "template", "html"];

impl TreeBuilder {
    /// Whether the stack of open elements has an element that `is_target` holds for in table
    /// scope: the scan stops at an HTML `html`, `table` or `template` element.
    pub(super) fn has_in_table_scope(&self, is_target: impl Fn(&Self, usize) -> bool) -> bool {
        let mut visited = 0;
        let mut found = false;
        for index in (0..self.open.len()).rev() {
            visited += 1;
            if is_target(self, index) {
                found = true;
                break;
            }
            if self.is_html(self.open[index], TABLE_CONTEXT) {
                break;
            }
        }
        self.add_steps(visited);
        found
    }

    /// Whether the stack of open elements has an HTML element named one of `names` in table
    /// scope.
    pub(super) fn has_named_in_table_scope(&self, names: &[&str]) -> bool {
        self.has_in_table_scope(|b, index| b.is_html(b.open[index], names))
    }

    /// Pops the current node while it is not an HTML element named one of `names` (the three
    /// "clear the stack back to a … context" steps; each list holds `html`).
    fn clear_stack_back_to(&mut self, names: &[&str]) {
        while let Some(&node) = self.open.last() {
            self.visit();
            if self.is_html(node, names) {
                return;
            }
            self.open.pop();
        }
    }

    /// 13.2.6.4.9 The "in table" insertion mode.
    pub(super) fn in_table(&mut self, token: &Token) -> Flow {
        match token {
            Token::Character(_)
                if self.current_is(&["table", "tbody", "template", "tfoot", "thead", "tr"]) =>
            {
                self.pending_table_text.clear();
                self.pending_table_foster = false;
                self.original_mode = self.mode;
                self.mode = Mode::InTableText;
                Flow::Reprocess
            }
            Token::Comment { data } => {
                self.insert_comment(data, None);
                Flow::Done
            }
            Token::ProcessingInstruction { target, data } => {
                self.insert_processing_instruction(target, data, None);
                Flow::Done
            }
            Token::Doctype { .. } => Flow::Done,
            Token::StartTag {
                name, attributes, ..
            } if name == "caption" => {
                self.clear_stack_back_to(TABLE_CONTEXT);
                self.active.push(super::Entry::Marker);
                self.insert_html_element(name, attributes);
                self.mode = Mode::InCaption;
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "colgroup" => {
                self.clear_stack_back_to(TABLE_CONTEXT);
                self.insert_html_element(name, attributes);
                self.mode = Mode::InColumnGroup;
                Flow::Done
            }
            t if is_start(t, &["col"]) => {
                self.clear_stack_back_to(TABLE_CONTEXT);
                self.insert_html_element("colgroup", &[]);
                self.mode = Mode::InColumnGroup;
                Flow::Reprocess
            }
            Token::StartTag {
                name, attributes, ..
            } if matches!(name.as_str(), "tbody" | "tfoot" | "thead") => {
                self.clear_stack_back_to(TABLE_CONTEXT);
                self.insert_html_element(name, attributes);
                self.mode = Mode::InTableBody;
                Flow::Done
            }
            t if is_start(t, &["td", "th", "tr"]) => {
                self.clear_stack_back_to(TABLE_CONTEXT);
                self.insert_html_element("tbody", &[]);
                self.mode = Mode::InTableBody;
                Flow::Reprocess
            }
            t if is_start(t, &["table"]) => {
                if !self.has_named_in_table_scope(&["table"]) {
                    return Flow::Done;
                }
                self.pop_until(&["table"]);
                self.reset_insertion_mode();
                Flow::Reprocess
            }
            t if is_end(t, &["table"]) => {
                if self.has_named_in_table_scope(&["table"]) {
                    self.pop_until(&["table"]);
                    self.reset_insertion_mode();
                }
                Flow::Done
            }
            t if is_end(
                t,
                &[
                    "body", "caption", "col", "colgroup", "html", "tbody", "td", "tfoot", "th",
                    "thead", "tr",
                ],
            ) =>
            {
                Flow::Done
            }
            t if is_start(t, &["style", "script", "template"]) || is_end(t, &["template"]) => {
                self.in_head(t)
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "input"
                && attributes
                    .iter()
                    .find(|a| a.name == "type")
                    .is_some_and(|a| a.value.eq_ignore_ascii_case("hidden")) =>
            {
                self.insert_void_element(name, attributes);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "form" => {
                let in_template = self.parsing_template_contents();
                if self.form.is_some() && !in_template {
                    return Flow::Done;
                }
                if let Some(form) = self.insert_html_element(name, attributes) {
                    if !in_template {
                        self.form = Some(form);
                    }
                    self.open.pop();
                }
                Flow::Done
            }
            Token::EndOfFile => self.in_body(token),
            _ => self.in_table_anything_else(token),
        }
    }

    /// The "anything else" rule of in table: the token by the rules of in body with foster
    /// parenting enabled.
    fn in_table_anything_else(&mut self, token: &Token) -> Flow {
        self.foster_parenting = true;
        let flow = self.in_body(token);
        self.foster_parenting = false;
        flow
    }

    /// 13.2.6.4.10 The "in table text" insertion mode, with the pending table character tokens
    /// kept as the module documentation says.
    pub(super) fn in_table_text(&mut self, token: &Token) -> Flow {
        match token {
            Token::Character(0) => Flow::Done,
            Token::Character(c) => {
                if !self.pending_table_foster && is_whitespace(*c) {
                    // ASCII whitespace is one byte of UTF-8.
                    self.pending_table_text.push(char::from(*c as u8));
                    return Flow::Done;
                }
                if !self.pending_table_foster {
                    self.pending_table_foster = true;
                    let pending = std::mem::take(&mut self.pending_table_text);
                    for b in pending.bytes() {
                        self.in_table_anything_else(&Token::Character(u32::from(b)));
                    }
                }
                self.in_table_anything_else(token);
                Flow::Done
            }
            _ => {
                if !self.pending_table_foster {
                    let pending = std::mem::take(&mut self.pending_table_text);
                    for b in pending.bytes() {
                        self.insert_character(u32::from(b));
                    }
                }
                self.pending_table_foster = false;
                self.mode = self.original_mode;
                Flow::Reprocess
            }
        }
    }

    /// 13.2.6.4.11 The "in caption" insertion mode.
    pub(super) fn in_caption(&mut self, token: &Token) -> Flow {
        match token {
            t if is_end(t, &["caption"])
                || is_start(
                    t,
                    &[
                        "caption", "col", "colgroup", "tbody", "td", "tfoot", "th", "thead", "tr",
                    ],
                )
                || is_end(t, &["table"]) =>
            {
                if !self.has_named_in_table_scope(&["caption"]) {
                    return Flow::Done;
                }
                self.generate_implied_end_tags(None);
                self.pop_until(&["caption"]);
                self.clear_active_formatting_elements_to_last_marker();
                self.mode = Mode::InTable;
                if is_end(t, &["caption"]) {
                    Flow::Done
                } else {
                    Flow::Reprocess
                }
            }
            t if is_end(
                t,
                &[
                    "body", "col", "colgroup", "html", "tbody", "td", "tfoot", "th", "thead", "tr",
                ],
            ) =>
            {
                Flow::Done
            }
            _ => self.in_body(token),
        }
    }

    /// 13.2.6.4.12 The "in column group" insertion mode.
    pub(super) fn in_column_group(&mut self, token: &Token) -> Flow {
        match token {
            Token::Character(c) if is_whitespace(*c) => {
                self.insert_character(*c);
                Flow::Done
            }
            Token::Comment { data } => {
                self.insert_comment(data, None);
                Flow::Done
            }
            Token::ProcessingInstruction { target, data } => {
                self.insert_processing_instruction(target, data, None);
                Flow::Done
            }
            Token::Doctype { .. } => Flow::Done,
            t if is_start(t, &["html"]) => self.in_body(t),
            Token::StartTag {
                name, attributes, ..
            } if name == "col" => {
                self.insert_void_element(name, attributes);
                Flow::Done
            }
            t if is_end(t, &["colgroup"]) => {
                if self.current_is(&["colgroup"]) {
                    self.open.pop();
                    self.mode = Mode::InTable;
                }
                Flow::Done
            }
            t if is_end(t, &["col"]) => Flow::Done,
            t if is_start(t, &["template"]) || is_end(t, &["template"]) => self.in_head(t),
            Token::EndOfFile => self.in_body(token),
            _ => {
                if !self.current_is(&["colgroup"]) {
                    return Flow::Done;
                }
                self.open.pop();
                self.mode = Mode::InTable;
                Flow::Reprocess
            }
        }
    }

    /// 13.2.6.4.13 The "in table body" insertion mode.
    pub(super) fn in_table_body(&mut self, token: &Token) -> Flow {
        match token {
            Token::StartTag {
                name, attributes, ..
            } if name == "tr" => {
                self.clear_stack_back_to(TABLE_BODY_CONTEXT);
                self.insert_html_element(name, attributes);
                self.mode = Mode::InRow;
                Flow::Done
            }
            t if is_start(t, &["th", "td"]) => {
                self.clear_stack_back_to(TABLE_BODY_CONTEXT);
                self.insert_html_element("tr", &[]);
                self.mode = Mode::InRow;
                Flow::Reprocess
            }
            Token::EndTag { name } if matches!(name.as_str(), "tbody" | "tfoot" | "thead") => {
                if self.has_named_in_table_scope(&[name]) {
                    self.clear_stack_back_to(TABLE_BODY_CONTEXT);
                    self.open.pop();
                    self.mode = Mode::InTable;
                }
                Flow::Done
            }
            t if is_start(
                t,
                &["caption", "col", "colgroup", "tbody", "tfoot", "thead"],
            ) || is_end(t, &["table"]) =>
            {
                if !self.has_named_in_table_scope(&["tbody", "thead", "tfoot"]) {
                    return Flow::Done;
                }
                self.clear_stack_back_to(TABLE_BODY_CONTEXT);
                self.open.pop();
                self.mode = Mode::InTable;
                Flow::Reprocess
            }
            t if is_end(
                t,
                &[
                    "body", "caption", "col", "colgroup", "html", "td", "th", "tr",
                ],
            ) =>
            {
                Flow::Done
            }
            _ => self.in_table(token),
        }
    }

    /// 13.2.6.4.14 The "in row" insertion mode.
    pub(super) fn in_row(&mut self, token: &Token) -> Flow {
        match token {
            Token::StartTag {
                name, attributes, ..
            } if name == "th" || name == "td" => {
                self.clear_stack_back_to(TABLE_ROW_CONTEXT);
                self.insert_html_element(name, attributes);
                self.mode = Mode::InCell;
                self.active.push(super::Entry::Marker);
                Flow::Done
            }
            t if is_end(t, &["tr"]) => {
                if self.has_named_in_table_scope(&["tr"]) {
                    self.close_the_row();
                }
                Flow::Done
            }
            t if is_start(
                t,
                &[
                    "caption", "col", "colgroup", "tbody", "tfoot", "thead", "tr",
                ],
            ) || is_end(t, &["table"]) =>
            {
                if !self.has_named_in_table_scope(&["tr"]) {
                    return Flow::Done;
                }
                self.close_the_row();
                Flow::Reprocess
            }
            Token::EndTag { name } if matches!(name.as_str(), "tbody" | "tfoot" | "thead") => {
                if !self.has_named_in_table_scope(&[name])
                    || !self.has_named_in_table_scope(&["tr"])
                {
                    return Flow::Done;
                }
                self.close_the_row();
                Flow::Reprocess
            }
            t if is_end(
                t,
                &["body", "caption", "col", "colgroup", "html", "td", "th"],
            ) =>
            {
                Flow::Done
            }
            _ => self.in_table(token),
        }
    }

    /// The steps the `tr` end tag rule of in row and the rules that reprocess after it share:
    /// clear the stack back to a table row context, pop the `tr`, switch to in table body.
    fn close_the_row(&mut self) {
        self.clear_stack_back_to(TABLE_ROW_CONTEXT);
        self.open.pop();
        self.mode = Mode::InTableBody;
    }

    /// 13.2.6.4.15 The "in cell" insertion mode.
    pub(super) fn in_cell(&mut self, token: &Token) -> Flow {
        match token {
            Token::EndTag { name } if name == "td" || name == "th" => {
                if !self.has_named_in_table_scope(&[name]) {
                    return Flow::Done;
                }
                self.generate_implied_end_tags(None);
                self.pop_until(&[name]);
                self.clear_active_formatting_elements_to_last_marker();
                self.mode = Mode::InRow;
                Flow::Done
            }
            t if is_start(
                t,
                &[
                    "caption", "col", "colgroup", "tbody", "td", "tfoot", "th", "thead", "tr",
                ],
            ) =>
            {
                // The standard asserts a `td` or `th` in table scope here; without one the token
                // is ignored, so that no input loops.
                if !self.has_named_in_table_scope(&["td", "th"]) {
                    return Flow::Done;
                }
                self.close_the_cell();
                Flow::Reprocess
            }
            t if is_end(t, &["body", "caption", "col", "colgroup", "html"]) => Flow::Done,
            Token::EndTag { name }
                if matches!(name.as_str(), "table" | "tbody" | "tfoot" | "thead" | "tr") =>
            {
                if !self.has_named_in_table_scope(&[name]) {
                    return Flow::Done;
                }
                self.close_the_cell();
                Flow::Reprocess
            }
            _ => self.in_body(token),
        }
    }

    /// Close the cell.
    fn close_the_cell(&mut self) {
        self.generate_implied_end_tags(None);
        self.pop_until(&["td", "th"]);
        self.clear_active_formatting_elements_to_last_marker();
        self.mode = Mode::InRow;
    }

    /// The `table` start tag rule of in body: a `p` in button scope is closed unless the
    /// document is in quirks mode.
    pub(super) fn in_body_table_start(
        &mut self,
        name: &str,
        attributes: &[crate::tokenizer::TokenAttribute],
    ) {
        if self.quirks_mode != QuirksMode::Quirks && self.has_named_in_scope("p", Scope::Button) {
            self.close_a_p_element();
        }
        self.insert_html_element(name, attributes);
        self.frameset_ok = false;
        self.mode = Mode::InTable;
    }
}
