//! The insertion modes of 13.2.6.4 that the tree builder writes, each in its own function named
//! after the mode, its rules in the standard's order. Parse errors are not reported, so a rule
//! that is only a parse error and "ignore the token" reads `Flow::Done`.

use super::{Entry, Flow, Mode, QuirksMode, Scope, TreeBuilder, is_whitespace};
use crate::tokenizer::{self, Token, TokenAttribute};
use crate::tree::{NodeData, NodeId, stored_bytes_of};

/// The public identifiers that make a DOCTYPE quirky when one is a prefix of the token's
/// (13.2.6.4.1).
const QUIRKY_PUBLIC_PREFIXES: &[&str] = &[
    "+//Silmaril//dtd html Pro v0r11 19970101//",
    "-//AS//DTD HTML 3.0 asWedit + extensions//",
    "-//AdvaSoft Ltd//DTD HTML 3.0 asWedit + extensions//",
    "-//IETF//DTD HTML 2.0 Level 1//",
    "-//IETF//DTD HTML 2.0 Level 2//",
    "-//IETF//DTD HTML 2.0 Strict Level 1//",
    "-//IETF//DTD HTML 2.0 Strict Level 2//",
    "-//IETF//DTD HTML 2.0 Strict//",
    "-//IETF//DTD HTML 2.0//",
    "-//IETF//DTD HTML 2.1E//",
    "-//IETF//DTD HTML 3.0//",
    "-//IETF//DTD HTML 3.2 Final//",
    "-//IETF//DTD HTML 3.2//",
    "-//IETF//DTD HTML 3//",
    "-//IETF//DTD HTML Level 0//",
    "-//IETF//DTD HTML Level 1//",
    "-//IETF//DTD HTML Level 2//",
    "-//IETF//DTD HTML Level 3//",
    "-//IETF//DTD HTML Strict Level 0//",
    "-//IETF//DTD HTML Strict Level 1//",
    "-//IETF//DTD HTML Strict Level 2//",
    "-//IETF//DTD HTML Strict Level 3//",
    "-//IETF//DTD HTML Strict//",
    "-//IETF//DTD HTML//",
    "-//Metrius//DTD Metrius Presentational//",
    "-//Microsoft//DTD Internet Explorer 2.0 HTML Strict//",
    "-//Microsoft//DTD Internet Explorer 2.0 HTML//",
    "-//Microsoft//DTD Internet Explorer 2.0 Tables//",
    "-//Microsoft//DTD Internet Explorer 3.0 HTML Strict//",
    "-//Microsoft//DTD Internet Explorer 3.0 HTML//",
    "-//Microsoft//DTD Internet Explorer 3.0 Tables//",
    "-//Netscape Comm. Corp.//DTD HTML//",
    "-//Netscape Comm. Corp.//DTD Strict HTML//",
    "-//O'Reilly and Associates//DTD HTML 2.0//",
    "-//O'Reilly and Associates//DTD HTML Extended 1.0//",
    "-//O'Reilly and Associates//DTD HTML Extended Relaxed 1.0//",
    "-//SQ//DTD HTML 2.0 HoTMetaL + extensions//",
    "-//SoftQuad Software//DTD HoTMetaL PRO 6.0::19990601::extensions to HTML 4.0//",
    "-//SoftQuad//DTD HoTMetaL PRO 4.0::19971010::extensions to HTML 4.0//",
    "-//Spyglass//DTD HTML 2.0 Extended//",
    "-//Sun Microsystems Corp.//DTD HotJava HTML//",
    "-//Sun Microsystems Corp.//DTD HotJava Strict HTML//",
    "-//W3C//DTD HTML 3 1995-03-24//",
    "-//W3C//DTD HTML 3.2 Draft//",
    "-//W3C//DTD HTML 3.2 Final//",
    "-//W3C//DTD HTML 3.2//",
    "-//W3C//DTD HTML 3.2S Draft//",
    "-//W3C//DTD HTML 4.0 Frameset//",
    "-//W3C//DTD HTML 4.0 Transitional//",
    "-//W3C//DTD HTML Experimental 19960712//",
    "-//W3C//DTD HTML Experimental 970421//",
    "-//W3C//DTD W3 HTML//",
    "-//W3O//DTD W3 HTML 3.0//",
    "-//WebTechs//DTD Mozilla HTML 2.0//",
    "-//WebTechs//DTD Mozilla HTML//",
];

/// Whether `s` starts with `prefix`, ASCII case-insensitively.
fn starts_with_ignoring_case(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len()
        && s.is_char_boundary(prefix.len())
        && s[..prefix.len()].eq_ignore_ascii_case(prefix)
}

