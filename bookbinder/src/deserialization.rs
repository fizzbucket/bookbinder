use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::convert::TryFrom;
use std::borrow::Cow;
use std::error::Error;
use bookbinder_latex::PaperSize;
use bookbinder_ast::BookEvent;
use crate::{EpubOptions, LatexOptions, BookSrc, BookSrcBuilder, create_epub, create_pdf};


/// Create a pdf from a json representation of a DeserializableBook.
pub fn create_pdf_from_json(src: &str) -> Result<Vec<u8>, Box<dyn Error>> {
	create_from_json(src, OutputFormat::Pdf)
}

/// Create an epub from a json representation of a DeserializableBook.
pub fn create_epub_from_json(src: &str) -> Result<Vec<u8>, Box<dyn Error>> {
	create_from_json(src, OutputFormat::Epub)
}

pub fn create_ast_from_json<'a>(src: &'a str) -> Result<Vec<BookEvent<'a>>, Box<dyn Error>> {
	let book = DeserializableBook::new(src)?;
	book.into_ast()
}

enum OutputFormat {
	Epub,
	Pdf
}

fn create_from_json(src: &str, fmt: OutputFormat) -> Result<Vec<u8>, Box<dyn Error>> {
	let book = DeserializableBook::new(src)?;
	match fmt {
		OutputFormat::Epub => book.into_epub(),
		OutputFormat::Pdf => book.into_pdf()
	}
}

/// Either a path to a markdown file, or a markdown string in its own right.
///
/// This is deserialized from a json str; if the str can be interpreted as a reference to
/// an existing file, it will be deserialized as a `Path`; if no such file exists,
/// it is taken to be a `Str`.
#[derive(Debug, Deserialize)]
#[serde(from="Cow<str>")]
pub enum PathOrString<'a> {
	Path(Cow<'a, Path>),
	Str(Cow<'a, str>)
}

impl <'a> From<Cow<'a, str>> for PathOrString<'a> {
	fn from(src: Cow<'a, str>) -> Self {
		let s: &str = &src;
		let as_path = Path::new(s);
		if as_path.is_file() {
			let p = match src {
				Cow::Borrowed(b) => Cow::Borrowed(Path::new(b)),
				Cow::Owned(p) => Cow::Owned(PathBuf::from(p))
			};
			PathOrString::Path(p)
		} else {
			PathOrString::Str(src)
		}
	}
}


/// Struct for deserializing authored ancillary texts like an introduction
#[derive(Deserialize, Debug)]
pub struct AuthoredAncillaryText<'a> {
	/// Either the text of this item, or a path to a markdown file containing the text
	pub text: PathOrString<'a>,
	/// The title of this item. If it is not set, but the markdown begins with a title,
	/// that will be used instead. If that is also missing, a default title, like `Introduction` will be used instead.
	#[serde(default)]
	pub title: Option<Cow<'a, str>>,
	/// names of the authors of this item
	#[serde(default)]
	pub authors: Vec<Cow<'a, str>>
}

/// Struct for deserializing ancillary texts which are by the author or unauthored, such as acknowledgments.
#[derive(Deserialize, Debug)]
pub struct UnauthoredAncillaryText<'a> {
	/// Either the text of this item, or a path to a markdown file containing the text
	pub text: PathOrString<'a>,
	/// A title to use for this item
	#[serde(default)]
	pub title: Option<Cow<'a, str>>
}

/// Deserializable representation of an epigraph
#[derive(Deserialize, Debug)]
pub struct Epigraph<'a> {
	/// The markdown text of the epigraph
	#[serde(borrow)]
	pub text: Cow<'a, str>,
	/// The author of the epigraph
	#[serde(default)]
	pub author: Option<Cow<'a, str>>,
	/// The title of the work from which the epigraph is taken
	#[serde(default)]
	pub title: Option<Cow<'a, str>>,
	/// If this flag is set, the title (if any) will be placed in single quote marks;
	/// otherwise it will be italicised
	#[serde(default)]
	pub quote_title: bool
}

