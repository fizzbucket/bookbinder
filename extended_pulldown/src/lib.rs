//! This crate extends `pulldown_cmark` to do the following:
//!
//!  - smarten quotes according to a more complex but substantially slower algorithm
//!   than that used in `pulldown_cmark` versions greater than 8.0
//!  - substitute unicode en-dashes, em-dashes and ellipsis for `--`, `---` and `...`.
//!  - allow multiple-paragraph footnotes by interpreting an indented and unlabelled code block
//!    within a footnote as text to be parsed again.
//!  - allow several new tags:
//!        
//!    * Sans
//!    * Centred
//!    * Right-aligned
//!    * Small caps
//!    * Subscript
//!    * Superscript
//!
//! It also provides a function, `flatten_footnotes`,
//! which replaces footnote references and definitions with a
//! single group of tagged text; this allows
//! rendering to targets like LaTeX which need a footnote to be
//! defined at the point to which it refers. It inserts empty footnotes where
//! a definition is missing.
//!
//! In general, this crate mimics the structs and methods of `pulldown_cmark`.
//! However its more complex conception of markdown comes at the cost
//! of much slower parsing.
//! It is therefore not recommended to use instead of `pulldown_cmark`
//! except where this complexity is required.
//!
//! The markdown syntax to use is otherwise essentially that of CommonMark
//! togther with `pulldown_cmark`'s extensions.
//!
//!
//! # Examples
//!
//! ## Inline Spans
//!
//! These are parsed preferentially from html spans:
//!
//! ```
//! use extended_pulldown::Parser;
//! use extended_pulldown::Event::*;
//! use extended_pulldown::Tag::*;
//!
//! let text = concat!(r#"<span class="sans">Sans text</span>"#,
//! r#"<span class="centred">Centred text</span>"#,
//! r#"<span class="right-aligned">Right-aligned text</span>"#,
//! r#"<span class="smallcaps">Small caps text</span>"#,
//! r#"<span class="subscript">Subscript text</span>"#,
//! r#"<span class="superscript">Superscript text</span>"#);
//!    
//! let parsed = Parser::new(text)
//!     .collect::<Vec<_>>();
//! let expected = vec![
//!     Start(Paragraph),
//!     Start(Sans),
//!     Text("Sans text".into()),
//!     End(Sans),
//!     Start(Centred),
//!     Text("Centred text".into()),
//!     End(Centred),
//!     Start(RightAligned),
//!     Text("Right-aligned text".into()),
//!     End(RightAligned),
//!     Start(SmallCaps),
//!     Text("Small caps text".into()),
//!     End(SmallCaps),
//!     Start(Subscript),
//!     Text("Subscript text".into()),
//!     End(Subscript),
//!     Start(Superscript),
//!     Text("Superscript text".into()),
//!     End(Superscript),
//!     End(Paragraph)
//! ];
//!  assert_eq!(parsed, expected);
//! ```
//!
//! However, markdown syntax is also extended slightly,
//! to allow wrapping a span of alphanumeric text in `^` to indicate superscript
//! and in `~` to indicate subscript: `25^th^ July`, `H~2~O`.
//!
//! ## Multipara footnotes
//!
//! ```
//! use extended_pulldown::Parser;
//! use extended_pulldown::Event::*;
//! use extended_pulldown::Tag::*;
//! use pulldown_cmark::CodeBlockKind::Indented;
//! let text = "Hello World[^footnote]\n\n[^footnote]:\n\tA footnote\n\n\tIn *multiple* pieces";
//! let output = Parser::new(text)
//!     .collect::<Vec<_>>();
//! let pulldown_output = vec![
//!     Start(Paragraph),
//!     Text("Hello World".into()),
//!     FootnoteReference("footnote".into()),
//!     End(Paragraph),
//!     Start(FootnoteDefinition("footnote".into())),
//!     Start(CodeBlock(Indented)),
//!     Text("A footnote\n\n".into()),
//!     Text("In *multiple* pieces".into()),
//!     End(CodeBlock(Indented)),
//!     End(FootnoteDefinition("footnote".into()))
//! ];
//! let extended_pulldown_output = vec![
//!     Start(Paragraph),
//!     Text("Hello World".into()),
//!     FootnoteReference("footnote".into()),
//!     End(Paragraph),
//!     Start(FootnoteDefinition("footnote".into())),
//!     Start(Paragraph),
//!     Text("A footnote".into()),
//!     End(Paragraph),
//!     Start(Paragraph),
//!     Text("In ".into()),
//!     Start(Emphasis),
//!     Text("multiple".into()),
//!     End(Emphasis),
//!     Text(" pieces".into()),
//!     End(Paragraph),
//!     End(FootnoteDefinition("footnote".into()))
//! ];
//! assert!(output != pulldown_output);
//! assert_eq!(output, extended_pulldown_output);
//! ```
//!
//! ## Flattening footnotes
//!
//! ```
//! use extended_pulldown::Event::*;
//! use extended_pulldown::Tag;
//!
//! let events = vec![
//!   Start(Tag::Paragraph),
//!   Text("Hello".into()),
//!   FootnoteReference("1".into()),
//!  End(Tag::Paragraph),
//!   Start(Tag::FootnoteDefinition("1".into())),
//!   Start(Tag::Paragraph),
//!   Text("World".into()),
//!  End(Tag::Paragraph),
//!   End(Tag::FootnoteDefinition("1".into())),
//! ];
//!
//! let flattened = extended_pulldown::flatten_footnotes(events);
//! let expected = vec![
//!   Start(Tag::Paragraph),
//!   Text("Hello".into()),
//!   Start(Tag::FlattenedFootnote),
//!   Text("World".into()),
//!   End(Tag::FlattenedFootnote),
//!   End(Tag::Paragraph)
//!];
//!
//! assert_eq!(flattened, expected);
//! ```
//!
//#![deny(dead_code)]
#![deny(unreachable_patterns)]
#![deny(unused_extern_crates)]
#![deny(unused_imports)]
#![deny(unused_qualifications)]
#![deny(clippy::all)]
#![deny(missing_docs)]
#![deny(variant_size_differences)]

