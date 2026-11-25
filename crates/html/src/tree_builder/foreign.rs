//! Foreign content (story `html-tree-foreign`): the part of the tree construction dispatcher
//! that sends a token to the rules for foreign content (13.2.6, before 13.2.6.1), those rules
//! (13.2.6.5), the MathML text integration points and HTML integration points, and the
//! adjustments of tag names and attributes (13.2.6.1: adjust MathML attributes, adjust SVG
//! attributes, adjust foreign attributes, and the SVG tag name table of 13.2.6.5).
//!
//! Scripts do not run: an SVG `script` end tag in foreign content pops the element, and the SVG
//! script element is not processed (the design note's `open` line `svg-script-processing`).

use super::{Flow, TreeBuilder, is_whitespace};
use crate::tokenizer::{Token, TokenAttribute};
use crate::tree::{Attribute, Categories, Namespace, NodeData, NodeId};

/// The start tags that break out of foreign content (13.2.6.5); `font` breaks out only with a
/// `color`, `face` or `size` attribute.
const BREAKOUT: &[&str] = &[
    "b",
    "big",
    "blockquote",
    "body",
    "br",
    "center",
    "code",
    "dd",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "hr",
    "i",
    "img",
    "li",
    "listing",
    "menu",
    "meta",
    "nobr",
    "ol",
    "p",
    "pre",
    "ruby",
    "s",
    "small",
    "span",
    "strong",
    "strike",
    "sub",
    "sup",
    "table",
    "tt",
    "u",
    "ul",
    "var",
];

/// The SVG element names whose case 13.2.6.5 fixes: the token's tag name, then the element's.
const SVG_TAG_NAMES: &[(&str, &str)] = &[
    ("altglyph", "altGlyph"),
    ("altglyphdef", "altGlyphDef"),
    ("altglyphitem", "altGlyphItem"),
    ("animatecolor", "animateColor"),
    ("animatemotion", "animateMotion"),
    ("animatetransform", "animateTransform"),
    ("clippath", "clipPath"),
    ("feblend", "feBlend"),
    ("fecolormatrix", "feColorMatrix"),
    ("fecomponenttransfer", "feComponentTransfer"),
    ("fecomposite", "feComposite"),
    ("feconvolvematrix", "feConvolveMatrix"),
    ("fediffuselighting", "feDiffuseLighting"),
    ("fedisplacementmap", "feDisplacementMap"),
    ("fedistantlight", "feDistantLight"),
    ("fedropshadow", "feDropShadow"),
    ("feflood", "feFlood"),
    ("fefunca", "feFuncA"),
    ("fefuncb", "feFuncB"),
    ("fefuncg", "feFuncG"),
    ("fefuncr", "feFuncR"),
    ("fegaussianblur", "feGaussianBlur"),
    ("feimage", "feImage"),
    ("femerge", "feMerge"),
    ("femergenode", "feMergeNode"),
    ("femorphology", "feMorphology"),
    ("feoffset", "feOffset"),
    ("fepointlight", "fePointLight"),
    ("fespecularlighting", "feSpecularLighting"),
    ("fespotlight", "feSpotLight"),
    ("fetile", "feTile"),
    ("feturbulence", "feTurbulence"),
    ("foreignobject", "foreignObject"),
    ("glyphref", "glyphRef"),
    ("lineargradient", "linearGradient"),
    ("radialgradient", "radialGradient"),
    ("textpath", "textPath"),
];