/// A simplified representation of a book source for easy deserializing.
///
/// The only required values are `title` and `mainmatter`.
#[derive(Deserialize, Debug)]
pub struct DeserializableBookSrc<'a> {
	/// The title of the book
	#[serde(borrow)]
	pub title: Cow<'a, str>,
	/// Optional custom halftitle
	#[serde(default)]
	pub halftitle: Option<Cow<'a, str>>,
	/// Whether to not generate a halftitle if it has not been set explicitly
	#[serde(default)]
	pub do_not_generate_halftitle: bool,
	#[serde(default)]
	pub do_not_generate_titlepage: bool,
	#[serde(default)]
	pub do_not_generate_copyrightpage: bool,
	#[serde(default)]
	pub authors: Option<Vec<Cow<'a, str>>>,
	#[serde(default)]
	pub translators: Option<Vec<Cow<'a, str>>>,
	#[serde(default)]
	pub editors: Option<Vec<Cow<'a, str>>>,
	#[serde(default)]
	pub cover_designer: Option<Cow<'a, str>>,
	#[serde(default)]
	pub author_photo_copyright_holder: Option<Cow<'a, str>>,
	#[serde(default)]
	pub shorttitle: Option<Cow<'a, str>>,
	#[serde(default)]
	pub subtitle: Option<Cow<'a, str>>,
	#[serde(default)]
	pub paperback_isbn: Option<Cow<'a, str>>,
	#[serde(default)]
	pub epub_isbn: Option<Cow<'a, str>>,
	#[serde(default)]
	pub hardback_isbn: Option<Cow<'a, str>>,
	#[serde(default)]
	pub publisher: Option<Cow<'a, str>>,
	#[serde(default)]
	pub publisher_address: Option<Cow<'a, str>>,
	#[serde(default)]
	pub publisher_url: Option<Cow<'a, str>>,
	#[serde(default)]
	pub print_location: Option<Cow<'a, str>>,
	#[serde(default)]
	pub copyright_statement: Option<Cow<'a, str>>,
	#[serde(default)]
	pub do_not_assert_moral_rights: bool,
	#[serde(default)]
	pub is_not_first_publication: bool,
	#[serde(default)]
	pub dedication: Option<Cow<'a, str>>,
	#[serde(default)]
	pub colophon: Option<Cow<'a, str>>,
	#[serde(default)]
	pub forewords: Vec<AuthoredAncillaryText<'a>>,
	#[serde(default)]
	pub afterwords: Vec<AuthoredAncillaryText<'a>>,
	#[serde(default)]
	pub introductions: Vec<AuthoredAncillaryText<'a>>,
	#[serde(default)]
	pub preface: Option<UnauthoredAncillaryText<'a>>,
	#[serde(default)]
	pub acknowledgements: Option<UnauthoredAncillaryText<'a>>,
	#[serde(default)]
	pub appendices: Vec<UnauthoredAncillaryText<'a>>,
	pub mainmatter: Vec<PathOrString<'a>>,
	#[serde(default, borrow)]
	pub epigraphs: Vec<Epigraph<'a>>
}