/// The mode a DOCTYPE token sets the document to (13.2.6.4.1). The document is never an
/// `iframe` `srcdoc` document, and the parser can always change the mode.
fn doctype_quirks_mode(
    name: Option<&str>,
    public_id: Option<&str>,
    system_id: Option<&str>,
    force_quirks: bool,
) -> QuirksMode {
    let public_is = |s: &str| public_id.is_some_and(|p| p.eq_ignore_ascii_case(s));
    let public_starts = |s: &str| public_id.is_some_and(|p| starts_with_ignoring_case(p, s));
    let system_missing_or_empty = system_id.is_none_or(str::is_empty);
    if force_quirks
        || name != Some("html")
        || public_is("-//W3O//DTD W3 HTML Strict 3.0//EN//")
        || public_is("-/W3C/DTD HTML 4.0 Transitional/EN")
        || public_is("HTML")
        || system_id.is_some_and(|s| {
            s.eq_ignore_ascii_case("http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd")
        })
        || QUIRKY_PUBLIC_PREFIXES.iter().any(|p| public_starts(p))
        || (system_missing_or_empty
            && (public_starts("-//W3C//DTD HTML 4.01 Frameset//")
                || public_starts("-//W3C//DTD HTML 4.01 Transitional//")))
    {
        QuirksMode::Quirks
    } else if public_starts("-//W3C//DTD XHTML 1.0 Frameset//")
        || public_starts("-//W3C//DTD XHTML 1.0 Transitional//")
        || (!system_missing_or_empty
            && (public_starts("-//W3C//DTD HTML 4.01 Frameset//")
                || public_starts("-//W3C//DTD HTML 4.01 Transitional//")))
    {
        QuirksMode::LimitedQuirks
    } else {
        QuirksMode::NoQuirks
    }
}

/// Whether `token` is a start tag named one of `names`.
fn is_start(token: &Token, names: &[&str]) -> bool {
    matches!(token, Token::StartTag { name, .. } if names.contains(&name.as_str()))
}

/// Whether `token` is an end tag named one of `names`.
fn is_end(token: &Token, names: &[&str]) -> bool {
    matches!(token, Token::EndTag { name } if names.contains(&name.as_str()))
}

/// Whether `token` is a character token for ASCII whitespace.
fn is_whitespace_character(token: &Token) -> bool {
    matches!(token, Token::Character(c) if is_whitespace(*c))
}

/// The elements whose end tags "in body" closes by name, as the standard lists them under "An
/// end tag whose tag name is one of: "address", …".
const BLOCK_END_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "button",
    "center",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "header",
    "hgroup",
    "listing",
    "main",
    "menu",
    "nav",
    "ol",
    "pre",
    "search",
    "section",
    "select",
    "summary",
    "ul",
];

/// The heading elements, `h1` to `h6`, as in body's start tag and end tag rules for them list
/// them.
const HEADINGS: &[&str] = &["h1", "h2", "h3", "h4", "h5", "h6"];

/// The formatting elements (13.2.4.2).
const FORMATTING: &[&str] = &[
    "a", "b", "big", "code", "em", "font", "i", "nobr", "s", "small", "strike", "strong", "tt", "u",
];

impl TreeBuilder {
    /// 13.2.6.4.1 The "initial" insertion mode.
    pub(super) fn initial(&mut self, token: &Token) -> Flow {
        match token {
            t if is_whitespace_character(t) => Flow::Done,
            Token::Comment { data } => {
                let document = self.document();
                self.insert_comment(data, Some(document));
                Flow::Done
            }
            Token::ProcessingInstruction { target, data } => {
                let document = self.document();
                self.insert_processing_instruction(target, data, Some(document));
                Flow::Done
            }
            Token::Doctype {
                name,
                public_id,
                system_id,
                force_quirks,
            } => {
                let document = self.document();
                let doctype = NodeData::Doctype {
                    name: name.clone().unwrap_or_default(),
                    public_id: public_id.clone().unwrap_or_default(),
                    system_id: system_id.clone().unwrap_or_default(),
                };
                if self.tree.children(document).iter().all(|&c| {
                    self.visit();
                    !matches!(
                        self.tree.data(c),
                        NodeData::Doctype { .. } | NodeData::Element { .. }
                    )
                }) && self.admit(1, stored_bytes_of(&doctype))
                {
                    self.tree.insert_new(document, doctype, None, &self.steps);
                }
                self.quirks_mode = doctype_quirks_mode(
                    name.as_deref(),
                    public_id.as_deref(),
                    system_id.as_deref(),
                    *force_quirks,
                );
                self.mode = Mode::BeforeHtml;
                Flow::Done
            }
            _ => {
                self.quirks_mode = QuirksMode::Quirks;
                self.mode = Mode::BeforeHtml;
                Flow::Reprocess
            }
        }
    }

