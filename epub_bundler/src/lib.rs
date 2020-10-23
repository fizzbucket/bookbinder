#![deny(dead_code)]
#![deny(unreachable_patterns)]
#![deny(unused_extern_crates)]
#![deny(unused_imports)]
#![deny(unused_qualifications)]
#![deny(clippy::all)]
#![deny(missing_debug_implementations)]
#![deny(variant_size_differences)]

use epub_metadata::{
    ContributorRole, EpubTitleType, MarcRelator, OnixContributorCode, OnixProductIdentifier,
    OnixTitleCode,
};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use temp_file_name::{HashToString, TempFilePath};
mod builder;
use bookbinder_common::{GuessMimeType, MimeType, MimeTypeHelper};
use builder::EpubBundler;
pub use builder::EpubBundlingError;
use regex::Regex;

/// A resource of some kind (i.e. something other than textual content,
/// such as an image, css, fonts, etc
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct EpubResource {
    pub output_path: PathBuf,
    pub data: Vec<u8>,
    pub mimetype: MimeType,
}

impl EpubResource {
    pub fn from_file<P: AsRef<Path>>(p: P) -> Result<Self, String> {
        let p = p.as_ref();
        if p.is_epub_supported_resource() {
            let op = p
                .file_name()
                .ok_or(format!("No file name: {}", p.display()))?;
            let output_path = PathBuf::from(op);
            let data =
                std::fs::read(p).map_err(|e| format!("{}: [{}]", e.to_string(), p.display()))?;
            Ok(EpubResource {
                output_path,
                data,
                mimetype: p.guess_mime().unwrap(),
            })
        } else {
            Err(format!(
                "Invalid mimetype for {}: {:?}",
                p.display(),
                p.guess_mime()
            ))
        }
    }

    pub fn new_jpg(data: Vec<u8>) -> Self {
        EpubResource {
            mimetype: MimeType::Jpeg,
            output_path: PathBuf::from(data.temp_filename("jpg")),
            data,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct TocEntry {
    /// the relative level of this entry, where lower is higher.
    pub level: usize,
    /// the text to display in the toc entry
    pub title: String,
}

/// A single piece of textual content;
#[derive(Debug)]
pub struct EpubContent {
    /// An xhtml string
    pub data: String,
    /// the path to write the content to within the epub
    pub output_path: PathBuf,
    /// how to display in table of contents
    pub toc_entry: Option<TocEntry>,
    /// whether this content has an embedded svg image.
    pub includes_svg: bool,
}

fn replace_links(text: &str) -> Cow<'_, str> {
    let href_regex = Regex::new(r"<a.*?>(?P<link_text>.+?)</a>").unwrap();
    href_regex.replace_all(text.as_ref(), r"$link_text")
}

impl EpubContent {
    pub fn new<S: ToString>(xhtml: S) -> Self {
        let data = xhtml.to_string();
        let output_path = PathBuf::from(data.temp_filename("xhtml"));
        EpubContent {
            data,
            output_path,
            toc_entry: None,
            includes_svg: false,
        }
    }

    pub fn does_include_svg(&mut self) -> &mut Self {
        self.includes_svg = true;
        self
    }

    /// Display this content in the table of contents with heading `title`
    pub fn set_toc_title<S: AsRef<str>>(
        &mut self,
        title: S,
        header_level: usize,
    ) -> Result<&mut Self, &'static str> {
        match self.toc_entry {
            Some(_) => Err("Toc entry already exists"),
            None => {
                // title cannot contain any <a> elements,
                // since it will itself be a link.
                let title = replace_links(title.as_ref());
                let toc = TocEntry {
                    level: header_level,
                    title: title.into(),
                };
                self.toc_entry = Some(toc);
                Ok(self)
            }
        }
    }
}

/// The source from which an epub is built
#[derive(Default, Debug)]
pub struct EpubSource {
    resources: Vec<EpubResource>,
    contents: Vec<EpubContent>,
    cover_image: Option<EpubResource>,
    css: Option<EpubResource>,
    title: Vec<Title>,
    identifier: Option<Identifier>,
    lang: Option<String>,
    creators: Vec<Contributor>,
    contributors: Vec<Contributor>,
    last_modification: Option<time::Tm>,
}

#[derive(Debug)]
enum TitleCode {
    Onix(OnixTitleCode),
    Unspecified(EpubTitleType),
}

#[derive(Debug)]
struct Title {
    code: TitleCode,
    text: String,
}

#[derive(Debug)]
struct Identifier {
    code: OnixProductIdentifier,
    text: String,
}

#[derive(Debug)]
struct Contributor {
    code: ContributorRole,
    name: String,
}

macro_rules! add_marc_contributor {
    ($fn_name:ident, $role:expr) => {
        pub fn $fn_name<S: ToString>(&mut self, contributor: S) -> Result<&mut Self, &'static str> {
            self.add_marc_contributor(contributor, $role)
        }
    };
    ($fn_doc:meta, $fn_name:ident, $role:expr) => {
        #[$fn_doc]
        pub fn $fn_name<S: ToString>(&mut self, contributor: S) -> Result<&mut Self, &'static str> {
            self.add_marc_contributor(contributor, $role)
        }
    };
}

