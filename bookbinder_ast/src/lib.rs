//! This crate makes the building of a `BookSrc` easy;
//! this struct can then be used to render to a specific backend
//! after being adapted to various options.
//!
//! General usage is to create a `BookSrcBuilder`, set metadata values and add content,
//! then create a `BookSrc` from the builder.
//!
//! Because books get complicated quickly, there are a lot of builder methods and toggles.
//! Simple usage, however, is very simple.
//!
//! ```
//! use bookbinder_ast::BookSrcBuilder;
//! let src = BookSrcBuilder::new("A Book")
//!    .subtitle("Serving as an Example")
//!    .author("A.N. Author")
//!    .add_mainmatter("# Hello World\n\nText goes here...")
//!    .process();
//! ```
//!
//! Because we incorporate the idea of semantic divisions,
//! various pieces of ancillary text can be added and formatted appropriately:
//! see, for example, `add_foreword`.
//!
//! # Markdown
//! The markdown source interpreted has a few small extensions from CommonMark:
//!
//! - footnotes: add footnotes using `[^marker] ... [^marker]: definition`
//! - formatting spans: text can be wrapped in a handful of spans to format it in various ways:
//!     * `<span class="sans">Sans Text</span>`
//!     * `<span class="smallcaps">Small caps text</span>`
//!     * `<span class="centred">Centred text</span>`
//!     * `<span class="right-aligned">Right aligned text</span>`
//! - escaping: `--` and `---` are turned into en and em-dashes respectively, while a row of three full stops
//! (`...`) becomes an ellipsis.
//! - quotes: straight quotes are -- at least in theory -- turned into appropriate curly quotes; this is a problem impossible
//! to get absolutely right without actually understanding text, but the algorithm used is fairly robust
//! and takes into account semantics (so that, for example,
//! quotes in code are not transformed). Still, for perfect accuracy, it's best to use curly quotes explicitly.
//! - sub and superscript: `22^nd^ July`, `H~2~0`;

#![deny(dead_code)]
#![deny(unreachable_patterns)]
#![deny(unused_extern_crates)]
#![deny(unused_imports)]
#![deny(unused_qualifications)]
#![deny(clippy::all)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![deny(variant_size_differences)]
use bookbinder_common::MimeTypeHelper;
use extended_pulldown::{flatten_footnotes, InlineParser, MakeStatic, Parser};
pub use extended_pulldown::{Event, Tag};
pub use pulldown_cmark::CowStr;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::path::Path;
use std::path::PathBuf;
mod metadata;
pub use metadata::Metadata;
pub mod helpers;

/// Specification of a section's semantic role, such as being a foreword or a chapter.
/// These are taken from Epub 3.2's structural semantics
#[derive(Debug, PartialEq, Clone, Copy, Hash)]
#[allow(missing_docs)]
pub enum SemanticRole {
    Halftitle,
    Copyrightpage,
    Titlepage,
    Dedication,
    Foreword,
    Afterword,
    Introduction,
    Colophon,
    Epigraph,
    Acknowledgements,
    Appendix,
    Chapter,
    Part,
    Preface,
}

impl SemanticRole {
    /// A suitable label for this role, if any,
    /// which can be used -- for example -- in labelling chapter headings
    ///
    /// For example, a `TitlePage` has no label, but an `Introduction` is labelled
    /// `Introduction`
    ///
    /// At some future point, this will be changed to allow multilingualism.
    pub const fn get_label(self) -> Option<&'static str> {
        use SemanticRole::*;
        match self {
            Epigraph | Halftitle | Copyrightpage | Titlepage | Dedication => None,
            Foreword => Some("Foreword"),
            Afterword => Some("Afterword"),
            Introduction => Some("Introduction"),
            Colophon => Some("Colophon"),
            Acknowledgements => Some("Acknowledgements"),
            Appendix => Some("Appendix"),
            Chapter => Some("Chapter"),
            Part => Some("Part"),
            Preface => Some("Preface"),
        }
    }
}

/// Indicate a contributor to the work who should appear on a titlepage
#[derive(Debug, Clone, Copy, Hash, PartialEq)]
#[allow(missing_docs)]
pub enum TitlePageContributorRole {
    Author,
    Editor,
    Translator,
    ForewordAuthor,
    AfterwordAuthor,
    IntroductionAuthor,
    IntroductionAndNotesAuthor,
}

impl TitlePageContributorRole {
    /// Get a human-readable label for this role
    pub const fn get_label(self) -> Option<&'static str> {
        use TitlePageContributorRole::*;
        match self {
            Author => None,
            Editor => Some("Edited by"),
            Translator => Some("Translated by"),
            ForewordAuthor => Some("With a foreword by"),
            AfterwordAuthor => Some("With an afterword by"),
            IntroductionAuthor => Some("With an introduction by"),
            IntroductionAndNotesAuthor => Some("With an introduction and notes by"),
        }
    }
}

/// The display format of a number
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum NumberFormat {
    /// In words: e.g. One
    Words,
    /// In arabic numberals: e.g 1
    Arabic,
    /// In roman numerals: e.g i
    Roman,
    /// As a letter: e.g. A
    Letter,
}

impl Default for NumberFormat {
    fn default() -> Self {
        NumberFormat::Arabic
    }
}

/// A particular event in a book, such as the beginning of a paragraph
/// or a span of text
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq)]
pub enum BookEvent<'a> {
    BeginSemantic(SemanticRole),
    EndSemantic(SemanticRole),
    Event(Event<'a>),
    BeginDivisionHeader(bool),
    EndDivisionHeader(bool),
    DivisionHeaderLabel {
        /// the text of this label
        text: Option<Cow<'a, str>>,
        /// the number of this label
        number: Option<u8>,
        /// the format the number of this label should be in
        number_format: NumberFormat,
    },
    DivisionAuthors(Vec<Cow<'a, str>>),
    BeginTitlePage,
    BeginTitlePageTitle,
    EndTitlePageTitle,
    BeginTitlePageSubTitle,
    EndTitlePageSubTitle,
    TitlePageContributors(Vec<(TitlePageContributorRole, Vec<Cow<'a, str>>)>),
    EndTitlePage,
    BeginMainmatter,
    BeginFrontmatter,
    BeginBackmatter,
    BeginEpigraphText,
    EndEpigraphText,
    BeginEpigraphSource,
    EndEpigraphSource,
    /// Empty event to be ignored
    Null,
}

impl<'a> From<Event<'a>> for BookEvent<'a> {
    fn from(src: Event<'a>) -> Self {
        BookEvent::Event(src)
    }
}

impl<'a> TryFrom<BookEvent<'a>> for Event<'a> {
    type Error = ();

    fn try_from(src: BookEvent<'a>) -> Result<Self, Self::Error> {
        match src {
            BookEvent::Event(e) => Ok(e),
            _ => Err(()),
        }
    }
}

/// The source for a particular book; essentially a way to create a `BookSrc` using a builder pattern.
///
/// # Example usage
///
/// ```
/// use bookbinder_ast::BookSrcBuilder;
///
/// let processed = BookSrcBuilder::new("Book Title")
///     .subtitle("Being the Memoirs of Book Author")
///     .author("Book Author")
///     .set_dedication("To Someone")
///     .publisher("Publisher Name")
///     .add_mainmatter("# I am born\n\nThe day of my birth (the 22^nd^ of January) was a dark cold day...")
///     .add_mainmatter("# I grow up\n\nI grew strong and I grew tall. I remember my uncle who left the farm after some wild quarrel and rode off whistling on his sorrel.")
///     .add_acknowledgements("This was of course a very serious and highly researched work", Some("Author's Note"))
///     .process();
/// // now the processed work can be rendered to a particular output format from a combination of metadata and book events.
/// ```
#[derive(Debug, Clone, Default)]
pub struct BookSrcBuilder<'a> {
    halftitle: Vec<BookEvent<'a>>,
    copyright_page: Vec<BookEvent<'a>>,
    dedication: Vec<BookEvent<'a>>,
    forewords: Vec<BookEvent<'a>>,
    epigraphs: Vec<BookEvent<'a>>,
    introductions: Vec<BookEvent<'a>>,
    prefaces: Vec<BookEvent<'a>>,
    mainmatter: Vec<Event<'a>>,
    appendices: Vec<BookEvent<'a>>,
    afterwords: Vec<BookEvent<'a>>,
    colophon: Vec<BookEvent<'a>>,
    acknowledgements: Vec<BookEvent<'a>>,
    metadata: Metadata<'a>,
    image_dirs: HashSet<PathBuf>,
    appendices_count: u8,
    epigraph_count: usize,
    no_titlepage: bool,
    no_copyrightpage: bool,
    no_halftitle: bool,
}

