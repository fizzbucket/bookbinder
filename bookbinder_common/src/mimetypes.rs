use std::path::Path;

/// A mimetype relevant to book production
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum MimeType {
	/// A jpeg file
	Jpeg,
	/// A css file
	Css,
	/// An epub file
	Epub,
	/// A png file
	Png,
	/// A svg file
	Svg,
	/// A gif file
	Gif,
	/// A pdf file
	Pdf,
	/// A txt file
	PlainText,
	/// A markdown file
	Markdown,
	/// An eps file
	Eps,
	/// An xhtml file
	Xhtml,
	/// A woff font file
	Woff,
	/// An opentype font file
	OpenType,
	/// A latex file
	Latex
}

impl MimeType {

	/// Guess a mimetype from an extension
	pub fn new_from_extension(ext: &str) -> Option<Self> {
		use MimeType::*;
		match ext {
			"jpg" => Some(Jpeg),
			"jpeg" => Some(Jpeg),
			"png" => Some(Png),
			"svg" => Some(Svg),
			"css" => Some(Css),
			"otf" => Some(OpenType),
			"woff" => Some(Woff),
			"xhtml" => Some(Xhtml),
			"md" | "txt" | "markdown" => Some(Markdown),
			"tex" | "latex" => Some(Latex),
			"eps" => Some(Eps),
			"gif" => Some(Gif),
			"pdf" => Some(Pdf),
			_ => None
		}
	}

	/// return the canonical str representation of this mimetype
	pub const fn to_str(&self) -> &'static str {
		use MimeType::*;
		match self {
			Jpeg => "image/jpeg",
			Css => "text/css",
			Epub => "application/epub+zip",
			Png => "image/png",
			Svg => "image/svg+xml",
			Gif => "image/gif",
			Pdf => "application/pdf",
			PlainText => "text/plain",
			Markdown => "text/markdown",
			Eps => "application/postscript",
			Xhtml => "application/xhtml+xml",
			Woff => "application/font-woff",
			OpenType => "application/font-sfnt",
			Latex => "text/latex"
		}
	}
}

/// Helper to guess the mimetype of paths
pub trait GuessMimeType {
	/// guess the mimetype of this object
	fn guess_mime(&self) -> Option<MimeType>;
}

impl <T> GuessMimeType for T where T: AsRef<Path> {
	/// guess the mimetype of this path-like object
	fn guess_mime(&self) -> Option<MimeType> {
		match self.as_ref().extension() {
			Some(ext) => {
				match ext.to_str() {
					Some(ext) => MimeType::new_from_extension(ext),
					None => None
				}				
			},
			None => None
		}
	}
}

/// Various helpful functions for analysing filepaths
pub trait MimeTypeHelper {
	/// is this likely to be a jpg file?
	fn is_jpg(&self) -> bool;
	/// is this likely to be a png file?
	fn is_png(&self) -> bool;
	/// is this likely to be a svg file?
	fn is_svg(&self) -> bool;
	/// Is this likely to represent a css file?
	fn is_css(&self) -> bool;
	/// Is this likely to represent an opentype font?
	fn is_opentype(&self) -> bool;
	/// Is this likely to represent a woff font?
	fn is_woff(&self) -> bool;
	/// Is this likely to represent xhtml?
	fn is_xhtml(&self) -> bool;
	/// Is this likely to represent markdown?
	fn is_markdown(&self) -> bool;
	/// Is this likely to represent a tex or txt file?
	fn is_tex_like(&self) -> bool;
	/// Is this likely to represent an eps file?
	fn is_eps(&self) -> bool;
	/// Is this likely to represent a gif?
	fn is_gif(&self) -> bool;
	/// Is this likely to represent a pdf?
	fn is_pdf(&self) -> bool;
	/// Is this able to be incorporated as the background of a cover image?
	/// i.e. is png, jpg, or svg
	fn is_suitable_cover_background(&self) -> bool;
	/// Is this an image which epub supports?
	/// These are png, jpg, gif and svg
	fn is_epub_supported_image(&self) -> bool;
	/// Is this likely to represent an image latex can render?
	/// These are pdf, png, jpg and eps
	fn is_latex_supported_image(&self) -> bool;
	/// Is this likely to represent a file which can be included in an epub?
	/// These are supported image formats, xhtml+xml, opentype, woff or css
	fn is_epub_supported_resource(&self) -> bool;
	/// can this be used as a publisher logo on the titlepage in both an epub and pdf?
	fn is_valid_logo_image(&self) -> bool;
}

impl MimeTypeHelper for MimeType {
	fn is_jpg(&self) -> bool {
		*self == MimeType::Jpeg
	}
	fn is_png(&self) -> bool {
		*self == MimeType::Png
	}
	fn is_svg(&self) -> bool {
		*self == MimeType::Svg
	}
	fn is_css(&self) -> bool {
		*self == MimeType::Css
	}
	fn is_opentype(&self) -> bool {
		*self == MimeType::OpenType
	}
	fn is_woff(&self) -> bool {
		*self == MimeType::Woff
	}
	fn is_xhtml(&self) -> bool {
		*self == MimeType::Xhtml
	}
	fn is_markdown(&self) -> bool {
		*self == MimeType::Markdown
	}
	fn is_tex_like(&self) -> bool {
		*self == MimeType::Latex
	}
	fn is_eps(&self) -> bool {
		*self == MimeType::Eps
	}
	fn is_gif(&self) -> bool {
		*self == MimeType::Gif
	}
	fn is_pdf(&self) -> bool {
		*self == MimeType::Pdf
	}

