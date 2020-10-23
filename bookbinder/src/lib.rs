//! Create pdf or epub books from markdown.
//!
//! # Example output
//!
//! 1. [pdf](https://raw.githubusercontent.com/fizzbucket/bookbinder/main/bookbinder/tests/test.pdf)
//! 2. [epub](https://raw.githubusercontent.com/fizzbucket/bookbinder/main/bookbinder/tests/test.epub)
//!
//! # Installation
//! 
//! 
//! Pdf support currently requires `xelatex` and `latexmk` to be installed; most LaTeX installations will have these already.
//! If you want to include images in pdf format, the command `pdfcairo` should also be available. Otherwise, we have no dependencies!
//! 
//! There is support for a limited binary which takes a json specification of a book -- see `DeserializableBook` -- from stdin and writes it to stdout; 
//! the easiest way to get that is probably `cargo install bookbinder`. Then usage is as simple as `cat in.json | bookbinder > out.pdf`
//! But it's more likely you'll be using this as part of a script or library.
//! Add `bookbinder = "0.1.0"` to your Cargo.toml, and read on for details!
//!
//! # Basic Example
//!
//! ```
//! use bookbinder::{BookSrc, BookSrcBuilder, create_epub, create_pdf, EpubOptions, LatexOptions};
//!
//! let src = BookSrcBuilder::new("A Book")
//!     .author("A.N. Author")
//!     .add_mainmatter("# Greetings\n\n Hello world...")
//!     .process();
//! let epub_options = EpubOptions::default();
//! let epub = create_epub(src.clone(), epub_options)
//!     .expect("Error producing epub");
//!
//! let pdf_options = LatexOptions::default();
//! let pdf = create_pdf(src, pdf_options)
//!     .expect("Error producing pdf");
//!```
//!
//! # Why use this
//!
//! There are plenty of options for creating books from markdown;
//! in the Rust world, there's [mdbook](https://docs.rs/mdbook/) and [crowbook](https://docs.rs/crowbook/); there's the amazing
//! [Pandoc](https://pandoc.org) in Haskell.
//!
//! But these all tend to take Markdown constructs and turn them into books.
//! This is great while it works, but books are complicated things. So, to take one simple example,
//! a top-level markdown header (`# Header`) might represent a section, a chapter or a part
//! depending on how a book is structured. Then it might need to be rendered with a label (`Chapter 1: Header`) or as a number alone (`I` or `One` or `1`),
//! or as a title alone (`Header`)...
//!
//! If it looks more like `# Foreword` or `# Appendix`
//! you've got another set of problems: in a proper book, these should display differently -- for example,
//! page numbers in a foreword should probably be represented as roman numerals and any labels should refer
//! to an appendix as `Appendix A`, not `Chapter 23`.
//!
//! Similarly, plain text in a dedication shouldn't look like plain text in the body of a document...
//!
//! This crate relies on the insight that within a text divided into semantic roles, Markdown is an ideal solution -- you can say
//! 'emphasise this text inside an epigraph' and all is well, but you can't say -- within Markdown itself --
//! 'this text is an epigraph'. Since there aren't an infinite number of possible parts to books, and things like [the epub structural semantics vocabulary](https://idpf.github.io/epub-vocabs/structure/) already have a range
//! of defined possibilities, it's relatively easy to set up a container which renders Markdown within a
//! more complex semantic system.
//!
//!
//! And since some of these elements, like a titlepage or a copyright page, are pretty strictly the product
//! of metadata, we further extend things so that they can be -- if you want -- generated automatically from the metadata
//! which should already be included.
//!
//! In other words, you can do this:
//!
//! ```
//! # use bookbinder::BookSrcBuilder;
//! let introduction = "This is an introduction...";
//! let dedication = "This is dedicated to someone";
//! let foreword_with_custom_heading = "# A peculiar light\n\nForeword goes here...";
//! let mainmatter = "# Early Life\n\n## I am born\n\nThe day of my birth was a dark cold day...";
//!
//! let src = BookSrcBuilder::new("A Book") // Start with a title
//!     .author("A.N. Author")
//!     .publisher("Publisher Name")
//!     .add_introduction(introduction, None, "Introduction Author")
//!     .set_dedication(dedication)
//!     .add_foreword(foreword_with_custom_heading, None, vec!["First Foreword Author", "Second Foreword Author"])
//!     .add_mainmatter(mainmatter);
//! ```
//!
//! The example above would give you a halftitle,
//! a titlepage, a copyright page explaining that this is copyright this year by A.N. Author and published by Publisher Name;
//! then you'd get a sequence of frontmatter pages with niceties like roman page numbers,
//! including an introduction with the header `Introduction` and a foreword with a nice label
//! explaining it's a foreword but the header `A Peculiar Light`, as well as an unlabelled dedication which shows by its format
//! (typically italicised text in a minipage, but you could specify otherwise) that it's a dedication.
//! (Of course, if you wanted to avoid having things like a copyright page generated, or provide your own, that's easily done too!)
//!
//! The mainmatter following would treat `# Early Life` as a part header, and `# I am born` as a chapter header, but if they'd
//! both been top-level headers they would have both been treated as chapters (or if one was a second-level header and the other a third and there were no top-level headers, they would have been treated as chapters and sections.)
//!
//! And so on. It's pretty cool.
//!
//! But even better is that it's easy to change how these basic semantic constructs are represented -- if you
//! just want to ignore whatever chapters are titled and call them `One`, `Two` and `Three`, you can just set an
//! option to do that! If you want your pdf book to be all in A4 paper and set in Comic Sans, but with headers in Papyrus -- many things
//! are possible, but only some are well-advised.
//!
//! ```
//! # use bookbinder_latex::PreambleOptions as LatexOptions;
//! # use bookbinder_latex::PaperSize;
//! let options = LatexOptions::default()
//!     .use_words_for_chapter_labels() // this says 'Number chapters using words!'
//!     .only_number_chapters() // And actually, while you're about it, don't do anything *but* number chapters
//!     .set_serif_typeface("Comic Sans") // and set the main text in Comic Sans
//!     .set_heading_typeface("Papyrus") // and headers in Papyrus
//!     .set_papersize(PaperSize::A4Paper); // and an A4 book is a good size
//! ```
//!
//! A book can be produced just by combining the options which can be applied to a particular output format with a source created through `BookSrcBuilder`.
//!
//! `BookSrcBuilder` lets you add a bewildering range of metadata (this was translated by a particular person, and someone else did the notes and gave an introduction, but the author was someone else again) and book elements (here's that introduction, here's the main text, here's a note by the translator).
//!
//! Meanwhile, there are a range of options for changing how this source is rendered. The options for different formats differ because what we can do with different formats also differs
//! -- there's no point changing page numbers when epubs don't have page numbers, but there's not much sense
//! in setting the cover image of a pdf file which doesn't have covers!
//!
//! # Goals
//!
//! This crate is meant to give an easy-to-use way to create nicely-formatted books
//! which can handle quite granular complexity but shouldn't thrust that complexity
//! on users who don't need it.
//!
//! It would be great to hear feedback on ways to make the user experience simpler, or things which are missing!
//!
//! # Our Markdown
//!
//! It's a slight exaggeration to say that we use simple Markdown -- actually, there are a couple of things books need
//! which CommonMark doesn't have. So source strings or files are parsed using a markdown dialect, inspired by Pandoc, that
//!
//! - smartens straight quotes and replaces sequences of hyphens with appropriate dashes, and gives ellipses instead of `...`
//! - includes footnotes
//! - includes sub and superscript
//!
//! For full details, see [extended_pulldown](../extended_pulldown). Incidentally, an advantage of doing things this way is that it's very easy to
//! build a pipeline `arbitrary input format -> pandoc -> markdown -> bookbinder`, so that books can be built from things like Word documents.
//!
//! # Technical details
//!
//! Architecturally, this crate is a very thin wrapper over:
//!   1. `bookbinder_ast`, which sets out an abstract book source, and
//!   2. `bookbinder_epub` and `bookbinder_latex`, which define how to render that source into a particular output format and the various options for such a rendering.
//!
//! So for full details of how something works, you'd best look to the specific crate!
//! This seperated design is intended to allow different backends to be added in future --
//! it'd be interesting to add a way to create an html book, and potentially
//! an alternative way to produce pdfs could be valuable, since LaTeX is gorgeous but slow,
//! and it's a big thing for people to install. The most likely candidates are an embedded version of
//! either `neatroff` or `SILE`.
//!
//! # Deserialization
//!
//! This crate includes two functions `create_pdf_from_json` and `create_epub_from_json` which support
//! deserializing and rendering from input json rather than controlling the process through manually building objects.
//!
//! The json used there represents a simplified format which offers a little less control over the process, primarily
//! to make deserialization easier and more understandable.
//!
//! For full details, see the `deserialization` module.
//!
pub use bookbinder_ast::{BookSrc, BookSrcBuilder};
use bookbinder_epub::EpubRenderer;
pub use bookbinder_epub::Options as EpubOptions;
pub use bookbinder_epub::RenderingError as EpubRenderingError;
pub use bookbinder_latex::LatexSecNumDepth;
use bookbinder_latex::PdfRenderer;
pub use bookbinder_latex::PreambleOptions as LatexOptions;
pub mod deserialization;

/// Create an epub 3.2 from a `BookSrc` with the given options
pub fn create_epub(src: BookSrc<'_>, options: EpubOptions) -> Result<Vec<u8>, EpubRenderingError> {
    src.render_to_epub(options)
}

/// Create a pdf from a `BookSrc` with the given options
pub fn create_pdf(src: BookSrc<'_>, options: LatexOptions) -> Result<Vec<u8>, std::io::Error> {
    src.render_to_pdf_with_options(options)
}

/// Create an epub from a `BookSrc` with default options
pub fn create_epub_default(src: BookSrc<'_>) -> Result<Vec<u8>, EpubRenderingError> {
    src.render_to_epub_default()
}

/// Create a pdf from a `BookSrc` with default options
pub fn create_pdf_default(src: BookSrc<'_>) -> Result<Vec<u8>, std::io::Error> {
    src.render_to_pdf()
}