/// set a metadata value from its parent `BookSrcBuilder`
macro_rules! metadata_func {
    ($name:ident, $doc:meta) => {
        #[$doc]
        pub fn $name<S: Into<Cow<'a, str>>>(&mut self, val: S) -> &mut Self {
            self.metadata.$name(val);
            self
        }
    };
}

/// add a name or names to metadata
macro_rules! metadata_name_func {
    ($name:ident, $doc:meta) => {
        #[$doc]
        pub fn $name<A: ContributorSource<'a>>(&mut self, val: A) -> &mut Self {
            self.metadata.$name(val);
            self
        }
    };
}

/// set a metadata flag from its parent `BookSrcBuilder`
macro_rules! metadata_bool_func {
    ($name:ident, $doc:meta) => {
        #[$doc]
        pub fn $name(&mut self) -> &mut Self {
            self.metadata.$name();
            self
        }
    };
}

/// Implemented for str-like types which can be interpreted as either a single name or multiple names;
/// this is a convenience trait to allow overloading methods to add names
/// rather than having to have a method to add a single name and one to
/// add multiple names.
/// It shouldn't really be a worry for end users, since it is pre-implemented
/// for likely types.
///
/// ```
/// # use bookbinder_ast::ContributorSource;
/// # use std::borrow::Cow;
/// let single_name = "Name One";
/// let multiple_names = vec!["Name One, Name Two"];
///
/// assert!(single_name.is_single_name());
/// assert_eq!(multiple_names.is_single_name(), false);
/// assert_eq!(single_name.to_vec(), vec!["Name One"]);
/// assert_eq!(multiple_names.clone().to_vec(), multiple_names.clone());
/// assert_eq!(single_name.to_single(), Some(Cow::Borrowed("Name One")));
/// assert_eq!(multiple_names.to_single(), None);
pub trait ContributorSource<'a>: Sized {
    /// get a vec of names, even if the source was a single name
    fn to_vec(self) -> Vec<Cow<'a, str>>;
    /// does this represent only a single name?
    fn is_single_name(&self) -> bool {
        false
    }
    /// get a single name if this represented only a single name
    fn to_single(self) -> Option<Cow<'a, str>> {
        None
    }
}

macro_rules! contributor_source {
    ($target:ty) => {
        impl<'a> ContributorSource<'a> for $target {
            fn to_vec(self) -> Vec<Cow<'a, str>> {
                vec![self.into()]
            }

            fn is_single_name(&self) -> bool {
                true
            }

            fn to_single(self) -> Option<Cow<'a, str>> {
                Some(self.into())
            }
        }

        impl<'a> ContributorSource<'a> for Vec<$target> {
            fn to_vec(self) -> Vec<Cow<'a, str>> {
                self.into_iter().map(|n| n.into()).collect()
            }
        }

        impl<'a> ContributorSource<'a> for &[$target] {
            fn to_vec(self) -> Vec<Cow<'a, str>> {
                self.iter().cloned().map(|n| n.into()).collect()
            }
        }
    };
}

contributor_source!(String);
contributor_source!(&'a str);
contributor_source!(Cow<'a, str>);

impl<'a> ContributorSource<'a> for CowStr<'a> {
    fn to_vec(self) -> Vec<Cow<'a, str>> {
        match self {
            CowStr::Borrowed(b) => vec![Cow::Borrowed(b)],
            _ => vec![Cow::Owned(self.to_string())],
        }
    }

    fn is_single_name(&self) -> bool {
        true
    }

    fn to_single(self) -> Option<Cow<'a, str>> {
        let c = match self {
            CowStr::Borrowed(b) => Cow::Borrowed(b),
            _ => Cow::Owned(self.to_string()),
        };
        Some(c)
    }
}

impl<'a> ContributorSource<'a> for Vec<CowStr<'a>> {
    fn to_vec(self) -> Vec<Cow<'a, str>> {
        self.into_iter()
            .map(|n| match n {
                CowStr::Borrowed(b) => Cow::Borrowed(b),
                _ => Cow::Owned(n.to_string()),
            })
            .collect()
    }
}

/// Macro for adding authored ancillary text like a foreword;
/// - fnname: the name of the primary function for adding text
/// - meta_add_func: the function to call on `metadata` to add the authors of this text
/// - doc and from_file_doc: documentation for the two functions
/// - role: the semantic role of this text
/// - field: the field to add the parsed events to
/// - from_file_fn: the name of a wrapper function to read a file and then call the primary function with its contents
macro_rules! add_authored_ancillary_text {
    ($fnname:ident, $meta_add_func:ident, $doc:meta, $role:expr, $field:ident, $from_file_fn:ident, $from_file_doc:meta) => {
        #[$doc]
        pub fn $fnname<P, A>(&mut self, text: P, title: Option<P>, authors: A) -> &mut Self
        where
            A: ContributorSource<'a>,
            P: ParseHelper<'a>,
        {
            let authors = authors.to_vec();
            self.metadata.$meta_add_func(authors.clone());
            let (mut title, mut text) = if title.is_none() {
                text.parse_and_remove_initial_title()
            } else {
                let title = title.map(|t| t.parse_inline());
                (title, text.parse())
            };
            let mut len = text.len() + 7;
            if let Some(ref t) = title {
                len += t.len();
            }
            let mut events = Vec::with_capacity(len);
            events.push(BookEvent::BeginSemantic($role));
            events.push(BookEvent::BeginDivisionHeader(false));
            if let Some(title_events) = title.as_mut() {
                events.append(title_events);
            }
            if let Some(label) = $role.get_label() {
                let label = BookEvent::DivisionHeaderLabel {
                    text: Some(label.into()),
                    number: None,
                    number_format: NumberFormat::Arabic,
                };
                events.push(label);
            }

            if !authors.is_empty() {
                events.push(BookEvent::DivisionAuthors(authors));
            }
            events.push(BookEvent::EndDivisionHeader(false));
            events.append(&mut text);
            events.push(BookEvent::EndSemantic($role));

            self.$field.append(&mut events);
            self
        }

        read_file_wrapper!($from_file_doc, $from_file_fn, $fnname, ".");
    };
}

