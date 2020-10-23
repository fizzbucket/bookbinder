//! Helpers for common ast manipulations required across different crates,
//! such as collating multi-event elements.
use crate::{BookEvent, EventHelper, NumberFormat, SemanticRole};
use bookbinder_common::MimeTypeHelper;
use extended_pulldown::CowStr;
use extended_pulldown::{Event, Tag};
use std::borrow::Cow;
use std::marker::PhantomData;
use std::path::PathBuf;
use temp_file_name::TempFilePath;

/// A trait which abstracts out some common requirements
/// when dealing with an Iterator of BookEvents.
pub trait BookEventIteratorHelper<'a> {
    /// collect events to the end of an epigraph and report on the contents
    fn collate_epigraph(&mut self) -> CollatedEpigraph<'a>;
    /// collate events to the end of a header and report on the contents
    fn collate_division_header<T>(&mut self, is_starred: bool) -> CollatedHeader<'a, T>;
    /// collate events to the end of a titlepage and report on the contents
    fn collate_titlepage(&mut self) -> CollatedTitlePage<'a>;
    /// collate events to the end of an image and report on the contents
    fn collate_image(&mut self, dest: CowStr<'a>, alt: CowStr<'a>) -> CollatedImage<'a>;
    /// collect extended pulldown events until the end of a flattened footnote
    fn collect_plain_until_end_of_footnote(&mut self) -> Vec<Event<'a>>;
}

/// A collation of events within an epigraph
#[derive(Debug)]
pub struct CollatedEpigraph<'a> {
    /// Events in the epigraph content
    pub text: Vec<Event<'a>>,
    /// Events in the epigraph source
    pub source: Vec<Event<'a>>,
}

/// empty struct used as a marker
#[derive(Debug)]
pub struct EpubMarker;
/// empty struct used as a marker
#[derive(Debug)]
pub struct LatexMarker;

/// Helpers to do with formatting and writing events for a particular output format.
pub trait MarkerHelper {
    /// write escaped title text to a string in the appropriate format.
    /// For example, `Hello *world*` (as events) might become the string `Hello <em>world</em>` or Hello \emph{world}`
    fn write_title_text<'a, I: IntoIterator<Item = Event<'a>> + std::fmt::Debug>(
        events: I,
    ) -> String;
    /// escape text for the output format
    fn escape<'a, S: Into<Cow<'a, str>>>(text: S) -> Cow<'a, str>;
    /// escape a CowStr for the output format
    fn escape_cowstr(text: CowStr<'_>) -> Cow<'_, str> {
        match text {
            CowStr::Borrowed(s) => Self::escape(s),
            CowStr::Inlined(s) => Self::escape(s.to_string()),
            CowStr::Boxed(t) => Self::escape(t.to_string()),
        }
    }
    /// Remove events which should not be present in a title,
    /// such as block or non-textual elements
    fn filter_title_events<'a, I: IntoIterator<Item = Event<'a>> + std::fmt::Debug>(
        events: I,
    ) -> Vec<Event<'a>> {
        events
            .into_iter()
            .filter(|event| match event {
                Event::Text(_) => true,
                Event::Start(Tag::Emphasis) => true,
                Event::End(Tag::Emphasis) => true,
                Event::Start(Tag::Strong) => true,
                Event::End(Tag::Strong) => true,
                Event::Start(Tag::Subscript) => true,
                Event::End(Tag::Subscript) => true,
                Event::Start(Tag::Superscript) => true,
                Event::End(Tag::Superscript) => true,
                _ => false,
            })
            .collect()
    }
}

