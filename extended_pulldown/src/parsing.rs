use crate::quotes::convert_quotes_in_text_segment;
use crate::sub_and_superscript;
use crate::sub_and_superscript::disambiguate_sub_and_superscript;
use crate::BoundaryMarker;
use crate::{Event, MakeStatic, Options, Tag};
use pulldown_cmark::Event as PulldownEvent;
use pulldown_cmark::Parser as PulldownParser;
use pulldown_cmark::Tag as PulldownTag;
use pulldown_cmark::{BrokenLink, CodeBlockKind, CowStr, InlineStr};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryFrom;

/// Markdown event iterator which drops non-inline elements,
/// keeping only plain text, links, and superscript, subscript,
/// emphasised, smallcaps and strong text
pub struct InlineParser<P> {
    inner: P,
    in_dropped_tag: bool,
}

impl<'a> InlineParser<Parser<'a>> {
    /// Create a new inline parser over `text`
    pub fn new(text: &'a str) -> Self {
        let options = Options {
            enable_footnotes: false,
            smarten: true,
            //enable_tables: false,
            //enable_tasklists: false,
            enable_strikethrough: false,
        };
        let inner = Parser::new_ext(text, options);
        InlineParser {
            inner,
            in_dropped_tag: false,
        }
    }
}

impl<'a> From<Vec<Event<'a>>> for InlineParser<std::vec::IntoIter<Event<'a>>> {
    fn from(src: Vec<Event<'a>>) -> Self {
        InlineParser {
            inner: src.into_iter(),
            in_dropped_tag: false,
        }
    }
}

