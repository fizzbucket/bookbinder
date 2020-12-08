//! We want to have a nice epub titlepage;
//! but there's a bit of a problem.
//! The state of epub rendering is such that
//! it's not really possible to have a titlepage with even small details
//! like a logo floated to the bottom.
//! Instead, this crate lets us generate a svg.
//! That presents its own problem; svg renderers don't do nice things
//! like break text for us, and we quickly run into problems like positioning a centred title
//! followed by a centred subtitle: how do we know where to position everything?
//!
//! The somewhat unorthodox answer of this crate is to render fragments of text in isolation
//! in order to give us a sense of their size; this isn't perfectly accurate but gives us enough information
//! to be getting on with.
//!
//! The alternative would be to generate either a pdf or html titlepage -- which will make use of far more sophisticated
//! layout algorithms. It's unfortunately difficult to do either of these in a way which doesn't bloat the crate
//! and lead to cross-compilation problems, since we can't generate an svg from a pdf without relying on -- most likely --
//! poppler and in turn all of cairo, or from an html file without pulling in all of an html renderer.
use crate::text_splitting::{split_text, SizedTextOrSpace};
use crate::TitlePageSource;
use bookbinder_common::fonts::{self, FULL_FONT_DB, LIMITED_FONT_DB};
use fontdb::{Family, Query, Stretch, Style, Weight};
use image::imageops::FilterType;
use image::{GenericImageView, ImageOutputFormat};
use rustybuzz::UnicodeBuffer;
use rustybuzz::Face as Font;
use std::borrow::Cow;
use std::convert::TryFrom;
use std::path::PathBuf;
use temp_file_name::TempFilePath;
use ttf_parser::GlyphId;

#[derive(Debug, Clone, Hash)]
pub(crate) enum TitleEvent<'a> {
    Text(Cow<'a, str>),
    Emphasised(Cow<'a, str>),
}

pub(crate) struct FontData<'a, 'b> {
    data: Cow<'b, [u8]>,
    face_id: u32,
    style: Style,
    weight: Weight,
    family: Family<'a>,
}

impl<'a, 'b> FontData<'a, 'b> {
    fn new(family: Family<'a>, weight: Weight, style: Style) -> Self {
        let families = vec![family];
        let query = Query {
            families: &families,
            weight,
            stretch: Stretch::Normal,
            style,
        };

        let (data, face_id) = match family {
            Family::SansSerif => {
                if weight == Weight::BOLD && style == Style::Normal {
                    (Cow::Borrowed(fonts::DEFAULT_SANS_BOLD), 0)
                } else if weight == Weight::BOLD && style == Style::Italic {
                    (Cow::Borrowed(fonts::DEFAULT_SANS_BOLD_ITALIC), 0)
                } else if weight == Weight::NORMAL && style == Style::Normal {
                    (Cow::Borrowed(fonts::DEFAULT_SANS), 0)
                } else if weight == Weight::NORMAL && style == Style::Italic {
                    (Cow::Borrowed(fonts::DEFAULT_SANS_ITALIC), 0)
                } else if weight == Weight::SEMIBOLD && style == Style::Normal {
                    (Cow::Borrowed(fonts::DEFAULT_SANS_SEMIBOLD), 0)
                } else if weight == Weight::SEMIBOLD && style == Style::Italic {
                    (Cow::Borrowed(fonts::DEFAULT_SANS_SEMIBOLD_ITALIC), 0)
                } else {
                    LIMITED_FONT_DB
                        .query(&query)
                        .map(|id| {
                            LIMITED_FONT_DB.with_face_data(id, |d, i| (Cow::Owned(d.to_vec()), i))
                        })
                        .flatten()
                        .unwrap()
                }
            }
            _ => {
                if let Some(id) = FULL_FONT_DB.query(&query) {
                    match FULL_FONT_DB.with_face_data(id, |d, i| (Cow::Owned(d.to_vec()), i)) {
                        Some((data, face_id)) => (data, face_id),
                        None => {
                            eprintln!("Could not find specified font: {:?}", family);
                            return FontData::new(Family::SansSerif, weight, style);
                        }
                    }
                } else {
                    eprintln!("Could not find specified font: {:?}", family);
                    return FontData::new(Family::SansSerif, weight, style);
                }
            }
        };

        FontData {
            data,
            face_id,
            family,
            style,
            weight,
        }
    }
}

pub(crate) struct TypeSetter<'a> {
    face: ttf_parser::Face<'a>,
    font: Font<'a>,
    px_size: f32,
    line_height: i32,
}