impl MarkerHelper for EpubMarker {
    fn escape<'a, S: Into<Cow<'a, str>>>(text: S) -> Cow<'a, str> {
        bookbinder_common::escape_to_html(text)
    }

    fn write_title_text<'a, I: IntoIterator<Item = Event<'a>> + std::fmt::Debug>(
        events: I,
    ) -> String {
        let mut out = String::new();
        let mut some_text = false;

        for event in Self::filter_title_events(events) {
            match event {
                Event::Text(t) => {
                    out.push_str(&Self::escape_cowstr(t));
                    some_text = true;
                }
                Event::Start(Tag::Emphasis) => out.push_str("<em>"),
                Event::End(Tag::Emphasis) => out.push_str("</em>"),
                Event::Start(Tag::Strong) => out.push_str("<strong>"),
                Event::End(Tag::Strong) => out.push_str("</strong>"),
                Event::Start(Tag::Subscript) => out.push_str("<sub>"),
                Event::End(Tag::Subscript) => out.push_str("</sub>"),
                Event::Start(Tag::Superscript) => out.push_str("<sup>"),
                Event::End(Tag::Superscript) => out.push_str("</sup>"),
                _ => unreachable!(),
            }
        }
        if some_text {
            out
        } else {
            String::new()
        }
    }
}

impl MarkerHelper for LatexMarker {
    fn escape<'a, S: Into<Cow<'a, str>>>(text: S) -> Cow<'a, str> {
        bookbinder_common::escape_to_latex(text)
    }

    fn write_title_text<'a, I: IntoIterator<Item = Event<'a>> + std::fmt::Debug>(
        events: I,
    ) -> String {
        let mut out = String::new();
        let mut some_text = false;
        for event in Self::filter_title_events(events) {
            match event {
                Event::Text(t) => {
                    some_text = true;
                    out.push_str(&Self::escape_cowstr(t));
                }
                Event::Start(Tag::Emphasis) => out.push_str("\\emph{"),
                Event::Start(Tag::Strong) => out.push_str("\\textbf{"),
                Event::Start(Tag::Subscript) => out.push_str("\\textsubscript{"),
                Event::Start(Tag::Superscript) => out.push_str("\\textsuperscript{"),
                Event::End(_) => out.push('}'),
                _ => unreachable!(),
            }
        }
        if some_text {
            out
        } else {
            String::new()
        }
    }
}

/// A collation of events within a header
#[derive(Debug)]
pub struct CollatedHeader<'a, T> {
    phantom: PhantomData<T>,
    /// Text of any label, e.g. `Chapter`
    pub label_text: Option<Cow<'a, str>>,
    /// Number of any label, e.g. 1 for the first chapter
    pub label_number: Option<u8>,
    /// Format in which to display the number, such as
    /// arabic, roman etc
    pub label_number_format: Option<NumberFormat>,
    /// The text of this header
    pub text: Option<Vec<Event<'a>>>,
    /// Names of any authors
    pub authors: Option<Vec<Cow<'a, str>>>,
    /// Whether the header should be treated as starred
    pub is_starred: bool,
}

