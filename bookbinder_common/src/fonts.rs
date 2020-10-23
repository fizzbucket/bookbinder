//! Bundle some default fonts and make them accessible, as well as standardising access to system fonts.
use fontdb::{Database, Family, Query};
use lazy_static::lazy_static;
use std::path::{Path, PathBuf};
use usvg::SystemFontDB;

lazy_static! {
    /// a minimal font db containing default sans and serif fonts
    /// which are guaranteed to be present
    pub static ref LIMITED_FONT_DB: Database = {
        let mut db = Database::new();
        db.load_font_data(DEFAULT_SANS_BOLD.to_vec());
        db.load_font_data(DEFAULT_SANS_BOLD.to_vec());
        db.load_font_data(DEFAULT_SANS_BOLD_ITALIC.to_vec());
        db.load_font_data(DEFAULT_SANS_ITALIC.to_vec());
        db.load_font_data(DEFAULT_SANS.to_vec());
        db.load_font_data(DEFAULT_SANS_SEMIBOLD.to_vec());
        db.load_font_data(DEFAULT_SANS_SEMIBOLD_ITALIC.to_vec());
        db.load_font_data(DEFAULT_SERIF_BOLD.to_vec());
        db.load_font_data(DEFAULT_SERIF_BOLD_ITALIC.to_vec());
        db.load_font_data(DEFAULT_SERIF_ITALIC.to_vec());
        db.load_font_data(DEFAULT_SERIF.to_vec());

        db.set_sans_serif_family(DEFAULT_SANS_FAMILY_NAME);
        db.set_serif_family(DEFAULT_SERIF_FAMILY_NAME);

        db
    };

    /// a full font db containing both default and system fonts
    pub static ref FULL_FONT_DB: Database = {
        let mut db = LIMITED_FONT_DB.clone();
        load_system_fonts(&mut db);
        db
    };

    /// a font db compatible with the older version of `fontdb` used by usvg
    pub static ref USVG_FONT_DB: usvg::fontdb::Database = {
        let mut db = usvg::fontdb::Database::new();
        db.load_font_data(DEFAULT_SANS_BOLD.to_vec());
        db.load_font_data(DEFAULT_SANS_BOLD.to_vec());
        db.load_font_data(DEFAULT_SANS_BOLD_ITALIC.to_vec());
        db.load_font_data(DEFAULT_SANS_ITALIC.to_vec());
        db.load_font_data(DEFAULT_SANS.to_vec());
        db.load_font_data(DEFAULT_SANS_SEMIBOLD.to_vec());
        db.load_font_data(DEFAULT_SANS_SEMIBOLD_ITALIC.to_vec());
        db.load_font_data(DEFAULT_SERIF_BOLD.to_vec());
        db.load_font_data(DEFAULT_SERIF_BOLD_ITALIC.to_vec());
        db.load_font_data(DEFAULT_SERIF_ITALIC.to_vec());
        db.load_font_data(DEFAULT_SERIF.to_vec());
        db.load_system_fonts();

        db.set_sans_serif_family(DEFAULT_SANS_FAMILY_NAME);
        db.set_serif_family(DEFAULT_SERIF_FAMILY_NAME);

        db
    };

    /// default temporary font directory
    static ref FONT_DIR: PathBuf = {
        std::env::temp_dir()
            .join("bookbinder")
            .join("fonts")
    };

    /// Paths to default sans fonts
    pub static ref SANS_FONT_PATHS: FontInfo = {
        let fi = FontInfo {
            base_filepath: FONT_DIR.to_path_buf(),
            filename: DEFAULT_SANS_FILENAME,
            bold: DEFAULT_SANS_BOLD_FILENAME,
            italic: DEFAULT_SANS_ITALIC_FILENAME,
            bolditalic: DEFAULT_SANS_BOLD_ITALIC_FILENAME,
            bold_data: DEFAULT_SANS_BOLD,
            italic_data: DEFAULT_SANS_ITALIC,
            bolditalic_data: DEFAULT_SANS_BOLD_ITALIC,
            regular_data: DEFAULT_SANS,
        };

        fi.write()
            .expect("Error writing default fonts to temporary directory");
        fi

    };

    /// Paths to default serif fonts
    pub static ref SERIF_FONT_PATHS: FontInfo = {
        let fi = FontInfo {
            base_filepath: FONT_DIR.to_path_buf(),
            filename: DEFAULT_SERIF_FILENAME,
            bold: DEFAULT_SERIF_BOLD_FILENAME,
            italic: DEFAULT_SERIF_ITALIC_FILENAME,
            bolditalic: DEFAULT_SERIF_BOLD_ITALIC_FILENAME,
            bold_data: DEFAULT_SERIF_BOLD,
            italic_data: DEFAULT_SERIF_ITALIC,
            bolditalic_data: DEFAULT_SERIF_BOLD_ITALIC,
            regular_data: DEFAULT_SERIF,
        };

        fi.write()
            .expect("Error writing default fonts to temporary directory");
        fi
    };

}