impl <'a> TryFrom<DeserializableBookSrc<'a>> for BookSrc<'a> {
	type Error = std::io::Error;

	fn try_from(src: DeserializableBookSrc<'a>) -> Result<Self, Self::Error> {
		let mut builder = BookSrcBuilder::new(src.title);
		
		macro_rules! ifsomethen {
			($srcfield:ident, $targetfunc:ident) => {
				if let Some(x) = src.$srcfield {
					builder.$targetfunc(x);
				}
			};
		}

		ifsomethen!(halftitle, set_halftitle);
		ifsomethen!(authors, author);
		ifsomethen!(translators, translator);
		ifsomethen!(editors, editor);
		ifsomethen!(cover_designer, cover_designer);
		ifsomethen!(author_photo_copyright_holder, author_photo_copyright_holder);
		ifsomethen!(shorttitle, shorttitle);
		ifsomethen!(paperback_isbn, paperback_isbn);
		ifsomethen!(epub_isbn, epub_isbn);
		ifsomethen!(hardback_isbn, hardback_isbn);
		ifsomethen!(publisher, publisher);
		ifsomethen!(publisher_address, publisher_address);
		ifsomethen!(publisher_url, publisher_url);
		ifsomethen!(subtitle, subtitle);
		ifsomethen!(copyright_statement, copyright_statement);
		ifsomethen!(print_location, print_location);
		ifsomethen!(dedication, set_dedication);
		ifsomethen!(colophon, set_colophon);


		macro_rules! add_authored_ancillary {
			($srcfield:ident, $targetfilefunc:ident, $targetstrfunc:ident) => {
				for x in src.$srcfield.into_iter() {
					let title = x.title;
					let authors = x.authors;
					match x.text {
						PathOrString::Path(p) => {
							builder.$targetfilefunc(p, authors)?;
						},
						PathOrString::Str(s) => {
							builder.$targetstrfunc(s, title, authors);
						}
					}
				}
			};
		}

		add_authored_ancillary!(forewords, add_foreword_from_file, add_foreword);
		add_authored_ancillary!(afterwords, add_afterword_from_file, add_afterword);
		add_authored_ancillary!(introductions, add_introduction_from_file, add_introduction);

		macro_rules! add_unauthored_ancillary {
			($srcfield:ident, $targetfilefunc:ident, $targetstrfunc:ident) => {
				if let Some(x) = src.$srcfield {
					let title = x.title;
					match x.text {
						PathOrString::Path(p) => {
							builder.$targetfilefunc(p, title)?;
						},
						PathOrString::Str(s) => {
							builder.$targetstrfunc(s, title);
						}
					}
				}
			};
		}

		add_unauthored_ancillary!(preface, add_preface_from_file, add_preface);
		add_unauthored_ancillary!(acknowledgements, add_acknowledgements_from_file, add_acknowledgements);


		for appendix in src.appendices.into_iter() {
			let title = appendix.title;
			match appendix.text {
				PathOrString::Path(p) => {
					builder.add_appendix_from_file(p, title)?;
				},
				PathOrString::Str(s) => {
					builder.add_appendix(s, title);
				}
			}
		}

		for item in src.mainmatter.into_iter() {
			match item {
				PathOrString::Path(p) => {
					builder.add_mainmatter_from_file(p)?;
				},
				PathOrString::Str(s) => {
					builder.add_mainmatter(s);
				}
			}
		}

		for epigraph in src.epigraphs.into_iter() {
			builder.add_explicit_epigraph(epigraph.text, epigraph.author.as_deref(), epigraph.title.as_deref(), epigraph.quote_title);
		}

		if src.do_not_generate_halftitle {
			builder.do_not_generate_halftitle();
		}

		if src.do_not_generate_titlepage {
			builder.do_not_generate_titlepage();
		}

		if src.do_not_generate_copyrightpage {
			builder.do_not_generate_copyrightpage();
		}

		if src.do_not_assert_moral_rights {
			builder.do_not_assert_moral_rights();
		}

		if src.is_not_first_publication {
			builder.is_not_first_publication();
		}

		let src = builder.process();
		Ok(src)
	}
}

/// A representation of options which combines options across output formats;
/// those which do not apply will be ignored.
///
/// Note that some flags are mutually contradictory.
/// Check the source for the specific handling of such cases,
/// but in general the smallest or the broadest option is chosen.
#[derive(Debug, Deserialize, Default)]
pub struct UnifiedOptions {
	/// path to custom css for epub
	pub css: Option<PathBuf>,
	/// path to an epub cover image
	pub cover_image: Option<PathBuf>,
	/// path to custom image for an epub titlepage
	pub titlepage: Option<PathBuf>,
	/// path to publisher logo for use in generated titlepages
	pub publisher_imprint_logo: Option<PathBuf>,
	/// name of the typeface to use in generated titlepages