macro_rules! add_onix_contributor {
    ($fn_name:ident, $role:expr) => {
        pub fn $fn_name<S: ToString>(&mut self, contributor: S) -> Result<&mut Self, &'static str> {
            self.add_onix_contributor(contributor, $role)
        }
    };
    ($fn_doc:meta, $fn_name:ident, $role:expr) => {
        #[$fn_doc]
        pub fn $fn_name<S: ToString>(&mut self, contributor: S) -> Result<&mut Self, &'static str> {
            self.add_onix_contributor(contributor, $role)
        }
    };
}

impl EpubSource {
    pub fn new() -> Self {
        EpubSource::default()
    }

    /// Set the main title
    pub fn set_title<S: ToString>(&mut self, title: S) -> Result<&mut Self, &'static str> {
        self.title
            .iter()
            .filter_map(|t| match t.code {
                TitleCode::Unspecified(EpubTitleType::Main) => Some(Err("Main title already set")),
                TitleCode::Onix(OnixTitleCode::T01) => Some(Err("Main title already set")),
                _ => None,
            })
            .collect::<Result<(), &'static str>>()?;

        let t = Title {
            code: TitleCode::Unspecified(EpubTitleType::Main),
            text: title.to_string(),
        };
        self.title.push(t);
        Ok(self)
    }

    /// Set a subtitle
    pub fn set_subtitle<S: ToString>(&mut self, subtitle: S) -> Result<&mut Self, &'static str> {
        self.title
            .iter()
            .filter_map(|t| match t.code {
                TitleCode::Unspecified(EpubTitleType::Subtitle) => {
                    Some(Err("Subtitle already set"))
                }
                _ => None,
            })
            .collect::<Result<(), &'static str>>()?;
        let t = Title {
            code: TitleCode::Unspecified(EpubTitleType::Subtitle),
            text: subtitle.to_string(),
        };
        self.title.push(t);
        Ok(self)
    }

    /// Set an onix title
    pub fn set_onix_title<S: ToString>(
        &mut self,
        title: S,
        kind: OnixTitleCode,
    ) -> Result<&mut Self, &'static str> {
        let t = Title {
            code: TitleCode::Onix(kind),
            text: title.to_string(),
        };
        self.title.push(t);
        Ok(self)
    }

    pub fn set_epub_title<S: ToString>(
        &mut self,
        title: S,
        kind: EpubTitleType,
    ) -> Result<&mut Self, &'static str> {
        let t = Title {
            code: TitleCode::Unspecified(kind),
            text: title.to_string(),
        };
        self.title.push(t);
        Ok(self)
    }

    /// Set an isbn as the identifier
    pub fn set_isbn<S: ToString>(&mut self, isbn: S) -> Result<&mut Self, &'static str> {
        let identifier = Identifier {
            code: OnixProductIdentifier::I15,
            text: isbn.to_string(),
        };
        self.identifier = Some(identifier);
        Ok(self)
    }

    /// set an urn as the identifer
    pub fn set_urn<S: ToString>(&mut self, urn: S) -> Result<&mut Self, &'static str> {
        let urn = urn.to_string();
        let identifier = Identifier {
            code: OnixProductIdentifier::I22,
            text: urn,
        };
        self.identifier = Some(identifier);
        Ok(self)
    }

    /// slightly modified from git version of langtags, which we can't use on Cargo
    fn parse_lang_tag(lang: String) -> Result<String, ()> {
        #[derive(PartialEq, Eq)]
        enum State {
            Start,
            AfterLanguage,
            AfterExtLang,
            AfterScript,
            AfterRegion,
            InExtension { expected: bool },
            InPrivateUse { expected: bool },
        }

        let mut serialization = String::with_capacity(lang.len());

        let mut state = State::Start;
        let mut extlangs_count = 0;
        for (subtag, _) in SubTagIterator::new(&lang) {
            if subtag.is_empty() {
                // All subtags have a maximum length of eight characters.
                return Err(());
            }
            if subtag.len() > 8 {
                // All subtags have a maximum length of eight characters.
                return Err(());
            }
            if state == State::Start {
                // Primary language
                if subtag.len() < 2 || !is_alphabetic(subtag) {
                    return Err(());
                }
                serialization.extend(to_lowercase(subtag));
                if subtag.len() < 4 {
                    // extlangs are only allowed for short language tags
                    state = State::AfterLanguage;
                } else {
                    state = State::AfterExtLang;
                }
            } else if let State::InPrivateUse { .. } = state {
                if !is_alphanumeric(subtag) {
                    return Err(());
                }
                serialization.push('-');
                serialization.extend(to_lowercase(subtag));
                state = State::InPrivateUse { expected: false };
            } else if subtag == "x" || subtag == "X" {
                // We make sure extension is found
                if let State::InExtension { expected: true } = state {
                    return Err(());
                }
                serialization.push('-');
                serialization.push('x');
                state = State::InPrivateUse { expected: true };
            } else if subtag.len() == 1 && is_alphanumeric(subtag) {
                // We make sure extension is found
                if let State::InExtension { expected: true } = state {
                    return Err(());
                }
                let extension_tag = subtag.chars().next().unwrap().to_ascii_lowercase();
                serialization.push('-');
                serialization.push(extension_tag);
                state = State::InExtension { expected: true };
            } else if let State::InExtension { .. } = state {
                if !is_alphanumeric(subtag) {
                    return Err(());
                }
                serialization.push('-');
                serialization.extend(to_lowercase(subtag));
                state = State::InExtension { expected: false };
            } else if state == State::AfterLanguage && subtag.len() == 3 && is_alphabetic(subtag) {
                extlangs_count += 1;
                if extlangs_count > 3 {
                    return Err(());
                }
                // valid extlangs
                serialization.push('-');
                serialization.extend(to_lowercase(subtag));
            } else if (state == State::AfterLanguage || state == State::AfterExtLang)
                && subtag.len() == 4
                && is_alphabetic(subtag)
            {
                // Script
                serialization.push('-');
                serialization.extend(to_uppercase_first(subtag));
                state = State::AfterScript;
            } else if (state == State::AfterLanguage
                || state == State::AfterExtLang
                || state == State::AfterScript)
                && (subtag.len() == 2 && is_alphabetic(subtag)
                    || subtag.len() == 3 && is_numeric(subtag))
            {
                // Region
                serialization.push('-');
                serialization.extend(to_uppercase(subtag));
                state = State::AfterRegion;
            } else if (state == State::AfterLanguage
                || state == State::AfterExtLang
                || state == State::AfterScript
                || state == State::AfterRegion)
                && is_alphanumeric(subtag)
                && (subtag.len() >= 5 && is_alphabetic(&subtag[0..1])
                    || subtag.len() >= 4 && is_numeric(&subtag[0..1]))
            {
                // Variant
                serialization.push('-');
                serialization.extend(to_lowercase(subtag));
                state = State::AfterRegion;
            } else {
                return Err(());
            }
        }

        //We make sure we are in a correct final state
        if let State::InExtension { expected: true } = state {
            return Err(());
        }
        if let State::InPrivateUse { expected: true } = state {
            return Err(());
        }
        Ok(serialization)
    }

    /// set the language of the epub
    pub fn set_language<S: ToString>(&mut self, lang: S) -> Result<&mut Self, &'static str> {
        let l = lang.to_string();
        match Self::parse_lang_tag(l) {
            Ok(val) => {
                self.lang = Some(val);
                Ok(self)
            }
            Err(_) => Err("Invalid language"),
        }
    }

    add_marc_contributor!(doc = "Add an author", add_author, MarcRelator::Aut);
    add_marc_contributor!(add_editor, MarcRelator::Edt);
    add_marc_contributor!(add_translator, MarcRelator::Trl);
    add_onix_contributor!(add_author_of_foreword, OnixContributorCode::A23);
    add_onix_contributor!(add_author_of_introduction, OnixContributorCode::A23);
    add_onix_contributor!(add_author_of_afterword, OnixContributorCode::A19);
    add_onix_contributor!(
        add_author_of_introduction_and_notes,
        OnixContributorCode::A29
    );

    /// add a contributor with an onix code
    pub fn add_onix_contributor<S: ToString>(
        &mut self,
        name: S,
        role: OnixContributorCode,
    ) -> Result<&mut Self, &'static str> {
        let contributor = Contributor {
            code: ContributorRole::Onix(role),
            name: name.to_string(),
        };
        if role == OnixContributorCode::A01 {
            self.creators.push(contributor);
        } else {
            self.contributors.push(contributor);
        }

        Ok(self)
    }

    /// add a contributor with a marc code
    pub fn add_marc_contributor<S: ToString>(
        &mut self,
        name: S,
        role: MarcRelator,
    ) -> Result<&mut Self, &'static str> {
        let contributor = Contributor {
            code: ContributorRole::Marc(role),
            name: name.to_string(),
        };

        if role == MarcRelator::Aut {
            self.creators.push(contributor);
        } else {
            self.contributors.push(contributor);
        }
        Ok(self)
    }

    /// set the modification date
    pub fn set_modification_date(&mut self, d: time::Tm) -> Result<&mut Self, &'static str> {
        self.last_modification = Some(d);
        Ok(self)
    }

    /// Add a resource
    pub fn add_resource(&mut self, r: EpubResource) -> Result<&mut Self, &'static str> {
        self.resources.push(r);
        Ok(self)
    }

    /// Add a resource from a filepath
    pub fn add_resource_from_file(&mut self, filename: PathBuf) -> Result<&mut Self, String> {
        let r = EpubResource::from_file(&filename)?;
        self.add_resource(r).map_err(|e| e.to_string())
    }

    /// Add a content document
    pub fn add_content(&mut self, r: EpubContent) -> Result<&mut Self, &'static str> {
        self.contents.push(r);
        Ok(self)
    }

    /// Set the base css of the epub
    pub fn set_css(&mut self, css: EpubResource) -> Result<&mut Self, &'static str> {
        if css.mimetype.is_css() {
            self.css = Some(css);
            Ok(self)
        } else {
            Err("Not css")
        }
    }

    /// Set the base css of the epub from a file
    pub fn set_css_from_file(&mut self, path: &Path) -> Result<&mut Self, String> {
        let resource = EpubResource::from_file(&path)?;
        self.set_css(resource).map_err(|e| e.to_string())
    }

    /// Set the epub cover image
    pub fn set_cover_image(&mut self, image: EpubResource) -> Result<&mut Self, String> {
        if image.mimetype.is_jpg() || image.mimetype.is_png() {
            self.cover_image = Some(image);
            Ok(self)
        } else if image.mimetype.is_svg() {
            let d = String::from_utf8(image.data)
                .map_err(|e| format!("Error converting svg bytes to string: {}", e))?;
            let data = bookbinder_common::convert_svg_to_jpg(&d, Some(300))
                .map_err(|e| format!("Error converting cover image to jpg: {:?}", e))?;
            let resource = EpubResource {
                output_path: PathBuf::from(format!("{}.jpg", d.hash_to_string())),
                data,
                mimetype: MimeType::Jpeg,
            };
            self.cover_image = Some(resource);
            Ok(self)
        } else {
            Err("Invalid mimetype for cover image".to_string())
        }
    }

    /// Set the epub cover image from a file; this should be a jpg file if possible,
    /// but an svg file will be converted if required.
    pub fn set_cover_image_from_file(&mut self, path: PathBuf) -> Result<&mut Self, String> {
        // convert from svg if required

        let resource = EpubResource::from_file(&path)?;
        self.set_cover_image(resource)
    }

    pub fn bundle(&mut self) -> Result<Vec<u8>, EpubBundlingError> {
        self.bundle_epub()
    }
}

