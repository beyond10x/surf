//! The `template` element (story `html-tree-template`): the `template` start and end tag rules
//! of in head and the "in template" insertion mode (13.2.6.4.16), with the stack of template
//! insertion modes they keep. The other modes consult that stack where the standard says: reset
//! the insertion mode appropriately, the end-of-file rule of in body, and the appropriate place
//! for inserting a node, which gives a `template` target's template contents.
//!
//! Not taken, each a deliberate deviation of the snapshot the design note names: declarative
//! shadow roots (the parser's "allow declarative shadow roots" is false, so a `shadowrootmode`
//! attribute changes nothing, as the standard says for that case), and content patching, the
//! `for` attribute of a `template` start tag in head (preparing content patching and its
//! insertion target, start and end markers). A `template` with a `for` attribute is inserted as
//! a plain `template`; its contents are parsed into its template contents and patch nothing.

use super::{Entry, Flow, Mode, TreeBuilder};
use crate::tokenizer::{Token, TokenAttribute};

/// The elements "generate all implied end tags thoroughly" pops (13.2.6.3).
const IMPLIED_END_TAGS_THOROUGHLY: &[&str] = &[
    "caption", "colgroup", "dd", "dt", "li", "optgroup", "option", "p", "rb", "rp", "rt", "rtc",
    "tbody", "td", "tfoot", "th", "thead", "tr",
];

/// The start tags in template hands to in head.
const IN_HEAD_START_TAGS: &[&str] = &[
    "base", "basefont", "bgsound", "link", "meta", "noframes", "script", "style", "template",
    "title",
];

impl TreeBuilder {
    /// The `template` start tag rule of in head.
    pub(super) fn template_start_tag(&mut self, name: &str, attributes: &[TokenAttribute]) {
        self.active.push(Entry::Marker);
        self.frameset_ok = false;
        self.mode = Mode::InTemplate;
        self.template_modes.push(Mode::InTemplate);
        if !self.room_on_stack() {
            return;
        }
        let Some(early) = self.place_with_room() else {
            return;
        };
        let Some(template) = self.create_element(name, attributes) else {
            return;
        };
        // The insertion location is taken after the element is created and before it is
        // pushed, so that it is not inside the template's own contents.
        let location = early.unwrap_or_else(|| self.appropriate_place(None));
        self.open.push(template);
        self.insert_element_at(template, location);
    }

    /// The `template` end tag rule of in head.
    pub(super) fn template_end_tag(&mut self) {
        if !self.template_on_stack() {
            return;
        }
        self.generate_all_implied_end_tags_thoroughly();
        self.pop_until(&["template"]);
        self.clear_active_formatting_elements_to_last_marker();
        self.template_modes.pop();
        self.reset_insertion_mode();
    }

    /// Generate all implied end tags thoroughly.
    fn generate_all_implied_end_tags_thoroughly(&mut self) {
        while let Some(&node) = self.open.last() {
            self.visit();
            if !self.is_html(node, IMPLIED_END_TAGS_THOROUGHLY) {
                return;
            }
            self.open.pop();
        }
    }

    /// The current template insertion mode; in body when the stack of template insertion modes
    /// is empty, which happens only after a limit refused the `template` the mode was pushed
    /// for.
    pub(super) fn current_template_mode(&self) -> Mode {
        self.template_modes.last().copied().unwrap_or(Mode::InBody)
    }

    /// Replaces the current template insertion mode with `mode`, switches to it and asks for
    /// the token to be processed again.
    fn switch_template_mode(&mut self, mode: Mode) -> Flow {
        self.template_modes.pop();
        self.template_modes.push(mode);
        self.mode = mode;
        Flow::Reprocess
    }

    /// 13.2.6.4.16 The "in template" insertion mode.
    pub(super) fn in_template(&mut self, token: &Token) -> Flow {
        match token {
            Token::Character(_)
            | Token::Comment { .. }
            | Token::ProcessingInstruction { .. }
            | Token::Doctype { .. } => self.in_body(token),
            Token::StartTag { name, .. } if IN_HEAD_START_TAGS.contains(&name.as_str()) => {
                self.in_head(token)
            }
            Token::EndTag { name } if name == "template" => self.in_head(token),
            Token::StartTag { name, .. } => match name.as_str() {
                "caption" | "colgroup" | "tbody" | "tfoot" | "thead" => {
                    self.switch_template_mode(Mode::InTable)
                }
                "col" => self.switch_template_mode(Mode::InColumnGroup),
                "tr" => self.switch_template_mode(Mode::InTableBody),
                "td" | "th" => self.switch_template_mode(Mode::InRow),
                _ => self.switch_template_mode(Mode::InBody),
            },
            Token::EndTag { .. } => Flow::Done,
            Token::EndOfFile => {
                if !self.template_on_stack() {
                    self.stop_parsing();
                    return Flow::Done;
                }
                self.pop_until(&["template"]);
                self.clear_active_formatting_elements_to_last_marker();
                self.template_modes.pop();
                self.reset_insertion_mode();
                Flow::Reprocess
            }
        }
    }
}