	pub titlepage_typeface: Option<String>,
	/// flag to use words for chapter numbers
	#[serde(default)]
	pub use_words_for_chapter_labels: bool,
	/// flag to use roman numerals for chapter numbers
	#[serde(default)]
	pub use_roman_numerals_for_chapter_labels: bool,
	/// flag to suppress chapter labels
	#[serde(default)]
	pub suppress_chapter_labels: bool,
	/// flag to suppress chapter titles
	#[serde(default)]
	pub suppress_chapter_titles: bool,
	/// flag to use chapter numbers only, rather than labels and titles
	#[serde(default)]
	pub only_number_chapters: bool,
	/// linespread value for latex
	pub linespread: Option<f32>,
	/// whether to set openany for latex
	#[serde(default)]
	pub open_any: bool,
	/// latex secnumdepth
	pub secnumdepth: Option<i32>,
	/// name of sans typeface to use in pdf output
	pub sans_typeface: Option<String>,
	/// name of serif typeface to use in pdf output
	pub serif_typeface: Option<String>,
	/// name of monospace typeface to use in pdf output
	pub mono_typeface: Option<String>,
	/// name of typeface to use for running headers and footers in pdf output
	pub headers_and_footers_typeface: Option<String>,
	/// name of typeface to use in pdf headings
	pub heading_typeface: Option<String>,
	/// flag to include a table of contents in pdf output
	#[serde(default)]
	pub include_toc: bool,
	/// flag to suppress pdf footers
	#[serde(default)]
	pub suppress_footers: bool,
	/// flag to include only page numbers in pdf footers
	#[serde(default)]
	pub page_number_only_in_footers: bool,
	/// flag to include figure labels in pdf output
	#[serde(default)]
	pub do_not_suppress_figure_labels: bool,
	/// flag to set pdf font size to 10pt
	#[serde(default)]
	pub ten_pt: bool,
	/// flag to set pdf font size to 11pt
	#[serde(default)]
	pub eleven_pt: bool,
	/// flag to set pdf font size to 12pt
	#[serde(default)]
	pub twelve_pt: bool,
	/// flag to set pdf papersize to 5x8 inches
	#[serde(default)]
	pub five_by_eight_inches: bool,
	/// flag to set pdf papersize to 5'25"x8"
	#[serde(default)]
	pub five_twenty_five_by_eight_inches: bool,
	/// flag to set pdf papersize to 5"x8'5"
	#[serde(default)]
	pub five_five_by_eight_five_inches: bool,
	/// flag to set pdf papersize to 6"x9"
	#[serde(default)]
	pub six_by_nine_inches: bool,
	/// flag to set pdf papersize to A4
	#[serde(default)]
	pub a4paper: bool,
	/// flag to set pdf papersize to US Letter
	#[serde(default)]
	pub usletter: bool,
	/// flag to set pdf papersize to US Legal
	#[serde(default)]
	pub uslegal: bool,
	/// custom label for chapters -- e.g. `Letter 1` instead of `Chapter 1`
	#[serde(default)]
	pub chapter_label: Option<String>
}

impl From<UnifiedOptions> for EpubOptions {
	fn from(src: UnifiedOptions) -> EpubOptions {
		let mut options = EpubOptions::default();
		if let Some(css) = src.css {
			options.css(css);
		}

		if let Some(chapter_label) = src.chapter_label {
			options.set_chapter_label(chapter_label);
		}


		if let Some(cover_image) = src.cover_image {
			options.cover_image(cover_image);
		}
		if let Some(titlepage) = src.titlepage {
			options.titlepage(titlepage);
		}
		if let Some(publisher_imprint_logo) = src.publisher_imprint_logo {
			options.publisher_imprint_logo(publisher_imprint_logo);
		}
		if let Some(titlepage_typeface) = src.titlepage_typeface {
			options.titlepage_typeface(titlepage_typeface);
		} else if let Some(sans_typeface) = src.sans_typeface {
			options.titlepage_typeface(sans_typeface);
		}

		if src.suppress_chapter_titles {
			options.suppress_chapter_titles();
		}
		if src.use_words_for_chapter_labels {
			options.use_words_for_chapter_labels();
		}
		if src.use_roman_numerals_for_chapter_labels {
			options.use_roman_numerals_for_chapter_labels();
		}
		if src.suppress_chapter_labels {
			options.suppress_chapter_label();
		}
		if src.only_number_chapters {
			options.only_number_chapters();
		}
		options
	}
}