/// The SVG attribute names whose case "adjust SVG attributes" fixes.
const SVG_ATTRIBUTES: &[(&str, &str)] = &[
    ("attributename", "attributeName"),
    ("attributetype", "attributeType"),
    ("basefrequency", "baseFrequency"),
    ("baseprofile", "baseProfile"),
    ("calcmode", "calcMode"),
    ("clippathunits", "clipPathUnits"),
    ("diffuseconstant", "diffuseConstant"),
    ("edgemode", "edgeMode"),
    ("filterunits", "filterUnits"),
    ("glyphref", "glyphRef"),
    ("gradienttransform", "gradientTransform"),
    ("gradientunits", "gradientUnits"),
    ("kernelmatrix", "kernelMatrix"),
    ("kernelunitlength", "kernelUnitLength"),
    ("keypoints", "keyPoints"),
    ("keysplines", "keySplines"),
    ("keytimes", "keyTimes"),
    ("lengthadjust", "lengthAdjust"),
    ("limitingconeangle", "limitingConeAngle"),
    ("markerheight", "markerHeight"),
    ("markerunits", "markerUnits"),
    ("markerwidth", "markerWidth"),
    ("maskcontentunits", "maskContentUnits"),
    ("maskunits", "maskUnits"),
    ("numoctaves", "numOctaves"),
    ("pathlength", "pathLength"),
    ("patterncontentunits", "patternContentUnits"),
    ("patterntransform", "patternTransform"),
    ("patternunits", "patternUnits"),
    ("pointsatx", "pointsAtX"),
    ("pointsaty", "pointsAtY"),
    ("pointsatz", "pointsAtZ"),
    ("preservealpha", "preserveAlpha"),
    ("preserveaspectratio", "preserveAspectRatio"),
    ("primitiveunits", "primitiveUnits"),
    ("refx", "refX"),
    ("refy", "refY"),
    ("repeatcount", "repeatCount"),
    ("repeatdur", "repeatDur"),
    ("requiredextensions", "requiredExtensions"),
    ("requiredfeatures", "requiredFeatures"),
    ("specularconstant", "specularConstant"),
    ("specularexponent", "specularExponent"),
    ("spreadmethod", "spreadMethod"),
    ("startoffset", "startOffset"),
    ("stddeviation", "stdDeviation"),
    ("stitchtiles", "stitchTiles"),
    ("surfacescale", "surfaceScale"),
    ("systemlanguage", "systemLanguage"),
    ("tablevalues", "tableValues"),
    ("targetx", "targetX"),
    ("targety", "targetY"),
    ("textlength", "textLength"),
    ("viewbox", "viewBox"),
    ("viewtarget", "viewTarget"),
    ("xchannelselector", "xChannelSelector"),
    ("ychannelselector", "yChannelSelector"),
    ("zoomandpan", "zoomAndPan"),
];

/// The attributes "adjust foreign attributes" puts in a namespace: the name on the token, the
/// prefix, the local name and the namespace.
const FOREIGN_ATTRIBUTES: &[(&str, Option<&str>, &str, Namespace)] = &[
    ("xlink:actuate", Some("xlink"), "actuate", Namespace::XLink),
    ("xlink:arcrole", Some("xlink"), "arcrole", Namespace::XLink),
    ("xlink:href", Some("xlink"), "href", Namespace::XLink),
    ("xlink:role", Some("xlink"), "role", Namespace::XLink),
    ("xlink:show", Some("xlink"), "show", Namespace::XLink),
    ("xlink:title", Some("xlink"), "title", Namespace::XLink),
    ("xlink:type", Some("xlink"), "type", Namespace::XLink),
    ("xml:lang", Some("xml"), "lang", Namespace::Xml),
    ("xml:space", Some("xml"), "space", Namespace::Xml),
    ("xmlns", None, "xmlns", Namespace::Xmlns),
    ("xmlns:xlink", Some("xmlns"), "xlink", Namespace::Xmlns),
];

/// The element name an SVG start tag's name becomes.
pub(super) fn svg_tag_name(name: &str) -> &str {
    SVG_TAG_NAMES
        .iter()
        .find(|(token, _)| *token == name)
        .map_or(name, |(_, element)| element)
}

/// Whether the attributes a start tag gave a MathML `annotation-xml` make it an HTML
/// integration point (13.2.6): an `encoding` of `text/html` or `application/xhtml+xml` in any
/// ASCII case. No adjustment renames `encoding`.
pub(super) fn encoding_is_html(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|a| {
        a.namespace.is_none()
            && a.local_name == "encoding"
            && (a.value.eq_ignore_ascii_case("text/html")
                || a.value.eq_ignore_ascii_case("application/xhtml+xml"))
    })
}

