#![deny(dead_code)]
#![deny(unreachable_patterns)]
#![deny(unused_extern_crates)]
#![deny(unused_imports)]
#![deny(unused_qualifications)]
#![deny(clippy::all)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![deny(unused_results)]
#![deny(variant_size_differences)]

//! A set of utilites used across crates.
//! Note that these call some external commands:
//! - `latexmk` (and by extension xelatex)
//! - `pdftocairo` (only if required to convert a pdf image -- will gracefully fallback if not present)
//! 
//! The following are not necessary for normal operation,
//! but are useful in development:
//! - `epubcheck`
//! - `pdftotext`
//!
//! If used in combination with `bookbinder`, the following packages are needed for LaTex calls:
//!
//! -`titlesec`
//! -`caption`
//! -`geometry`
//! -`ulem`
//! -`textcase`
//! -`xpatch`
//! -`amsmath`
//! -`amssymb`
//! -`bookmark`
//! -`booktabs`
//! -`etoolbox`
//! -`fancyhdr`
//! -`fancyvrb`
//! -`footnotehyper`
//! -`listings`
//! -`longtable`
//! -`unicode-math`
//! -`upquote`
//! -`xcolor`
//! -`xurl`
//! -`fontspec`
//! -`graphicx`
//! -`microtype`
//! -`hyperref`
//! -`fmtcount`
//! -`appendix`


use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::error::Error;
use std::process::{Command, Stdio};
mod num_conversions;
use std::io::Write;
mod isbn;
mod mimetypes;
pub use isbn::{validate_isbn, display_isbn};
use aho_corasick::{AhoCorasick};
pub use mimetypes::{MimeTypeHelper, MimeType, GuessMimeType};
mod svg;
pub use svg::{convert_svg_to_png, convert_svg_to_jpg, convert_svg_file_to_png, simplify_svg};
use temp_file_name::HashToString;
use lazy_static::lazy_static;
pub mod fonts;

lazy_static!{
	static ref HTML_FINDER: AhoCorasick = AhoCorasick::new(&HTML_TARGET_CHARS);
	static ref LATEX_FINDER: AhoCorasick = AhoCorasick::new(&LATEX_TARGET_CHARS);
}

static HTML_TARGET_CHARS: [&str; 4] = [
	"<",
	">",
	"&",
	"'"
];

static HTML_REPLACEMENTS: [&str; 4] = [
	"&lt;",
	"&gt;",
	"&amp;",
	"’"
];

/// escape `input` for html output
pub fn escape_to_html<'a, S: Into<Cow<'a, str>>>(input: S) -> Cow<'a, str> {
	let input = input.into();
	let input_bytes = input.as_bytes();
	if HTML_FINDER.is_match(input_bytes) {
		let mut wtr = Vec::with_capacity(input.len());
		HTML_FINDER.stream_replace_all(input_bytes, &mut wtr, &HTML_REPLACEMENTS)
			.expect("Aho-Corasick error");
		unsafe {
			Cow::Owned(String::from_utf8_unchecked(wtr))
		}
	} else {
		input
	}
}

static LATEX_TARGET_CHARS: [&str; 16] = [
	"…",
	"–",
	"—",
	"\u{a0}",
	"&",
	"%",
	"$",
	"#",
	"_",
	"{",
	"}",
	"[",
	"]",
	"~",
	"^",
	"\\",
];

static LATEX_REPLACEMENTS: [&str; 16] = [
	"\\ldots{}",
	"--",
	"---",
	"~",
	"\\&",
	r"\%",
	r"\$",
	r"\#",
	r"\_",
	r"\{",
	r"\}",
	r"{[}",
	r"{]}",
	r"\textasciitilde{}",
	r"\textasciicircum{}",
	r"\textbackslash{}"
];


/// escape `input` for latex output
pub fn escape_to_latex<'a, S: Into<Cow<'a, str>>>(input: S) -> Cow<'a, str> {
	let input = input.into();
	let input_bytes = input.as_bytes();
	if LATEX_FINDER.is_match(input_bytes) {
		let mut wtr = Vec::with_capacity(input.len());
		LATEX_FINDER.stream_replace_all(input_bytes, &mut wtr, &LATEX_REPLACEMENTS)
			.expect("Aho-Corasick error");
		unsafe {
			Cow::Owned(String::from_utf8_unchecked(wtr))
		}
	} else {
		input
	}
}