impl<'a> TypeSetter<'a> {
    fn new(data: &'a FontData, size: f32) -> Result<Self, ()> {
        let face = ttf_parser::Face::from_slice(&data.data, data.face_id).map_err(|_| ())?;
        let font = Font::from_slice(&data.data, data.face_id).ok_or(())?;

        let units_per_em: f32 = face
            .units_per_em()
            .expect("Error getting units per em")
            .into();

        let line_height = ((face.height() as f32 * size) / units_per_em) as i32;

        Ok(TypeSetter {
            face,
            font,
            px_size: size,
            line_height,
        })
    }

    /// work out:
    /// (a) the horizontal length of `text` when set,
    /// (b) its vertical height, defined
    /// the maximum ascent above the baseline by a glyph,
    /// the maximum descent below the baseline,
    /// and the font's recommended line gap.
    /// This fn returns the length, height above baseline and height below baseline
    fn shape(&self, text: &str) -> (i32, i32, i32) {
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        let shaped = rustybuzz::shape(&self.font, &[], buffer);
        let positions = shaped.glyph_positions();
        let horizontal_len: i32 = positions.iter().map(|p| p.x_advance).sum();
        let (y_mins, y_maxes): (Vec<_>, Vec<_>) = shaped
            .glyph_infos()
            .iter()
            .map(|g| g.codepoint)
            .map(|c| GlyphId(u16::try_from(c).unwrap()))
            .filter_map(|id| self.face.glyph_bounding_box(id))
            .map(|x| (x.y_min, x.y_max))
            .unzip();
        let y_max = y_maxes.into_iter().max().unwrap_or(0);
        let y_min = y_mins.into_iter().min().unwrap_or(0);
        let units_per_em: i32 = self
            .face
            .units_per_em()
            .expect("Error getting units per em")
            .into();

        // 1em is equal to the font size
        // so if font_size == 16px, 1 em = 16px

        let horizontal_len = (horizontal_len as f32 * self.px_size) as i32 / units_per_em;
        let y_max = (y_max as f32 * self.px_size) as i32 / units_per_em;
        let y_min = (y_min as f32 * self.px_size) as i32 / units_per_em;

        (horizontal_len, y_max, y_min)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SizedText<'a> {
    pub(crate) text: Cow<'a, str>,
    pub(crate) width: i32,
    pub(crate) height_above_baseline: i32,
    pub(crate) height_below_baseline: i32,
    pub(crate) family: Family<'a>,
    pub(crate) size: f32,
    pub(crate) weight: Weight,
    pub(crate) style: Style,
    pub(crate) line_height: i32,
}

impl<'a> SizedText<'a> {
    pub(crate) fn new<S: Into<Cow<'a, str>>>(
        data: &FontData<'a, '_>,
        setter: &TypeSetter,
        text: S,
    ) -> Self {
        let text = text.into();
        let (width, height_above_baseline, height_below_baseline) = setter.shape(&text);
        let height_below_baseline = -height_below_baseline;

        let family = data.family;
        let size = setter.px_size;
        let weight = data.weight;
        let style = data.style;
        SizedText {
            text,
            width,
            height_above_baseline,
            height_below_baseline,
            family,
            size,
            weight,
            style,
            line_height: setter.line_height,
        }
    }

    fn get_family(&'a self) -> &'a str {
        match self.family {
            Family::Serif => fonts::DEFAULT_SERIF_FAMILY_NAME,
            Family::SansSerif => fonts::DEFAULT_SANS_FAMILY_NAME,
            Family::Cursive => "cursive",
            Family::Fantasy => "fantasy",
            Family::Monospace => "monospace",
            Family::Name(ref f) => f,
        }
    }

    fn get_style(&self) -> &'static str {
        match self.style {
            Style::Normal => "normal",
            Style::Italic => "italic",
            Style::Oblique => "oblique",
        }
    }

    fn get_weight(&self) -> u16 {
        self.weight.0
    }