impl<'a, P> Iterator for InlineParser<P>
where
    P: Iterator<Item = Event<'a>>,
{
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        if let Some(event) = self.inner.next() {
            match event {
                e @ Event::Start(Tag::Emphasis)
                | e @ Event::End(Tag::Emphasis)
                | e @ Event::Start(Tag::Strong)
                | e @ Event::End(Tag::Strong)
                | e @ Event::Start(Tag::SmallCaps)
                | e @ Event::End(Tag::SmallCaps)
                | e @ Event::Start(Tag::Subscript)
                | e @ Event::End(Tag::Subscript)
                | e @ Event::Start(Tag::Superscript)
                | e @ Event::End(Tag::Superscript)
                | e @ Event::Start(Tag::Link(_, _, _))
                | e @ Event::End(Tag::Link(_, _, _))
                | e @ Event::Text(_) => {
                    if !self.in_dropped_tag {
                        Some(e)
                    } else {
                        self.next()
                    }
                }
                // ignore these tags rather than dropping their contents
                Event::Start(Tag::Paragraph)
                | Event::End(Tag::Paragraph)
                | Event::Start(Tag::UnindentedParagraph)
                | Event::End(Tag::UnindentedParagraph)
                | Event::Start(Tag::BlockQuote)
                | Event::End(Tag::BlockQuote) => self.next(),
                Event::Start(_) => {
                    self.in_dropped_tag = true;
                    self.next()
                }
                Event::End(_) => {
                    self.in_dropped_tag = false;
                    self.next()
                }
                _ => self.next(),
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum CurrentSpan {
    Generic,
    Sans,
    SmallCaps,
    Centred,
    RightAligned,
    Superscript,
    Subscript,
}

/// Markdown event iterator
pub struct Parser<'a> {
    buffered: VecDeque<Event<'a>>,
    in_verbatim: bool,
    inner: PulldownParser<'a>,
    smarten: bool,
    current_spans: Vec<CurrentSpan>,
    // this is just here to avoid allocations
    buffering: Vec<PulldownEvent<'a>>,
}

impl<'a> Parser<'a> {
    /// Create a new parser without options enabled
    pub fn new(text: &'a str) -> Self {
        let options = Options::default();
        Self::new_ext(text, options)
    }

    /// Create a new parser with the given options
    pub fn new_ext(text: &'a str, options: Options) -> Self {
        Self::new_with_broken_link_callback(text, options, None)
    }

    fn convert<'b>(text: Cow<'b, str>) -> CowStr<'b> {
        match text {
            Cow::Borrowed(b) => CowStr::Borrowed(b),
            Cow::Owned(s) => match InlineStr::try_from(s.as_str()) {
                Ok(i) => CowStr::Inlined(i),
                Err(_) => s.into(),
            },
        }
    }

    /// Create a new parser with the given options,
    /// and with an optional callback for broken links
    #[allow(clippy::type_complexity)]
    pub fn new_with_broken_link_callback(
        text: &'a str,
        options: Options,
        broken_link_callback: Option<
            &'a mut dyn FnMut(BrokenLink<'_>) -> Option<(CowStr<'a>, CowStr<'a>)>,
        >,
    ) -> Self {
        let smarten = options.smarten;
        let inner = PulldownParser::new_with_broken_link_callback(
            text,
            options.into(),
            broken_link_callback,
        );
        Parser {
            inner,
            buffering: Vec::new(),
            buffered: VecDeque::new(),
            in_verbatim: false,
            smarten,
            current_spans: Vec::new(),
        }
    }

    fn _next(&mut self) -> Option<Event<'a>> {
        if self.buffered.is_empty() {
            self.load_buffer();
        }
        self.buffered.pop_front()
    }

    fn add_super_or_subscript_to_buffering(&mut self, text: CowStr<'a>) {
        let disambiguated = disambiguate_sub_and_superscript(text);

        let reinsert = match disambiguated.chars().next() {
            Some(' ') => Some(' '),
            Some('\n') => Some('\n'),
            _ => None,
        };

        let mut p = match disambiguated {
            Cow::Borrowed(b) => InlineParser::new(b)
                .map(PulldownEvent::from)
                .collect::<Vec<_>>(),
            Cow::Owned(b) => InlineParser::new(&b)
                .map(|e| e.into_static())
                .map(PulldownEvent::from)
                .collect::<Vec<_>>(),
        };
        if let Some(reinsert) = reinsert {
            p.insert(0, PulldownEvent::Text(CowStr::Inlined(reinsert.into())));
        }

        self.buffering.append(&mut p);
    }

    /// load as few events as possible from pulldown,
    /// then push them into the buffer
    fn load_buffer(&mut self) {
        if let Some(next) = self.inner.next() {
            match next {
                PulldownEvent::Text(t) if !self.in_verbatim => {
                    // so, first we're going to deal with sub and superscript;
                    // that way we'll have the same number of elements at the start and end of the
                    // process, since these are the only changes which might introduce extra events.
                    // In turn that'll let us play tricks to do with knowing what's where.

                    if sub_and_superscript::REGGIE.is_match(&t) {
                        self.add_super_or_subscript_to_buffering(t);
                    } else {
                        self.buffering.push(PulldownEvent::Text(t));
                    }

                    while let Some(n) = self.inner.next() {
                        match n {
                            PulldownEvent::Text(t) if sub_and_superscript::REGGIE.is_match(&t) => {
                                self.add_super_or_subscript_to_buffering(t);
                            }
                            n if n.resets_quotes() => {
                                self.buffering.push(n);
                                break;
                            }
                            n => self.buffering.push(n),
                        }
                    }
                }
                PulldownEvent::Start(PulldownTag::BlockQuote) => {
                    let mut score = 1;
                    self.buffering.push(next);
                    let mut in_verbatim = false;
                    while let Some(n) = self.inner.next() {
                        match n {
                            e @ PulldownEvent::Start(PulldownTag::CodeBlock(_)) => {
                                in_verbatim = true;
                                self.buffering.push(e);
                            }
                            e @ PulldownEvent::End(PulldownTag::CodeBlock(_)) => {
                                in_verbatim = false;
                                self.buffering.push(e);
                            }
                            PulldownEvent::Text(t)
                                if !in_verbatim && sub_and_superscript::REGGIE.is_match(&t) =>
                            {
                                self.add_super_or_subscript_to_buffering(t);
                            }
                            PulldownEvent::Start(PulldownTag::BlockQuote) => {
                                self.buffering.push(n);
                                score += 1;
                            }
                            PulldownEvent::End(PulldownTag::BlockQuote) => {
                                self.buffering.push(n);
                                score -= 1;
                                if score == 0 {
                                    break;
                                }
                            }
                            n => self.buffering.push(n),
                        }
                    }
                }
                PulldownEvent::Html(_) => {
                    self.buffering.push(next);
                }
                e @ PulldownEvent::Start(PulldownTag::FootnoteDefinition(_)) => {
                    let mut in_verbatim = false;
                    self.buffering.push(e);
                    while let Some(event) = self.inner.next() {
                        match event {
                            PulldownEvent::Start(PulldownTag::Image(_, _, _)) => {
                                while let Some(e) = self.inner.next() {
                                    if let PulldownEvent::End(PulldownTag::Image(_, _, _)) = e {
                                        break;
                                    }
                                }
                            }
                            PulldownEvent::Start(PulldownTag::CodeBlock(
                                CodeBlockKind::Indented,
                            )) => {
                                let mut footnote_text = String::new();
                                while let Some(e) = self.inner.next() {
                                    match e {
                                        PulldownEvent::End(PulldownTag::CodeBlock(
                                            CodeBlockKind::Indented,
                                        )) => break,
                                        PulldownEvent::Text(t) => footnote_text.push_str(&t),
                                        e => {
                                            println!("{:?}", e);
                                        }
                                    };
                                }
                                footnote_text = footnote_text.replace("\n", "\n\n");

                                let parsed =
                                    PulldownParser::new(&footnote_text).map(|e| e.into_static());

                                for event in parsed {
                                    match event {
                                        PulldownEvent::Text(t)
                                            if sub_and_superscript::REGGIE.is_match(&t) =>
                                        {
                                            self.add_super_or_subscript_to_buffering(t);
                                        }
                                        t => self.buffering.push(t),
                                    }
                                }
                            }
                            e @ PulldownEvent::Start(PulldownTag::CodeBlock(_)) => {
                                in_verbatim = true;
                                self.buffering.push(e);
                            }
                            e @ PulldownEvent::End(PulldownTag::CodeBlock(_)) => {
                                in_verbatim = false;
                                self.buffering.push(e);
                            }
                            PulldownEvent::Text(t)
                                if !in_verbatim && sub_and_superscript::REGGIE.is_match(&t) =>
                            {
                                self.add_super_or_subscript_to_buffering(t);
                            }
                            e @ PulldownEvent::End(PulldownTag::FootnoteDefinition(_)) => {
                                self.buffering.push(e);
                                break;
                            }
                            e => self.buffering.push(e),
                        }
                    }
                }
                e @ PulldownEvent::Start(PulldownTag::CodeBlock(_)) => {
                    self.in_verbatim = true;
                    self.buffered.push_back(e.into());
                    return;
                }
                e @ PulldownEvent::End(PulldownTag::CodeBlock(_)) => {
                    self.in_verbatim = false;
                    self.buffered.push_back(e.into());
                    return;
                }
                other => {
                    self.buffered.push_back(other.into());
                    return;
                }
            }
        }

        // so we have special cases to handle:

        // - transform any blockquotes consisting of more than a single paragraph into quotations, and everything else into quotes
        // - replace straight quotes and other chars
        // - parse html for the various special spans

        let mut text_groups = Vec::new();
        let mut texts: Vec<(usize, CowStr<'a>)> = Vec::new();
        let mut in_verbatim = false;
        let mut block_quotations = HashMap::new();
        let mut in_block_quotation = None;

        for (idx, event) in self.buffering.iter_mut().enumerate() {
            match event {
                PulldownEvent::Start(PulldownTag::BlockQuote) => {
                    if in_block_quotation.is_none() {
                        in_block_quotation = Some(idx);
                        block_quotations.insert(idx, (0, 0));
                    } else {
                        in_block_quotation = None;
                    }
                }
                PulldownEvent::End(PulldownTag::BlockQuote) => {
                    if let Some(i) = in_block_quotation.take() {
                        if let Some((count, end)) = block_quotations.get_mut(&i) {
                            if *count > 1 {
                                *end = idx;
                            } else {
                                block_quotations.remove(&i);
                            }
                        }
                    }
                    in_block_quotation = None;
                }
                PulldownEvent::Start(PulldownTag::Paragraph) => {
                    if let Some(ref i) = in_block_quotation {
                        block_quotations.get_mut(i).map(|(count, _)| *count += 1);
                    }
                }
                PulldownEvent::Text(t) if !in_verbatim && self.smarten => {
                    texts.push((idx, std::mem::replace(t, CowStr::Borrowed(""))));
                }
                PulldownEvent::Start(PulldownTag::CodeBlock(_)) => {
                    in_verbatim = true;
                    if !texts.is_empty() {
                        text_groups.push(std::mem::take(&mut texts));
                    }
                }
                PulldownEvent::End(PulldownTag::CodeBlock(_)) => {
                    in_verbatim = true;
                    texts.clear();
                }
                e if e.resets_quotes() => {
                    if !texts.is_empty() {
                        text_groups.push(std::mem::take(&mut texts));
                    }
                }
                _ => {}
            }
        }

        let mut replacements = Vec::new();

        for group in text_groups.into_iter() {
            let (indices, text_strs): (Vec<usize>, Vec<CowStr>) = group.into_iter().unzip();

            let text_strs = text_strs.into_iter().map(|s| match s {
                CowStr::Borrowed(s) => Cow::Borrowed(s),
                CowStr::Inlined(i) => Cow::Owned(i.to_string()),
                CowStr::Boxed(b) => Cow::Owned(b.to_string()),
            });

            let converted = convert_quotes_in_text_segment(text_strs);
            for x in indices.into_iter().zip(converted) {
                replacements.push(x);
            }
        }

        for (idx, replacement) in replacements.into_iter() {
            let target = self.buffering.get_mut(idx).unwrap();
            let replacement = PulldownEvent::Text(Self::convert(replacement));
            *target = replacement;
        }

        let mut quotations = HashSet::with_capacity(block_quotations.len() * 2);
        for (start, (count, end)) in block_quotations.into_iter() {
            if count > 1 {
                quotations.insert(start);
                quotations.insert(end);
            }
        }

        for (idx, event) in self.buffering.drain(..).enumerate() {
            match event {
                PulldownEvent::Start(PulldownTag::BlockQuote) if quotations.contains(&idx) => {
                    self.buffered.push_back(Event::Start(Tag::BlockQuotation));
                }
                PulldownEvent::End(PulldownTag::BlockQuote) if quotations.contains(&idx) => {
                    self.buffered.push_back(Event::End(Tag::BlockQuotation));
                }
                PulldownEvent::Html(html) => {
                    if html.starts_with("<span") {
                        let find_val = |key| {
                            let k = format!("{}=\"", key);
                            html.find(&k)
                                .map(|i| i + k.len())
                                .map(|i| html.get(i..))
                                .flatten()
                                .map(|s| {
                                    if let Some(x) = s.find('"') {
                                        s.get(..x)
                                    } else {
                                        Some(s)
                                    }
                                })
                                .flatten()
                        };

                        let style = find_val("style");
                        let class = find_val("class");

                        let find_current_span =
                            |style: Option<&str>, class: Option<&str>| -> CurrentSpan {
                                if let Some(style) = style {
                                    if style.contains("font-family: \"sans\"")
                                        || style.contains("font-family:\"sans\"")
                                    {
                                        return CurrentSpan::Sans;
                                    } else if style.contains("font-variant: \"small-caps\"")
                                        || style.contains("font-variant:\"small-caps\"")
                                    {
                                        return CurrentSpan::SmallCaps;
                                    } else if style.contains("text-align: center")
                                        || style.contains("text-align:center")
                                    {
                                        return CurrentSpan::Centred;
                                    } else if style.contains("text-align: right")
                                        || style.contains("text-align:right")
                                    {
                                        return CurrentSpan::RightAligned;
                                    }
                                } else if let Some(class) = class {
                                    if class.contains("sans") {
                                        return CurrentSpan::Sans;
                                    } else if class.contains("smallcaps") {
                                        return CurrentSpan::SmallCaps;
                                    } else if class.contains("centred") {
                                        return CurrentSpan::Centred;
                                    } else if class.contains("right-aligned") {
                                        return CurrentSpan::RightAligned;
                                    } else if class.contains("superscript") {
                                        return CurrentSpan::Superscript;
                                    } else if class.contains("subscript") {
                                        return CurrentSpan::Subscript;
                                    }
                                }
                                CurrentSpan::Generic
                            };

                        let current_span = find_current_span(style, class);

                        match current_span {
                            CurrentSpan::Sans => self.buffered.push_back(Event::Start(Tag::Sans)),
                            CurrentSpan::SmallCaps => {
                                self.buffered.push_back(Event::Start(Tag::SmallCaps))
                            }
                            CurrentSpan::Centred => {
                                self.buffered.push_back(Event::Start(Tag::Centred))
                            }
                            CurrentSpan::RightAligned => {
                                self.buffered.push_back(Event::Start(Tag::RightAligned))
                            }
                            CurrentSpan::Superscript => {
                                self.buffered.push_back(Event::Start(Tag::Superscript))
                            }
                            CurrentSpan::Subscript => {
                                self.buffered.push_back(Event::Start(Tag::Subscript))
                            }
                            CurrentSpan::Generic => {
                                self.buffered.push_back(Event::Html(html));
                            }
                        }
                        self.current_spans.push(current_span);
                    } else if html.starts_with("</span") {
                        match self.current_spans.pop() {
                            Some(CurrentSpan::Generic) => {
                                self.buffered.push_back(Event::Html(html));
                            }
                            Some(CurrentSpan::Sans) => {
                                self.buffered.push_back(Event::End(Tag::Sans));
                            }
                            Some(CurrentSpan::SmallCaps) => {
                                self.buffered.push_back(Event::End(Tag::SmallCaps));
                            }
                            Some(CurrentSpan::Centred) => {
                                self.buffered.push_back(Event::End(Tag::Centred));
                            }
                            Some(CurrentSpan::RightAligned) => {
                                self.buffered.push_back(Event::End(Tag::RightAligned));
                            }
                            Some(CurrentSpan::Subscript) => {
                                self.buffered.push_back(Event::End(Tag::Subscript));
                            }
                            Some(CurrentSpan::Superscript) => {
                                self.buffered.push_back(Event::End(Tag::Superscript));
                            }
                            None => {
                                self.buffered.push_back(Event::Html(html));
                            }
                        }
                    } else {
                        self.buffered.push_back(Event::Html(html));
                    }
                }
                other => self.buffered.push_back(other.into()),
            }
        }
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Event<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self._next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borrow_when_possible() {
        let text = "This text has no quotations *yet*, but the next clause will: 'Hello world!'";
        let parsed = Parser::new(text)
            .filter_map(|event| {
                if let Event::Text(CowStr::Borrowed(b)) = event {
                    Some(b)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(parsed, vec!["This text has no quotations ", "yet"]);
    }

    #[test]
    fn multi_para_footnote() {
        use CowStr::*;
        use Event::*;
        use Tag::*;

        let text = "Hello[^fn1].\n\n[^fn1]:    Here we *go*\n    Para 2'\n";
        let parsed = Parser::new(text)
            .map(|e| {
                if let Text(Inlined(s)) = e {
                    Text(s.to_string().into())
                } else {
                    e
                }
            })
            .collect::<Vec<_>>();
        let expected = vec![
            Start(Paragraph),
            Text(Borrowed("Hello")),
            FootnoteReference(Borrowed("fn1")),
            Text(Borrowed(".")),
            End(Paragraph),
            Start(FootnoteDefinition(Borrowed("fn1"))),
            Start(Paragraph),
            Text(Borrowed("Here we ")),
            Start(Emphasis),
            Text(Borrowed("go")),
            End(Emphasis),
            End(Paragraph),
            Start(Paragraph),
            Text("Para 2’".into()),
            End(Paragraph),
            End(FootnoteDefinition(Borrowed("fn1"))),
        ];
        assert_eq!(parsed, expected);
    }
}
