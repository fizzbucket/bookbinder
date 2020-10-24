//! This crate provides support, with various options, for transforming a `BookSrc` into an epub 3.2 file.
#![deny(dead_code)]
#![deny(unreachable_patterns)]
#![deny(unused_extern_crates)]
#![deny(unused_imports)]
#![deny(unused_qualifications)]
#![deny(clippy::all)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

use bookbinder_ast::helpers::{BookEventIteratorHelper, EpubMarker};
use bookbinder_ast::Metadata;
use bookbinder_ast::{BookEvent, BookSrc, SemanticRole, TextHeaderOptions};
use bookbinder_common::MimeTypeHelper;
use epub_bundler::{EpubBundlingError, EpubContent, EpubResource, EpubSource};
use extended_pulldown::{CodeBlockKind, CowStr, Event, Tag};
use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use temp_file_name::TempFilePath;
use uuid::Uuid;
mod svg_titlepage_generator;
mod text_splitting;
use bookbinder_ast::helpers::{CollatedEpigraph, CollatedHeader, CollatedTitlePage};
use std::error::Error;
use svg_titlepage_generator::{generate_svg_titlepage, TitleEvent};

// A fragment added by the build script;
// it contains a set of fns:
//  - const fn get_header_level(role: SemanticRole) -> Option<usize>
//  - const fn get_header_classes(role: SemanticRole) -> Option<&'static str>
//  - const fn get_section_classes(role: SemanticRole) -> Option<&'static str>
//  - const fn get_epub_type(role: SemanticRole) -> &'static str
//  - const fn get_matter(role: SemanticRole) -> &'static str
//  - const fn get_default_toc_format(role: SemanticRole) -> TocFormat
//  - const fn get_include_stylesheet(role: SemanticRole) -> bool
//  - const fn get_additional_head(role: SemanticRole) -> Option<&'static str>
//  - const fn get_section_wrapper_div_classes(role: SemanticRole) -> Option<&'static str>
//  - const fn get_default_toc_level(role: SemanticRole) -> Option<usize>
// These are basically ways to get from a `role` to a particular constant value:
// e.g. the `epub-type` of this role is `x`, it's part of the frontmatter,
// put it in a section with these special classes, etc.
// As can be imagined, it's nicer to have this built from a toml source than
// to expresss it very long-windedly in code.
include!(concat!(env!("OUT_DIR"), "/semantic_role_const_fns.rs"));

static DEFAULT_CSS: &str = include_str!("default_css.css");

/// Options for rendering as an epub
#[derive(Debug, Default, Clone)]
pub struct Options {
    /// Custom css
    pub css: Option<PathBuf>,
    /// A cover image to use
    pub cover_image: Option<PathBuf>,
    /// A titlepage image to use
    pub titlepage: Option<PathBuf>,
    /// A publisher logo to use while generating the titlepage
    pub publisher_imprint_logo: Option<PathBuf>,
    /// A typeface to use while generating the titlepage
    pub titlepage_typeface: Option<Cow<'static, str>>,
    /// The format of headers
    pub header_options: TextHeaderOptions,
    /// A new label for chapters -- e.g. `Letter 1` instead of `Chapter 1`
    pub chapter_label: Option<Cow<'static, str>>,
}

impl Options {
    /// Set custom css to use
    pub fn css<P: Into<PathBuf>>(&mut self, css: P) -> &mut Self {
        self.css = Some(css.into());
        self
    }