/// The attributes an element in `namespace` gets for a token's attributes: for MathML, adjust
/// MathML attributes and adjust foreign attributes; for SVG, adjust SVG attributes and adjust
/// foreign attributes; for HTML, the token's as they are. Each rule that creates an element in
/// a foreign namespace runs exactly the adjustments of that namespace.
pub(super) fn element_attributes(namespace: Namespace, token: &[TokenAttribute]) -> Vec<Attribute> {
    token
        .iter()
        .map(|a| {
            let plain = |local_name: &str| Attribute {
                namespace: None,
                prefix: None,
                local_name: local_name.to_string(),
                value: a.value.clone(),
            };
            match namespace {
                Namespace::Html => plain(&a.name),
                _ => {
                    if let Some(&(_, prefix, local_name, ns)) =
                        FOREIGN_ATTRIBUTES.iter().find(|(name, ..)| *name == a.name)
                    {
                        return Attribute {
                            namespace: Some(ns),
                            prefix: prefix.map(str::to_string),
                            local_name: local_name.to_string(),
                            value: a.value.clone(),
                        };
                    }
                    match namespace {
                        Namespace::MathMl if a.name == "definitionurl" => plain("definitionURL"),
                        Namespace::Svg => plain(
                            SVG_ATTRIBUTES
                                .iter()
                                .find(|(token, _)| *token == a.name)
                                .map_or(a.name.as_str(), |(_, element)| element),
                        ),
                        _ => plain(&a.name),
                    }
                }
            }
        })
        .collect()
}

impl TreeBuilder {
    /// The adjusted current node: the context element while the stack holds only the `html`
    /// element of a fragment, else the current node; `None` with an empty stack.
    pub(super) fn adjusted_current_node(&self) -> Option<NodeId> {
        match (self.open.len(), self.context) {
            (1, Some(context)) => Some(context),
            (_, _) => self.open.last().copied(),
        }
    }

    /// The namespace and local name of the element `node`.
    fn element_name(&self, node: NodeId) -> Option<(Namespace, &str)> {
        match self.tree.data(node) {
            NodeData::Element {
                namespace,
                local_name,
                ..
            } => Some((*namespace, local_name.as_str())),
            _ => None,
        }
    }

    /// Whether `node` is a MathML text integration point: MathML `mi`, `mo`, `mn`, `ms` or
    /// `mtext`.
    fn is_mathml_text_integration_point(&self, node: NodeId) -> bool {
        matches!(
            self.element_name(node),
            Some((Namespace::MathMl, "mi" | "mo" | "mn" | "ms" | "mtext"))
        )
    }

    /// Whether `node` is an HTML integration point: a MathML `annotation-xml` whose start tag
    /// had an `encoding` of `text/html` or `application/xhtml+xml` in any ASCII case, which
    /// the element's categories keep from the token when it is created
    /// ([`Categories::HTML_INTEGRATION_POINT`]), so a hook's later write to the attribute
    /// changes nothing; or an SVG `foreignObject`, `desc` or `title`. The context element of a
    /// fragment is created with no attribute, as its "fake" start tag token has none here.
    fn is_html_integration_point(&self, node: NodeId) -> bool {
        match self.tree.data(node) {
            NodeData::Element {
                namespace: Namespace::MathMl,
                local_name,
                ..
            } if local_name == "annotation-xml" => self
                .tree
                .categories(node)
                .has(Categories::HTML_INTEGRATION_POINT),
            NodeData::Element {
                namespace: Namespace::Svg,
                local_name,
                ..
            } => matches!(local_name.as_str(), "foreignObject" | "desc" | "title"),
            _ => false,
        }
    }

    /// The tree construction dispatcher's choice: whether `token` goes to the rules for foreign
    /// content rather than to those of the current insertion mode.
    pub(super) fn in_foreign_content(&self, token: &Token) -> bool {
        let Some(node) = self.adjusted_current_node() else {
            return false;
        };
        let Some((namespace, local_name)) = self.element_name(node) else {
            return false;
        };
        if namespace == Namespace::Html {
            return false;
        }
        let start = |names: &[&str]| matches!(token, Token::StartTag { name, .. } if !names.contains(&name.as_str()));
        if self.is_mathml_text_integration_point(node)
            && (start(&["mglyph", "malignmark"]) || matches!(token, Token::Character(_)))
        {
            return false;
        }
        if namespace == Namespace::MathMl
            && local_name == "annotation-xml"
            && matches!(token, Token::StartTag { name, .. } if name == "svg")
        {
            return false;
        }
        if self.is_html_integration_point(node)
            && matches!(token, Token::StartTag { .. } | Token::Character(_))
        {
            return false;
        }
        !matches!(token, Token::EndOfFile)
    }