macro_rules! read_file_wrapper {
    ($doc:meta, $fnname:ident, $wrapped_fn:ident) => {
        #[$doc]
        pub fn $fnname<P, S>(
            &mut self,
            filepath: P,
            title: Option<S>,
        ) -> Result<&mut Self, std::io::Error>
        where
            P: AsRef<Path>,
            S: ToString,
        {
            let s = self.read_file_and_add_to_sources(filepath)?;
            self.$wrapped_fn(s, title.map(|s| s.to_string()));
            Ok(self)
        }
    };
    ($doc:meta, $fnname:ident, $wrapped_fn:ident, $include_authors:expr) => {
        #[$doc]
        pub fn $fnname<A, P>(
            &mut self,
            filepath: P,
            authors: A,
        ) -> Result<&mut Self, std::io::Error>
        where
            A: ContributorSource<'a>,
            P: AsRef<Path>,
        {
            let s = self.read_file_and_add_to_sources(filepath)?;
            self.$wrapped_fn(s, None, authors);
            Ok(self)
        }
    };
}

/// Macro for adding unauthored ancillary text like a preface;
/// - fnname: the name of the primary function for adding text
/// - doc and from_file_doc: documentation for the two functions
/// - role: the semantic role of this text
/// - field: the field to add the parsed events to
/// - from_file_fn: the name of a wrapper function to read a file and then call the primary function with its contents
macro_rules! add_unauthored_ancillary_text {
    ($fnname:ident, $doc:meta, $role:expr, $field:ident, $from_file_fn:ident, $from_file_doc:meta) => {
        #[$doc]
        pub fn $fnname<P>(&mut self, text: P, title: Option<P>) -> &mut Self
        where
            P: ParseHelper<'a>,
        {
            let (mut title, mut text) = if title.is_none() {
                text.parse_and_remove_initial_title()
            } else {
                let title = title.map(|t| t.parse_inline());
                (title, text.parse())
            };
            let mut len = text.len() + 7;
            if let Some(ref t) = title {
                len += t.len();
            }
            let mut events = Vec::with_capacity(len);
            events.push(BookEvent::BeginSemantic($role));
            events.push(BookEvent::BeginDivisionHeader(false));
            if let Some(title_events) = title.as_mut() {
                events.append(title_events);
            }
            if let Some(label) = $role.get_label() {
                let label = BookEvent::DivisionHeaderLabel {
                    text: Some(label.into()),
                    number: None,
                    number_format: NumberFormat::Arabic,
                };
                events.push(label);
            }
            events.push(BookEvent::EndDivisionHeader(false));
            events.append(&mut text);
            events.push(BookEvent::EndSemantic($role));

            self.$field.append(&mut events);
            self
        }

        read_file_wrapper!($from_file_doc, $from_file_fn, $fnname);
    };
}

/// What a top-level markdown heading in mainmatter represents
enum StepLevel {
    TopToParts,
    TopToChapter,
}

impl StepLevel {
    fn get<'a, I>(events: I) -> Self
    where
        I: Iterator<Item = &'a Event<'a>>,
    {
        let mut header_levels = events
            .filter_map(|event| match event {
                Event::Start(Tag::Heading(h)) => Some(h),
                _ => None,
            })
            .collect::<HashSet<_>>();

        let has_l1_headers = header_levels.remove(&1);
        let has_l2_headers = header_levels.remove(&2);
        let has_ln_headers = !header_levels.is_empty();

        match (has_l1_headers, has_l2_headers, has_ln_headers) {
            (true, true, _) => StepLevel::TopToParts, // l1 represents parts
            (true, false, true) => StepLevel::TopToChapter,
            (false, true, true) => StepLevel::TopToParts, // l1 is not present, but l2 still represents chapters
            (false, true, false) => StepLevel::TopToChapter,
            (false, false, false) => StepLevel::TopToChapter,
            (true, false, false) => StepLevel::TopToChapter,
            (false, false, true) => StepLevel::TopToChapter,
        }
    }
}