    /// Set a custom label to use for chapters instead of `Chapter`
    pub fn set_chapter_label<S: Into<Cow<'static, str>>>(&mut self, label: S) -> &mut Self {
        self.chapter_label = Some(label.into());
        self
    }

    /// Set a cover image
    pub fn cover_image<P: Into<PathBuf>>(&mut self, cover_image: P) -> &mut Self {
        self.cover_image = Some(cover_image.into());
        self
    }

    /// Set a titlepage image to use instead of generating one
    pub fn titlepage<P: Into<PathBuf>>(&mut self, titlepage_image: P) -> &mut Self {
        self.titlepage = Some(titlepage_image.into());
        self
    }

    /// Set a publisher logo to use when generating the titlepage
    pub fn publisher_imprint_logo<P: Into<PathBuf>>(&mut self, logo: P) -> &mut Self {
        self.publisher_imprint_logo = Some(logo.into());
        self
    }

    /// Set the name of a typeface to use when generating the titlepage
    pub fn titlepage_typeface<S: Into<Cow<'static, str>>>(&mut self, typeface: S) -> &mut Self {
        self.titlepage_typeface = Some(typeface.into());
        self
    }

    /// Do not show any given chapter's title -- rely instead
    /// on its label.
    /// For example, `Chapter 1: Wolves Attack!` would be
    /// represented as `Chapter 1`
    pub fn suppress_chapter_titles(&mut self) -> &mut Self {
        self.header_options.suppress_chapter_titles();
        self
    }

    /// Do not label chapters as such in headings; i.e. use only the
    /// chapter title.
    /// For example, `Chapter 1: Wolves Attack!` would be
    /// represented as `Wolves Attack!`
    pub fn suppress_chapter_label(&mut self) -> &mut Self {
        self.header_options.suppress_chapter_labels();
        self
    }

    /// Indicate chapters only by using a numerical indication,
    /// in whatever format.
    /// For example, `Chapter 1: Wolves Attack!` would be
    /// represented as `1`,
    /// or `I` if `use_roman_numerals_for_chapter_labels` was called
    pub fn only_number_chapters(&mut self) -> &mut Self {
        self.header_options.only_number_chapters();
        self
    }

    /// Label chapters with roman rather than arabic numerals:
    /// e.g. a third chapter would be labelled
    /// as `Chapter III` not `Chapter 3`
    pub fn use_roman_numerals_for_chapter_labels(&mut self) -> &mut Self {
        self.header_options.use_roman_numerals_for_chapter_labels();
        self
    }

    /// Label chapters with words rather than numbers:
    /// e.g. a first chapter would be labelled
    /// as `Chapter One` not `Chapter 1`
    pub fn use_words_for_chapter_labels(&mut self) -> &mut Self {
        self.header_options.use_words_for_chapter_labels();
        self
    }

    /// Modify a Vec of events in accordance with these options
    fn modify_events(&self, events: &mut Vec<BookEvent<'_>>) {
        // replace chapter labels if necessary
        if self.chapter_label.is_some() {
            let default_chapter_label = bookbinder_ast::SemanticRole::Chapter.get_label();
            let labels = events.iter_mut().filter_map(|event| {
                if let BookEvent::DivisionHeaderLabel { text, .. } = event {
                    match text {
                        t if t.as_deref() == default_chapter_label => Some(t),
                        _ => None,
                    }
                } else {
                    None
                }
            });
            for old_label in labels {
                *old_label = self.chapter_label.clone();
            }
        }
    }

    /// Return either the path to this option set's specified css,
    /// or (writing it if necessary) a path to default css instead
    fn get_css(&self) -> Result<PathBuf, RenderingError> {
        if let Some(ref css) = self.css {
            return Ok(css.clone());
        }
        let css_path = DEFAULT_CSS.temp_file_path(Some("bookbinder"), "css");
        if css_path.exists() {
            Ok(css_path)
        } else {
            std::fs::write(&css_path, DEFAULT_CSS)
                .map_err(|_| RenderingError::FileWriteError(css_path.clone()))?;
            Ok(css_path)
        }
    }

    fn get_titlepage(&self) -> Option<PathBuf> {
        if let Some(ref p) = self.titlepage {
            if p.is_epub_supported_image() {
                // implies has extension
                if p.is_file() {
                    return Some(p.clone());
                }
            } else if p.is_pdf() {
                if let Some(svg) = std::fs::read(&p)
                    .ok()
                    .map(|d| bookbinder_common::convert_pdf_to_svg(&d, None).ok())
                    .flatten()
                {
                    let svg_path = svg.temp_file_path(Some("bookbinder"), "svg");
                    if std::fs::write(&svg_path, svg).is_ok() {
                        return Some(svg_path);
                    }
                }
            }
        }
        None
    }
}

