use image::jpeg::JpegEncoder;
use image::png::PngEncoder;
use image::ColorType;
use resvg::render;
use resvg::Image;
use std::error::Error;
use std::fmt;
use std::path::Path;
use usvg::{Tree, XmlOptions};

/// Use usvg to simplify an svg; this is most important in that text is rendered as paths
pub fn simplify_svg(svg: &str, dpi: Option<usize>) -> Result<String, SvgError> {
    let mut options = usvg::Options::default();
    if let Some(dpi) = dpi {
        options.dpi = dpi as f64;
    }

    options.fontdb = crate::fonts::USVG_FONT_DB.clone();

    let tree = Tree::from_str(svg, &options)?;
    Ok(tree.to_string(XmlOptions::default()))
}

/// Convert an svg file to a png with a resolution of `dpi` or the default
pub fn convert_svg_file_to_png(svg: &Path, dpi: Option<usize>) -> Result<Vec<u8>, SvgError> {
    let data = std::fs::read_to_string(svg)?;
    convert_svg_to_png(&data, dpi)
}

fn render_svg(svg: &str, dpi: Option<usize>, alpha: bool) -> Result<Image, SvgError> {
    let mut options = usvg::Options::default();
    if let Some(dpi) = dpi {
        options.dpi = dpi as f64;
    }
    let tree = Tree::from_str(svg, &options)?;
    let fit_to = usvg::FitTo::Original;
    let background = if alpha {
        None
    } else {
        Some(svgtypes::Color::white())
    };
    render(&tree, fit_to, background).ok_or(SvgError::Unspecified)
}

/// Convert a svg str to a png file
pub fn convert_svg_to_png(svg: &str, dpi: Option<usize>) -> Result<Vec<u8>, SvgError> {
    let image = render_svg(svg, dpi, true)?;
    let width = image.width();
    let height = image.height();

    let mut png = Vec::new();
    let encoder = PngEncoder::new(&mut png);
    encoder.encode(image.data(), width, height, ColorType::Rgba8)?;
    Ok(png)
}

/// Convert a svg str to a jpg file
pub fn convert_svg_to_jpg(svg: &str, dpi: Option<usize>) -> Result<Vec<u8>, SvgError> {
    let image = render_svg(svg, dpi, false)?;
    let width = image.width();
    let height = image.height();

    let mut jpeg = Vec::new();
    let mut encoder = JpegEncoder::new(&mut jpeg);
    encoder.encode(image.data(), width, height, ColorType::Rgba8)?;
    Ok(jpeg)
}

#[derive(Debug)]
pub enum SvgError {
    Unspecified,
    Usvg(usvg::Error),
    Image(image::error::ImageError),
    Io(std::io::Error),
}

impl Error for SvgError {}

impl fmt::Display for SvgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<usvg::Error> for SvgError {
    fn from(src: usvg::Error) -> Self {
        SvgError::Usvg(src)
    }
}

impl From<image::error::ImageError> for SvgError {
    fn from(src: image::error::ImageError) -> Self {
        SvgError::Image(src)
    }
}

impl From<std::io::Error> for SvgError {
    fn from(src: std::io::Error) -> Self {
        SvgError::Io(src)
    }
}