impl<'a> BookSrcBuilder<'a> {
    /// Begin building a new book with `title`
    pub fn new<S: Into<Cow<'a, str>>>(title: S) -> Self {
        BookSrcBuilder {
            metadata: Metadata::new(title),
            ..Default::default()
        }
    }

    /// Do not include a halftitle if it has not been explicitly set
    pub fn do_not_generate_halftitle(&mut self) -> &mut Self {
        self.no_halftitle = true;
        self
    }

    /// Do not generate a titlepage if it has not been explicitly set
    pub fn do_not_generate_titlepage(&mut self) -> &mut Self {
        self.no_titlepage = true;
        self
    }

    /// Do not generate a copyright page if it has not been explicitly set
    pub fn do_not_generate_copyrightpage(&mut self) -> &mut Self {
        self.no_copyrightpage = true;
        self
    }

    /// Set the copyright page of this book explicitly; if this is not set,
    /// it will be generated from metadata unless `do_not_generate_copyrightpage` was called.
    pub fn add_copyright_page<P: ParseHelper<'a>>(
        &mut self,
        src: P,
        title: Option<P>,
    ) -> &mut Self {
        let mut events = src.parse_plain();
        let title = title.map(|title| {
            let mut title = title.parse_inline_plain();
            title.make_uppercase();
            title.parse()
        });
        events.make_paragraphs_unindented();
        let mut events = events.parse();

        if let Some(mut title) = title {
            self.copyright_page = Vec::with_capacity(10 + events.len() + title.len());
            self.copyright_page
                .push(BookEvent::BeginSemantic(SemanticRole::Copyrightpage));
            self.copyright_page
                .push(Event::Start(Tag::UnindentedParagraph).into());
            self.copyright_page.push(Event::Start(Tag::Sans).into());
            self.copyright_page.push(Event::Start(Tag::Strong).into());
            self.copyright_page.append(&mut title);
            self.copyright_page.push(Event::End(Tag::Strong).into());
            self.copyright_page.push(Event::End(Tag::Sans).into());
            self.copyright_page
                .push(Event::End(Tag::UnindentedParagraph).into());
            self.copyright_page.append(&mut events);
            self.copyright_page
                .push(BookEvent::EndSemantic(SemanticRole::Copyrightpage));
        } else {
            events.wrap_division(SemanticRole::Copyrightpage);
            self.copyright_page = events;
        }
        self
    }

    /// Set the copyright page from a file
    pub fn add_copyright_page_from_file<P: AsRef<Path>>(
        &mut self,
        p: P,
    ) -> Result<&mut Self, std::io::Error> {
        let s = self.read_file_and_add_to_sources(p)?;
        let (title, text) = s.parse_and_remove_initial_title_plain();
        self.add_copyright_page(text, title);
        Ok(self)
    }

    metadata_name_func!(author, doc = "Add an author or authors");
    metadata_name_func!(translator, doc = "Add a translator or translators");
    metadata_name_func!(editor, doc = "Add an editor or editors");
    metadata_func!(cover_designer, doc = "Name the cover designer of this work");
    metadata_func!(shorttitle, doc="Set the short title, a briefer version of the title used where a full title might be unneccesarily verbose");
    metadata_func!(subtitle, doc = "Set the work's subtitle");
    metadata_func!(author_photo_copyright_holder, doc="Name the copyright holder in any author photograph included in the work or on the covers");
    metadata_func!(
        paperback_isbn,
        doc = "An isbn for the paperback edition of this book"
    );
    metadata_func!(
        hardback_isbn,
        doc = "An isbn for the hardback edition of this book"
    );
    metadata_func!(epub_isbn, doc = "An isbn for the epub edition of this book");
    metadata_func!(publisher, doc = "The name of the publisher of this book");
    metadata_func!(publisher_address, doc = "An address for the publisher");
    metadata_func!(publisher_url, doc = "A url for the publisher");
    metadata_func!(print_location, doc = "State where the work will be printed");
    metadata_func!(
        copyright_statement,
        doc = "Set a custom copyright statement"
    );
    metadata_bool_func!(do_not_assert_moral_rights, doc="Set this flag if you do not wish to assert the moral rights of the author on the copyright page");
    metadata_bool_func!(
        is_not_first_publication,
        doc = "Set this flag if this is not the book's first publication"
    );

    /// a small helper function: given a filepath, check it is markdown, read it,
    /// add the path to this book's resource list and return the string
    fn read_file_and_add_to_sources<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<String, std::io::Error> {
        let path = path.as_ref();
        if path.is_markdown() {
            let text = std::fs::read_to_string(path)?;
            let r = path.canonicalize();
            if let Ok(dir) = r {
                if dir.is_dir() {
                    self.image_dirs.insert(dir);
                } else if let Some(parent) = dir.parent() {
                    self.image_dirs.insert(parent.into());
                }
            }
            Ok(text)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Non-markdown file",
            ))
        }
    }

    /// Specifically set the halftitle of this work, rather than relying on its generation
    /// from metadata.
    pub fn set_halftitle<S: ParseHelper<'a>>(&mut self, halftitle: S) -> &mut Self {
        let mut halftitle_events = halftitle.parse_inline_plain();
        halftitle_events.make_uppercase();
        let mut halftitle_events = halftitle_events.parse();

        let mut halftitle = Vec::with_capacity(halftitle_events.len() + 5);
        halftitle.push(BookEvent::BeginDivisionHeader(false));
        halftitle.append(&mut halftitle_events);
        halftitle.push(BookEvent::EndDivisionHeader(false));
        halftitle.wrap_division(SemanticRole::Halftitle);
        self.halftitle = halftitle;
        self
    }

    /// Set the work's dedication, a brief inscription to a person or note of thanks.
    /// Longer thanks should be added as an acknowledgement.
    pub fn set_dedication<S: ParseHelper<'a>>(&mut self, dedication: S) -> &mut Self {
        let mut dedication = dedication.parse_inline();
        dedication.wrap_division(SemanticRole::Dedication);
        self.dedication = dedication;
        self
    }

    /// Set the work's colophon, a block of inline text with simple information
    /// about its production or licensing.
    pub fn set_colophon<S: ParseHelper<'a>>(&mut self, colophon: S) -> &mut Self {
        let mut contents = vec![
            BookEvent::BeginSemantic(SemanticRole::Colophon),
            BookEvent::BeginDivisionHeader(false),
            BookEvent::EndDivisionHeader(false),
            BookEvent::Event(Event::Start(Tag::UnindentedParagraph)),
        ];

        contents.append(&mut colophon.parse_inline());
        contents.push(BookEvent::Event(Event::End(Tag::UnindentedParagraph)));
        contents.push(BookEvent::EndSemantic(SemanticRole::Colophon));
        self.colophon = contents;
        self
    }

    add_authored_ancillary_text!(
		add_foreword,
		add_foreword_authors,
		doc="Add a foreword, with an optional title and authors. If no title is set but the text of the foreword begins with a title, this will be used instead.",
		SemanticRole::Foreword,
		forewords,
		add_foreword_from_file,
		doc="Add a foreword from a file"
	);

    add_authored_ancillary_text!(
		add_afterword,
		add_afterword_authors,
		doc="Add an afterword, with an optional title and authors. If no title is set but the text of the afterword begins with a title, this will be used instead.",
		SemanticRole::Afterword,
		afterwords,
		add_afterword_from_file,
		doc="Add an afterword from a file"
	);

    add_authored_ancillary_text!(
		add_introduction,
		add_introduction_authors,
		doc="Add an introduction, with an optional title and authors. If no title is set but the text of the introduction begins with a title, this will be used instead.",
		SemanticRole::Introduction,
		introductions,
		add_introduction_from_file,
		doc="Add an introduction from a file"
	);

    add_unauthored_ancillary_text!(
        add_preface,
        doc = "Add a preface",
        SemanticRole::Preface,
        prefaces,
        add_preface_from_file,
        doc = "Add a preface from a file"
    );

    add_unauthored_ancillary_text!(
        add_acknowledgements,
        doc = "Add acknowledgements or an author's note",
        SemanticRole::Acknowledgements,
        acknowledgements,
        add_acknowledgements_from_file,
        doc = "Add acknowledgements from a file"
    );

    /// Add an appendix
    pub fn add_appendix<P>(&mut self, text: P, title: Option<P>) -> &mut Self
    where
        P: ParseHelper<'a>,
    {
        let (mut title, mut text) = if title.is_none() {
            text.parse_and_remove_initial_title()
        } else {
            let title = title.map(|t| t.parse_inline());
            (title, text.parse())
        };
        let mut len = text.len() + 7;
        if let Some(ref t) = title {
            len += t.len();
        }
        let mut events = Vec::with_capacity(len);
        events.push(BookEvent::BeginSemantic(SemanticRole::Appendix));
        events.push(BookEvent::BeginDivisionHeader(false));
        if let Some(title_events) = title.as_mut() {
            events.append(title_events);
        }
        if let Some(label) = SemanticRole::Appendix.get_label() {
            let label = BookEvent::DivisionHeaderLabel {
                text: Some(label.into()),
                number: Some(self.appendices_count),
                number_format: NumberFormat::Letter,
            };
            events.push(label);
        }
        events.push(BookEvent::EndDivisionHeader(false));
        events.append(&mut text);
        events.push(BookEvent::EndSemantic(SemanticRole::Appendix));

        self.appendices.append(&mut events);
        self.appendices_count += 1;
        self
    }

    read_file_wrapper!(
        doc = "Add an appendix from a file",
        add_appendix_from_file,
        add_appendix
    );

    /// Add a fragment of mainmatter
    pub fn add_mainmatter<P: ParseHelper<'a>>(&mut self, text: P) -> &mut Self {
        let mut text = text.parse_plain();
        self.mainmatter.append(&mut text);
        self
    }

    /// Add a fragment of mainmatter from a file
    pub fn add_mainmatter_from_file<P: AsRef<Path>>(
        &mut self,
        filepath: P,
    ) -> Result<&mut Self, std::io::Error> {
        let s = self.read_file_and_add_to_sources(filepath)?;
        self.add_mainmatter(s);
        Ok(self)
    }

    /// Add an epigraph
    pub fn add_epigraph<P1, P2>(&mut self, text: P1, source: Option<P2>) -> &mut Self
    where
        P1: ParseHelper<'a>,
        P2: ParseHelper<'a>,
    {
        let mut epigraph = vec![
            BookEvent::BeginSemantic(SemanticRole::Epigraph),
            BookEvent::BeginEpigraphText,
        ];
        let mut text = text.parse_plain();
        if let Some(Event::Start(Tag::Paragraph)) = text.first() {
            text[0] = Event::Start(Tag::UnindentedParagraph);
            let end = text
                .iter_mut()
                .find(|e| matches!(e, Event::End(Tag::Paragraph)))
                .unwrap();
            *end = Event::End(Tag::UnindentedParagraph);
        };
        epigraph.append(&mut text.parse());
        epigraph.push(BookEvent::EndEpigraphText);
        if let Some(source) = source {
            epigraph.push(BookEvent::BeginEpigraphSource);
            epigraph.append(&mut source.parse_inline());
            epigraph.push(BookEvent::EndEpigraphSource);
        }
        epigraph.push(BookEvent::EndSemantic(SemanticRole::Epigraph));
        self.epigraphs.append(&mut epigraph);
        self.epigraph_count += 1;
        self
    }

    /// Add an epigraph where the source is generated;
    /// it will default to representing the author of the epigraph's source in small caps,
    /// and the title of the epigraph's source in italics or in single quotes
    /// if `quote_title` is true
    pub fn add_explicit_epigraph<P: ParseHelper<'a>>(
        &mut self,
        text: P,
        author: Option<&str>,
        title_of_source: Option<&str>,
        quote_title: bool,
    ) -> &mut Self {
        let title = if let Some(title) = title_of_source {
            if quote_title {
                Some(format!("‘{}’", title))
            } else {
                Some(format!("*{}*", title))
            }
        } else {
            None
        };

        let author = author.map(|n| format!("<span class=\"smallcaps\">{}</span>", n));
        let mut source = String::new();
        if let Some(author) = author.as_ref() {
            source.push_str(author);
        }
        if title.is_some() && author.is_some() {
            source.push_str(", ");
        }
        if let Some(title) = title.as_ref() {
            source.push_str(title);
        }

        let source = if source.is_empty() {
            None
        } else {
            Some(source)
        };

        self.add_epigraph(text, source)
    }

    fn get_titlepage(&self) -> Vec<BookEvent<'a>> {
        let mut contents = Vec::with_capacity(5);
        let contributors = self.metadata.get_titlepage_contributors();
        let mut title = self.metadata.get_title().to_string().parse_inline();
        let subtitle = self
            .metadata
            .get_subtitle()
            .map(|s| s.to_string().parse_inline());

        contents.push(BookEvent::BeginTitlePage);
        contents.push(BookEvent::BeginTitlePageTitle);
        contents.append(&mut title);
        contents.push(BookEvent::EndTitlePageTitle);
        if let Some(mut subtitle) = subtitle {
            contents.push(BookEvent::BeginTitlePageSubTitle);
            contents.append(&mut subtitle);
            contents.push(BookEvent::EndTitlePageSubTitle);
        }
        contents.push(BookEvent::TitlePageContributors(contributors));
        contents.push(BookEvent::EndTitlePage);
        contents
    }

    fn frontmatter_len(&self) -> usize {
        self.halftitle.len()
            + self.copyright_page.len()
            + self.dedication.len()
            + self.forewords.len()
            + self.epigraphs.len()
            + self.introductions.len()
            + self.prefaces.len()
    }

    fn backmatter_len(&self) -> usize {
        self.appendices.len()
            + self.afterwords.len()
            + self.colophon.len()
            + self.acknowledgements.len()
    }

    /// Finish processing this book, generate missing sections such as a titlepage (unless they have been suppressed),
    /// do some tidying like normalising image paths
    /// and return a `BookSrc` ready for rendering
    pub fn process(&mut self) -> BookSrc<'a> {
        // first set some missing elements

        let mut estimated_len = 3;

        let titlepage = if self.no_titlepage {
            None
        } else {
            let titlepage = self.get_titlepage();
            estimated_len += titlepage.len();
            Some(titlepage)
        };

        if !self.no_halftitle && self.halftitle.is_empty() {
            let shorttitle: Option<Cow<'a, str>> = match self.metadata.shorttitle.as_ref() {
                Some(Cow::Borrowed(s)) => Some(Cow::Borrowed(s)),
                Some(e) => Some(Cow::Owned(e.to_string())),
                None => None,
            };

            let shorttitle = shorttitle.unwrap_or_else(|| match &self.metadata.title {
                Cow::Borrowed(s) => Cow::Borrowed(s),
                Cow::Owned(t) => Cow::Owned(t.clone()),
            });
            self.set_halftitle(shorttitle);
        }

        if !self.no_copyrightpage && self.copyright_page.is_empty() {
            let shorttitle = self.metadata.get_short_title().to_string();
            let cp = self.metadata.get_copyright_page_text();
            self.add_copyright_page(cp, Some(shorttitle));
        }

        estimated_len += self.frontmatter_len();

        let mut mainmatter = std::mem::take(&mut self.mainmatter).divide_into_sections();

        estimated_len += mainmatter.len();
        estimated_len += self.backmatter_len();

        let mut contents = Vec::with_capacity(estimated_len);

        contents.push(BookEvent::BeginFrontmatter);
        if !self.no_halftitle {
            contents.append(&mut self.halftitle);
        }

        if let Some(mut titlepage) = titlepage {
            contents.append(&mut titlepage);
        }

        if !self.no_copyrightpage {
            contents.append(&mut self.copyright_page);
        }

        macro_rules! add_if_not_empty {
            ($field:ident) => {
                if !self.$field.is_empty() {
                    contents.append(&mut self.$field);
                }
            };
        }

        add_if_not_empty!(dedication);
        add_if_not_empty!(forewords);

        // just a slight trickiness here -- if the primary authors wrote the introduction, we want to put any epigraphs
        // before any introduction, but otherwise afterwards

        if self.metadata.has_no_introduction_authors() {
            add_if_not_empty!(epigraphs);
            add_if_not_empty!(introductions);
        } else {
            add_if_not_empty!(introductions);
            add_if_not_empty!(epigraphs);
        }

        add_if_not_empty!(prefaces);
        contents.push(BookEvent::BeginMainmatter);
        contents.append(&mut mainmatter);

        if !self.appendices.is_empty()
            || !self.afterwords.is_empty()
            || !self.acknowledgements.is_empty()
            || !self.colophon.is_empty()
        {
            contents.push(BookEvent::BeginBackmatter);
        }

        add_if_not_empty!(appendices);
        add_if_not_empty!(afterwords);
        add_if_not_empty!(acknowledgements);
        add_if_not_empty!(colophon);

        let image_dirs = self.image_dirs.drain().collect::<Vec<_>>();

        match contents.replace_missing_image_paths(&image_dirs) {
            Ok(_) => {}
            Err(missing_paths) => {
                eprintln!("The following images could not be found:");
                for item in missing_paths {
                    eprintln!("- {:?}", item.display());
                }
            }
        }

        BookSrc {
            metadata: std::mem::take(&mut self.metadata),
            contents,
            expected_epigraph_count: self.epigraph_count,
            expected_appendices_count: self.appendices_count.into(),
        }
    }
}