    fn to_element_at_midpoint(&self, midpoint: i32, y: i32) -> String {
        let family = self.get_family();
        let style = self.get_style();
        let weight = self.get_weight();
        let x = midpoint - (self.width / 2);
        let y = y + self.height_above_baseline;
        format!("<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-style=\"{}\" font-size=\"{}px\" font-weight=\"{}\">{}</text>", x, y, family, style, self.size, weight, self.text)
    }

    fn to_element_at_specified_point(&self, x: i32, y: i32) -> String {
        let family = self.get_family();
        let style = self.get_style();
        let weight = self.get_weight();
        let y = y + self.height_above_baseline;
        format!("<text x=\"{}\" y=\"{}\" font-family=\"{}\" font-style=\"{}\" font-size=\"{}px\" font-weight=\"{}\">{}</text>", x, y, family, style, self.size, weight, self.text)
    }
}

// various helpful constants; these are meant to
// produce similar output to a latex titlepage.
// the units there were set in parts of an inch.
// In our imagined co-ordinate system for this file,
// note that 200px == 1in.
// These values have been adjusted from the latex originals
// to reflect this

pub(crate) const WIDTH: i32 = 1200;
const HEIGHT: i32 = 1800;
pub(crate) const HORIZONTAL_MARGIN: i32 = 100;
const TOP_MARGIN: i32 = 300;
const BOTTOM_MARGIN: i32 = 120;
const RULE_LENGTH: i32 = 400;
const RULE_SPACING: i32 = 39;
const CONTRIBUTOR_GROUP_SPACING: i32 = 60;
const CONTRIBUTOR_AND_SPACING: i32 = 16;
const CONTRIBUTOR_INTRO_SPACING: i32 = 39;
const HUGE_FONT_SIZE_PTS: u8 = 25; // Latex `Huge` at 12pt base
const LARGE_FONT_SIZE_PTS: u8 = 17; // Latex `Large` at 12pt base

// 1in == 72.26999pt in LaTeX
// e.g. 25pt is approx 0.345in; that is 69.18px.
const HUGE_FONT_SIZE: f32 = (HUGE_FONT_SIZE_PTS as f32 / 72.26999) * 200.0;
const LARGE_FONT_SIZE: f32 = (LARGE_FONT_SIZE_PTS as f32 / 72.26999) * 200.0;

const MAX_LOGO_HEIGHT: u32 = 200;

struct SVGWriter {
    svg: String,
    midpoint: i32,
    y: i32,
}

impl SVGWriter {
    fn new() -> Self {
        SVGWriter {
            midpoint: WIDTH / 2,
            y: TOP_MARGIN,
            svg: String::new(),
        }
    }

    fn add_line(&mut self, line: Vec<SizedTextOrSpace<'_>>) -> i32 {
        let mut line_width = 0;
        for item in line.iter() {
            match item {
                SizedTextOrSpace::Space(w) => line_width += *w as i32,
                SizedTextOrSpace::Text(t) => line_width += t.width,
            }
        }

        let mut x = self.midpoint - (line_width / 2);
        let mut line_height = 0;

        for item in line.into_iter() {
            match item {
                SizedTextOrSpace::Text(group) => {
                    line_height = std::cmp::max(line_height, group.line_height);
                    x = self.add_text_at_horizontal_point(group, x);
                }
                SizedTextOrSpace::Space(width) => {
                    x += width as i32;
                }
            }
        }
        self.move_y(line_height);
        line_height
    }

    /// add text so that its top left corner is at (`point`, `self.y`);
    /// return the horizontal point to which the added text extends
    fn add_text_at_horizontal_point(&mut self, text: SizedText<'_>, point: i32) -> i32 {
        let element = text.to_element_at_specified_point(point, self.y);
        self.svg.push_str(&element);
        point + text.width
    }

    /// add text so that it is centred around the midpoint of this svg,
    /// and the top of its bounding box is at `self.y`.
    /// Set `self.y` to be the bottom of the text's bounding box
    fn add_text_and_move_down(&mut self, text: SizedText<'_>) {
        let element = text.to_element_at_midpoint(self.midpoint, self.y);
        self.svg.push_str(&element);
        self.y += text.line_height;
    }