impl<'a, T: MarkerHelper> CollatedHeader<'a, T> {
    /// escape text suitably for the output format
    fn escape<'b, S: Into<Cow<'b, str>>>(text: S) -> Cow<'b, str> {
        T::escape(text)
    }

    /// get label and title if there are both,
    /// just title if there's only a title,
    /// label as title if there's no title
    /// or just title if label and title are equivalent
    pub fn reconcile_joined_label_and_title(
        &'a self,
    ) -> Option<(Option<Cow<'a, str>>, Option<Cow<'a, str>>)> {
        match (self.get_joined_label(), self.get_title_text()) {
            (None, None) => None,
            (Some(label), None) => Some((None, Some(label))),
            (Some(label), Some(title)) => {
                if title.to_lowercase().starts_with(&label.to_lowercase()) {
                    Some((None, Some(title.into())))
                } else {
                    Some((Some(label), Some(title.into())))
                }
            }
            (None, Some(title)) => Some((None, Some(title.into()))),
        }
    }

    /// Escaped version of the label text
    pub fn get_label_text(&'a self) -> Option<Cow<'a, str>> {
        self.label_text.as_deref().map(Self::escape)
    }

    /// Get a representation of the label number in the appropriate format,
    /// if any.
    pub fn get_label_number(&self) -> Option<Cow<'a, str>> {
        match (self.label_number, self.label_number_format) {
            (None, _) => None,
            (Some(n), Some(NumberFormat::Arabic)) | (Some(n), None) => {
                Some(format!("{}", n).into())
            }
            (Some(n), Some(NumberFormat::Roman)) => {
                let r = bookbinder_common::number_to_roman(n);
                Some(r.into())
            }
            (Some(n), Some(NumberFormat::Words)) => {
                let w = bookbinder_common::number_to_words(n);
                Some(w.into())
            }
            (Some(n), Some(NumberFormat::Letter)) => {
                let l = bookbinder_common::number_to_letter(n)
                    .map(|n| n.to_string())
                    .unwrap_or(format!("{}", n));
                Some(l.into())
            }
        }
    }

    /// write the text of the title to a string,
    /// in the format represented by T
    pub fn get_title_text(&self) -> Option<String> {
        if let Some(ref text) = self.text {
            let text = T::write_title_text(text.clone());
            if !text.is_empty() {
                Some(text)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// get a label which joins label text and a formatted number
    pub fn get_joined_label(&'a self) -> Option<Cow<'a, str>> {
        let label_number = self.get_label_number();
        let label_text = if self.is_starred {
            None
        } else {
            self.get_label_text()
        };
        match (label_text, label_number) {
            (Some(label), Some(number)) => Some(format!("{} {}", label, number).into()),
            (Some(label), None) => Some(label.clone()),
            (None, Some(number)) => Some(number),
            (None, None) => None,
        }
    }

    /// If there is only one author, return the name of the author and an empty companion;
    /// if two, return two seperate authors,
    /// and if more than two, join all but the last with ", " and return the last name seperately.
    /// If there are no authors, return None.
    /// Note that all names will be escaped.
    pub fn get_authors(&'a self) -> Option<(Cow<'a, str>, Option<Cow<'a, str>>)> {
        if let Some(ref authors) = self.authors {
            let mut authors = authors
                .iter()
                .map(|n| match n {
                    Cow::Owned(s) => Self::escape(s),
                    Cow::Borrowed(s) => Self::escape(*s),
                })
                .collect::<Vec<_>>();
            match authors.len() {
                0 => None,
                1 => Some((authors.pop().unwrap(), None)),
                2 => {
                    let second = authors.pop();
                    let first = authors.pop().unwrap();
                    Some((first, second))
                }
                _ => {
                    let last = authors.pop();
                    let authors = authors
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    Some((authors.into(), last))
                }
            }
        } else {
            None
        }
    }
}

/// A collation of events within a titlepage
#[derive(Debug)]
pub struct CollatedTitlePage<'a> {
    /// Events representing the title
    pub title: Vec<Event<'a>>,
    /// A subtitle
    pub subtitle: Option<Vec<Event<'a>>>,
    /// Role and names of contributors
    pub contributors: Option<Vec<(Option<&'a str>, Vec<Cow<'a, str>>)>>,
}

/// A collation of events and information about an image
#[derive(Debug)]
pub struct CollatedImage<'a> {
    /// Events in the image caption
    pub caption: Option<Vec<Event<'a>>>,
    /// Path to the image
    pub dest: CowStr<'a>,
    /// Alt text of the image
    pub alt: Option<CowStr<'a>>,
}

impl CollatedImage<'_> {
    /// return this image's path if it is valid as a LaTeX image,
    /// or attempt to convert it to a more suitable format and return
    /// that path instead
    pub fn get_latex_image_path(&self) -> Result<String, ()> {
        let d: &str = self.dest.as_ref();
        let p = PathBuf::from(d);
        if p.is_latex_supported_image() {
            return Ok(p.to_string_lossy().to_string());
        } else if p.is_svg() {
            if let Ok(png) = bookbinder_common::convert_svg_file_to_png(&p, None) {
                let png_path = png
                    .temp_file_path(Some("bookbinder"), "png")
                    .to_string_lossy()
                    .to_string();
                if std::fs::write(&png_path, &png).is_ok() {
                    return Ok(png_path);
                }
            }
        }
        Err(())
    }
}