#[cfg(all(unix, not(target_os = "macos")))]
fn load_system_fonts(db: &mut Database) {
    db.load_fonts_dir("/usr/share/fonts/");
    db.load_fonts_dir("/usr/local/share/fonts/");

    if let Ok(ref home) = std::env::var("HOME") {
        let path = std::path::Path::new(home).join(".local/share/fonts");
        db.load_fonts_dir(path);
    }
}

#[cfg(target_os = "windows")]
fn load_system_fonts(db: &mut Database) {
    db.load_fonts_dir("C:\\Windows\\Fonts\\");
}

#[cfg(target_os = "macos")]
fn load_system_fonts(db: &mut Database) {
    db.load_fonts_dir("/Library/Fonts");
    db.load_fonts_dir("/System/Library/Fonts");

    if let Ok(ref home) = std::env::var("HOME") {
        let path = std::path::Path::new(home).join("Library/Fonts");
        db.load_fonts_dir(path);
    }
}

#[derive(Debug)]
/// Bundled information about a font family,
/// containing references to file paths and to font data
pub struct FontInfo {
    /// the base directory where this family's files can be found
    base_filepath: PathBuf,
    /// the regular font filename
    filename: &'static str,
    /// the bold font filename
    bold: &'static str,
    /// the italic font filename
    italic: &'static str,
    /// the bold italic font filename
    bolditalic: &'static str,
    /// the regular font data
    regular_data: &'static [u8],
    /// the bold font data
    bold_data: &'static [u8],
    /// the italic font data
    italic_data: &'static [u8],
    /// the bold italic font data
    bolditalic_data: &'static [u8],
}

macro_rules! getter {
    ($fn_name:ident, $filename:ident, $data:ident, $doc:meta) => {
        #[$doc]
        pub fn $fn_name(&self) -> Result<&'static str, std::io::Error> {
            self.write_path_if_not_exists(self.$filename, self.$data)?;
            Ok(self.$filename)
        }
    };
}

impl FontInfo {
    /// Get the base directory of this font family; that is, the directory
    /// within which each filename is a file.
    /// E.g. if the result of `get_base_file_path` is `~/Fonts`, and of
    /// `get_bold` is `bold.otf`, then the bold font can be found at `~/Fonts/bold.otf`.
    pub fn get_base_filepath(&self) -> Result<&Path, std::io::Error> {
        std::fs::create_dir_all(&self.base_filepath)?;
        Ok(&self.base_filepath)
    }

    getter!(
        get_filename,
        filename,
        regular_data,
        doc = "Filename of the regular font"
    );
    getter!(get_bold, bold, bold_data, doc = "Filename of the bold font");
    getter!(
        get_italic,
        italic,
        italic_data,
        doc = "Filename of the italic font"
    );
    getter!(
        get_bolditalic,
        bolditalic,
        bolditalic_data,
        doc = "Filename of the bold italic font"
    );