	fn is_suitable_cover_background(&self) -> bool {
		match self {
			MimeType::Png | MimeType::Jpeg | MimeType::Svg => true,
			_ => false
		}
	}
	fn is_epub_supported_image(&self) -> bool {
		match self {
			MimeType::Png | MimeType::Jpeg | MimeType::Svg | MimeType::Gif => true,
			_ => false
		}	
	}
	fn is_latex_supported_image(&self) -> bool {
		match self {
			MimeType::Png | MimeType::Jpeg | MimeType::Eps | MimeType::Pdf => true,
			_ => false
		}
	}
	fn is_epub_supported_resource(&self) -> bool {
		match self {
			m if m.is_epub_supported_image() => true,
			MimeType::Xhtml | MimeType::OpenType | MimeType::Css | MimeType::Woff => true,
			_ => false
		}
	}

	fn is_valid_logo_image(&self) -> bool {
		let works_for_epub = self.is_epub_supported_image();
		let works_for_latex = self.is_latex_supported_image();
		works_for_latex && works_for_epub
	}
}

macro_rules! reroute_func {
	($fn_name:ident) => {
		fn $fn_name(&self) -> bool {
			match self.guess_mime() {
				Some(m) => m.$fn_name(),
				None => false
			}
		}
	};
}

impl <T> MimeTypeHelper for T where T: GuessMimeType {
	reroute_func!(is_jpg);
	reroute_func!(is_png);
	reroute_func!(is_svg);
	reroute_func!(is_css);
	reroute_func!(is_opentype);
	reroute_func!(is_woff);
	reroute_func!(is_epub_supported_image);
	reroute_func!(is_epub_supported_resource);
	reroute_func!(is_xhtml);
	reroute_func!(is_eps);
	reroute_func!(is_gif);
	reroute_func!(is_tex_like);
	reroute_func!(is_markdown);
	reroute_func!(is_pdf);
	reroute_func!(is_latex_supported_image);
	reroute_func!(is_suitable_cover_background);
	reroute_func!(is_valid_logo_image);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_mimetypes() {
		let jpg = Path::new("example.jpg");
		let jpeg = Path::new("example.jpeg");
		let png = Path::new("example.png");
		let gif = Path::new("example.gif");
		let svg = Path::new("example.svg");
		let eps = Path::new("example.eps");
		let css = Path::new("example.css");
		let otf = Path::new("example.otf");
		let woff = Path::new("example.woff");
		let xhtml = Path::new("example.xhtml");
		let text = Path::new("example.txt");
		let md = Path::new("example.md");
		let pdf = Path::new("example.pdf");

		let jpg_paths = [jpg, jpeg];
		let epub_supported_images = [jpg, jpeg, png, gif, svg];
		let latex_supported_images = [png, pdf, jpg, jpeg, eps];
		let css_paths = [css];
		let epub_supported_resources = [css, woff, otf, xhtml, jpg, jpeg, png, gif, svg];
		let opentype_paths = [otf];
		let woff_paths = [woff];
		let xhtml_paths = [xhtml];
		let markdown_paths = [text, md];

		let all_paths = [jpg, jpeg, png, gif, svg, eps, css, otf, woff, xhtml, text, md];


		macro_rules! test_mimetype {
			($target:expr, $path_list:expr, $test:expr) => {
				if $path_list.contains($target) {
					assert_eq!($test, true);
				} else {
					assert_eq!($test, false);
				}
			};
		}


		for p in all_paths.iter() {
			println!("{:?}", p);
			let is_jpg = p.is_jpg();
			let is_epub_supported_image = p.is_epub_supported_image();
			let is_latex_supported_image = p.is_latex_supported_image();
			let is_css = p.is_css();
			let is_epub_supported_resource = p.is_epub_supported_resource();
			let is_opentype = p.is_opentype();
			let is_woff = p.is_woff();
			let is_xhtml = p.is_xhtml();
			let is_markdown = p.is_markdown();

			println!("is_jpg: {}", is_jpg);
			println!("epub img: {}", is_epub_supported_image);
			println!("latex img: {}", is_latex_supported_image);
			println!("css: {}", is_css);
			println!("epub resource: {}", is_epub_supported_resource);
			println!("opentype: {}", is_opentype);
			println!("woff: {}", is_woff);
			println!("xhtml: {}", is_xhtml);
			println!("is_markdown: {}", is_markdown);

			test_mimetype!(p, jpg_paths, is_jpg);
			test_mimetype!(p, epub_supported_images, is_epub_supported_image);
			test_mimetype!(p, latex_supported_images, is_latex_supported_image);
			test_mimetype!(p, css_paths, is_css);
			test_mimetype!(p, epub_supported_resources, is_epub_supported_resource);
			test_mimetype!(p, opentype_paths, is_opentype);
			test_mimetype!(p, woff_paths, is_woff);
			test_mimetype!(p, xhtml_paths, is_xhtml);
			test_mimetype!(p, markdown_paths, is_markdown);
		}
	}
}