enum TocFormat {
    NoTocEntry,
    TitleOnly,
    TitleAndLabel,
    Provided(&'static str),
}

/// escape a CowStr<'_> for use in html
fn escape_cowstr_for_html(cowstr: CowStr<'_>) -> Cow<'_, str> {
    match cowstr {
        CowStr::Borrowed(t) => bookbinder_common::escape_to_html(t),
        CowStr::Boxed(t) => bookbinder_common::escape_to_html(t.to_string()),
        CowStr::Inlined(s) => bookbinder_common::escape_to_html(s.to_string()),
    }
}

#[derive(Debug, Clone, Hash)]
pub(crate) struct TitlePageSource<'a, S>
where
    S: AsRef<str> + std::hash::Hash,
{
    title_events: Vec<TitleEvent<'a>>,
    subtitle_events: Option<Vec<TitleEvent<'a>>>,
    contributors: Option<Vec<(Option<&'a str>, Vec<S>)>>,
    logo: Option<&'a Path>,
    typeface: Option<&'a str>,
}

impl<'a> TitlePageSource<'a, Cow<'a, str>> {
    fn map_events(events: Vec<Event<'a>>) -> Vec<TitleEvent<'a>> {
        let mut out = Vec::with_capacity(events.len());
        let mut emph_count = 0;

        for event in events.into_iter() {
            match event {
                Event::Text(t) => {
                    let text = escape_cowstr_for_html(t);
                    if emph_count > 0 {
                        out.push(TitleEvent::Emphasised(text));
                    } else {
                        out.push(TitleEvent::Text(text));
                    }
                }
                Event::Start(Tag::Emphasis) => {
                    emph_count += 1;
                }
                Event::End(Tag::Emphasis) => {
                    emph_count -= 1;
                }
                _ => {}
            }
        }
        out
    }

    fn new(page: CollatedTitlePage<'a>, logo: Option<&'a Path>, typeface: Option<&'a str>) -> Self {
        TitlePageSource {
            title_events: Self::map_events(page.title),
            subtitle_events: page.subtitle.map(Self::map_events),
            contributors: page.contributors,
            logo,
            typeface,
        }
    }
}

/// Support for rendering to an epub
pub trait EpubRenderer: Sized {
    /// render to an epub with the given options
    fn render_to_epub(self, options: Options) -> Result<Vec<u8>, RenderingError>;
    /// render to an epub with default options
    fn render_to_epub_default(self) -> Result<Vec<u8>, RenderingError> {
        let options = Options::default();
        self.render_to_epub(options)
    }
}

impl EpubRenderer for BookSrc<'_> {
    fn render_to_epub(mut self, mut options: Options) -> Result<Vec<u8>, RenderingError> {
        let pages = self.get_pages(&mut options)?;

        let mut resources = HashSet::new();
        let mut contents = Vec::with_capacity(pages.len());

        for page in pages.into_iter() {
            for resource in page.associated_resources.into_iter() {
                resources.insert(resource);
            }
            let mut content = EpubContent::new(page.xhtml);
            if let Some(toc_title) = page.toc_title {
                content
                    .set_toc_title(toc_title, page.toc_level.unwrap_or(1))
                    .unwrap();
            }
            contents.push(content);
        }

        let resources = resources
            .into_iter()
            .map(EpubResource::from_file)
            .collect::<Result<Vec<_>, _>>()
            .map_err(RenderingError::ResourceError)?;

        let mut epub_source = EpubSource::new();
        for content in contents.into_iter() {
            epub_source.add_content(content)?;
        }
        for resource in resources.into_iter() {
            epub_source.add_resource(resource)?;
        }

        // now add metadata

        self.metadata.add_to_epub_source(&mut epub_source);

        // now set the cover image, if we can, or default
        // to the titlepage if we can't.

        // In theory this can be of any format;
        // in practice, iBooks requires jpeg and most
        // other providers are happier with this also.

        if let Some(cover_image) = options.cover_image.take() {
            let jpeg = bookbinder_common::convert_to_jpg(&cover_image)
                .map_err(|_| RenderingError::ImageConversionError(cover_image.clone()))?;

            let cover_resource = EpubResource::new_jpg(jpeg);
            epub_source
                .set_cover_image(cover_resource)
                .map_err(|_| RenderingError::ImageConversionError(cover_image.clone()))?;
        } else if let Some(titlepage) = options.titlepage.take() {
            let ext = titlepage.extension().map(|s| s.to_str()).flatten();
            match ext {
                Some("jpg") => {
                    let jpeg = std::fs::read(&titlepage)
                        .map_err(|_| RenderingError::MissingImage(titlepage.clone()))?;
                    let cover_resource = EpubResource::new_jpg(jpeg);
                    epub_source
                        .set_cover_image(cover_resource)
                        .map_err(|_| RenderingError::ImageConversionError(titlepage.clone()))?;
                }
                _ => {
                    let mut jpeg_path = titlepage.clone();
                    jpeg_path.set_extension("jpg");
                    if jpeg_path.exists() {
                        let jpeg = std::fs::read(&jpeg_path)
                            .map_err(|_| RenderingError::MissingImage(titlepage))?;
                        let cover_resource = EpubResource::new_jpg(jpeg);
                        epub_source
                            .set_cover_image(cover_resource)
                            .map_err(|_| RenderingError::ImageConversionError(jpeg_path.clone()))?;
                    } else {
                        let jpeg = bookbinder_common::convert_to_jpg(&titlepage)
                            .map_err(|_| RenderingError::ImageConversionError(titlepage.clone()))?;
                        let _ = std::fs::write(&jpeg_path, &jpeg);
                        let cover_resource = EpubResource::new_jpg(jpeg);
                        epub_source
                            .set_cover_image(cover_resource)
                            .map_err(|_| RenderingError::ImageConversionError(titlepage.clone()))?;
                    }
                }
            }
        }

        // finally, we can bundle the epub

        let bundled = epub_source.bundle()?;
        Ok(bundled)
    }
}

/// Errors possible while creating an epub
#[derive(Debug)]
pub enum RenderingError {
    /// An image was missing
    MissingImage(PathBuf),
    /// An image was of an unsupported filetype and could not be converted
    UnsupportedImage(PathBuf),
    /// While attempting to convert an unsupported image type, an error occurred
    ImageConversionError(PathBuf),
    /// An error occurred while generating the titlepage
    TitlepageGeneration,
    /// The css file specified could not be found
    MissingCss(PathBuf),
    /// There was an error while writing a file
    FileWriteError(PathBuf),
    /// An unspecified error occurred
    Unspecified(&'static str),
    /// There was an error building an epub resource
    ResourceError(String),
    /// There was an error bundling the epub
    BundlingError(EpubBundlingError),
}

impl From<&'static str> for RenderingError {
    fn from(src: &'static str) -> Self {
        RenderingError::Unspecified(src)
    }
}

impl From<EpubBundlingError> for RenderingError {
    fn from(src: EpubBundlingError) -> Self {
        RenderingError::BundlingError(src)
    }
}

impl std::fmt::Display for RenderingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        formatter.write_fmt(format_args!("{:?}", self))
    }
}

impl Error for RenderingError {}