use pulldown_cmark::Event as PulldownEvent;
use pulldown_cmark::Options as PulldownOptions;
use pulldown_cmark::Tag as PulldownTag;
use pulldown_cmark::{Alignment, LinkType};
pub use pulldown_cmark::{CodeBlockKind, CowStr, InlineStr};
use std::collections::HashMap;
mod parsing;
mod quotes;
pub use parsing::{InlineParser, Parser};
use std::convert::TryFrom;
mod sub_and_superscript;

/// Options for rendering
#[derive(Debug)]
pub struct Options {
    /// include footnotes
    enable_footnotes: bool,
    /// Replace punctuation and use curly quotes
    /// instead of straight
    smarten: bool,
    // Include GFM tables
    //enable_tables: bool,
    // Include GFM tasklists
    //enable_tasklists: bool,
    /// Include strikethrough
    enable_strikethrough: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            enable_footnotes: true,
            smarten: true,
            //enable_tables: false,
            //enable_tasklists: false,
            enable_strikethrough: false,
        }
    }
}

impl From<Options> for PulldownOptions {
    fn from(src: Options) -> Self {
        let mut options = PulldownOptions::empty();
        if src.enable_strikethrough {
            options.insert(PulldownOptions::ENABLE_STRIKETHROUGH);
        }
        if src.enable_footnotes {
            options.insert(PulldownOptions::ENABLE_FOOTNOTES);
        }
        // if src.enable_tables {
        // 	options.insert(PulldownOptions::ENABLE_TABLES);
        // }
        // if src.enable_tasklists {
        // 	options.insert(PulldownOptions::ENABLE_TASKLISTS);
        // }
        options
    }
}