impl<'a, I> BookEventIteratorHelper<'a> for I
where
    I: Iterator<Item = BookEvent<'a>>,
{
    fn collate_epigraph(&mut self) -> CollatedEpigraph<'a> {
        let mut text = Vec::new();
        let mut source = Vec::new();

        let mut in_text = false;
        let mut in_source = false;

        for event in self {
            match event {
                BookEvent::EndSemantic(SemanticRole::Epigraph) => break,
                BookEvent::BeginEpigraphText => {
                    in_text = true;
                }
                BookEvent::EndEpigraphText => {
                    in_text = false;
                }
                BookEvent::BeginEpigraphSource => {
                    in_source = true;
                }
                BookEvent::EndEpigraphSource => {
                    in_source = false;
                }
                BookEvent::Event(e) => {
                    if in_text {
                        text.push(e);
                    } else if in_source {
                        source.push(e);
                    }
                }
                _ => {}
            }
        }

        CollatedEpigraph { text, source }
    }

    fn collate_division_header<T>(&mut self, is_starred: bool) -> CollatedHeader<'a, T> {
        let mut text = Vec::new();
        let mut label_text = None;
        let mut label_number = None;
        let mut label_number_format = None;
        let mut authors = None;

        let mut events = Vec::new();
        for event in self {
            if matches!(event, BookEvent::EndDivisionHeader(_)) {
                break;
            } else {
                events.push(event);
            }
        }

        for event in events.into_iter() {
            match event {
                BookEvent::DivisionHeaderLabel {
                    text,
                    number,
                    number_format,
                } => {
                    label_text = text;
                    label_number = number;
                    label_number_format = Some(number_format);
                }
                BookEvent::DivisionAuthors(a) => {
                    if !a.is_empty() {
                        authors = Some(a);
                    }
                }
                BookEvent::Event(e) => text.push(e),
                _ => {}
            }
        }

        println!("{:#?}", text);

        let text = if text.is_empty() { None } else { Some(text) };

        CollatedHeader {
            phantom: PhantomData,
            text,
            label_text,
            label_number,
            label_number_format,
            authors,
            is_starred,
        }
    }

    fn collate_titlepage(&mut self) -> CollatedTitlePage<'a> {
        let mut title = Vec::new();
        let mut subtitle_events = Vec::new();
        let mut contributors = None;
        let mut in_title = false;
        let mut in_subtitle = false;

        while let Some(event) = self.next() {
            match event {
                BookEvent::EndTitlePage => break,
                BookEvent::BeginTitlePageTitle => {
                    in_title = true;
                }
                BookEvent::EndTitlePageTitle => {
                    in_title = false;
                }
                BookEvent::BeginTitlePageSubTitle => {
                    in_subtitle = true;
                }
                BookEvent::EndTitlePageSubTitle => {
                    in_subtitle = false;
                }
                BookEvent::TitlePageContributors(v) => {
                    let v = v
                        .into_iter()
                        .map(|(role, names)| (role.get_label(), names))
                        .collect();
                    contributors = Some(v);
                }
                BookEvent::Event(e) => {
                    if in_title {
                        title.push(e);
                    } else if in_subtitle {
                        subtitle_events.push(e);
                    }
                }
                _ => {}
            }
        }

        title.make_uppercase();
        subtitle_events.make_uppercase();

        let subtitle = if subtitle_events.is_empty() {
            None
        } else {
            Some(subtitle_events)
        };

        CollatedTitlePage {
            title,
            subtitle,
            contributors,
        }
    }

    fn collect_plain_until_end_of_footnote(&mut self) -> Vec<Event<'a>> {
        let mut events = Vec::new();
        while let Some(event) = self.next() {
            match event {
                BookEvent::Event(Event::End(Tag::FlattenedFootnote)) => break,
                BookEvent::Event(e) => events.push(e),
                _ => {}
            }
        }
        events
    }

    fn collate_image(&mut self, dest: CowStr<'a>, alt: CowStr<'a>) -> CollatedImage<'a> {
        let mut caption = Vec::new();
        while let Some(event) = self.next() {
            match event {
                BookEvent::Event(Event::End(Tag::Image(_, _, _))) => break,
                BookEvent::Event(e) => caption.push(e),
                _ => {}
            }
        }

        let caption = if caption.is_empty() {
            None
        } else {
            Some(caption)
        };

        let alt = if alt.is_empty() { None } else { Some(alt) };

        CollatedImage { dest, alt, caption }
    }
}