/// - check if parts are present
/// - convert any images in the wrong format
/// - unflatten any flattened footnotes
///   and place them in a block at the end of the semantic division
///   in which they occurred
fn preprocess<'a>(src: Vec<BookEvent<'a>>) -> Result<(bool, Vec<BookEvent<'a>>), RenderingError> {
    let mut events = Vec::with_capacity(src.len());
    let mut footnotes = Vec::new();
    let mut current_footnote = Vec::new();
    let mut in_footnote = false;
    let mut has_parts = false;
    let mut changed_image_dests = HashMap::new();

    for event in src.into_iter() {
        match event {
            e @ BookEvent::BeginSemantic(SemanticRole::Part) => {
                has_parts = true;
                events.push(e);
            }
            e @ BookEvent::EndSemantic(_) => {
                if !footnotes.is_empty() {
                    events.append(&mut footnotes);
                }
                events.push(e);
            }
            BookEvent::Event(Event::Start(Tag::FlattenedFootnote)) => {
                in_footnote = true;
            }
            BookEvent::Event(Event::End(Tag::FlattenedFootnote)) => {
                let marker = Uuid::new_v4().to_string();
                let marker: CowStr = marker.into();

                footnotes.push(Event::Start(Tag::FootnoteDefinition(marker.clone())).into());
                footnotes.append(&mut current_footnote);
                footnotes.push(Event::End(Tag::FootnoteDefinition(marker.clone())).into());
                in_footnote = false;
                events.push(BookEvent::Event(Event::FootnoteReference(marker)));
            }
            BookEvent::Event(Event::End(Tag::Image(t, dest, alt))) => {
                if let Some(new_dest) = changed_image_dests.remove(&dest) {
                    events.push(Event::End(Tag::Image(t, new_dest, alt)).into());
                } else {
                    events.push(Event::End(Tag::Image(t, dest, alt)).into());
                }
            }
            BookEvent::Event(Event::Start(Tag::Image(t, dest, alt))) => {
                let image_path = PathBuf::from(dest.as_ref());
                if image_path.is_epub_supported_image() {
                    events.push(Event::Start(Tag::Image(t, dest, alt)).into());
                } else if image_path.is_pdf() {
                    let pdf_data = std::fs::read(&image_path)
                        .map_err(|_| RenderingError::MissingImage(image_path.clone()))?;
                    let svg = bookbinder_common::convert_pdf_to_svg(&pdf_data, None)
                        .map_err(|_| RenderingError::ImageConversionError(image_path.clone()))?;
                    let svg_path = svg.temp_file_path(Some("bookbinder"), "svg");
                    std::fs::write(&svg_path, &svg)
                        .map_err(|_| RenderingError::ImageConversionError(image_path.clone()))?;
                    let new_path: CowStr = svg_path.to_str().unwrap().to_string().into();
                    changed_image_dests.insert(dest, new_path.clone());
                    events.push(Event::Start(Tag::Image(t, new_path, alt)).into());
                } else {
                    return Err(RenderingError::UnsupportedImage(image_path));
                }
            }
            other if in_footnote => {
                current_footnote.push(other);
            }
            other => events.push(other),
        }
    }
    Ok((has_parts, events))
}

#[derive(Debug)]
struct EpubPage {
    xhtml: String,
    associated_resources: Vec<PathBuf>,
    toc_level: Option<usize>,
    toc_title: Option<String>,
}

impl EpubPage {
    fn new_empty(title: &str) -> Self {
        EpubPage {
			xhtml: format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\n<head><title>{}</title></head><body></body></html>", title),
			associated_resources: Vec::new(),
			toc_title: Some(title.into()),
			toc_level: Some(0)
		}
    }
}

trait MetadataAdder {
    fn add_to_epub_source(&self, epub_src: &mut EpubSource);
}

impl<'a> MetadataAdder for Metadata<'a> {
    fn add_to_epub_source(&self, epub_src: &mut EpubSource) {
        epub_src.set_title(self.title.clone()).unwrap();
        if let Some(subtitle) = self.subtitle.as_ref() {
            epub_src.set_subtitle(subtitle).unwrap();
        }
        macro_rules! add_contributors {
            ($ctb_list:ident, $add_fn:ident) => {
                for name in self.$ctb_list.iter() {
                    epub_src.$add_fn(name).unwrap();
                }
            };
        }

        if let Some(ref epub_isbn) = self.epub_isbn {
            epub_src.set_isbn(epub_isbn).unwrap();
        }

        add_contributors!(authors, add_author);
        add_contributors!(editors, add_editor);
        add_contributors!(translators, add_translator);
        add_contributors!(foreword_authors, add_author_of_foreword);
        add_contributors!(introduction_authors, add_author_of_introduction);
        add_contributors!(afterword_authors, add_author_of_afterword);
        add_contributors!(
            introduction_and_notes_authors,
            add_author_of_introduction_and_notes
        );
    }
}

#[derive(Debug)]
struct XhtmlWriter {
    target: String,
    numbers: HashMap<String, usize>,
    current_division: SemanticRole,
    in_inline_image: bool,
    inline_image_alt_buffer: String,
    /// any label seen for this page
    observed_label: Option<String>,
    /// any title seen for this page
    observed_title: Option<String>,
    associated_resources: Vec<PathBuf>,
    /// whether we have started writing footnote definitions;
    /// a flag to check whether a 'Notes' header needs to be written
    in_footnote_definitions: bool,
    /// flag to track whether the last element
    /// was something like a horizontal rule,
    /// which means a following paragraph should not
    /// be indented
    do_not_indent_next_para: bool,
    css_path: Option<PathBuf>,
    in_heading: bool,
    in_para: bool,
}

impl XhtmlWriter {
    fn new(role: SemanticRole) -> Self {
        XhtmlWriter {
            target: String::new(),
            numbers: HashMap::new(),
            current_division: role,
            in_inline_image: false,
            inline_image_alt_buffer: String::new(),
            observed_label: None,
            observed_title: None,
            associated_resources: Vec::new(),
            in_footnote_definitions: false,
            do_not_indent_next_para: false,
            css_path: None,
            in_heading: false,
            in_para: false,
        }
    }

