use bookbinder_ast::{NumberFormat, TextHeaderOptions};
use crate::LatexSecNumDepth;
use std::path::{PathBuf, Path};
use std::borrow::Cow;
use crate::OptionsWithRenderedPreamble;
use temp_file_name::TempFilePath;
use bookbinder_common::MimeTypeHelper;
use bookbinder_common::fonts::{font_exists, FontInfo, SANS_FONT_PATHS, SERIF_FONT_PATHS};

const DEFAULT_LINESPREAD: f32 = 1.1;
const DEFAULT_PART_FORMAT: &str = r#"\titleformat{\part}[display]{\headingtypeface\Huge}{\itshape Part \thepart}{1em}{\thispagestyle{empty}}{}"#;
const DEFAULT_TITLESEC_OPTIONS: [&str; 4] = ["center", "sf", "small", "uppercase"];



#[derive(Debug, Clone, Default)]
pub struct RunningHeader {
	left_even: Option<Cow<'static, str>>,
	left_odd: Option<Cow<'static, str>>,
	centre_even: Option<Cow<'static, str>>,
	centre_odd: Option<Cow<'static, str>>,
	right_even: Option<Cow<'static, str>>,
	right_odd: Option<Cow<'static, str>>,
}

impl RunningHeader {
	fn to_preamble_commands(&self, is_footer: bool) -> String {
		let cmd = if is_footer {
			"\\fancyfoot"
		} else {
			"\\fancyhead"
		};

		let mut out = String::new();

		macro_rules! running {
			($field:ident, $code:expr) => {
				match self.$field {
					Some(ref t) => {
						out.push_str(cmd);
						out.push('[');
						out.push_str($code);
						out.push(']');
						out.push_str("{{");
						out.push_str(t);
						out.push_str("}}\n");
					},
					None => {
						out.push_str(cmd);
						out.push('[');
						out.push_str($code);
						out.push(']');
						out.push_str("{}\n");
					}
				}
			};
		}

		running!(left_even, "LE");
		running!(left_odd, "LO");
		running!(centre_even, "CE");
		running!(centre_odd, "CO");
		running!(right_even, "RE");
		running!(right_odd, "RO");

		out
	}
}

#[derive(Debug, Clone)]
pub struct LatexPageStyle {
	header: RunningHeader,
	footer: RunningHeader
}

impl LatexPageStyle {
	fn to_preamble_commands(&self) -> String {
		let mut header_commands = self.header.to_preamble_commands(false);
		let footer_commands = self.footer.to_preamble_commands(true);
		header_commands.push('\n');
		header_commands.push_str(&footer_commands);
		header_commands
	}
}

impl LatexPageStyle {

	pub(crate) fn make_empty(&mut self) {
		*self = Self::empty()
	}

	pub(crate) fn make_plain(&mut self) {
		*self = Self::default_plain()
	}
	
	pub(crate) fn empty() -> Self {
		let header = RunningHeader::default();
		let footer = RunningHeader::default();
		LatexPageStyle {
			header,
			footer
		}
	}

	pub(crate) fn default_plain() -> Self {
		let header = RunningHeader::default();
		let mut footer = RunningHeader::default();
		footer.left_even = Some("\\runningtypeface \\thepage".into());
		footer.right_odd = Some("\\runningtypeface \\thepage".into());
		LatexPageStyle {
			header,
			footer
		}
	}