/// A processed book source ready to be rendered
#[derive(Debug, Clone, Default)]
pub struct BookSrc<'a> {
    /// associated metadata of this book
    pub metadata: Metadata<'a>,
    /// the events which make up this book
    pub contents: Vec<BookEvent<'a>>,
    /// the number of appendices in this book
    pub expected_appendices_count: usize,
    /// the number of epigraphs to this work
    pub expected_epigraph_count: usize,
}

impl<'a> BookSrc<'a> {
    /// Change chapter and part headers to be of the specified format
    pub fn change_headers(&mut self, format: TextHeaderOptions) {
        let mut in_chapter = false;
        let mut in_part = false;
        let mut in_chapter_header = false;
        let mut in_part_header = false;
        let mut chapter_header_text = Vec::new();
        let mut part_header_text = Vec::new();
        let mut chapter_labels = Vec::new();
        let mut part_labels = Vec::new();
        for event in self.contents.iter_mut() {
            match event {
                BookEvent::BeginSemantic(SemanticRole::Chapter) => {
                    in_chapter = true;
                }
                BookEvent::EndSemantic(SemanticRole::Chapter) => {
                    in_chapter = false;
                }
                BookEvent::BeginSemantic(SemanticRole::Part) => {
                    in_part = true;
                }
                BookEvent::EndSemantic(SemanticRole::Part) => {
                    in_part = false;
                }
                BookEvent::BeginDivisionHeader(_) if in_chapter => {
                    in_chapter_header = true;
                }
                BookEvent::BeginDivisionHeader(_) if in_part => {
                    in_part_header = true;
                }
                BookEvent::EndDivisionHeader(_) if in_chapter => {
                    in_chapter_header = false;
                }
                BookEvent::EndDivisionHeader(_) if in_part => {
                    in_part_header = false;
                }
                label
                @
                BookEvent::DivisionHeaderLabel {
                    text: _,
                    number: _,
                    number_format: _,
                } if in_chapter_header => {
                    chapter_labels.push(label);
                }
                label
                @
                BookEvent::DivisionHeaderLabel {
                    text: _,
                    number: _,
                    number_format: _,
                } if in_part_header => part_labels.push(label),
                e @ BookEvent::Event(Event::Text(_)) if in_chapter_header => {
                    chapter_header_text.push(e);
                }
                e @ BookEvent::Event(Event::Text(_)) if in_part_header => {
                    part_header_text.push(e);
                }
                _ => {}
            }
        }
        match (format.chapter_header_format, format.chapter_number_format) {
            (HeaderFormat::NumberAlone, n) => {
                for label in chapter_labels.into_iter() {
                    if let BookEvent::DivisionHeaderLabel {
                        text: _,
                        number: _,
                        number_format,
                    } = label
                    {
                        *number_format = n;
                    }
                }
                for text in chapter_header_text.into_iter() {
                    *text = BookEvent::Null;
                }
            }
            (HeaderFormat::TitleAlone, _) => {
                for label in chapter_labels.into_iter() {
                    *label = BookEvent::Null;
                }
            }
            (HeaderFormat::LabelAndNumberAndTitle, n) => {
                for label in chapter_labels.iter_mut() {
                    if let BookEvent::DivisionHeaderLabel {
                        text: _,
                        number: _,
                        number_format,
                    } = label
                    {
                        *number_format = n;
                    }
                }
            }
            (HeaderFormat::LabelAlone, n) => {
                for label in chapter_labels.into_iter() {
                    if let BookEvent::DivisionHeaderLabel {
                        text: _,
                        number: _,
                        number_format,
                    } = label
                    {
                        *number_format = n;
                    }
                }
                for text in chapter_header_text.into_iter() {
                    *text = BookEvent::Null;
                }
            }
            (HeaderFormat::NumberAndTitle, n) => {
                for label in chapter_labels.into_iter() {
                    if let BookEvent::DivisionHeaderLabel {
                        text,
                        number: _,
                        number_format,
                    } = label
                    {
                        *number_format = n;
                        *text = None;
                    }
                }
            }
        }
        match (format.part_header_format, format.part_number_format) {
            (HeaderFormat::NumberAlone, n) => {
                for label in part_labels.into_iter() {
                    if let BookEvent::DivisionHeaderLabel {
                        text: _,
                        number: _,
                        number_format,
                    } = label
                    {
                        *number_format = n;
                    }
                }
                for text in part_header_text.into_iter() {
                    *text = BookEvent::Null;
                }
            }
            (HeaderFormat::TitleAlone, _) => {
                for label in part_labels.into_iter() {
                    *label = BookEvent::Null;
                }
            }
            (HeaderFormat::LabelAndNumberAndTitle, n) => {
                for label in part_labels.into_iter() {
                    if let BookEvent::DivisionHeaderLabel {
                        text: _,
                        number: _,
                        number_format,
                    } = label
                    {
                        *number_format = n;
                    }
                }
            }
            (HeaderFormat::LabelAlone, n) => {
                for label in part_labels.into_iter() {
                    if let BookEvent::DivisionHeaderLabel {
                        text: _,
                        number,
                        number_format,
                    } = label
                    {
                        *number_format = n;
                        *number = None;
                    }
                }
                for text in part_header_text.into_iter() {
                    *text = BookEvent::Null;
                }
            }
            (HeaderFormat::NumberAndTitle, n) => {
                for label in part_labels.into_iter() {
                    if let BookEvent::DivisionHeaderLabel {
                        text,
                        number: _,
                        number_format,
                    } = label
                    {
                        *number_format = n;
                        *text = None;
                    }
                }
            }
        }
    }
}