    fn write_epigraph(&mut self, epigraph: CollatedEpigraph<'_>) {
        let text = epigraph.text;
        let source = epigraph.source;
        self.target.push_str("\n<div class=\"epigraph_content\">\n");
        for event in text.into_iter() {
            self.push(event);
        }
        self.target.push_str("</div>");
        if !source.is_empty() {
            self.target.push_str("<p class=\"epigraph_source\">");
            for event in source.into_iter() {
                self.push(event);
            }
            self.target.push_str("</p>");
        }
    }

    fn write_division_header(&mut self, header: CollatedHeader<'_, EpubMarker>) {
        let label_and_title = header.reconcile_joined_label_and_title();

        if let Some((label, title)) = label_and_title {
            let authors = header.get_authors();

            if let Some(label) = label {
                self.target.push_str(&format!(
                    "<p class=\"division_label\">{}</p>\n",
                    label.to_uppercase()
                ));
            }

            if let Some(title) = title {
                let header_level = match get_header_level(self.current_division) {
                    Some(0) | Some(1) | None => "h1",
                    Some(2) => "h2",
                    Some(3) => "h3",
                    Some(4) => "h4",
                    Some(5) => "h5",
                    _ => "h6",
                };
                if let Some(ref classes) = get_header_classes(self.current_division) {
                    // is there a less awkward way to do this?
                    let htag = if *classes == "generic_header" && authors.is_some() {
                        format!("<{} class=\"generic_header_with_authors\">", header_level)
                    } else {
                        format!("<{} class=\"{}\">", header_level, classes)
                    };
                    self.target.push_str(&htag);
                } else {
                    self.target.push('<');
                    self.target.push_str(header_level);
                    self.target.push('>');
                };
                self.target.push_str(&title);
                self.target.push_str("</");
                self.target.push_str(header_level);
                self.target.push_str(">\n");
            }

            if let Some(authors) = authors {
                self.target.push_str("\n<p class=\"division_authors\">");

                match authors {
                    (name, None) => self.target.push_str(&name.to_uppercase()),
                    (names, Some(final_name)) => {
                        self.target.push_str(&names.to_uppercase());
                        self.target.push_str(" and ");
                        self.target.push_str(&final_name.to_uppercase());
                    }
                }
                self.target.push_str("</p>\n");
            }
            self.do_not_indent_next_para = true;
        }
    }

    fn get_body(&self) -> String {
        let mut body = format!(
            "<body epub:type=\"{}\">\n\t",
            get_matter(self.current_division)
        );
        body.push_str("<section epub:type=\"");
        body.push_str(get_epub_type(self.current_division));
        body.push('"');
        if let Some(ref classes) = get_section_classes(self.current_division) {
            body.push_str(" class=\"");
            body.push_str(classes);
            body.push('"');
        }
        body.push_str(">\n");
        let mut has_wrapper_div = false;
        if let Some(ref classes) = get_section_wrapper_div_classes(self.current_division) {
            body.push_str("<div class=\"");
            body.push_str(classes);
            body.push_str("\">\n");
            has_wrapper_div = true;
        }
        body.push_str(&self.target);
        if has_wrapper_div {
            body.push_str("</div>");
        }
        body.push_str("\n</section>");
        body.push_str("\n</body>");
        body
    }