    /// 13.2.6.4.2 The "before html" insertion mode.
    pub(super) fn before_html(&mut self, token: &Token) -> Flow {
        match token {
            Token::Doctype { .. } => Flow::Done,
            Token::Comment { data } => {
                let document = self.document();
                self.insert_comment(data, Some(document));
                Flow::Done
            }
            Token::ProcessingInstruction { target, data } => {
                let document = self.document();
                self.insert_processing_instruction(target, data, Some(document));
                Flow::Done
            }
            t if is_whitespace_character(t) => Flow::Done,
            Token::StartTag {
                name, attributes, ..
            } if name == "html" => {
                self.insert_html_root(name, attributes);
                self.mode = Mode::BeforeHead;
                Flow::Done
            }
            Token::EndTag { name } if !matches!(name.as_str(), "head" | "body" | "html" | "br") => {
                Flow::Done
            }
            _ => {
                self.insert_html_root("html", &[]);
                self.mode = Mode::BeforeHead;
                Flow::Reprocess
            }
        }
    }

    /// The `html` element of the before html mode: created for the token, appended to the
    /// document and put on the stack of open elements, when the limits admit it.
    fn insert_html_root(&mut self, name: &str, attributes: &[TokenAttribute]) {
        if !self.room_on_stack() {
            return;
        }
        let Some(element) = self.create_element(name, attributes) else {
            return;
        };
        let document = self.document();
        self.insert_element_at(element, (document, None));
        self.open.push(element);
    }