/// Specification of chapter header format
#[derive(Debug, Clone, Copy)]
pub enum HeaderFormat {
    /// A number alone: e.g. `1`
    NumberAlone,
    /// A title alone: e.g. `Wolves Attack!`
    TitleAlone,
    /// A label and number and title: e.g. `Chapter 1: Wolves Attack!`
    LabelAndNumberAndTitle,
    /// A label alone: e.g. `Chapter 1`
    LabelAlone,
    /// Number and title: e.g. `1: Wolves Attack!`
    NumberAndTitle,
}

impl Default for HeaderFormat {
    fn default() -> Self {
        HeaderFormat::LabelAndNumberAndTitle
    }
}

/// Specification of text headers
#[derive(Debug, Clone, Copy)]
pub struct TextHeaderOptions {
    /// how to format chapter headers
    pub chapter_header_format: HeaderFormat,
    /// how to format part headers
    pub part_header_format: HeaderFormat,
    /// how to format numbers in chapter headers
    pub chapter_number_format: NumberFormat,
    /// the format of part numbers
    pub part_number_format: NumberFormat,
}

impl Default for TextHeaderOptions {
    fn default() -> Self {
        TextHeaderOptions {
            chapter_header_format: HeaderFormat::default(),
            part_header_format: HeaderFormat::default(),
            chapter_number_format: NumberFormat::Arabic,
            part_number_format: NumberFormat::Roman,
        }
    }
}