    fn get_head(&mut self) -> String {
        let mut head = String::from("<head>\n");

        let title = self
            .observed_title
            .as_deref()
            .or_else(|| self.observed_label.as_deref())
            .unwrap_or_else(|| get_epub_type(self.current_division));
        head.push_str(&format!("\t<title>{}</title>\n", title));

        if get_include_stylesheet(self.current_division) {
            if let Some(css_path) = self.css_path.take() {
                let css_filename = css_path.file_name().unwrap().to_str().unwrap();

                head.push('\t');
                head.push_str(r#"<link rel="stylesheet" type="text/css" href=""#);
                head.push_str(css_filename);
                head.push_str("\"></link>\n");
                self.associated_resources.push(css_path);
            }
        }

        if let Some(ref additional_head) = get_additional_head(self.current_division) {
            head.push_str("\n\t");
            head.push_str(additional_head.trim());
            head.push('\n');
        }
        head.push_str("</head>");
        head
    }

    fn finish(mut self) -> EpubPage {
        let mut xhtml = String::new();
        xhtml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>"#);
        xhtml.push('\n');
        xhtml.push_str(r#"<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">"#);
        xhtml.push_str(&self.get_head());
        xhtml.push_str(&self.get_body());
        xhtml.push_str("\n</html>");

        let toc_title = match get_default_toc_format(self.current_division) {
            TocFormat::NoTocEntry => None,
            TocFormat::Provided(s) => Some(s.into()),
            TocFormat::TitleOnly => {
                let title = self.observed_title;
                let label = self.observed_label;
                title.or(label)
            }
            TocFormat::TitleAndLabel => {
                let title = self.observed_title;
                let label = self.observed_label;
                match (label, title) {
                    (Some(label), Some(title)) => Some(format!("{}: {}", label, title)),
                    (Some(label), None) => Some(label),
                    (None, Some(title)) => Some(title),
                    (None, None) => None,
                }
            }
        };

        EpubPage {
            xhtml,
            associated_resources: self.associated_resources,
            toc_title,
            toc_level: get_default_toc_level(self.current_division),
        }
    }

    fn push_in_inline_image(&mut self, item: Event<'_>) {
        match item {
            Event::Text(text) => {
                let escaped = escape_cowstr_for_html(text);
                self.inline_image_alt_buffer.push_str(&escaped)
            }
            Event::End(Tag::Image(_, _, _)) => {
                self.in_inline_image = false;
                if self.inline_image_alt_buffer.is_empty() {
                    self.target.push_str("</img>");
                } else {
                    self.target.pop();
                    self.target.push_str(" alt=\"");
                    self.target
                        .push_str(&std::mem::take(&mut self.inline_image_alt_buffer));
                    self.target.push_str("></img>");
                }
            }
            _ => {}
        }
    }

    fn push_start_tag(&mut self, tag: Tag) {
        use Tag::*;
        match tag {
            UnindentedParagraph => {
                if !self.target.ends_with('\n') {
                    self.target.push('\n');
                }
                self.target.push_str("<p class=\"noindent\">");
                self.in_para = true;
            }
            Paragraph => {
                self.in_para = true;
                if !self.target.ends_with('\n') {
                    self.target.push('\n');
                }
                if self.do_not_indent_next_para {
                    self.target.push_str("<p class=\"noindent\">");
                    self.do_not_indent_next_para = false;
                } else {
                    self.target.push_str("<p>");
                }
            }
            Heading(l) => {
                self.in_heading = true;
                match l {
                    i if i < 2 => self.target.push_str("<h2 class=\"generic_subheading\">"),
                    2 => self.target.push_str("<h3 class=\"generic_subheading\">"),
                    3 => self.target.push_str("<h4 class=\"generic_subheading\">"),
                    4 => self.target.push_str("<h5 class=\"generic_subheading\">"),
                    _ => self.target.push_str("<h6 class=\"generic_subheading\">"),
                }
            }
            BlockQuote => {
                if !self.target.ends_with('\n') {
                    self.target.push('\n');
                }
                self.target.push_str("<blockquote>\n");
                self.do_not_indent_next_para = true;
            }
            BlockQuotation => {
                if !self.target.ends_with('\n') {
                    self.target.push('\n');
                }
                self.target.push_str("<blockquote>\n");
            }
            CodeBlock(CodeBlockKind::Fenced(l)) => {
                let lang = l
                    .chars()
                    .take_while(|c| *c != ',' && *c != ' ')
                    .collect::<String>();
                if !lang.is_empty() {
                    self.target.push_str("<pre><code class=\"language-");
                    self.target.push_str(&lang);
                    self.target.push_str("\">");
                } else {
                    self.target.push_str("<pre><code>");
                }
            }
            CodeBlock(CodeBlockKind::Indented) => {
                if !self.target.ends_with('\n') {
                    self.target.push('\n');
                }
                self.target.push_str("<pre><code>");
            }
            List(Some(1)) => {
                if !self.target.ends_with('\n') {
                    self.target.push('\n');
                }
                self.target.push_str("<ol>\n");
            }
            List(Some(start)) => {
                if !self.target.ends_with('\n') {
                    self.target.push('\n');
                }
                self.target.push_str(&format!("<ol start=\"{}\">\n", start));
            }
            List(None) => {
                if !self.target.ends_with('\n') {
                    self.target.push('\n');
                }
                self.target.push_str("<ul>\n");
            }
            Item => {
                if !self.target.ends_with('\n') {
                    self.target.push('\n');
                }
                self.target.push_str("<li>");
            }
            FootnoteDefinition(name) => {
                if !self.in_footnote_definitions {
                    self.target
                        .push_str("\n<h6 class=\"notes_heading\">Notes</h6>\n");
                    self.in_footnote_definitions = true;
                }

                let name = name.to_string();
                let id = bookbinder_common::escape_to_html(&name);

                let len = self.numbers.len() + 1;
                let number = *self.numbers.entry(name.clone()).or_insert(len);
                let combined = format!("\n<p id=\"{name}\" epub:type=\"footnote\" class=\"footnote\"><a href=\"#fn_ref_{name}\">{number}.</a> ", name=id, number=number);
                self.target.push_str(&combined);
            }
            Emphasis => self.target.push_str("<em>"),
            Strong => self.target.push_str("<strong>"),
            Strikethrough => self.target.push_str("<del>"),
            Link(_, dest, title) => {
                self.target.push_str("<a href=\"");
                self.target.push_str(&dest);
                if !title.is_empty() {
                    self.target.push_str("\" title=\"");
                    self.target.push_str(&escape_cowstr_for_html(title));
                }
                self.target.push_str("\">")
            }
            Image(_, dest, title) => {
                // is this a standalone figure, or an inline image?
                let mut is_figure = false;
                if self.target.ends_with("<p>") {
                    self.target.drain(self.target.len() - 3..);
                    is_figure = true;
                } else if self.target.ends_with("<p class=\"noindent\">") {
                    self.target.drain(self.target.len() - 20..);
                    is_figure = true
                };

                let p = PathBuf::from(dest.as_ref());
                let filename = p.file_name().unwrap().to_str().unwrap();

                if is_figure {
                    let figure = String::from("\n<figure>\n");
                    let mut img = String::from("  <img");
                    img.push_str(&format!(" src=\"{}\"", &filename));
                    if !title.is_empty() {
                        let title = escape_cowstr_for_html(title);
                        img.push_str(" title=\"");
                        img.push_str(&title);
                        img.push_str("\"");
                    }
                    img.push('>');
                    img.push_str("</img>\n");
                    self.target.push_str(&figure);
                    self.target.push_str(&img);
                    self.target.push_str("  <figcaption>");
                } else {
                    // this is an inline image
                    self.in_inline_image = true;
                    let mut img = format!("<img src=\"{}\"", &filename);
                    if !title.is_empty() {
                        let title = escape_cowstr_for_html(title);
                        img.push_str(" title=\"");
                        img.push_str(&title);
                        img.push_str("\"");
                    }
                    img.push('>');
                    self.target.push_str(&img);
                }
                self.associated_resources.push(dest.to_string().into());
            }
            Table(_) | TableHead | TableRow | TableCell => {}
            Sans => self.target.push_str("<span class=\"sans\">"),
            SmallCaps => self.target.push_str("<span class=\"caps-to-small-caps\">"),
            Centred => {
                if self.in_para {
                    if self.target.ends_with("<p>") {
                        self.target.drain(self.target.len() - 3..);
                    } else if self.target.ends_with("<p class=\"noindent\">") {
                        self.target.drain(self.target.len() - 20..);
                    } else {
                        self.target.push_str("</p>\n");
                    }
                    self.target.push_str("<p class=\"noindent align-center\">");
                } else {
                    self.target.push_str("<br/><span class=\"float-center\">")
                }
            }
            RightAligned => {
                self.target
                    .push_str("<span class=\"align-right float-right\">");
            }
            FlattenedFootnote => unreachable!(),
            Superscript => self.target.push_str("<sup>"),
            Subscript => self.target.push_str("<sub>"),
        }
    }

    fn push_end_tag(&mut self, tag: Tag) {
        use Tag::*;
        match tag {
            UnindentedParagraph | Paragraph => {
                self.in_para = false;
                if !self.target.ends_with("</figure>") {
                    self.target.push_str("</p>\n");
                }
            }
            Heading(l) => {
                self.in_heading = false;
                match l {
                    1 => self.target.push_str("</h2>\n"),
                    2 => self.target.push_str("</h3>\n"),
                    3 => self.target.push_str("</h4>\n"),
                    4 => self.target.push_str("</h5>\n"),
                    _ => self.target.push_str("</h6>\n"),
                }
            }
            BlockQuote | BlockQuotation => {
                self.target.push_str("</blockquote>\n");
                self.do_not_indent_next_para = true;
            }
            CodeBlock(_) => self.target.push_str("</code></pre>\n"),
            List(Some(_)) => self.target.push_str("</ol>\n"),
            List(None) => self.target.push_str("</ul>\n"),
            Item => self.target.push_str("</li>\n"),
            FootnoteDefinition(_) => self.target.push_str("</p>"),
            Emphasis => self.target.push_str("</em>"),
            Strong => self.target.push_str("</strong>"),
            Strikethrough => self.target.push_str("</del>"),
            Link(_, _, _) => self.target.push_str("</a>"),
            Image(_, _, _) => {
                // since we're not in an inline image, this must be a figure
                if self.target.ends_with("  <figcaption>") {
                    self.target.drain(self.target.len() - 40..);
                    self.target.push_str("</figure>");
                } else {
                    self.target.push_str("</figcaption>\n");
                    self.target.push_str("</figure>");
                }
            }
            Sans | SmallCaps | RightAligned => self.target.push_str("</span>"),
            Centred => {
                if !self.in_para {
                    self.target.push_str("</span>");
                }
            }
            Superscript => self.target.push_str("</sup>"),
            Subscript => self.target.push_str("</sub>"),
            Table(_) | TableHead | TableRow | TableCell => {}
            FlattenedFootnote => unreachable!(),
        }
    }

    fn push(&mut self, item: Event<'_>) {
        if self.in_inline_image {
            self.push_in_inline_image(item)
        } else {
            match item {
                Event::Start(t) => self.push_start_tag(t),
                Event::End(t) => self.push_end_tag(t),
                Event::Text(text) => {
                    if self.in_heading {
                        self.target
                            .push_str(&bookbinder_common::escape_to_html(text.to_uppercase()));
                    } else {
                        self.target.push_str(&escape_cowstr_for_html(text));
                    }
                }
                Event::Code(text) => {
                    self.target.push_str("<code>");
                    self.target.push_str(&escape_cowstr_for_html(text));
                    self.target.push_str("</code>");
                }
                Event::Html(html) => self.target.push_str(&html),
                Event::SoftBreak => self.target.push('\n'),
                Event::HardBreak => self.target.push_str("<br/> \n"),
                Event::Rule => {
                    if !self.target.ends_with('\n') {
                        self.target.push('\n');
                    }
                    self.target.push_str("<hr/>\n");
                    self.do_not_indent_next_para = true;
                }
                Event::FootnoteReference(name) => {
                    let len = self.numbers.len() + 1;
                    self.target.push_str(&format!(
                        "<a href=\"#{name}\" id=\"fn_ref_{name}\" epub:type=\"noteref\">",
                        name = escape_cowstr_for_html(name.clone())
                    ));
                    let number = *self.numbers.entry(name.to_string()).or_insert(len);
                    self.target.push_str(&format!("<sup>{}</sup></a>", number));
                }
                Event::TaskListMarker(_) => {}
            }
        }
    }
}

trait EpubPageGetter {
    fn get_pages(&mut self, options: &mut Options) -> Result<Vec<EpubPage>, RenderingError>;
}

impl EpubPageGetter for BookSrc<'_> {
    /// transform the events stream into an intermediate format
    /// representing an epub page
    fn get_pages(&mut self, options: &mut Options) -> Result<Vec<EpubPage>, RenderingError> {
        self.change_headers(options.header_options);
        options.modify_events(&mut self.contents);

        let (has_parts, events) = preprocess(std::mem::take(&mut self.contents))?;
        let mut events = events.into_iter();
        let mut current_page: Option<XhtmlWriter> = None;
        let mut pages = Vec::new();

        let css = options.get_css()?;

        #[allow(clippy::while_let_on_iterator)]
        while let Some(event) = events.next() {
            match event {
                BookEvent::Event(e) => {
                    if let Some(ref mut current_page) = current_page {
                        current_page.push(e);
                    } else {
                        let mut cp = XhtmlWriter::new(SemanticRole::Chapter);
                        cp.css_path = Some(css.clone());
                        cp.push(e);
                        current_page = Some(cp);
                    }
                }
                BookEvent::Null => {}
                BookEvent::BeginTitlePage => {
                    let titlepage = events.collate_titlepage();
                    let (titlepage_filename, titlepage_filepath) =
                        if let Some(p) = options.get_titlepage() {
                            (
                                p.file_name()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap(),
                                p,
                            )
                        } else {
                            let titlepage_source = TitlePageSource::new(
                                titlepage,
                                options.publisher_imprint_logo.as_deref(),
                                options.titlepage_typeface.as_deref(),
                            );
                            let svg = generate_svg_titlepage(titlepage_source)
                                .map_err(|_| RenderingError::TitlepageGeneration)?;
                            let svg_path = svg.temp_file_path(Some("bookbinder"), "svg");
                            let svg_name = svg.temp_filename("svg");
                            std::fs::write(&svg_path, svg)
                                .map_err(|_| RenderingError::TitlepageGeneration)?;
                            (svg_name, svg_path)
                        };

                    let mut writer = XhtmlWriter::new(SemanticRole::Titlepage);
                    let image = format!("<img alt=\"The titlepage\" src=\"{}\" style=\"display: block; width: 100%; margin: auto; page-break-after: always;\"/>", titlepage_filename);
                    writer.push(Event::Html(image.into()));
                    writer.associated_resources.push(titlepage_filepath.clone());
                    options.titlepage = Some(titlepage_filepath);
                    pages.push(writer.finish());
                }
                BookEvent::BeginDivisionHeader(is_starred) => {
                    let div_header = events.collate_division_header::<EpubMarker>(is_starred);
                    let cp = if let Some(ref mut cp) = current_page {
                        cp
                    } else {
                        let mut cp = XhtmlWriter::new(SemanticRole::Chapter);
                        cp.css_path = Some(css.clone());
                        current_page = Some(cp);
                        current_page.as_mut().unwrap()
                    };
                    cp.write_division_header(div_header);
                }
                BookEvent::BeginSemantic(SemanticRole::Epigraph) => {
                    if let Some(cp) = current_page.take() {
                        pages.push(cp.finish());
                    }
                    let mut writer = XhtmlWriter::new(SemanticRole::Epigraph);
                    writer.css_path = Some(css.clone());

                    let epigraph = events.collate_epigraph();
                    writer.write_epigraph(epigraph);
                    pages.push(writer.finish());
                }
                BookEvent::BeginSemantic(role) => {
                    let mut writer = XhtmlWriter::new(role);
                    writer.css_path = Some(css.clone());
                    current_page = Some(writer);
                }
                BookEvent::EndSemantic(_) => {
                    if let Some(cp) = current_page {
                        let page = cp.finish();
                        pages.push(page);
                        current_page = None;
                    }
                }
                BookEvent::BeginMainmatter => {
                    if !has_parts {
                        pages.push(EpubPage::new_empty("Mainmatter"));
                    }
                }
                BookEvent::BeginFrontmatter => {
                    pages.push(EpubPage::new_empty("Frontmatter"));
                }
                BookEvent::BeginBackmatter => {
                    pages.push(EpubPage::new_empty("Backmatter"));
                }
                _ => {}
            }
        }

        if let Some(cp) = current_page {
            let page = cp.finish();
            pages.push(page);
        }
        Ok(pages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_footnotes() {
        let events = vec![
            Event::Start(Tag::Paragraph),
            Event::Text("Text".into()),
            Event::FootnoteReference("fn".into()),
            Event::End(Tag::Paragraph),
            Event::Start(Tag::FootnoteDefinition("fn".into())),
            Event::Start(Tag::Paragraph),
            Event::Text("Footnote text".into()),
            Event::End(Tag::Paragraph),
            Event::End(Tag::FootnoteDefinition("fn".into())),
        ];
        let mut writer = XhtmlWriter::new(SemanticRole::Chapter);
        for event in events.into_iter() {
            writer.push(event);
        }
        assert_eq!("\n<p>Text<a href=\"#fn\" id=\"fn_ref_fn\" epub:type=\"noteref\"><sup>1</sup></a></p>\n\n<h6 class=\"notes_heading\">Notes</h6>\n\n<p id=\"fn\" epub:type=\"footnote\" class=\"footnote\"><a href=\"#fn_ref_fn\">1.</a> \n<p>Footnote text</p>\n</p>", writer.target);
    }
}