	pub(crate) fn default_fancy() -> Self {
		let header = RunningHeader::default();
		let mut footer = RunningHeader::default();
		footer.left_even = Some(r"\runningtypeface \thepage\footerseperator\currentcontributor".into());
		footer.right_odd = Some(r"\runningtypeface \MakeUppercase{\pageidentifier}\footerseperator\thepage".into());
		LatexPageStyle {
			header,
			footer
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub enum MeasurementUnit {
	Inches,
	Mm
}

#[derive(Debug, Clone)]
pub struct LatexMargins {
	pub paper_width: f32,
	pub paper_height: f32,
	pub top: f32,
	pub bottom: f32,
	pub left: f32,
	pub right: f32,
	pub unit: MeasurementUnit
}

// AMAZON KDP SIZES for expanded distribution with black ink on cream paper are:
// 5" x 8" (12.7 x 20.32 cm)
// 5.25" x 8" (13.34 x 20.32 cm)
// 5.5" x 8.5" (13.97 x 21.59 cm)
// 6" x 9" (15.24 x 22.86 cm)

// also have A4 and letter paper size options:
//	A4: 210 × 297 	8 1⁄4 × 11 17⁄24
//  Letter: 8.5 x 11 (215.9 by 279.4 mm)
//  Legal : 8 1⁄2 × 14 	216 × 356

/// Possible paper sizes with preconfigured margins
#[derive(Debug, Clone, Copy)]
pub enum PaperSize {
	/// 5" by 8"
	Inches5x8,
	/// 5"25' by 8"
	Inches5_25x8,
	/// 5"5' by 8"5'
	Inches5_5x8_5,
	/// 6" by 9"
	Inches6x9,
	/// A4 Paper
	A4Paper,
	/// North American letter size
	USLetter,
	/// North American legal size
	USLegal
}

impl Default for PaperSize {
	fn default() -> Self {PaperSize::Inches6x9}
}

impl From<PaperSize> for LatexMargins {
	fn from(src: PaperSize) -> Self {
		match src {
			PaperSize::Inches5x8 => INCHES5X8_MARGINS.clone(),
			PaperSize::Inches5_25x8 => INCHES5_25X8_MARGINS.clone(),
			PaperSize::Inches5_5x8_5 => INCHES5_5X8_5_MARGINS.clone(),
			PaperSize::Inches6x9 => INCHES6X9_MARGINS.clone(),
			PaperSize::A4Paper => A4_PAPER_MARGINS.clone(),
			PaperSize::USLetter => USLETTER_MARGINS.clone(),
			PaperSize::USLegal => USLEGAL_MARGINS.clone(),
		}
	}
}


static INCHES5X8_MARGINS: LatexMargins = LatexMargins {
	unit: MeasurementUnit::Inches,
	paper_width: 5.0,
	paper_height: 8.0,
	top: 0.4,
	bottom: 0.8,
	left: 0.875,
	right: 0.75
};

static INCHES5_25X8_MARGINS: LatexMargins = LatexMargins {
	unit: MeasurementUnit::Inches,
	paper_width: 5.25,
	paper_height: 8.0,
	top: 0.4,
	bottom: 0.8,
	left: 0.875,
	right: 0.75
};

static INCHES5_5X8_5_MARGINS: LatexMargins = LatexMargins {
	unit: MeasurementUnit::Inches,
	paper_width: 5.5,
	paper_height: 8.5,
	top: 0.5,
	bottom: 0.9,
	left: 0.875,
	right: 0.75
};

static INCHES6X9_MARGINS: LatexMargins = LatexMargins {
	unit: MeasurementUnit::Inches,
	paper_width: 6.0,
	paper_height: 9.0,
	top: 0.5,
	bottom: 1.0,
	left: 0.875,
	right: 0.75
};

static A4_PAPER_MARGINS: LatexMargins = LatexMargins {
	unit: MeasurementUnit::Mm,
	paper_width: 210.0,
	paper_height: 297.0,
	top: 20.0,
	bottom: 30.0,
	left: 40.0,
	right: 30.0
};

static USLETTER_MARGINS: LatexMargins = LatexMargins {
	unit: MeasurementUnit::Inches,
	paper_width: 8.5,
	paper_height: 11.0,
	top: 1.0,
	bottom: 1.4,
	left: 1.4,
	right: 1.4
};

static USLEGAL_MARGINS: LatexMargins = LatexMargins {
	unit: MeasurementUnit::Inches,
	paper_width: 8.5,
	paper_height: 14.0,
	top: 1.0,
	bottom: 1.4,
	left: 1.4,
	right: 1.4
};

impl Default for LatexMargins {
	fn default() -> Self {
		LatexMargins::from(PaperSize::default())
	}
}

/// A representation of a font to be used in latex;
/// since fontspec can load fonts either using names
/// or paths, we support both options
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum LatexFont {
	/// use a named font -- e.g. 'Minion Pro'
	Named(Cow<'static, str>),
	/// load from files
	FromFile {
		/// the base filename of the font
		filename: Cow<'static, str>,
		/// the filename of the font's bold variant (`BoldFont`)
		bold: Cow<'static, str>,
		/// the filename of the font's italic variant (`ItalicFont`)
		italic: Cow<'static, str>,
		/// the path to the directory these fonts are in,
		/// if it isn't a standard systems font location
		path: Option<Cow<'static, Path>>,
		/// the filename of the font's bold italic variant (`BoldItalicFont`)
		bolditalic: Option<Cow<'static, str>>,
		/// the gilename of the font's smallcaps variant (`SmallCapsFont`)
		smallcaps: Option<Cow<'static, str>>
	}
}

impl <T> From<T> for LatexFont where T: Into<Cow<'static, str>> {
	fn from(src: T) -> Self {
		LatexFont::Named(src.into())
	}
}

impl LatexFont {

	pub(crate) fn default_serif() -> Self {
		Self::new_from_font_info(&SERIF_FONT_PATHS)
			.expect("Error producing default latex serif font")
	}

	pub(crate) fn default_sans() -> Self {
		Self::new_from_font_info(&SANS_FONT_PATHS)
			.expect("Error producing default latex sans font")
	}

	fn new_from_font_info(src: &FontInfo) -> Result<Self, std::io::Error> {
		let path = src.get_base_filepath()?.to_path_buf().into();
		let bold = src.get_bold()?.into();
		let italic = src.get_italic()?.into();
		let filename = src.get_filename()?.into();
		let bolditalic = src.get_bolditalic()?.into();

		Ok(LatexFont::FromFile {
			path: Some(path),
			bold,
			italic,
			filename,
			bolditalic: Some(bolditalic),
			smallcaps: None
		})
	}

	fn display(&self) -> String {
		match self {
			LatexFont::Named(n) => format!("{{{}}}", n),
			LatexFont::FromFile{filename, path, bold, italic, bolditalic, smallcaps} => {
				let bold = format!("\tBoldFont = {},\n", bold);
				let italic = format!("\tItalicFont = {},\n", italic);
				let bolditalic = bolditalic.as_ref()
					.map(|s| format!("\tBoldItalicFont = {},\n", s));
				let smallcaps = smallcaps.as_ref()
					.map(|s| format!("\tSmallCapsFont = {},\n", s));
				let path = path.as_ref()
					.map(|p| format!("\tPath = {}/ ,\n", p.display()));

				let mut output = format!("{{{}}}[\n", filename);
				output.push_str(&bold);
				output.push_str(&italic);
				if let Some(ref x) = bolditalic {
					output.push_str(x);
				}
				if let Some(ref x) = smallcaps {
					output.push_str(x);
				}
				if let Some(ref x) = path {
					output.push_str(x);
				}
				// last two chars will no matter what be ",\n"
				output.pop();
				output.pop();
				output.push_str(" ]");
				output
			}
		}
	}


	pub(crate) fn display_main(&self) -> String {
		let mut out = String::from("\\setmainfont");
		out.push_str(&self.display());
		out.push('\n');
		out
	}

	/// string representing the necessary command for a latex preamble
	/// to set this up as a new family with command `\$name'
	pub(crate) fn display_new_family(&self, name: &str) -> String {
		let mut out = String::from("\\newfontfamily\\");
		out.push_str(name);
		out.push_str(&self.display());
		if out.ends_with(" ]") {
			out.pop();
			out.pop();
			out.push_str(",\n\tScale=MatchUppercase ]");
		} else {
			out.push_str("[Scale=MatchUppercase]\n");
		}
		out
	}

	pub(crate) fn display_sans(&self) -> String {
		let mut out = String::from("\\setsansfont");
		out.push_str(&self.display());
		if out.ends_with(" ]") {
			out.pop();
			out.pop();
			out.push_str(",\n\tScale=MatchLowercase ]");
		} else {
			out.push_str("[Scale=MatchLowercase]\n");
		}
		out
	}

	pub(crate) fn display_mono(&self) -> String {
		let mut out = String::from("\\setmonofont");
		out.push_str(&self.display());
		if out.ends_with(" ]") {
			out.pop();
			out.pop();
			out.push_str(",\n\tScale=MatchLowercase ]");
		} else {
			out.push_str("[Scale=MatchLowercase]\n");
		}
		out

	}
}


#[derive(Debug, Clone, Default)]
pub struct LatexFonts {
	pub sans: Option<LatexFont>,
	pub serif: Option<LatexFont>,
	pub mono: Option<LatexFont>,
	pub titlepage: Option<LatexFont>,
	pub running: Option<LatexFont>,
	pub heading: Option<LatexFont>
}

/// Font sizes supported by LaTeX
#[derive(Debug, Clone, Copy)]
pub enum LatexFontSize {
	/// 10pt body text
	TenPt,
	/// 11pt body text
	ElevenPt,
	/// 12pt body text
	TwelvePt
}

impl Default for LatexFontSize {
	fn default() -> Self {LatexFontSize::TwelvePt}
}

static PACKAGES_WITHOUT_OPTIONS: [&str; 19] = [
	"amsmath",
	"amssymb",
	"bookmark",
	"booktabs",
	"etoolbox",
	"fancyhdr",
	"fancyvrb",
	"footnotehyper",
	"listings",
	"longtable",
	"unicode-math",
	"upquote",
	"xcolor",
	"xurl",
 	"fontspec",
 	"graphicx",
 	"microtype",
 	"hyperref",
 	"fmtcount",
];

static PFBREAK_COMMAND: &str = include_str!("resources/preamble_pfbreak.tex");
static HOUSEKEEPING: &str = include_str!("resources/preamble_housekeeping.tex");
static NEW_COMMANDS_AND_ENVIRONMENTS: &str = include_str!("resources/preamble_new.tex");
static PATCHES: &str = include_str!("resources/preamble_patches.tex");

macro_rules! set_font {
	($fn_name:ident, $field:ident, $doc:meta) => {
		#[$doc]
		pub fn $fn_name<T: Into<LatexFont>>(&mut self, typeface: T) -> &mut Self {
			
			match typeface.into() {
				LatexFont::Named(s) => {
					if font_exists(&s) {
						self.latex_fonts.$field = Some(LatexFont::Named(s));
						self
					} else {
						eprintln!("Could not set font '{:?}'; it may not be installed on this system", s);
						self
					}
				},
				complex => {
					self.latex_fonts.$field = Some(complex);
					self
				}
			}
		}
	};
}

/// Options for generating a pdf through LaTeX
#[derive(Debug, Clone)]
pub struct PreambleOptions {
	latex_plain_page_style: LatexPageStyle,
	latex_fancy_page_style: LatexPageStyle,
	latex_fontsize: LatexFontSize,
	latex_openany: bool,
	latex_secnumdepth: LatexSecNumDepth,
	latex_margins: LatexMargins,
	latex_fonts: LatexFonts,
	latex_linespread: f32,
	latex_part_format: Cow<'static, str>,
	latex_titlesec_options: Vec<Cow<'static, str>>,
	/// in latex captions for figures, label them e.g. "Figure 1.1: Caption Text",
	/// rather than using the caption alone
	do_not_suppress_figure_labels: bool,
	/// The path to a logo of the publisher for use on the titlepage
	publisher_imprint_logo: Option<PathBuf>,
	header_options: TextHeaderOptions,
	include_toc: bool,
	// custom label for chapters
	chapter_label: Option<Cow<'static, str>>
}




impl PreambleOptions {

	/// Set a custom label for chapters (i.e. renew `\\chaptername` in the preamble)
	pub fn chapter_label<S: Into<Cow<'static, str>>>(&mut self, label: S) -> &mut Self {
		self.chapter_label = Some(label.into());
		self
	}

	/// Set the linespread
	pub fn set_linespread(&mut self, linespread: f32) -> &mut Self {
		self.latex_linespread = linespread;
		self
	}

	/// Open on any
	pub fn open_any(&mut self) -> &mut Self {
		self.latex_openany = true;
		self
	}

	/// Set the secnumdepth
	pub fn set_secnumdepth(&mut self, secnumdepth: LatexSecNumDepth) -> &mut Self {
		self.latex_secnumdepth = secnumdepth;
		self
	}

	/// Set the logo of a publisher to use on the titlepage
	pub fn set_publisher_logo(&mut self, mut p: PathBuf) -> Result<&mut Self, String> {
		p = p.canonicalize()
			.map_err(|e| format!("Error canonicalizing path ({}): {}", p.display(), e.to_string()))?;

		if p.is_svg() {
			let svg = std::fs::read_to_string(&p)
				.map_err(|e| format!("Error reading svg path ({}): {}", p.display(), e.to_string()))?;
			let png_path = svg.temp_file_path(Some("bookbinder"), "png");
			if png_path.exists() {
				p = png_path;
			} else {
				let png = bookbinder_common::convert_svg_file_to_png(&p, Some(150))
					.map_err(|e| format!("Error converting svg file to png ({}): {}", p.display(), e.to_string()))?;
				std::fs::write(&png_path, png)
					.map_err(|e| format!("Error writing new png path ({}): {}", p.display(), e.to_string()))?;
				p = png_path;
			}
		};
		if p.exists() && p.is_valid_logo_image() {
			self.publisher_imprint_logo = Some(p);
			Ok(self)
		} else {
			Err(format!("Error setting logo path: {}", p.display()))
		}
	}

	set_font!(set_sans_typeface, sans, doc="Set the sans typeface; this is used in sans text, but is also the fallback typeface for headings, headers and footers, and the titlepage");
	set_font!(set_serif_typeface, serif, doc="Set the serif typeface");
	set_font!(set_mono_typeface, mono, doc="Set the monospace typeface to use in code samples, urls, etc");
	set_font!(set_titlepage_typeface, titlepage, doc="Set the typeface to use on the titlepage");
	set_font!(set_headers_and_footers_typeface, running, doc="Set the typeface to use in running headers and footers");
	set_font!(set_heading_typeface, heading, doc="Set the typeface to use in headings");


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

	/// Include a table of contents
	pub fn include_toc(&mut self) -> &mut Self {
		self.include_toc = true;
		self
	}

	/// have blank pdf running footers
	pub fn suppress_footers(&mut self) -> &mut Self {
		self.latex_plain_page_style.make_empty();
		self.latex_fancy_page_style.make_empty();
		self
	}

	/// only include the page number in pdf running footers
	pub fn page_number_only_in_footers(&mut self) -> &mut Self {
		self.latex_plain_page_style.make_plain();
		self.latex_fancy_page_style.make_plain();
		self
	}

	/// label figures in pdf output
	pub fn do_not_suppress_figure_labels(&mut self) -> &mut Self {
		self.do_not_suppress_figure_labels = true;
		self
	}

	/// set the font size to use in pdf output at 11pt
	pub fn ten_pt(&mut self) -> &mut Self {
		self.latex_fontsize = LatexFontSize::TenPt;
		self
	}

	/// set the font size to use in pdf output at 11pt
	pub fn eleven_pt(&mut self) -> &mut Self {
		self.latex_fontsize = LatexFontSize::ElevenPt;
		self
	}

	/// set the font size to use in pdf output at 11pt
	pub fn twelve_pt(&mut self) -> &mut Self {
		self.latex_fontsize = LatexFontSize::TwelvePt;
		self
	}

	/// Set the size of paper to use in this book
	pub fn set_papersize(&mut self, size: PaperSize) -> &mut Self {
		self.latex_margins = LatexMargins::from(size);
		self
	}


	fn get_preamble_opening(&self) -> String {
		let mut opening = String::new();
		opening.push_str("\\documentclass[");

		let fontsize = match self.latex_fontsize {
			LatexFontSize::TenPt => "10pt",
			LatexFontSize::ElevenPt => "11pt",
			LatexFontSize::TwelvePt => "12pt"
		};
		opening.push_str(fontsize);
		if self.latex_openany {
			opening.push_str(", openany");
		}
		opening.push_str("]{book}\n");
		opening
	}

	fn get_preamble_geometry(&self) -> String {
		let unit = match self.latex_margins.unit {
			MeasurementUnit::Mm => "mm",
			MeasurementUnit::Inches => "in"
		};

		let paperwidth = &self.latex_margins.paper_width;
		let paperheight = &self.latex_margins.paper_height;
		let top_margin = &self.latex_margins.top;
		let bottom_margin = &self.latex_margins.bottom;
		let left_margin = &self.latex_margins.left;
		let right_margin = &self.latex_margins.right;

		let mut geometry = String::new();
		geometry.push_str("\n\\usepackage[\n");
		let papersize = format!("papersize={{{}{unit}, {}{unit}}},", paperwidth, paperheight, unit=unit);
		geometry.push_str(&papersize);
		let vmargin = format!("\nvmargin={{{}{unit}, {}{unit}}},", top_margin, bottom_margin, unit=unit);
		geometry.push_str(&vmargin);
		geometry.push_str(&format!("\nleft={}{},", left_margin, unit));
		geometry.push_str(&format!("\nright={}{}", right_margin, unit));
		geometry.push_str("\n]{geometry}");
		geometry

	}

	fn get_preamble_packages(&self) -> String {
		let mut packages = String::new();
		for package in PACKAGES_WITHOUT_OPTIONS.iter() {
			let p = format!("\n\\usepackage{{{}}}", package);
			packages.push_str(&p);
		}

		if !self.do_not_suppress_figure_labels {
			packages.push_str("\n\\usepackage[labelformat=empty, font=sf]{caption}");
		}

		packages.push_str("\n\\usepackage[");
		packages.push_str(&self.latex_titlesec_options.join(", "));
		packages.push_str("]{titlesec}\n");
		packages.push_str("\n\\usepackage[normalem]{ulem}\n");
		packages.push_str("\n\\usepackage[overload]{textcase}\n");
		packages
	}

	fn get_plain_layout(&self) -> String {
		let mut plain_layout = String::new();
		plain_layout.push_str("\n\\fancypagestyle{plain}{%\n");
		plain_layout.push_str(&self.latex_plain_page_style.to_preamble_commands());
		plain_layout.push_str("}\n");
		plain_layout
	}

	fn get_fancy_layout(&self) -> String {
		let mut fancy_layout = String::from("\\pagestyle{fancy}\n");
		fancy_layout.push_str(&self.latex_fancy_page_style.to_preamble_commands());
		fancy_layout.push('\n');
		fancy_layout
	}

	fn generate_latex_preamble(&self) -> String {
		let fixed_len = PATCHES.len() + HOUSEKEEPING.len() + PFBREAK_COMMAND.len() + NEW_COMMANDS_AND_ENVIRONMENTS.len();
		let len = fixed_len + 200;

		let mut preamble = String::with_capacity(len);

		preamble.push_str(&self.get_preamble_opening());
		preamble.push_str(&self.get_preamble_geometry());
		// patch as early as we can
		preamble.push_str("\\usepackage{xpatch}\n");
		preamble.push_str(PATCHES);
		preamble.push('\n');
		preamble.push_str(&self.get_preamble_packages());

		
		if let Some(ref serif) = self.latex_fonts.serif {
			preamble.push_str(&serif.display_main());
		} else {
			let serif = LatexFont::default_serif();
			preamble.push_str(&serif.display_main());
		}
		
		if let Some(ref sans) = self.latex_fonts.sans {
			preamble.push_str(&sans.display_sans());
		} else {
			let sans = LatexFont::default_sans();
			preamble.push_str(&sans.display_sans());
		}

		if let Some(ref mono) = self.latex_fonts.mono {
			preamble.push_str(&mono.display_mono());
		}

		if let Some(ref titlepage_typeface) = self.latex_fonts.titlepage {
			preamble.push_str(&titlepage_typeface.display_new_family("titlepagetypeface"));
		} else {
			preamble.push_str("\\newcommand{\\titlepagetypeface}{\\sffamily}\n");
		}

		if let Some(ref running_typeface) = self.latex_fonts.running {
			preamble.push_str(&running_typeface.display_new_family("runningtypeface"));
		} else {
			preamble.push_str("\\newcommand{\\runningtypeface}{\\sffamily}\n");
		}

		if let Some(ref heading_typeface) = self.latex_fonts.heading {
			preamble.push_str(&heading_typeface.display_new_family("headingtypeface"));
		} else {
			preamble.push_str("\\newcommand{\\headingtypeface}{\\sffamily}\n");
		}

		preamble.push_str(HOUSEKEEPING);
		preamble.push('\n');
		preamble.push_str(PFBREAK_COMMAND);
		preamble.push('\n');
		preamble.push_str(NEW_COMMANDS_AND_ENVIRONMENTS);
		preamble.push('\n');
		preamble.push_str(&self.latex_part_format);
		preamble.push('\n');


		preamble.push_str("\\linespread{");
		preamble.push_str(&format!("{}", self.latex_linespread));
		preamble.push('}');

		preamble.push_str(&self.get_plain_layout());
		preamble.push_str(&self.get_fancy_layout());

		preamble.push_str("\\setcounter{tocdepth}{3}\n");
		
		match self.header_options.chapter_number_format {
			NumberFormat::Arabic => {},
			NumberFormat::Roman => {
				preamble.push_str("\\renewcommand{\\thechapter}{\\Roman{chapter}}");
			},
			NumberFormat::Words => {
				preamble.push_str("\\renewcommand{\\thechapter}{\\NUMBERstring{chapter}}");
			},
			NumberFormat::Letter => {
				preamble.push_str("\\renewcommand{\\thechapter}{\\Alph{chapter}}");
			}
		}

		match self.header_options.part_number_format {
			NumberFormat::Arabic => {
				preamble.push_str("\\renewcommand{\\thepart}{\\arabic{part}}");
			},
			NumberFormat::Roman => {},
			NumberFormat::Words => {
				preamble.push_str("\\renewcommand{\\thepart}{\\NUMBERstring{part}}");
			},
			NumberFormat::Letter => {
				preamble.push_str("\\renewcommand{\\thepart}{\\Alph{part}}");
			}
		}

		if let Some(ref label) = self.chapter_label {
			preamble.push_str("\n\\renewcommand{\\chaptername}{");
			preamble.push_str(label);
			preamble.push_str("}\n");
		}

		preamble
	}
}

impl From<PreambleOptions> for OptionsWithRenderedPreamble {
	fn from(src: PreambleOptions) -> Self {
		let preamble = src.generate_latex_preamble();
		OptionsWithRenderedPreamble {
			publisher_imprint_logo: src.publisher_imprint_logo,
			header_format: src.header_options,
			preamble,
			page_identifier: None,
			contributor_identifier: None,
			include_toc: src.include_toc,
			latex_secnumdepth: src.latex_secnumdepth
		}
	}
}

impl Default for PreambleOptions {
	fn default() -> Self {
		PreambleOptions {
			header_options: TextHeaderOptions::default(),
			latex_plain_page_style: LatexPageStyle::default_plain(),
			latex_fancy_page_style: LatexPageStyle::default_fancy(),
			latex_fontsize: LatexFontSize::default(),
			latex_openany: false,
			latex_secnumdepth: LatexSecNumDepth::default(),
			latex_margins: LatexMargins::default(),
			latex_fonts: LatexFonts::default(),
			latex_linespread: DEFAULT_LINESPREAD,
			latex_part_format: DEFAULT_PART_FORMAT.into(),
			publisher_imprint_logo: None,
			include_toc: false,
			latex_titlesec_options: DEFAULT_TITLESEC_OPTIONS.iter()
				.copied()
				.map(|s| s.into())
				.collect(),
			do_not_suppress_figure_labels: false,
			chapter_label: None
		}
	}
}