/// A markdown event
#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    /// Start of a tagged element. Events that are yielded after this event and before its corresponding End event are inside this element. Start and end events are guaranteed to be balanced.
    Start(Tag<'a>),
    /// End of a tagged element
    End(Tag<'a>),
    /// Text
    Text(CowStr<'a>),
    /// Inline code
    Code(CowStr<'a>),
    /// Reference to a footnote
    FootnoteReference(CowStr<'a>),
    /// A soft line break ('\n')
    SoftBreak,
    /// A hard line break, corresponding to `<br/>` in html
    HardBreak,
    /// A horizontal rule
    Rule,
    /// A html node
    Html(CowStr<'a>),
    /// A tasklist marker, rendered as a checkbox in html;
    /// an inner value of true indicates that it is checked.
    TaskListMarker(bool),
}

impl<'a> From<Event<'a>> for PulldownEvent<'a> {
    fn from(event: Event<'a>) -> Self {
        match event {
            Event::Start(Tag::Sans) => PulldownEvent::Html("<span class=\"sans\">".into()),
            Event::Start(Tag::Centred) => PulldownEvent::Html("<span class=\"centred\">".into()),
            Event::Start(Tag::SmallCaps) => {
                PulldownEvent::Html("<span class=\"smallcaps\">".into())
            }
            Event::Start(Tag::RightAligned) => {
                PulldownEvent::Html("<span class=\"right-aligned\">".into())
            }
            Event::Start(Tag::Superscript) => {
                PulldownEvent::Html("<span class=\"superscript\">".into())
            }
            Event::Start(Tag::Subscript) => {
                PulldownEvent::Html("<span class=\"subscript\">".into())
            }
            Event::End(Tag::Sans) => PulldownEvent::Html("</span>".into()),
            Event::End(Tag::Centred) => PulldownEvent::Html("</span>".into()),
            Event::End(Tag::SmallCaps) => PulldownEvent::Html("</span>".into()),
            Event::End(Tag::RightAligned) => PulldownEvent::Html("</span>".into()),
            Event::End(Tag::Superscript) => PulldownEvent::Html("</span>".into()),
            Event::End(Tag::Subscript) => PulldownEvent::Html("</span>".into()),
            Event::Start(t) => PulldownEvent::Start(PulldownTag::try_from(t).unwrap()),
            Event::End(t) => PulldownEvent::End(PulldownTag::try_from(t).unwrap()),
            Event::Text(t) => PulldownEvent::Text(t),
            Event::Code(c) => PulldownEvent::Code(c),
            Event::FootnoteReference(f) => PulldownEvent::FootnoteReference(f),
            Event::SoftBreak => PulldownEvent::SoftBreak,
            Event::HardBreak => PulldownEvent::HardBreak,
            Event::Rule => PulldownEvent::Rule,
            Event::Html(h) => PulldownEvent::Html(h),
            Event::TaskListMarker(b) => PulldownEvent::TaskListMarker(b),
        }
    }
}

// note that this is not a nice complete transition;
// it's just a rough conversion, so for example, html
// won't be parsed into spans, quotations are left ambiguous, etc
impl<'a> From<PulldownEvent<'a>> for Event<'a> {
    fn from(src: PulldownEvent<'a>) -> Event<'a> {
        match src {
            PulldownEvent::Start(t) => Event::Start(t.into()),
            PulldownEvent::End(t) => Event::End(t.into()),
            PulldownEvent::Text(t) => Event::Text(t),
            PulldownEvent::Code(t) => Event::Code(t),
            PulldownEvent::Html(t) => Event::Html(t),
            PulldownEvent::FootnoteReference(t) => Event::FootnoteReference(t),
            PulldownEvent::SoftBreak => Event::SoftBreak,
            PulldownEvent::HardBreak => Event::HardBreak,
            PulldownEvent::Rule => Event::Rule,
            PulldownEvent::TaskListMarker(t) => Event::TaskListMarker(t),
        }
    }
}

impl<'a> From<PulldownTag<'a>> for Tag<'a> {
    fn from(src: PulldownTag<'a>) -> Tag<'a> {
        match src {
            PulldownTag::Paragraph => Tag::Paragraph,
            PulldownTag::Heading(x) => Tag::Heading(x),
            PulldownTag::BlockQuote => Tag::BlockQuote,
            PulldownTag::CodeBlock(x) => Tag::CodeBlock(x),
            PulldownTag::List(x) => Tag::List(x),
            PulldownTag::Item => Tag::Item,
            PulldownTag::FootnoteDefinition(x) => Tag::FootnoteDefinition(x),
            PulldownTag::Table(x) => Tag::Table(x),
            PulldownTag::TableHead => Tag::TableHead,
            PulldownTag::TableRow => Tag::TableRow,
            PulldownTag::TableCell => Tag::TableCell,
            PulldownTag::Emphasis => Tag::Emphasis,
            PulldownTag::Strong => Tag::Strong,
            PulldownTag::Link(a, b, c) => Tag::Link(a, b, c),
            PulldownTag::Image(a, b, c) => Tag::Image(a, b, c),
            PulldownTag::Strikethrough => Tag::Strikethrough,
        }
    }
}

impl<'a> TryFrom<Tag<'a>> for PulldownTag<'a> {
    type Error = ();
    fn try_from(src: Tag<'a>) -> Result<Self, Self::Error> {
        match src {
            Tag::Paragraph => Ok(PulldownTag::Paragraph),
            Tag::Heading(x) => Ok(PulldownTag::Heading(x)),
            Tag::BlockQuote => Ok(PulldownTag::BlockQuote),
            Tag::BlockQuotation => Ok(PulldownTag::BlockQuote),
            Tag::CodeBlock(x) => Ok(PulldownTag::CodeBlock(x)),
            Tag::List(x) => Ok(PulldownTag::List(x)),
            Tag::Item => Ok(PulldownTag::Item),
            Tag::FootnoteDefinition(x) => Ok(PulldownTag::FootnoteDefinition(x)),
            Tag::Table(x) => Ok(PulldownTag::Table(x)),
            Tag::TableHead => Ok(PulldownTag::TableHead),
            Tag::TableRow => Ok(PulldownTag::TableRow),
            Tag::TableCell => Ok(PulldownTag::TableCell),
            Tag::Emphasis => Ok(PulldownTag::Emphasis),
            Tag::Strong => Ok(PulldownTag::Strong),
            Tag::Link(a, b, c) => Ok(PulldownTag::Link(a, b, c)),
            Tag::Image(a, b, c) => Ok(PulldownTag::Image(a, b, c)),
            _ => Err(()),
        }
    }
}

/// A tag containing other events
#[derive(Debug, Clone, PartialEq)]
pub enum Tag<'a> {
    /// A paragraph of text and other inline elements
    Paragraph,
    /// A heading. The field indicates the level of the heading.
    Heading(u32),
    /// A block quote to be rendered as a `quote` in latex
    BlockQuote,
    /// A block quote to be rendered as a `quotation` in latex
    BlockQuotation,
    /// A code block
    CodeBlock(CodeBlockKind<'a>),
    /// A list. If the list is ordered the field indicates the number of the first item. Contains only list items.
    List(Option<u64>),
    /// A list item
    Item,
    /// The definition of a footnote
    FootnoteDefinition(CowStr<'a>),
    /// A table. Contains a vector describing the text-alignment for each of its columns.
    Table(Vec<Alignment>),
    /// A table header. Contains only `TableRows`. Note that the table body starts immediately after the closure of the `TableHead` tag. There is no `TableBody` tag.
    TableHead,
    /// A table row. Is used both for header rows as body rows. Contains only `TableCells`.
    TableRow,
    /// An individual table cell
    TableCell,
    /// Emphasised text
    Emphasis,
    /// Strong (bold) text
    Strong,
    /// An image. The first field is the link type, the second the destination URL and the third is a title.
    Link(LinkType, CowStr<'a>, CowStr<'a>),
    /// A link. The first field is the link type, the second the destination URL and the third is a title.
    Image(LinkType, CowStr<'a>, CowStr<'a>),
    /// Struck through text
    Strikethrough,
    // additions begin here
    /// Sans text
    Sans,
    /// Centred text
    Centred,
    /// Text in small caps
    SmallCaps,
    /// Text that is aligned right
    RightAligned,
    /// Superscript text
    Superscript,
    /// Subscript text
    Subscript,
    /// A flattened footnote produced by `flatten_footnotes`
    FlattenedFootnote,
    /// A paragraph without an initial indent
    UnindentedParagraph,
}

trait BoundaryMarker {
    fn resets_quotes(&self) -> bool;
}

impl BoundaryMarker for PulldownEvent<'_> {
    /// whether this event means that any quotes must necessarily be broken
    fn resets_quotes(&self) -> bool {
        use PulldownEvent::*;
        match self {
            Rule => true,
            Text(_) => false,
            Code(_) | Html(_) | FootnoteReference(_) => false,
            SoftBreak | HardBreak | TaskListMarker(_) => false,
            Start(PulldownTag::Emphasis) => false,
            Start(PulldownTag::Strong) => false,
            Start(PulldownTag::Link(_, _, _)) => false,
            Start(PulldownTag::Image(_, _, _)) => false,
            End(PulldownTag::Emphasis) => false,
            End(PulldownTag::Strong) => false,
            End(PulldownTag::Link(_, _, _)) => false,
            End(PulldownTag::Image(_, _, _)) => false,
            Start(_) => true,
            End(_) => true,
        }
    }
}

/// Make a markdown event static; i.e. no longer pinned to the lifetime of the str used to produce it
pub trait MakeStatic {
    /// This type should simply be the original type with a static lifetime,
    /// but has to be represented in this way to work around language limitations
    type AsStatic;
    /// transform this event
    fn into_static(self) -> Self::AsStatic;
}

impl MakeStatic for CowStr<'_> {
    type AsStatic = CowStr<'static>;
    fn into_static(self) -> Self::AsStatic {
        match self {
            CowStr::Boxed(b) => CowStr::Boxed(b),
            CowStr::Inlined(i) => CowStr::Inlined(i),
            CowStr::Borrowed(s) => s.to_string().into(),
        }
    }
}

impl<'a> MakeStatic for PulldownTag<'a> {
    type AsStatic = PulldownTag<'static>;
    fn into_static(self) -> Self::AsStatic {
        use PulldownTag::*;

        match self {
            CodeBlock(x) => CodeBlock(x.into_static()),
            List(x) => List(x),
            Item => Item,
            FootnoteDefinition(x) => FootnoteDefinition(x.into_static()),
            Table(x) => Table(x),
            TableHead => TableHead,
            TableRow => TableRow,
            TableCell => TableCell,
            Emphasis => Emphasis,
            Strong => Strong,
            Link(a, b, c) => Link(a, b.into_static(), c.into_static()),
            Image(a, b, c) => Image(a, b.into_static(), c.into_static()),
            Paragraph => Paragraph,
            Heading(x) => Heading(x),
            BlockQuote => BlockQuote,
            Strikethrough => Strikethrough,
        }
    }
}

impl MakeStatic for PulldownEvent<'_> {
    type AsStatic = PulldownEvent<'static>;
    fn into_static(self) -> Self::AsStatic {
        use PulldownEvent::*;
        match self {
            Text(t) => Text(t.into_static()),
            Start(t) => Start(t.into_static()),
            End(t) => End(t.into_static()),
            Code(c) => Code(c.into_static()),
            FootnoteReference(f) => FootnoteReference(f.into_static()),
            SoftBreak => SoftBreak,
            HardBreak => HardBreak,
            Rule => Rule,
            Html(h) => Html(h.into_static()),
            TaskListMarker(b) => TaskListMarker(b),
        }
    }
}

impl MakeStatic for CodeBlockKind<'_> {
    type AsStatic = CodeBlockKind<'static>;
    fn into_static(self) -> Self::AsStatic {
        match self {
            CodeBlockKind::Indented => CodeBlockKind::Indented,
            CodeBlockKind::Fenced(l) => CodeBlockKind::Fenced(l.into_static()),
        }
    }
}