    /// 13.2.6.4.3 The "before head" insertion mode.
    pub(super) fn before_head(&mut self, token: &Token) -> Flow {
        match token {
            t if is_whitespace_character(t) => Flow::Done,
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
            } if name == "head" => {
                self.head = self.insert_html_element(name, attributes);
                self.mode = Mode::InHead;
                Flow::Done
            }
            Token::EndTag { name } if !matches!(name.as_str(), "head" | "body" | "html" | "br") => {
                Flow::Done
            }
            _ => {
                self.head = self.insert_html_element("head", &[]);
                self.mode = Mode::InHead;
                Flow::Reprocess
            }
        }
    }

    /// 13.2.6.4.4 The "in head" insertion mode. The `template` start and end tag rules are
    /// story `html-tree-template`'s: until then those tokens go to in body's "any other start
    /// tag" and "any other end tag".
    pub(super) fn in_head(&mut self, token: &Token) -> Flow {
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
            } if matches!(name.as_str(), "base" | "basefont" | "bgsound" | "link") => {
                self.insert_void_element(name, attributes);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "meta" => {
                if self.insert_void_element(name, attributes) {
                    self.meta_changes_the_encoding(attributes);
                }
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "title" => {
                self.parse_text_element(name, attributes, false);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if (name == "noscript" && !self.scripting_disabled)
                || name == "noframes"
                || name == "style" =>
            {
                self.parse_text_element(name, attributes, true);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "noscript" => {
                self.insert_html_element(name, attributes);
                self.mode = Mode::InHeadNoscript;
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "script" => {
                let (parent, reference) = self.appropriate_place(None);
                if !self.room_on_stack() {
                    return Flow::Done;
                }
                let Some(element) = self.create_element(name, attributes) else {
                    return Flow::Done;
                };
                self.insert_element_at(element, (parent, reference));
                self.open.push(element);
                self.tokenizer.switch_to(tokenizer::State::ScriptData);
                self.original_mode = self.mode;
                self.mode = Mode::Text;
                Flow::Done
            }
            t if is_end(t, &["head"]) => {
                self.open.pop();
                self.mode = Mode::AfterHead;
                Flow::Done
            }
            t if is_end(t, &["body", "html", "br"]) => self.in_head_anything_else(),
            Token::StartTag {
                name, attributes, ..
            } if name == "template" => {
                self.any_other_start_tag(name, attributes);
                Flow::Done
            }
            Token::EndTag { name } if name == "template" => {
                self.any_other_end_tag(name);
                Flow::Done
            }
            t if is_start(t, &["head"]) => Flow::Done,
            Token::EndTag { .. } => Flow::Done,
            _ => self.in_head_anything_else(),
        }
    }

    fn in_head_anything_else(&mut self) -> Flow {
        self.open.pop();
        self.mode = Mode::AfterHead;
        Flow::Reprocess
    }

    /// 13.2.6.4.5 The "in head noscript" insertion mode.
    pub(super) fn in_head_noscript(&mut self, token: &Token) -> Flow {
        match token {
            Token::Doctype { .. } => Flow::Done,
            t if is_start(t, &["html"]) => self.in_body(t),
            t if is_end(t, &["noscript"]) => {
                self.open.pop();
                self.mode = Mode::InHead;
                Flow::Done
            }
            t if is_whitespace_character(t)
                || matches!(
                    t,
                    Token::Comment { .. } | Token::ProcessingInstruction { .. }
                )
                || is_start(
                    t,
                    &["basefont", "bgsound", "link", "meta", "noframes", "style"],
                ) =>
            {
                self.in_head(t)
            }
            t if is_end(t, &["br"]) => self.in_head_noscript_anything_else(),
            t if is_start(t, &["head", "noscript"]) => Flow::Done,
            Token::EndTag { .. } => Flow::Done,
            _ => self.in_head_noscript_anything_else(),
        }
    }

    fn in_head_noscript_anything_else(&mut self) -> Flow {
        self.open.pop();
        self.mode = Mode::InHead;
        Flow::Reprocess
    }

    /// 13.2.6.4.6 The "after head" insertion mode.
    pub(super) fn after_head(&mut self, token: &Token) -> Flow {
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
            } if name == "body" => {
                self.insert_html_element(name, attributes);
                self.frameset_ok = false;
                self.mode = Mode::InBody;
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "frameset" => {
                self.insert_html_element(name, attributes);
                self.mode = Mode::InFrameset;
                Flow::Done
            }
            t if is_start(
                t,
                &[
                    "base", "basefont", "bgsound", "link", "meta", "noframes", "script", "style",
                    "template", "title",
                ],
            ) =>
            {
                // The head element pointer is set after head: before head set it, or a limit
                // refused the `head` and parsing stopped.
                let Some(head) = self.head else {
                    return Flow::Done;
                };
                self.open.push(head);
                let flow = self.in_head(t);
                self.remove_from_stack(head);
                flow
            }
            t if is_end(t, &["template"]) => self.in_head(t),
            t if is_end(t, &["body", "html", "br"]) => self.after_head_anything_else(),
            t if is_start(t, &["head"]) => Flow::Done,
            Token::EndTag { .. } => Flow::Done,
            _ => self.after_head_anything_else(),
        }
    }

    fn after_head_anything_else(&mut self) -> Flow {
        self.insert_html_element("body", &[]);
        self.frameset_ok = true;
        self.mode = Mode::InBody;
        Flow::Reprocess
    }

    /// 13.2.6.4.7 The "in body" insertion mode, bounded as `NOT_YET_PARSED` says: the `table`,
    /// `select`, `svg` and `math` start tags are processed by "any other start tag". The end
    /// tags of the formatting elements, an `a` start tag with an `a` in the list and a `nobr`
    /// start tag with a `nobr` in scope run the adoption agency algorithm (`formatting`).
    pub(super) fn in_body(&mut self, token: &Token) -> Flow {
        match token {
            Token::Character(0) => Flow::Done,
            Token::Character(c) if is_whitespace(*c) => {
                self.reconstruct_active_formatting_elements();
                self.insert_character(*c);
                Flow::Done
            }
            Token::Character(c) => {
                self.reconstruct_active_formatting_elements();
                self.insert_character(*c);
                self.frameset_ok = false;
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
            Token::StartTag {
                name, attributes, ..
            } if name == "html" => {
                if !self.template_on_stack() {
                    let html = self.open[0];
                    self.merge_attributes(html, attributes);
                }
                Flow::Done
            }
            t if is_start(
                t,
                &[
                    "base", "basefont", "bgsound", "link", "meta", "noframes", "script", "style",
                    "template", "title",
                ],
            ) || is_end(t, &["template"]) =>
            {
                self.in_head(t)
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "body" => {
                if self.open.len() > 1
                    && self.is_html(self.open[1], &["body"])
                    && !self.template_on_stack()
                {
                    self.frameset_ok = false;
                    let body = self.open[1];
                    self.merge_attributes(body, attributes);
                }
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "frameset" => {
                if self.open.len() == 1 || !self.is_html(self.open[1], &["body"]) {
                    return Flow::Done;
                }
                if !self.frameset_ok {
                    return Flow::Done;
                }
                let body = self.open[1];
                self.tree.remove(body, &self.steps);
                self.open.truncate(1);
                self.insert_html_element(name, attributes);
                self.mode = Mode::InFrameset;
                Flow::Done
            }
            Token::EndOfFile => {
                self.stop_parsing();
                Flow::Done
            }
            t if is_end(t, &["body"]) => {
                if self.has_named_in_scope("body", Scope::Default) {
                    self.mode = Mode::AfterBody;
                }
                Flow::Done
            }
            t if is_end(t, &["html"]) => {
                if self.has_named_in_scope("body", Scope::Default) {
                    self.mode = Mode::AfterBody;
                    return Flow::Reprocess;
                }
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if matches!(
                name.as_str(),
                "address"
                    | "article"
                    | "aside"
                    | "blockquote"
                    | "center"
                    | "details"
                    | "dialog"
                    | "dir"
                    | "div"
                    | "dl"
                    | "fieldset"
                    | "figcaption"
                    | "figure"
                    | "footer"
                    | "header"
                    | "hgroup"
                    | "main"
                    | "menu"
                    | "nav"
                    | "ol"
                    | "p"
                    | "search"
                    | "section"
                    | "summary"
                    | "ul"
            ) =>
            {
                self.close_a_p_element_in_button_scope();
                self.insert_html_element(name, attributes);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if HEADINGS.contains(&name.as_str()) => {
                self.close_a_p_element_in_button_scope();
                if self.current_is(HEADINGS) {
                    self.open.pop();
                }
                self.insert_html_element(name, attributes);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "pre" || name == "listing" => {
                self.close_a_p_element_in_button_scope();
                self.insert_html_element(name, attributes);
                self.ignore_line_feed = true;
                self.frameset_ok = false;
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "form" => {
                // The parser is never parsing template contents: `template` is story
                // `html-tree-template`'s, and its start tag is "any other start tag" until then.
                if self.form.is_some() && !self.template_on_stack() {
                    return Flow::Done;
                }
                self.close_a_p_element_in_button_scope();
                if let Some(form) = self.insert_html_element(name, attributes)
                    && !self.template_on_stack()
                {
                    self.form = Some(form);
                }
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "li" => {
                self.frameset_ok = false;
                for index in (0..self.open.len()).rev() {
                    self.visit();
                    let node = self.open[index];
                    if self.is_html(node, &["li"]) {
                        self.generate_implied_end_tags(Some("li"));
                        self.pop_until(&["li"]);
                        break;
                    }
                    if self.is_special(node) && !self.is_html(node, &["address", "div", "p"]) {
                        break;
                    }
                }
                self.close_a_p_element_in_button_scope();
                self.insert_html_element(name, attributes);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "dd" || name == "dt" => {
                self.frameset_ok = false;
                for index in (0..self.open.len()).rev() {
                    self.visit();
                    let node = self.open[index];
                    if let Some(found @ ("dd" | "dt")) = self.html_name(node) {
                        let found = if found == "dd" { "dd" } else { "dt" };
                        self.generate_implied_end_tags(Some(found));
                        self.pop_until(&[found]);
                        break;
                    }
                    if self.is_special(node) && !self.is_html(node, &["address", "div", "p"]) {
                        break;
                    }
                }
                self.close_a_p_element_in_button_scope();
                self.insert_html_element(name, attributes);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "plaintext" => {
                self.close_a_p_element_in_button_scope();
                self.insert_html_element(name, attributes);
                self.tokenizer.switch_to(tokenizer::State::Plaintext);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "button" => {
                if self.has_named_in_scope("button", Scope::Default) {
                    self.generate_implied_end_tags(None);
                    self.pop_until(&["button"]);
                }
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(name, attributes);
                self.frameset_ok = false;
                Flow::Done
            }
            Token::EndTag { name } if BLOCK_END_TAGS.contains(&name.as_str()) => {
                if self.has_named_in_scope(name, Scope::Default) {
                    self.generate_implied_end_tags(None);
                    self.pop_until(&[name]);
                }
                Flow::Done
            }
            t if is_end(t, &["form"]) => {
                // Not parsing template contents (see the `form` start tag).
                if !self.template_on_stack() {
                    let node = self.form.take();
                    let Some(node) = node else {
                        return Flow::Done;
                    };
                    if !self.has_in_scope(Scope::Default, |_, n| n == node) {
                        return Flow::Done;
                    }
                    self.generate_implied_end_tags(None);
                    self.remove_from_stack(node);
                } else if self.has_named_in_scope("form", Scope::Default) {
                    self.generate_implied_end_tags(None);
                    self.pop_until(&["form"]);
                }
                Flow::Done
            }
            t if is_end(t, &["p"]) => {
                if !self.has_named_in_scope("p", Scope::Button)
                    && self.insert_html_element("p", &[]).is_none()
                {
                    return Flow::Done;
                }
                self.close_a_p_element();
                Flow::Done
            }
            t if is_end(t, &["li"]) => {
                if self.has_named_in_scope("li", Scope::ListItem) {
                    self.generate_implied_end_tags(Some("li"));
                    self.pop_until(&["li"]);
                }
                Flow::Done
            }
            Token::EndTag { name } if name == "dd" || name == "dt" => {
                if self.has_named_in_scope(name, Scope::Default) {
                    self.generate_implied_end_tags(Some(name));
                    self.pop_until(&[name]);
                }
                Flow::Done
            }
            Token::EndTag { name } if HEADINGS.contains(&name.as_str()) => {
                if self.has_in_scope(Scope::Default, |b, n| b.is_html(n, HEADINGS)) {
                    self.generate_implied_end_tags(None);
                    self.pop_until(HEADINGS);
                }
                Flow::Done
            }
            Token::EndTag { name } if name == "sarcasm" => {
                self.any_other_end_tag(name);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "a" => {
                // 1. An `a` in the list after the last marker: a parse error, the adoption
                // agency algorithm, then that `a` out of the list and the stack if still there.
                if let Some((_, existing_a, _)) = self.last_active_named("a") {
                    self.adoption_agency("a");
                    // "If existingA is in the list", it is after the last marker: the algorithm
                    // adds no marker and moves no entry, it only removes and replaces. It takes
                    // existingA off the stack only with its entry (steps 4.8, 4.25 and 4.26),
                    // and at step 4.4 it was not on the stack; so "if existingA is in the stack
                    // of open elements" can hold only while its entry is left (the algorithm
                    // returned at step 2 or 4.5), and only then is the stack scanned, from the
                    // top.
                    if let Some(at) = self.active_position_after_last_marker(existing_a) {
                        self.active.remove(at);
                        self.remove_from_stack(existing_a);
                    }
                }
                // 2.
                self.reconstruct_active_formatting_elements();
                // 3.
                self.insert_formatting_element(name, attributes);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "nobr" => {
                self.reconstruct_active_formatting_elements();
                // A `nobr` in scope: a parse error, the adoption agency algorithm, and the
                // reconstruction once again.
                if self.has_named_in_scope("nobr", Scope::Default) {
                    self.adoption_agency("nobr");
                    self.reconstruct_active_formatting_elements();
                }
                self.insert_formatting_element(name, attributes);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if FORMATTING.contains(&name.as_str()) => {
                self.reconstruct_active_formatting_elements();
                self.insert_formatting_element(name, attributes);
                Flow::Done
            }
            Token::EndTag { name } if FORMATTING.contains(&name.as_str()) => {
                self.adoption_agency(name);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if matches!(name.as_str(), "applet" | "marquee" | "object") => {
                self.reconstruct_active_formatting_elements();
                if self.insert_html_element(name, attributes).is_some() {
                    self.active.push(Entry::Marker);
                }
                self.frameset_ok = false;
                Flow::Done
            }
            Token::EndTag { name } if matches!(name.as_str(), "applet" | "marquee" | "object") => {
                if self.has_named_in_scope(name, Scope::Default) {
                    self.generate_implied_end_tags(None);
                    self.pop_until(&[name]);
                    self.clear_active_formatting_elements_to_last_marker();
                }
                Flow::Done
            }
            t if is_end(t, &["br"]) => Flow::Replace(Token::StartTag {
                name: "br".to_string(),
                attributes: Vec::new(),
                self_closing: false,
            }),
            Token::StartTag {
                name, attributes, ..
            } if matches!(
                name.as_str(),
                "area" | "br" | "embed" | "img" | "keygen" | "wbr"
            ) =>
            {
                self.reconstruct_active_formatting_elements();
                self.insert_void_element(name, attributes);
                self.frameset_ok = false;
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "input" => {
                // The clauses for a `select` context element and a `select` in scope are story
                // `html-tree-tables`'s: written as if there is none.
                self.reconstruct_active_formatting_elements();
                self.insert_void_element(name, attributes);
                let hidden = attributes
                    .iter()
                    .find(|a| a.name == "type")
                    .is_some_and(|a| a.value.eq_ignore_ascii_case("hidden"));
                if !hidden {
                    self.frameset_ok = false;
                }
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if matches!(name.as_str(), "param" | "source" | "track") => {
                self.insert_void_element(name, attributes);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "hr" => {
                // The clause for a `select` in scope is story `html-tree-tables`'s.
                self.close_a_p_element_in_button_scope();
                self.insert_void_element(name, attributes);
                self.frameset_ok = false;
                Flow::Done
            }
            Token::StartTag {
                name,
                attributes,
                self_closing,
            } if name == "image" => Flow::Replace(Token::StartTag {
                name: "img".to_string(),
                attributes: attributes.clone(),
                self_closing: *self_closing,
            }),
            Token::StartTag {
                name, attributes, ..
            } if name == "textarea" => {
                self.insert_html_element(name, attributes);
                self.ignore_line_feed = true;
                self.tokenizer.switch_to(tokenizer::State::Rcdata);
                self.original_mode = self.mode;
                self.frameset_ok = false;
                self.mode = Mode::Text;
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "xmp" => {
                self.close_a_p_element_in_button_scope();
                self.reconstruct_active_formatting_elements();
                self.frameset_ok = false;
                self.parse_text_element(name, attributes, true);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "iframe" => {
                self.frameset_ok = false;
                self.parse_text_element(name, attributes, true);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "noembed" || (name == "noscript" && !self.scripting_disabled) => {
                self.parse_text_element(name, attributes, true);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "option" => {
                // The clause for a `select` in scope is story `html-tree-tables`'s.
                if self.current_is(&["option"]) {
                    self.open.pop();
                }
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(name, attributes);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "optgroup" => {
                // The clause for a `select` in scope is story `html-tree-tables`'s.
                if self.current_is(&["option"]) {
                    self.open.pop();
                }
                self.reconstruct_active_formatting_elements();
                self.insert_html_element(name, attributes);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "rb" || name == "rtc" => {
                if self.has_named_in_scope("ruby", Scope::Default) {
                    self.generate_implied_end_tags(None);
                }
                self.insert_html_element(name, attributes);
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "rp" || name == "rt" => {
                if self.has_named_in_scope("ruby", Scope::Default) {
                    self.generate_implied_end_tags(Some("rtc"));
                }
                self.insert_html_element(name, attributes);
                Flow::Done
            }
            t if is_start(
                t,
                &[
                    "caption", "col", "colgroup", "frame", "head", "tbody", "td", "tfoot", "th",
                    "thead", "tr",
                ],
            ) =>
            {
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } => {
                self.any_other_start_tag(name, attributes);
                Flow::Done
            }
            Token::EndTag { name } => {
                self.any_other_end_tag(name);
                Flow::Done
            }
        }
    }

    /// Whether a `template` element is on the stack of open elements.
    fn template_on_stack(&self) -> bool {
        self.open.iter().any(|&n| {
            self.visit();
            self.is_html(n, &["template"])
        })
    }

    /// Insert an HTML element for a token and pop it off the stack of open elements at once:
    /// whether a limit admitted it.
    fn insert_void_element(&mut self, name: &str, attributes: &[TokenAttribute]) -> bool {
        let inserted = self.insert_html_element(name, attributes).is_some();
        if inserted {
            self.open.pop();
        }
        inserted
    }

    /// Insert an HTML element for a formatting element's start tag token and push it onto the
    /// list of active formatting elements, when a limit admits it.
    fn insert_formatting_element(&mut self, name: &str, attributes: &[TokenAttribute]) {
        if let Some(element) = self.insert_html_element(name, attributes) {
            self.push_active_formatting_element(element, name, attributes);
        }
    }

    /// In body's "any other start tag".
    fn any_other_start_tag(&mut self, name: &str, attributes: &[TokenAttribute]) {
        self.reconstruct_active_formatting_elements();
        self.insert_html_element(name, attributes);
    }

    /// In body's "any other end tag", which step 4.3 of the adoption agency algorithm also runs:
    /// the element it popped down to, `None` when it ignored the token.
    pub(super) fn any_other_end_tag(&mut self, name: &str) -> Option<NodeId> {
        let len = self.open.len();
        for index in (0..len).rev() {
            let node = self.open[index];
            if self.html_name(node) == Some(name) {
                self.add_steps(len - index);
                self.generate_implied_end_tags(Some(name));
                self.open.truncate(index);
                return Some(node);
            }
            if self.is_special(node) {
                self.add_steps(len - index);
                return None;
            }
        }
        self.add_steps(len);
        None
    }

    /// "If the stack of open elements has a `p` element in button scope, then close a `p`
    /// element."
    fn close_a_p_element_in_button_scope(&mut self) {
        if self.has_named_in_scope("p", Scope::Button) {
            self.close_a_p_element();
        }
    }

    /// Close a `p` element.
    fn close_a_p_element(&mut self) {
        self.generate_implied_end_tags(Some("p"));
        self.pop_until(&["p"]);
    }

    /// 13.2.6.4.8 The "text" insertion mode. A `script` end tag pops the element and returns to
    /// the original insertion mode: no script is prepared or run.
    pub(super) fn text(&mut self, token: &Token) -> Flow {
        match token {
            Token::Character(c) => {
                self.insert_character(*c);
                Flow::Done
            }
            Token::EndOfFile => {
                self.open.pop();
                self.mode = self.original_mode;
                Flow::Reprocess
            }
            _ => {
                // An end tag whose tag name is "script", and any other end tag; the tokenizer
                // emits no other token in the text states.
                self.open.pop();
                self.mode = self.original_mode;
                Flow::Done
            }
        }
    }

    /// 13.2.6.4.17 The "after body" insertion mode.
    pub(super) fn after_body(&mut self, token: &Token) -> Flow {
        match token {
            t if is_whitespace_character(t) => self.in_body(t),
            Token::Comment { data } => {
                let html = self.open[0];
                self.insert_comment(data, Some(html));
                Flow::Done
            }
            Token::ProcessingInstruction { target, data } => {
                let html = self.open[0];
                self.insert_processing_instruction(target, data, Some(html));
                Flow::Done
            }
            Token::Doctype { .. } => Flow::Done,
            t if is_start(t, &["html"]) => self.in_body(t),
            t if is_end(t, &["html"]) => {
                self.mode = Mode::AfterAfterBody;
                Flow::Done
            }
            Token::EndOfFile => {
                self.stop_parsing();
                Flow::Done
            }
            _ => {
                self.mode = Mode::InBody;
                Flow::Reprocess
            }
        }
    }

    /// 13.2.6.4.18 The "in frameset" insertion mode.
    pub(super) fn in_frameset(&mut self, token: &Token) -> Flow {
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
            } if name == "frameset" => {
                self.insert_html_element(name, attributes);
                Flow::Done
            }
            t if is_end(t, &["frameset"]) => {
                if self.open.len() > 1 {
                    self.open.pop();
                    if !self.current_is(&["frameset"]) {
                        self.mode = Mode::AfterFrameset;
                    }
                }
                Flow::Done
            }
            Token::StartTag {
                name, attributes, ..
            } if name == "frame" => {
                self.insert_void_element(name, attributes);
                Flow::Done
            }
            t if is_start(t, &["noframes"]) => self.in_head(t),
            Token::EndOfFile => {
                self.stop_parsing();
                Flow::Done
            }
            _ => Flow::Done,
        }
    }

    /// 13.2.6.4.19 The "after frameset" insertion mode.
    pub(super) fn after_frameset(&mut self, token: &Token) -> Flow {
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
            t if is_end(t, &["html"]) => {
                self.mode = Mode::AfterAfterFrameset;
                Flow::Done
            }
            t if is_start(t, &["noframes"]) => self.in_head(t),
            Token::EndOfFile => {
                self.stop_parsing();
                Flow::Done
            }
            _ => Flow::Done,
        }
    }

    /// 13.2.6.4.20 The "after after body" insertion mode.
    pub(super) fn after_after_body(&mut self, token: &Token) -> Flow {
        match token {
            Token::Comment { data } => {
                let document = self.document();
                self.insert_comment(data, Some(document));
                Flow::Done
            }
            Token::ProcessingInstruction { target, data } => {
                let document = self.document();
                self.insert_processing_instruction(target, data, Some(document));
                Flow::Done
            }
            t if matches!(t, Token::Doctype { .. })
                || is_whitespace_character(t)
                || is_start(t, &["html"]) =>
            {
                self.in_body(t)
            }
            Token::EndOfFile => {
                self.stop_parsing();
                Flow::Done
            }
            _ => {
                self.mode = Mode::InBody;
                Flow::Reprocess
            }
        }
    }

    /// 13.2.6.4.21 The "after after frameset" insertion mode.
    pub(super) fn after_after_frameset(&mut self, token: &Token) -> Flow {
        match token {
            Token::Comment { data } => {
                let document = self.document();
                self.insert_comment(data, Some(document));
                Flow::Done
            }
            Token::ProcessingInstruction { target, data } => {
                let document = self.document();
                self.insert_processing_instruction(target, data, Some(document));
                Flow::Done
            }
            t if matches!(t, Token::Doctype { .. })
                || is_whitespace_character(t)
                || is_start(t, &["html"]) =>
            {
                self.in_body(t)
            }
            Token::EndOfFile => {
                self.stop_parsing();
                Flow::Done
            }
            t if is_start(t, &["noframes"]) => self.in_head(t),
            _ => Flow::Done,
        }
    }
}