struct SubTagIterator<'a> {
    split: std::str::Split<'a, char>,
    position: usize,
}

impl<'a> SubTagIterator<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            split: input.split('-'),
            position: 0,
        }
    }
}

impl<'a> Iterator for SubTagIterator<'a> {
    type Item = (&'a str, usize);

    fn next(&mut self) -> Option<(&'a str, usize)> {
        let tag = self.split.next()?;
        let tag_end = self.position + tag.len();
        self.position = tag_end + 1;
        Some((tag, tag_end))
    }
}

fn is_alphabetic(s: &str) -> bool {
    s.chars().all(|x| x.is_ascii_alphabetic())
}

fn is_numeric(s: &str) -> bool {
    s.chars().all(|x| x.is_ascii_digit())
}

fn is_alphanumeric(s: &str) -> bool {
    s.chars().all(|x| x.is_ascii_alphanumeric())
}

fn to_uppercase<'a>(s: &'a str) -> impl Iterator<Item = char> + 'a {
    s.chars().map(|c| c.to_ascii_uppercase())
}

// Beware: panics if s.len() == 0 (should never happen in our code)
fn to_uppercase_first<'a>(s: &'a str) -> impl Iterator<Item = char> + 'a {
    let mut chars = s.chars();
    std::iter::once(chars.next().unwrap().to_ascii_uppercase()).chain(chars.map(|c| c.to_ascii_lowercase()))
}

fn to_lowercase<'a>(s: &'a str) -> impl Iterator<Item = char> + 'a {
    s.chars().map(|c| c.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_links() {
        let text = "<a>Link A</a> <a id=\"i\">Link B</a> <abbrev>A</abbrev>";
        assert_eq!(replace_links(text), "Link A Link B <abbrev>A</abbrev>");
        let no_links = "Hello World";
        assert_eq!(replace_links(no_links), Cow::Borrowed(no_links));
    }
}