    /// Add a dividing line and move `self.y` down.
    fn add_divider(&mut self) {
        self.y += RULE_SPACING;
        let half_rule = RULE_LENGTH / 2;
        let x1 = self.midpoint - half_rule;
        let x2 = self.midpoint + half_rule;
        self.svg.push_str(&format!(
            "<line x1=\"{}\" x2=\"{}\" y1=\"{}\" y2=\"{}\" stroke=\"black\"/>\n",
            x1, x2, self.y, self.y
        ));
        self.y += RULE_SPACING;
    }

    /// Add a logo to the bottom of the page, sized to fit within
    /// a 1000x200 box (while maintaining aspect ratio) with the bottom of the logo at 100px from the bottom
    /// of the svg.
    /// e.g. a 100*100 logo will be added with its top left corner at
    /// (500, 1500) and resized to be 200*200
    /// a 1000X100 logo will be added with its top-left corner at (100, 1600)
    /// and not be resized
    fn add_logo(&mut self, logo: PathBuf) {
        match image::open(&logo) {
            Ok(img) => {
                let max_image_width = WIDTH - (HORIZONTAL_MARGIN * 2);
                let img = img.resize(
                    max_image_width as u32,
                    MAX_LOGO_HEIGHT,
                    FilterType::Gaussian,
                );
                let (width, height) = img.dimensions();
                let mut new_image = Vec::new();
                img.write_to(&mut new_image, ImageOutputFormat::Png)
                    .unwrap();
                let b64 = base64::encode(new_image);

                let y_pos = HEIGHT - (BOTTOM_MARGIN + height as i32);
                let x_pos = self.midpoint - (width / 2) as i32;

                let e = format!("<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" href=\"data:image/png;base64,{}\"/>", x_pos, y_pos, width, height, b64);
                self.svg.push_str(&e);
                self.svg.push('\n');
            }
            Err(e) => match bookbinder_common::convert_to_jpg(&logo) {
                Err(_) => eprintln!(
                    "could not render image for titlepage logo ({:?}): {}",
                    logo.display(),
                    e
                ),
                Ok(jpg) => {
                    let p = jpg.temp_file_path(Some("bookbinder"), "jpg");
                    if std::fs::write(&p, jpg).is_ok() {
                        self.add_logo(p)
                    } else {
                        eprintln!("Could not write logo image: {:?}", p);
                    }
                }
            },
        };
    }

    fn move_y(&mut self, i: i32) {
        self.y += i;
    }

    fn finish(self) -> String {
        let mut svg = String::from("<svg width=\"1200\" height=\"1800\" xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 1200 1800\">\n");
        svg.push_str(&self.svg);
        svg.push_str("\n</svg>");
        svg
    }
}