/// call lualatex on a particular str and return the pdf
pub fn call_latex(tex: &str) -> Result<Vec<u8>, std::io::Error> {
	_call_latex(tex, false)
}

/// call lualatex on a particular str and return the pdf,
/// displaying lualatex's output as it goes
pub fn call_latex_verbose(tex: &str) -> Result<Vec<u8>, std::io::Error> {
	_call_latex(tex, true)
}

/// call a latex engine on a particular str and return the pdf
fn _call_latex(tex: &str, verbose: bool) -> Result<Vec<u8>, std::io::Error> {
	let filename_base = tex.hash_to_string();
	let mut outdir = std::env::temp_dir();
	outdir = outdir.join("bookbinder");

	let tex_fn = format!("{}.tex", &filename_base);
	let texpath = outdir.join(tex_fn);
	let filename = format!("{}.pdf", &filename_base);
	let outpath = outdir.join(&filename);
	
	std::fs::write(&texpath, tex)?;

	let odir_arg = format!("-output-directory={}", &outdir.to_string_lossy());

	let mut ltx = if !verbose {
		Command::new("latexmk")
			.args(&[&odir_arg, "-xelatex", "-interaction=batchmode", "-halt-on-error", texpath.to_string_lossy().as_ref()])
			.spawn()?
	} else {
		Command::new("latexmk")
			.args(&[&odir_arg, "-xelatex", texpath.to_string_lossy().as_ref()])
			.spawn()?
	};

	let _ = ltx.wait()?;

	if !outpath.exists() {
		let mut log = texpath.clone();
		let _ = log.set_extension("log");
		let log = std::fs::read_to_string(log)
			.unwrap_or_else(|_| "Latex error without log generated; perhaps LaTeX is not installed?".to_string());
		let e = std::io::Error::new(std::io::ErrorKind::Other, log);
		return Err(e);
	}
	let o = std::fs::read(outpath)?;
	Ok(o)
}

/// Call `epubcheck` on a file to check that it is a valid epub
pub fn epubcheck(p: PathBuf) -> Result<(), String> {

	let epubcheck = Command::new("epubcheck")
		.arg(p.to_str().unwrap())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.output()
		.map_err(|_| "Error launching epubcheck -- is it installed?".to_string())?;

	if epubcheck.status.success() {
		Ok(())
	} else {
		let (stdout, stderr) = unsafe {
			let stdout = String::from_utf8_unchecked(epubcheck.stdout);
			let stderr = String::from_utf8_unchecked(epubcheck.stderr);
			(stdout, stderr)
		};
		let mut msg = String::new();
		msg.push_str(&stdout);
		msg.push_str(&stderr);
		Err(msg)
	}
}

/// Convert an image at path `filepath` to a jpeg;
/// generally common raster formats as well as svg and pdf are supported,
/// but note that eps files are not
pub fn convert_to_jpg<P: AsRef<Path>>(filepath: P) -> Result<Vec<u8>, Box<dyn Error>> {
	let p = filepath.as_ref();
	let ext = p.extension()
		.map(|o| o.to_str())
		.flatten();

	match ext {
		Some("pdf") => {
			let data = std::fs::read(p)?;
			let svg = convert_pdf_to_svg(&data, None)?;
			let jpg = convert_svg_to_jpg(&svg, None)?;
			Ok(jpg)
		},
		Some("svg") => {
			let svg = std::fs::read_to_string(p)?;
			let jpg = convert_svg_to_jpg(&svg, None)?;
			Ok(jpg)
		},
		_ => {
			let mut output = Vec::new();
			let dynamic_image = image::open(p)?;
			dynamic_image.write_to(&mut output, image::ImageOutputFormat::Jpeg(100))?;
			Ok(output)
		}
	}
}