#[allow(unused_must_use)]
impl From<UnifiedOptions> for LatexOptions {
	fn from(src: UnifiedOptions) -> LatexOptions {
		let mut options = LatexOptions::default();
		
		if src.open_any {
			options.open_any();
		}

		if let Some(chapter_label) = src.chapter_label {
			options.chapter_label(chapter_label);
		}

		if let Some(publisher_imprint_logo) = src.publisher_imprint_logo {
			options.set_publisher_logo(publisher_imprint_logo);
		}
		if let Some(titlepage_typeface) = src.titlepage_typeface {
			options.set_titlepage_typeface(titlepage_typeface);
		}
		if src.suppress_chapter_titles {
			options.suppress_chapter_titles();
		}
		if src.use_words_for_chapter_labels {
			options.use_words_for_chapter_labels();
		}
		if src.use_roman_numerals_for_chapter_labels {
			options.use_roman_numerals_for_chapter_labels();
		}
		if src.suppress_chapter_labels {
			options.suppress_chapter_label();
		}
		if src.only_number_chapters {
			options.only_number_chapters();
		}

		if let Some(l) = src.linespread {
			options.set_linespread(l);
		}
		if let Some(n) = src.secnumdepth {
			options.set_secnumdepth(n.into());
		}

		if let Some(sans_typeface) = src.sans_typeface {
			options.set_sans_typeface(sans_typeface);
		}

		if let Some(serif_typeface) = src.serif_typeface {
			options.set_serif_typeface(serif_typeface);
		}

		if let Some(mono_typeface) = src.mono_typeface {
			options.set_mono_typeface(mono_typeface);
		}

		if let Some(headers_and_footers_typeface) = src.headers_and_footers_typeface {
			options.set_headers_and_footers_typeface(headers_and_footers_typeface);
		}

		if let Some(heading_typeface) = src.heading_typeface {
			options.set_heading_typeface(heading_typeface);
		}

		if src.include_toc {
			options.include_toc();
		}

		if src.suppress_footers {
			options.suppress_footers();
		} else if src.page_number_only_in_footers {
			options.page_number_only_in_footers();
		}

		if src.do_not_suppress_figure_labels {
			options.do_not_suppress_figure_labels();
		}

		if src.ten_pt {
			options.ten_pt();
		} else if src.eleven_pt {
			options.eleven_pt();
		} else if src.twelve_pt {
			options.twelve_pt();
		}

		if src.five_by_eight_inches {
			options.set_papersize(PaperSize::Inches5x8);
		} else if src.five_twenty_five_by_eight_inches {
			options.set_papersize(PaperSize::Inches5_25x8);
		} else if src.five_five_by_eight_five_inches {
			options.set_papersize(PaperSize::Inches5_5x8_5);
		} else if src.six_by_nine_inches {
			options.set_papersize(PaperSize::Inches6x9);
		}else if src.a4paper {
			options.set_papersize(PaperSize::A4Paper);
		} else if src.usletter {
			options.set_papersize(PaperSize::USLetter);
		} else if src.uslegal {
			options.set_papersize(PaperSize::USLegal);
		}

		options
	}
}

/// Deserializable container for a book's source and options;
/// note that these are flattened when deserialized, so that keys for
/// both options and src are at the same level in the same json object.
/// 
/// ```
/// # use bookbinder::deserialization::DeserializableBook;
/// let json = r#"{"title": "Hello World", "mainmatter": ["Text goes here"], "suppress_footers": true}"#;
/// let book: DeserializableBook = serde_json::from_str(json).unwrap();
/// ```
#[derive(Debug, Deserialize)]
pub struct DeserializableBook<'a> {
	#[serde(borrow, flatten)]
	pub src: DeserializableBookSrc<'a>,
	#[serde(default, flatten)]
	pub options: UnifiedOptions
}

impl <'a> DeserializableBook<'a> {
	
	fn new(src: &'a str) -> Result<Self, serde_json::Error> {
		let book: DeserializableBook = serde_json::from_str(src)?;
		Ok(book)
	}

	fn into_ast(self) -> Result<Vec<BookEvent<'a>>, Box<dyn Error>> {
		let src = BookSrc::try_from(self.src)?;
		Ok(src.contents)
	}


	fn into_epub(self) -> Result<Vec<u8>, Box<dyn Error>> {
		let src = BookSrc::try_from(self.src)?;
		let epub = create_epub(src, self.options.into())?;
		Ok(epub)
	}

	fn into_pdf(self) -> Result<Vec<u8>, Box<dyn Error>> {
		let src = BookSrc::try_from(self.src)?;
		let pdf = create_pdf(src, self.options.into())?;
		Ok(pdf)
	}
}