    fn write_path_if_not_exists(&self, filename: &str, data: &[u8]) -> Result<(), std::io::Error> {
        let p = self.base_filepath.join(filename);
        if !p.exists() {
            std::fs::write(&p, data)?;
        }
        Ok(())
    }

    fn write(&self) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(&self.base_filepath)?;
        self.write_path_if_not_exists(self.filename, self.regular_data)?;
        self.write_path_if_not_exists(self.bold, self.bold_data)?;
        self.write_path_if_not_exists(self.italic, self.italic_data)?;
        self.write_path_if_not_exists(self.bolditalic, self.bolditalic_data)?;
        Ok(())
    }
}

/// return whether this font exists on the system
pub fn font_exists(name: &str) -> bool {
    let q = Query {
        families: &[Family::Name(name)],
        ..Default::default()
    };
    FULL_FONT_DB.query(&q).is_some()
}

/// the family name of the default sans typeface
pub static DEFAULT_SANS_FAMILY_NAME: &str = "Open Sans";
/// the family name of the default serif typeface
pub static DEFAULT_SERIF_FAMILY_NAME: &str = "Source Serif Pro";

static DEFAULT_SANS_BOLD_FILENAME: &str = "OpenSans-Bold.ttf";
static DEFAULT_SANS_BOLD_ITALIC_FILENAME: &str = "OpenSans-BoldItalic.ttf";
static DEFAULT_SANS_ITALIC_FILENAME: &str = "OpenSans-SemiBoldItalic.ttf";
static DEFAULT_SANS_FILENAME: &str = "OpenSans-SemiBold.ttf";

/// font data for a default bold sans
pub static DEFAULT_SANS_BOLD: &[u8] = include_bytes!("fonts/OpenSans-Bold.ttf");
/// font data for a default bold italic sans
pub static DEFAULT_SANS_BOLD_ITALIC: &[u8] = include_bytes!("fonts/OpenSans-BoldItalic.ttf");
/// font data for a default semibold italic sans
pub static DEFAULT_SANS_SEMIBOLD_ITALIC: &[u8] =
    include_bytes!("fonts/OpenSans-SemiBoldItalic.ttf");
/// font data for a default semibold sans
pub static DEFAULT_SANS_SEMIBOLD: &[u8] = include_bytes!("fonts/OpenSans-SemiBold.ttf");
/// font data for a default italic sans
pub static DEFAULT_SANS_ITALIC: &[u8] = include_bytes!("fonts/OpenSans-Italic.ttf");
/// font data for a default sans
pub static DEFAULT_SANS: &[u8] = include_bytes!("fonts/OpenSans-Regular.ttf");

static DEFAULT_SERIF_BOLD_FILENAME: &str = "SourceSerifPro-Bold.otf";
static DEFAULT_SERIF_BOLD_ITALIC_FILENAME: &str = "SourceSerifPro-BoldIt.otf";
static DEFAULT_SERIF_ITALIC_FILENAME: &str = "SourceSerifPro-It.otf";
static DEFAULT_SERIF_FILENAME: &str = "SourceSerifPro-Regular.otf";

/// font data for a default bold serif
pub static DEFAULT_SERIF_BOLD: &[u8] = include_bytes!("fonts/SourceSerifPro-Bold.otf");
/// font data for a default bold italic serif
pub static DEFAULT_SERIF_BOLD_ITALIC: &[u8] = include_bytes!("fonts/SourceSerifPro-BoldIt.otf");
/// font data for a default italic serif
pub static DEFAULT_SERIF_ITALIC: &[u8] = include_bytes!("fonts/SourceSerifPro-It.otf");
/// font data for a default serif
pub static DEFAULT_SERIF: &[u8] = include_bytes!("fonts/SourceSerifPro-Regular.otf");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_exists() {
        assert!(!(font_exists("font which doesn't exist")));
        assert!(font_exists(DEFAULT_SANS_FAMILY_NAME));
        assert!(font_exists(DEFAULT_SERIF_FAMILY_NAME));
    }
}