/// convert a pdf file to an svg; requires that pdftocairo (part of poppler)
/// be installed.
/// Note that we can't link poppler without licensing difficulties, so there are no plans
/// to incorporate this as a dependency.
pub fn convert_pdf_to_svg(pdf: &[u8], dpi: Option<usize>) -> Result<String, Box<dyn Error>> {
	let dpi = dpi.unwrap_or(150).to_string();
	let mut cv = Command::new("pdftocairo")
		.args(&["-svg", "-origpagesizes", "-r", &dpi, "-", "-"])
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.spawn()?;

	let stdin = cv.stdin.as_mut().unwrap();
	stdin.write_all(&pdf)?;
	let o = cv.wait_with_output()?;
	let mut svg = String::from_utf8(o.stdout)?;

	// remove hardcoded widths
	svg = svg.replacen(r#"width="432pt""#, r#"width="100%""#, 1);
	svg = svg.replacen(r#"height="648pt""#, r#"height="100%" preserveAspectRatio="xMidYMid meet" x="0px" y="0px""#, 1);

	Ok(svg)
}



/// get the current year as a string
pub fn get_current_year() -> String {
	let now = time::now_utc();
	time::strftime("%Y", &now)
		.unwrap()
}

/// given a number, return the corresponding letter
/// e.g. 0 -> A, 1 -> B, 2 -> C.
/// Returns an error if the number is greater than 25
/// ```
/// # use bookbinder_common::number_to_letter;
/// let number = 1;
/// assert_eq!(number_to_letter(number), Ok('B'));
/// ```
pub const fn number_to_letter(n: u8) -> Result<char, ()> {
	if n > 25 {
		Err(())
	} else {
		let codepoint = 65 + n;
		let letter = codepoint as char;
		Ok(letter)
	}
}

/// given a number, return it in roman format
/// e.g. 1 -> I, 10 -> X, etc
/// ```
/// # use bookbinder_common::number_to_roman;
/// let number = 1;
/// assert_eq!(number_to_roman(number), "I");
/// ```
pub const fn number_to_roman(n: u8) -> &'static str {
	num_conversions::number_to_roman(n)
}

/// given a number, return its equivalent in words
/// ```
/// # use bookbinder_common::number_to_words;
/// let number = 1;
/// assert_eq!(number_to_words(number), "ONE");
/// ```
pub const fn number_to_words(n: u8) -> &'static str {
	num_conversions::number_to_words(n)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_numbers_to_letter() {
		assert_eq!(number_to_letter(0), Ok('A'));
		assert_eq!(number_to_letter(25), Ok('Z'));
		assert_eq!(number_to_letter(27), Err(()));
	}

	#[test]
	fn test_get_current_year() {
		assert_eq!(get_current_year(), "2020".to_string());
	}

	#[test]
	fn test_hash_to_string() {
		let s = "Hello world".hash_to_string();
		assert_eq!(s, "2216321107127430384");
	}


	#[test]
	fn test_latex_escapes() {
		let escapes = [
			("&", "\\&"),
			("%", "\\%"),
			("$", "\\$"),
			("#", "\\#"),
			("_", "\\_"),
			("{Hello}", "\\{Hello\\}"),
			("[Hello]", "{[}Hello{]}"),
			("~", "\\textasciitilde{}"),
			("^", "\\textasciicircum{}"),
			("\\", "\\textbackslash{}"),
			//("'quoted'", "\\textquotesingle{}quoted\\textquotesingle{}"),
			//("\"doublequoted\"", "\\textquoteddbl{}doublequoted\\textquoteddbl{}"),
			//("`", "\\textasciigrave{}"),
			//("<>", "\\textless{}\\textgreater{}"),
			//("|", "\\textbar{}")
		];
		for (input, expected) in escapes.iter() {
			let s = input.to_string();
			let out = escape_to_latex(&s);
			assert_eq!(out.to_string(), *expected);
		}
	}

	#[test]
	fn test_numbers_to_word() {
		assert_eq!(number_to_words(0), "ZERO");
		assert_eq!(number_to_words(5), "FIVE");
		assert_eq!(number_to_words(12), "TWELVE");
		assert_eq!(number_to_words(25), "TWENTY-FIVE");
		assert_eq!(number_to_words(125), "ONE HUNDRED AND TWENTY-FIVE");
	}

	#[test]
	fn test_numbers_to_roman() {
		assert_eq!(number_to_roman(0), "");
		assert_eq!(number_to_roman(1), "I");
	}

}