impl TextHeaderOptions {
    /// set these options to use words for chapter labels
    pub fn use_words_for_chapter_labels(&mut self) {
        self.chapter_number_format = NumberFormat::Words;
    }

    /// set these options to use roman numerals for chapter numbers
    pub fn use_roman_numerals_for_chapter_labels(&mut self) {
        self.chapter_number_format = NumberFormat::Roman;
    }

    /// set these options to suppress chapter labels
    pub fn suppress_chapter_labels(&mut self) {
        self.chapter_header_format = HeaderFormat::TitleAlone;
    }

    /// set these options to suppress chapter titles
    pub fn suppress_chapter_titles(&mut self) {
        self.chapter_header_format = HeaderFormat::LabelAlone;
    }

    /// set these options to show only a chapter number
    pub fn only_number_chapters(&mut self) {
        self.chapter_header_format = HeaderFormat::NumberAlone;
    }
}

/// A helper trait to parse source material
pub trait ParseHelper<'a>: Sized {
    /// parse into inline Events with the appropriate lifetime,
    fn parse_inline_plain(self) -> Vec<Event<'a>>;
    /// parse into Events with the appropriate lifetime,
    /// flattening footnotes if required
    fn parse_plain(self) -> Vec<Event<'a>>;
    /// parse into inline BookEvents with the appropriate lifetime,
    fn parse_inline(self) -> Vec<BookEvent<'a>> {
        self.parse_inline_plain()
            .into_iter()
            .map(BookEvent::from)
            .collect()
    }
    /// parse into BookEvents with the appropriate lifetime,
    /// flattening footnotes if required
    fn parse(self) -> Vec<BookEvent<'a>> {
        self.parse_plain()
            .into_iter()
            .map(BookEvent::from)
            .collect()
    }
    /// parse into Events and split off any intial title
    fn parse_and_remove_initial_title_plain(self) -> (Option<Vec<Event<'a>>>, Vec<Event<'a>>) {
        let mut events = self.parse_plain();
        let title = events.remove_initial_title();
        (title, events)
    }
    /// parse into BookEvents and split off any intial title
    fn parse_and_remove_initial_title(self) -> (Option<Vec<BookEvent<'a>>>, Vec<BookEvent<'a>>) {
        let (title, events) = self.parse_and_remove_initial_title_plain();
        let events = events.into_iter().map(BookEvent::from).collect::<Vec<_>>();
        let title = if let Some(title) = title {
            Some(title.into_iter().map(BookEvent::from).collect::<Vec<_>>())
        } else {
            None
        };
        (title, events)
    }
}

impl<'a> ParseHelper<'a> for String {
    fn parse_inline_plain(self) -> Vec<Event<'a>> {
        InlineParser::new(&self)
            .map(|e| e.into_static())
            .collect::<Vec<_>>()
    }

    fn parse_plain(self) -> Vec<Event<'a>> {
        let mut has_footnotes = false;
        let mut events = Parser::new(&self)
            .map(|e| match e {
                e @ Event::FootnoteReference(_) => {
                    has_footnotes = true;
                    e.into_static()
                }
                e => e.into_static(),
            })
            .collect::<Vec<_>>();

        if has_footnotes {
            events = flatten_footnotes(events);
        }
        events
    }
}

impl<'a> ParseHelper<'a> for &'a str {
    fn parse_inline_plain(self) -> Vec<Event<'a>> {
        InlineParser::new(self).collect::<Vec<_>>()
    }

    fn parse_plain(self) -> Vec<Event<'a>> {
        let mut has_footnotes = false;
        let mut events = Parser::new(self)
            .map(|e| match e {
                e @ Event::FootnoteReference(_) => {
                    has_footnotes = true;
                    e
                }
                _ => e,
            })
            .collect::<Vec<_>>();
        if has_footnotes {
            events = flatten_footnotes(events);
        }
        events
    }
}

impl<'a> ParseHelper<'a> for Cow<'a, str> {
    fn parse_inline_plain(self) -> Vec<Event<'a>> {
        match self {
            Cow::Borrowed(s) => s.parse_inline_plain(),
            Cow::Owned(s) => s.parse_inline_plain(),
        }
    }

    fn parse_plain(self) -> Vec<Event<'a>> {
        match self {
            Cow::Borrowed(s) => s.parse_plain(),
            Cow::Owned(s) => s.parse_plain(),
        }
    }
}

impl<'a> ParseHelper<'a> for CowStr<'a> {
    fn parse_inline_plain(self) -> Vec<Event<'a>> {
        match self {
            CowStr::Borrowed(s) => s.parse_inline_plain(),
            _ => self.to_string().parse_inline_plain(),
        }
    }

    fn parse_plain(self) -> Vec<Event<'a>> {
        match self {
            CowStr::Borrowed(s) => s.parse_plain(),
            _ => self.to_string().parse_plain(),
        }
    }
}

impl<'a> ParseHelper<'a> for Vec<Event<'a>> {
    fn parse_inline_plain(self) -> Vec<Event<'a>> {
        InlineParser::from(self).collect()
    }

    fn parse_plain(self) -> Vec<Event<'a>> {
        self
    }
}

/// Helper functions for collections of events
pub trait EventHelper<'a> {
    /// make any paragraphs in this vec unindented
    fn make_paragraphs_unindented(&mut self);
    /// remove an initial top-level header, leaving behind either trailing events
    /// or an empty collection if no events followed the header
    fn remove_initial_title(&mut self) -> Option<Self>
    where
        Self: Sized;
    /// make plain text in this uppercase
    fn make_uppercase(&mut self);
    /// divide into semantic sections
    fn divide_into_sections(self) -> Vec<BookEvent<'a>>;
}