impl MakeStatic for Tag<'_> {
    type AsStatic = Tag<'static>;
    fn into_static(self) -> Self::AsStatic {
        use Tag::*;
        match self {
            CodeBlock(x) => CodeBlock(x.into_static()),
            List(x) => List(x),
            Item => Item,
            FootnoteDefinition(x) => FootnoteDefinition(x.into_static()),
            Table(x) => Table(x),
            TableHead => TableHead,
            TableRow => TableRow,
            TableCell => TableCell,
            Emphasis => Emphasis,
            Strong => Strong,
            Link(a, b, c) => Link(a, b.into_static(), c.into_static()),
            Image(a, b, c) => Image(a, b.into_static(), c.into_static()),
            Sans => Sans,
            Centred => Centred,
            SmallCaps => SmallCaps,
            RightAligned => RightAligned,
            Superscript => Superscript,
            Subscript => Subscript,
            FlattenedFootnote => FlattenedFootnote,
            Paragraph => Paragraph,
            Heading(x) => Heading(x),
            BlockQuote => BlockQuote,
            BlockQuotation => BlockQuotation,
            Strikethrough => Strikethrough,
            UnindentedParagraph => UnindentedParagraph,
        }
    }
}

impl MakeStatic for Event<'_> {
    type AsStatic = Event<'static>;
    fn into_static(self) -> Self::AsStatic {
        use Event::*;
        match self {
            Text(t) => Text(t.into_static()),
            Start(t) => Start(t.into_static()),
            End(t) => End(t.into_static()),
            Code(c) => Code(c.into_static()),
            FootnoteReference(f) => FootnoteReference(f.into_static()),
            SoftBreak => SoftBreak,
            HardBreak => HardBreak,
            Rule => Rule,
            Html(h) => Html(h.into_static()),
            TaskListMarker(b) => TaskListMarker(b),
        }
    }
}