fn generate_titlepage_svg<'a, S, I, P>(
    title: I,
    subtitle: Option<I>,
    contributors: Option<Vec<(Option<&'a str>, Vec<S>)>>,
    logo: Option<P>,
    typeface: Option<&'a str>,
) -> Result<String, ()>
where
    I: IntoIterator<Item = TitleEvent<'a>>,
    S: AsRef<str>,
    PathBuf: From<P>,
{
    let mut writer = SVGWriter::new();

    let font_family = if let Some(typeface) = typeface.as_ref() {
        Family::Name(typeface)
    } else {
        Family::SansSerif
    };

    let bold_font_data = FontData::new(font_family, Weight::BOLD, Style::Normal);
    let normal_font_data = FontData::new(font_family, Weight::NORMAL, Style::Normal);
    let italic_font_data = FontData::new(font_family, Weight::NORMAL, Style::Italic);
    let bold_italic_font_data = FontData::new(font_family, Weight::BOLD, Style::Italic);
    let title_setter = TypeSetter::new(&bold_font_data, HUGE_FONT_SIZE)?;
    let title_emph_setter = TypeSetter::new(&bold_italic_font_data, HUGE_FONT_SIZE)?;
    let subtitle_setter = TypeSetter::new(&normal_font_data, LARGE_FONT_SIZE)?;
    let subtitle_emph_setter = TypeSetter::new(&italic_font_data, LARGE_FONT_SIZE)?;
    let contributor_setter = TypeSetter::new(&normal_font_data, HUGE_FONT_SIZE)?;
    let contributor_ancillary_setter = TypeSetter::new(&normal_font_data, LARGE_FONT_SIZE)?;

    let title_lines = split_text(
        title.into_iter(),
        &bold_font_data,
        &bold_italic_font_data,
        &title_setter,
        &title_emph_setter,
    );
    let subtitle_lines = subtitle.map(|subtitle| {
        split_text(
            subtitle.into_iter(),
            &normal_font_data,
            &italic_font_data,
            &subtitle_setter,
            &subtitle_emph_setter,
        )
    });

    let title_height = title_setter.line_height;
    let subtitle_height = subtitle_setter.line_height;

    let go_back = match title_lines.last() {
        Some(v) => v
            .iter()
            .filter_map(|item| match item {
                SizedTextOrSpace::Text(t) => Some(t.height_above_baseline),
                _ => None,
            })
            .max()
            .unwrap_or(0),
        None => 0,
    };

    for line in title_lines.into_iter() {
        writer.add_line(line);
    }

    writer.move_y(-(title_height - go_back));

    if let Some(subtitle) = subtitle_lines {
        let go_back = match subtitle.last() {
            Some(v) => v
                .iter()
                .filter_map(|item| match item {
                    SizedTextOrSpace::Text(t) => Some(t.height_above_baseline),
                    _ => None,
                })
                .max()
                .unwrap_or(0),
            None => 0,
        };

        writer.move_y(subtitle_height);
        for line in subtitle.into_iter() {
            writer.add_line(line);
        }
        writer.move_y(-(subtitle_height - go_back));
    }

    if let Some(contributors) = contributors {
        writer.add_divider();

        for (role, names) in contributors.iter() {
            if let Some(role) = role {
                let sized_role = SizedText::new(
                    &normal_font_data,
                    &contributor_ancillary_setter,
                    role.to_uppercase(),
                );
                writer.add_text_and_move_down(sized_role);
                writer.move_y(CONTRIBUTOR_INTRO_SPACING);
            }

            match names.len() {
                0 => {}
                1 => {
                    let name = names.first().unwrap().as_ref().to_uppercase();
                    let sized_name = SizedText::new(&normal_font_data, &contributor_setter, &name);
                    writer.add_text_and_move_down(sized_name);
                    writer.move_y(CONTRIBUTOR_GROUP_SPACING);
                }
                _ => {
                    let l = names.len();
                    let mut names = names
                        .iter()
                        .map(|n| n.as_ref().to_uppercase())
                        .enumerate()
                        .map(|(i, mut n)| {
                            if i < l - 2 {
                                n.push(',');
                            }
                            n
                        })
                        .collect::<Vec<String>>();
                    let last = names.pop().unwrap();
                    for name in names.into_iter() {
                        let sized = SizedText::new(&normal_font_data, &contributor_setter, &name);
                        writer.add_text_and_move_down(sized);
                        writer.move_y(CONTRIBUTOR_AND_SPACING);
                    }
                    let and =
                        SizedText::new(&normal_font_data, &contributor_ancillary_setter, "AND");
                    writer.add_text_and_move_down(and);
                    writer.move_y(CONTRIBUTOR_AND_SPACING);
                    let last_name = SizedText::new(&normal_font_data, &contributor_setter, &last);
                    writer.add_text_and_move_down(last_name);
                    writer.move_y(CONTRIBUTOR_GROUP_SPACING);
                }
            }
        }
    }

    if let Some(logo) = logo {
        writer.add_logo(logo.into());
    }

    let svg_with_font = writer.finish();
    bookbinder_common::simplify_svg(&svg_with_font, None).map_err(|_| ())
}

pub(crate) fn generate_svg_titlepage<S>(source: TitlePageSource<'_, S>) -> Result<String, ()>
where
    S: AsRef<str> + std::hash::Hash,
{
    let expected_filepath = source.temp_file_path(Some("bookbinder"), "svg");
    // if expected_filepath.exists() {
    // 	if let Ok(v) = std::fs::read_to_string(&expected_filepath) {
    // 		return Ok(v)
    // 	}
    // }

    let generated = generate_titlepage_svg(
        source.title_events,
        source.subtitle_events,
        source.contributors,
        source.logo,
        source.typeface,
    )?;
    let _ = std::fs::write(&expected_filepath, &generated);
    Ok(generated)
}