impl<'a> EventHelper<'a> for Vec<Event<'a>> {
    fn make_paragraphs_unindented(&mut self) {
        for event in self.iter_mut() {
            match event {
                Event::Start(Tag::Paragraph) => *event = Event::Start(Tag::UnindentedParagraph),
                Event::End(Tag::Paragraph) => *event = Event::End(Tag::UnindentedParagraph),
                _ => {}
            }
        }
    }

    fn remove_initial_title(&mut self) -> Option<Self> {
        if let Some(Event::Start(Tag::Heading(1))) = self.first() {
            let title_end = self
                .iter()
                .position(|e| matches!(e, Event::End(Tag::Heading(1))))
                .unwrap();
            if title_end == self.len() {
                Some(std::mem::take(self))
            } else {
                let mut title_events = self.drain(..=title_end);
                let _title_start = title_events.next();
                let mut title = title_events.collect::<Vec<_>>();
                title.pop(); // remove the title end
                Some(title)
            }
        } else {
            None
        }
    }

    fn make_uppercase(&mut self) {
        let mut ignore = 0;
        for event in self.iter_mut() {
            match event {
                Event::Text(t) if ignore == 0 => *t = t.to_uppercase().into(),
                Event::Start(t) => match t {
                    Tag::CodeBlock(_) => ignore += 1,
                    Tag::List(_) => ignore += 1,
                    Tag::FootnoteDefinition(_) => ignore += 1,
                    Tag::Link(_, _, _) => ignore += 1,
                    Tag::Image(_, _, _) => ignore += 1,
                    Tag::SmallCaps => ignore += 1,
                    Tag::Subscript => ignore += 1,
                    Tag::Superscript => ignore += 1,
                    Tag::FlattenedFootnote => ignore += 1,
                    _ => {}
                },
                Event::End(t) => match t {
                    Tag::CodeBlock(_) => ignore -= 1,
                    Tag::List(_) => ignore -= 1,
                    Tag::FootnoteDefinition(_) => ignore -= 1,
                    Tag::Link(_, _, _) => ignore -= 1,
                    Tag::Image(_, _, _) => ignore -= 1,
                    Tag::SmallCaps => ignore -= 1,
                    Tag::Subscript => ignore -= 1,
                    Tag::Superscript => ignore -= 1,
                    Tag::FlattenedFootnote => ignore -= 1,
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn divide_into_sections(self) -> Vec<BookEvent<'a>> {
        let step_level = StepLevel::get(self.iter());
        let mut collated = Vec::with_capacity(self.len());
        let mut in_chapter = false;
        let mut part_count = 0;
        let mut chapter_count = 0;

        let chapter_label: Option<Cow<'static, str>> =
            SemanticRole::Chapter.get_label().map(|l| l.into());
        let part_label: Option<Cow<'static, str>> =
            SemanticRole::Part.get_label().map(|l| l.into());

        for event in self.into_iter() {
            match step_level {
                StepLevel::TopToChapter => match event {
                    Event::Start(Tag::Heading(1)) => {
                        if in_chapter {
                            collated.push(BookEvent::EndSemantic(SemanticRole::Chapter));
                        }
                        collated.push(BookEvent::BeginSemantic(SemanticRole::Chapter));
                        in_chapter = true;
                        collated.push(BookEvent::BeginDivisionHeader(false));
                        chapter_count += 1;
                        collated.push(BookEvent::DivisionHeaderLabel {
                            text: chapter_label.clone(),
                            number: Some(chapter_count),
                            number_format: NumberFormat::Arabic,
                        });
                    }
                    Event::End(Tag::Heading(1)) => {
                        collated.push(BookEvent::EndDivisionHeader(false));
                    }
                    Event::Start(Tag::Heading(h)) => {
                        collated.push(Event::Start(Tag::Heading(h + 1)).into())
                    }
                    Event::End(Tag::Heading(h)) => {
                        collated.push(Event::End(Tag::Heading(h + 1)).into())
                    }
                    other => collated.push(other.into()),
                },
                StepLevel::TopToParts => match event {
                    Event::Start(Tag::Heading(1)) => {
                        if in_chapter {
                            collated.push(BookEvent::EndSemantic(SemanticRole::Chapter));
                            in_chapter = false;
                        }
                        collated.push(BookEvent::BeginSemantic(SemanticRole::Part));
                        collated.push(BookEvent::BeginDivisionHeader(false));
                        part_count += 1;
                        collated.push(BookEvent::DivisionHeaderLabel {
                            text: part_label.clone(),
                            number: Some(part_count),
                            number_format: NumberFormat::Roman,
                        });
                    }
                    Event::End(Tag::Heading(1)) => {
                        collated.push(BookEvent::EndDivisionHeader(false));
                        collated.push(BookEvent::EndSemantic(SemanticRole::Part));
                    }
                    Event::Start(Tag::Heading(2)) => {
                        if in_chapter {
                            collated.push(BookEvent::EndSemantic(SemanticRole::Chapter));
                        }
                        collated.push(BookEvent::BeginSemantic(SemanticRole::Chapter));
                        in_chapter = true;
                        collated.push(BookEvent::BeginDivisionHeader(false));
                        chapter_count += 1;
                        collated.push(BookEvent::DivisionHeaderLabel {
                            text: chapter_label.clone(),
                            number: Some(chapter_count),
                            number_format: NumberFormat::Arabic,
                        });
                    }
                    Event::End(Tag::Heading(2)) => {
                        collated.push(BookEvent::EndDivisionHeader(false));
                    }
                    Event::Start(Tag::Heading(h)) => {
                        collated.push(Event::Start(Tag::Heading(h + 2)).into());
                    }
                    Event::End(Tag::Heading(h)) => {
                        collated.push(Event::End(Tag::Heading(h + 2)).into());
                    }
                    other => collated.push(other.into()),
                },
            }
        }

        if in_chapter {
            collated.push(BookEvent::EndSemantic(SemanticRole::Chapter));
        }
        collated
    }
}

trait BookEventHelper {
    fn wrap_division(&mut self, role: SemanticRole);
    fn replace_missing_image_paths(&mut self, image_dirs: &[PathBuf]) -> Result<(), Vec<PathBuf>>;
}

impl<'a> BookEventHelper for Vec<BookEvent<'a>> {
    fn wrap_division(&mut self, role: SemanticRole) {
        self.insert(0, BookEvent::BeginSemantic(role));
        self.push(BookEvent::EndSemantic(role));
    }

    fn replace_missing_image_paths(&mut self, image_dirs: &[PathBuf]) -> Result<(), Vec<PathBuf>> {
        let image_dests = self
            .iter_mut()
            .filter_map(|event| match event {
                BookEvent::Event(Event::Start(Tag::Image(_, dest, _))) => Some(dest),
                BookEvent::Event(Event::End(Tag::Image(_, dest, _))) => Some(dest),
                _ => None,
            })
            .map(|mref| {
                let p = match mref {
                    CowStr::Borrowed(ref s) => PathBuf::from(s),
                    CowStr::Boxed(ref s) => PathBuf::from(s.to_string()),
                    CowStr::Inlined(ref s) => PathBuf::from(s.to_string()),
                };

                (mref, p)
            });

        let mut mdests: HashMap<PathBuf, Vec<&mut CowStr>> = HashMap::new();
        for (mref, p) in image_dests {
            mdests.entry(p).or_default().push(mref);
        }

        let mut missing = Vec::new();

        for (p, v) in mdests.into_iter().filter(|(k, _v)| !k.exists()) {
            let replacement = image_dirs
                .iter()
                .filter_map(|d| match d.join(&p).canonicalize() {
                    Ok(p) => Some(p),
                    Err(_) => None,
                })
                .filter(|p| p.is_file())
                .filter_map(|p| match p.to_string_lossy() {
                    Cow::Borrowed(s) => Some(s.to_string()),
                    _ => None,
                })
                .map(CowStr::from)
                .next();
            if let Some(replacement) = replacement {
                for item in v.into_iter() {
                    *item = replacement.clone();
                }
            } else {
                missing.push(p);
            }
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_title() {
        let title = "# Hello *world*\n\nBoring old text";
        let (title, _text) = title.parse_and_remove_initial_title_plain();
        let expected_title = vec![
            Event::Text(CowStr::Borrowed("Hello ")),
            Event::Start(Tag::Emphasis),
            Event::Text(CowStr::Borrowed("world")),
            Event::End(Tag::Emphasis),
        ];
        assert_eq!(title, Some(expected_title));
    }
}