/// Replace `Event::FootnoteReference(f)` and a seperate definition `Event::Start(Tag::FootnoteDefinition(f))...Event::End(Tag::FootnoteDefinition(f))`
/// with (at the point where `Event::FootnoteReference(f)` was) `Event::Start(Tag::FlattenedFootnote)...Event::End(Tag::FlattenedFootnote)`
///
/// If a footnote reference has no definition, an empty string of text will be inserted instead.
/// # Example
///
/// ```
/// use extended_pulldown::Event::*;
/// use extended_pulldown::Tag;
///
/// let events = vec![
///   Start(Tag::Paragraph),
///   Text("Hello".into()),
///   FootnoteReference("1".into()),
///   End(Tag::Paragraph),
///   Start(Tag::FootnoteDefinition("1".into())),
///   Start(Tag::Paragraph),
///   Text("World".into()),
///   End(Tag::Paragraph),
///   End(Tag::FootnoteDefinition("1".into())),
/// ];
///
/// let flattened = extended_pulldown::flatten_footnotes(events);
/// let expected = vec![
///   Start(Tag::Paragraph),
///   Text("Hello".into()),
///   Start(Tag::FlattenedFootnote),
///   Text("World".into()),
///   End(Tag::FlattenedFootnote),
///   End(Tag::Paragraph)
///];
///
/// assert_eq!(flattened, expected);
/// ```
pub fn flatten_footnotes<'a, I>(src: I) -> Vec<Event<'a>>
where
    I: IntoIterator<Item = Event<'a>>,
{
    let mut non_footnotes = Vec::new();
    let mut footnotes = HashMap::new();

    let mut definitions_len = 0;

    let mut fb = Vec::new();
    let mut in_footnote = false;
    for event in src {
        match event {
            Event::Start(Tag::FootnoteDefinition(_)) => {
                in_footnote = true;
            }
            Event::End(Tag::FootnoteDefinition(d)) => {
                in_footnote = false;
                let mut definition = std::mem::take(&mut fb);
                if let (Some(Event::Start(Tag::Paragraph)), Some(Event::End(Tag::Paragraph))) =
                    (definition.first(), definition.last())
                {
                    definition.remove(0);
                    definition.pop();
                }
                definitions_len += definition.len() + 1;
                footnotes.insert(d, definition);
            }
            other => {
                if in_footnote {
                    fb.push(other);
                } else {
                    non_footnotes.push(other);
                }
            }
        }
    }

    let mut out = Vec::with_capacity(non_footnotes.len() + definitions_len);
    for event in non_footnotes.into_iter() {
        match event {
            Event::FootnoteReference(f) => match footnotes.remove(&f) {
                Some(mut definition) => {
                    out.push(Event::Start(Tag::FlattenedFootnote));
                    out.append(&mut definition);
                    out.push(Event::End(Tag::FlattenedFootnote));
                }
                None => {
                    out.push(Event::Start(Tag::FlattenedFootnote));
                    out.push(Event::Text("".into()));
                    out.push(Event::End(Tag::FlattenedFootnote));
                }
            },
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_boundaries() {
        let markdown = "Pre 'Hello\nWorld' Post";
        let smart = smart_markdown(markdown);
        assert_eq!(smart, "Pre ‘Hello\nWorld’ Post");
        let a = "She wrote: 'It will be delightful. I am ready to do
anything, anything for you. It is a glorious idea. I know the wife of a
very high personage in the Administration, and also a man who has lots
of influence with,' etc.";
        let smart = smart_markdown(a);
        let expected = "She wrote: ‘It will be delightful. I am ready to do
anything, anything for you. It is a glorious idea. I know the wife of a
very high personage in the Administration, and also a man who has lots
of influence with,’ etc.";
        assert_eq!(smart, expected);
        let b = "'When Mr. Kurtz,' I continued, severely, 'is
General Manager, you won't have the opportunity.'";
        let smart = smart_markdown(b);
        let expected = "‘When Mr. Kurtz,’ I continued, severely, ‘is
General Manager, you won't have the opportunity.’";
        assert_eq!(smart, expected);
        let c = "A blinding sunlight drowned all this at times
in a sudden recrudescence of glare. 'There's your Company's station,'
said the Swede, pointing to three wooden barrack-like structures on the
rocky slope. 'I will send your things up. Four boxes did you say? So.
Farewell.'";
        let smart = smart_markdown(c);
        let expected = "A blinding sunlight drowned all this at times
in a sudden recrudescence of glare. ‘There's your Company's station,’
said the Swede, pointing to three wooden barrack-like structures on the
rocky slope. ‘I will send your things up. Four boxes did you say? So.
Farewell.’";
        assert_eq!(smart, expected);
    }

    #[test]
    fn prev_integration_test_failures() {
        let a = "then--you see--I felt somehow
I must get there by hook or by crook. The men said
'My dear fellow,' and did nothing.";
        let smart = smart_markdown(a);
        let expected = "then–you see–I felt somehow
I must get there by hook or by crook. The men said
‘My dear fellow,’ and did nothing.";
        assert_eq!(smart, expected);
        let a = "He lifted a warning forefinger....
'*Du calme, du calme*.'";
        let expected = "He lifted a warning forefinger….
‘*Du calme, du calme*.’";
        let smart = smart_markdown(a);
        assert_eq!(smart, expected);

        let a = "'catch 'im. Give 'im to us.'";
        let expected = "‘catch 'im. Give 'im to us.’";
        let smart = smart_markdown(a);
        assert_eq!(smart, expected);
    }

    /// smarten markdown by turning a handful of latex glyphs into
    /// unicode characters, and by attempting to replace straight with curly quotes,
    /// with a very dodgy writer to turn them back into a string
    fn smart_markdown(markdown: &str) -> String {
        let parser = Parser::new(markdown).map(PulldownEvent::from);
        let mut out = String::new();

        use PulldownEvent::*;

        for event in parser {
            match event {
                Text(t) => out.push_str(&t),
                Start(PulldownTag::Paragraph) => {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                }
                End(PulldownTag::Paragraph) => {
                    out.push_str("\n");
                }
                Start(PulldownTag::Emphasis) | End(PulldownTag::Emphasis) => {
                    out.push('*');
                }
                Start(PulldownTag::CodeBlock(_)) => out.push_str("\n````\n"),
                End(PulldownTag::CodeBlock(_)) => out.push_str("````\n"),
                SoftBreak => out.push_str("\n"),
                e => {
                    println!("{:?}", e);
                    panic!()
                }
            }
        }

        out.trim_end().to_string()
    }

    #[test]
    fn tricky_quotes() {
        let markdown = "'I'd like to see some of that 70's style again,' Patrick O'Postrophe said, 'even though it's '20.'";
        let smart = smart_markdown(markdown);
        assert_eq!(smart, "‘I'd like to see some of that 70's style again,’ Patrick O'Postrophe said, ‘even though it's '20.’");

        let a = smart_markdown("'Hmm. 'Tis all one, Robert Post's child.'");
        let c = smart_markdown("'Gossip on Forsyte 'Change was not restrained.'");

        assert_eq!(a, "‘Hmm. 'Tis all one, Robert Post's child.’");
        assert_eq!(c, "‘Gossip on Forsyte 'Change was not restrained.’");
    }

    #[test]
    fn forgotten_closing_quote_does_not_extend_over_para_boundaries() {
        let with_break = "'He's so meticulous\n\nThere was a pause... 'If you're sure.'";
        let smart = smart_markdown(with_break);
        assert_eq!(
            smart,
            "‘He's so meticulous\n\nThere was a pause… ‘If you're sure.’"
        );
    }

    #[test]
    fn galsworthy() {
        let markdown = "'E'en so many years later, 'tis an item of gossip on Forsyte 'Change that I'd marry 'im yet.'";
        let smart = smart_markdown(markdown);
        assert_eq!(smart, "‘E'en so many years later, 'tis an item of gossip on Forsyte 'Change that I'd marry 'im yet.’");
    }

    #[test]
    fn leave_verbatim_alone() {
        let markdown = "'Hello World' is a traditional first program. Here it is in Python:\n\n```\nprint(\"Hello World\")\n```\n\nThat's nice.";
        let smart = smart_markdown(markdown);
        assert_eq!(smart, "‘Hello World’ is a traditional first program. Here it is in Python:\n\n````\nprint(\"Hello World\")\n````\n\nThat's nice.");
    }

    #[test]
    fn multi_para_open_quote() {
        // lots of old texts use a single double quote at the opening of a paragraph for reported speech. Check that we don't close such:
        let text = "\"A\n\n\"B";
        let smart = smart_markdown(text);
        assert_eq!(smart, "“A\n\n“B");
    }

    #[test]
    fn double_and_single_confluence() {
        let a = "'It's---after all---the season, e'en if the situation is a *little* complicated,' he said. 'My mother always said \"Say something nice if you can.\"'";
        let smart = smart_markdown(a);
        let expected = "‘It's—after all—the season, e'en if the situation is a *little* complicated,’ he said. ‘My mother always said “Say something nice if you can.”’";
        assert_eq!(smart, expected);
    }

    #[test]
    fn quote_transformation() {
        let markdown = "'This isn't that clever,' she said. 'No, \"Real cleverness would understand semantics, not stacks\" --- as Hiram Maxim didn't quite get around to saying.'\n\n'It'll just have to do,' he replied.";
        let smart = smart_markdown(markdown);
        assert_eq!("‘This isn't that clever,’ she said. ‘No, “Real cleverness would understand semantics, not stacks” — as Hiram Maxim didn't quite get around to saying.’\n\n‘It'll just have to do,’ he replied.", smart);
    }

    #[test]
    fn simple_as() {
        let markdown = "'Hello World,' he said.";
        let smart = smart_markdown(markdown);
        assert_eq!(smart, "‘Hello World,’ he said.");
    }

    #[test]
    fn apostrophe_after_opening_quote() {
        let markdown = "''Tis after all the season, e'en if the situation is a *little* complicated,' he said.";
        let smart = smart_markdown(markdown);
        assert_eq!("‘'Tis after all the season, e'en if the situation is a *little* complicated,’ he said.", smart);
    }

    #[test]
    fn special_spans() {
        let text = "<span class=\"sans\">Hello</span> <span class=\"smallcaps\">World</span>";

        let a = Parser::new(text).collect::<Vec<_>>();
        use Event::*;
        use Tag::*;

        let expected_a = vec![
            Start(Paragraph),
            Start(Sans),
            Text("Hello".into()),
            End(Sans),
            Text(" ".into()),
            Start(SmallCaps),
            Text("World".into()),
            End(SmallCaps),
            End(Paragraph),
        ];
        assert_eq!(a, expected_a);
    }

    #[test]
    fn stacked_special_spans() {
        let text = "<span class=\"sans\"><span class=\"inner\">Hello's</span></span> <span class=\"smallcaps\">World</span>";
        let b = Parser::new(text).collect::<Vec<_>>();
        use Event::*;
        use Tag::*;
        let expected_b = vec![
            Start(Paragraph),
            Start(Sans),
            Html("<span class=\"inner\">".into()),
            Text("Hello's".into()),
            Html("</span>".into()),
            End(Sans),
            Text(" ".into()),
            Start(SmallCaps),
            Text("World".into()),
            End(SmallCaps),
            End(Paragraph),
        ];
        assert_eq!(b, expected_b);
    }

    #[test]
    fn multi_para_footnotes() {
        let text = "Hello World[^footnote]\n\n[^footnote]:\n\tA footnote\n\n\tIn *multiple* pieces";
        let p = Parser::new(text).collect::<Vec<_>>();
        use Event::*;
        use Tag::*;

        let expected = vec![
            Start(Paragraph),
            Text("Hello World".into()),
            FootnoteReference("footnote".into()),
            End(Paragraph),
            Start(FootnoteDefinition("footnote".into())),
            Start(Paragraph),
            Text("A footnote".into()),
            End(Paragraph),
            Start(Paragraph),
            Text("In ".into()),
            Start(Emphasis),
            Text("multiple".into()),
            End(Emphasis),
            Text(" pieces".into()),
            End(Paragraph),
            End(FootnoteDefinition("footnote".into())),
        ];

        assert_eq!(p, expected);
    }

    #[test]
    fn super_and_sub() {
        let valid_superscripts = Parser::new("'Quoted.' a^bc^d a^hello^").collect::<Vec<_>>();
        let invalid_superscripts =
            Parser::new("'Quoted.' a^^ a^With space^ unpaired^").collect::<Vec<_>>();

        let expected_invalid = vec![
            Event::Start(Tag::Paragraph),
            Event::Text("‘Quoted.’ a^^ a^With space^ unpaired^".into()),
            Event::End(Tag::Paragraph),
        ];

        let expected_valid = vec![
            Event::Start(Tag::Paragraph),
            Event::Text("‘Quoted.’ a".into()),
            Event::Start(Tag::Superscript),
            Event::Text("bc".into()),
            Event::End(Tag::Superscript),
            Event::Text("d a".into()),
            Event::Start(Tag::Superscript),
            Event::Text("hello".into()),
            Event::End(Tag::Superscript),
            Event::End(Tag::Paragraph),
        ];

        assert_eq!(invalid_superscripts, expected_invalid);
        assert_eq!(valid_superscripts, expected_valid);
    }

    #[test]
    fn blockquotes() {
        use Event::*;
        use Tag::*;

        let text = Parser::new(
            "This checks quotes.\n\n> Single para\n\nNow multi:\n\n> Para 1...\n>\n> Para 2",
        )
        .collect::<Vec<_>>();
        let expected = vec![
            Start(Paragraph),
            Text(CowStr::Borrowed("This checks quotes.")),
            End(Paragraph),
            Start(BlockQuote),
            Start(Paragraph),
            Text(CowStr::Borrowed("Single para")),
            End(Paragraph),
            End(BlockQuote),
            Start(Paragraph),
            Text(CowStr::Borrowed("Now multi:")),
            End(Paragraph),
            Start(BlockQuotation),
            Start(Paragraph),
            Text(CowStr::Boxed("Para 1…".into())),
            End(Paragraph),
            Start(Paragraph),
            Text(CowStr::Borrowed("Para 2")),
            End(Paragraph),
            End(BlockQuotation),
        ];
        assert_eq!(text, expected);
    }

    #[test]
    fn emphasis_drop() {
        use Event::*;
        use Tag::*;

        let text =
            Parser::new("This has *emphasis* (among the 1^st^ of its kind)").collect::<Vec<_>>();

        let expected = vec![
            Start(Paragraph),
            Text(CowStr::Borrowed("This has ")),
            Start(Emphasis),
            Text(CowStr::Borrowed("emphasis")),
            End(Emphasis),
            Text(CowStr::Inlined(' '.into())),
            Text(CowStr::Inlined(
                InlineStr::try_from("(among the 1").unwrap(),
            )),
            Start(Superscript),
            Text(CowStr::Inlined(InlineStr::try_from("st").unwrap())),
            End(Superscript),
            Text(CowStr::Inlined(
                InlineStr::try_from(" of its kind)").unwrap(),
            )),
            End(Paragraph),
        ];

        let paired = text.into_iter().zip(expected);

        for (received, expected) in paired {
            assert_eq!(received, expected);
        }
    }
}