    /// Whether the tokenizer may read a CDATA section: the adjusted current node is an element
    /// that is not in the HTML namespace (13.2.5.42).
    pub(super) fn cdata_allowed(&self) -> bool {
        self.adjusted_current_node()
            .and_then(|node| self.element_name(node))
            .is_some_and(|(namespace, _)| namespace != Namespace::Html)
    }

    /// Insert a foreign element for a start tag in `namespace`, its name and attributes
    /// adjusted as that namespace's rules say, and pop it again when the token is self-closing.
    pub(super) fn insert_foreign_start_tag(
        &mut self,
        namespace: Namespace,
        name: &str,
        attributes: &[TokenAttribute],
        self_closing: bool,
    ) {
        let name = if namespace == Namespace::Svg {
            svg_tag_name(name)
        } else {
            name
        };
        if self.insert_element(namespace, name, attributes).is_some() && self_closing {
            // An SVG `script` acknowledges the flag and is then ended as by its end tag: popped,
            // and not processed.
            self.open.pop();
        }
    }

    /// 13.2.6.5 The rules for parsing tokens in foreign content.
    pub(super) fn foreign_content(&mut self, token: &Token) -> Flow {
        match token {
            Token::Character(0) => {
                self.insert_character(0xFFFD);
                Flow::Done
            }
            Token::Character(c) => {
                self.insert_character(*c);
                if !is_whitespace(*c) {
                    self.frameset_ok = false;
                }
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
            } if BREAKOUT.contains(&name.as_str())
                || (name == "font"
                    && attributes
                        .iter()
                        .any(|a| matches!(a.name.as_str(), "color" | "face" | "size"))) =>
            {
                self.break_out_of_foreign_content(token)
            }
            Token::EndTag { name } if name == "br" || name == "p" => {
                self.break_out_of_foreign_content(token)
            }
            Token::StartTag {
                name,
                attributes,
                self_closing,
            } => {
                let namespace = self
                    .adjusted_current_node()
                    .and_then(|node| self.element_name(node))
                    .map_or(Namespace::Html, |(namespace, _)| namespace);
                self.insert_foreign_start_tag(namespace, name, attributes, *self_closing);
                Flow::Done
            }
            Token::EndTag { name }
                if name == "script"
                    && self.open.last().is_some_and(|&n| {
                        self.element_name(n) == Some((Namespace::Svg, "script"))
                    }) =>
            {
                self.open.pop();
                Flow::Done
            }
            Token::EndTag { name } => self.foreign_end_tag(token, name),
            Token::EndOfFile => self.process_by_mode(token),
        }
    }

    /// The breakout of foreign content: pop until the current node is a MathML text
    /// integration point, an HTML integration point or an HTML element, then process the token
    /// by the rules of the current insertion mode.
    fn break_out_of_foreign_content(&mut self, token: &Token) -> Flow {
        while let Some(&node) = self.open.last() {
            if self.spent(0) {
                return Flow::Done;
            }
            self.visit();
            if self.is_mathml_text_integration_point(node)
                || self.is_html_integration_point(node)
                || self
                    .element_name(node)
                    .is_some_and(|(ns, _)| ns == Namespace::Html)
            {
                break;
            }
            self.open.pop();
        }
        self.process_by_mode(token)
    }

    /// The "any other end tag" rule of foreign content.
    fn foreign_end_tag(&mut self, token: &Token, name: &str) -> Flow {
        let mut index = self.open.len();
        while index > 0 && !self.spent(0) {
            index -= 1;
            self.visit();
            let node = self.open[index];
            // Loop: the topmost element ends the scan (fragment case).
            if index == 0 {
                return Flow::Done;
            }
            if self
                .element_name(node)
                .is_some_and(|(_, local_name)| local_name.eq_ignore_ascii_case(name))
            {
                self.open.truncate(index);
                return Flow::Done;
            }
            let above = self.open[index - 1];
            if self
                .element_name(above)
                .is_some_and(|(ns, _)| ns == Namespace::Html)
            {
                return self.process_by_mode(token);
            }
        }
        Flow::Done
    }
}
