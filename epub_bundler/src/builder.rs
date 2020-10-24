use crate::{Contributor, EpubContent, EpubResource, EpubSource, Identifier, Title, TitleCode};
use bookbinder_common::MimeType;
use epub_metadata::{
    ContributorRole, DublinCoreElement, EpubTitleType, MarcRelator, OnixContributorCode,
    OnixProductIdentifier, OnixTitleCode, ValueMapping,
};
use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::io::{Cursor, Write};
use std::path::Path;
use std::path::PathBuf;
use uuid::Uuid;
use zip::ZipWriter;

static IDENTIFIER_ID: &str = "main_identifier";
static NAV_PATH: &str = "toc.xhtml";
static OPF_PATH: &str = "document.opf";
static COVER_IMAGE_ID: &str = "cover_image";
static CSS_ID: &str = "base_css";
static NAV_ID: &str = "mainnav";

#[derive(Debug)]
pub enum EpubBundlingError {
    Zip(zip::result::ZipError),
    Io(std::io::Error),
    EmptyContainer,
    EmptySpine,
    EmptyManifest,
    NonUnicodeFilePath(PathBuf),
}

impl fmt::Display for EpubBundlingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for EpubBundlingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            EpubBundlingError::Zip(e) => Some(e),
            EpubBundlingError::Io(e) => Some(e),
            _ => None,
        }
    }
}

macro_rules! error_conv {
    ($from:ty, $to:ident) => {
        impl From<$from> for EpubBundlingError {
            fn from(src: $from) -> Self {
                EpubBundlingError::$to(src)
            }
        }
    };
}

error_conv!(zip::result::ZipError, Zip);
error_conv!(std::io::Error, Io);

// metadata based around a dublin core element -- i.e. a dc:[element] tag in an opf file
pub(crate) struct DcMetadata<'a> {
    core: DublinCoreElement,
    id: Option<Cow<'a, str>>,
    value: Option<Cow<'a, str>>,
}

// generic metadata -- i.e. a `meta` tag in an opf file
pub(crate) struct MetaMetadata<'a> {
    dir: Option<&'a str>,
    id: Option<&'a str>,
    property: &'a str,
    refines: Option<Cow<'a, str>>,
    scheme: Option<&'a str>,
    value: Cow<'a, str>,
}

impl<'a> MetaMetadata<'a> {
    fn from_last_modified_date(date: &time::Tm) -> Self {
        let date = date.strftime("%FT%TZ").unwrap().to_string();

        MetaMetadata {
            dir: None,
            id: Some("last_modification"),
            property: "dcterms:modified",
            refines: None,
            scheme: None,
            value: Cow::Owned(date),
        }
    }

    fn from_onix_title_code(code: OnixTitleCode, target: &str) -> Self {
        let id_ref = format!("#{}", target);
        let value = format!("{:?}", code).trim_start_matches('T').to_string();
        MetaMetadata {
            dir: None,
            id: None,
            property: "title-type",
            refines: Some(Cow::Owned(id_ref)),
            scheme: Some("onix:codelist15"),
            value: Cow::Owned(value),
        }
    }

    fn from_epub_title_code(code: EpubTitleType, target: &str) -> Self {
        let id_ref = format!("#{}", target);
        let value = format!("{:?}", code).to_lowercase();

        MetaMetadata {
            dir: None,
            id: None,
            property: "title-type",
            refines: Some(Cow::Owned(id_ref)),
            scheme: None,
            value: Cow::Owned(value),
        }
    }

    fn from_onix_product_id(code: OnixProductIdentifier, target: &str) -> Self {
        let id_ref = format!("#{}", target);
        let value = format!("{:?}", code).to_lowercase();

        MetaMetadata {
            dir: None,
            id: None,
            property: "identifier-type",
            refines: Some(Cow::Owned(id_ref)),
            scheme: Some("onix:codelist5"),
            value: Cow::Owned(value),
        }
    }

    fn from_marc_relator_code(code: MarcRelator, target: &str) -> Self {
        let id_ref = format!("#{}", target);
        let value = format!("{:?}", code).to_lowercase();

        MetaMetadata {
            dir: None,
            id: None,
            property: "role",
            refines: Some(Cow::Owned(id_ref)),
            scheme: Some("marc:relators"),
            value: Cow::Owned(value),
        }
    }

    fn from_onix_contributor_code(code: OnixContributorCode, target: &str) -> Self {
        let id_ref = format!("#{}", target);
        let value = format!("{:?}", code).to_lowercase();

        MetaMetadata {
            dir: None,
            id: None,
            property: "role",
            refines: Some(Cow::Owned(id_ref)),
            scheme: Some("onix:codelist17"),
            value: Cow::Owned(value),
        }
    }
}

// a grouping of a core metadata tag,
// and possibly a set of meta tags refining it
struct MetadataGrouping<'a> {
    base: DcMetadata<'a>,
    refines: Vec<MetaMetadata<'a>>,
}

impl<'a> MetadataGrouping<'a> {
    fn from_title(title: &'a Title, i: usize) -> Self {
        let id = format!("title{}", i);
        let mut refines = Vec::new();
        match title.code {
            TitleCode::Onix(t) => {
                refines.push(MetaMetadata::from_onix_title_code(t, &id));
                if let Some(e) = t.map_code() {
                    refines.push(MetaMetadata::from_epub_title_code(e, &id));
                }
            }
            TitleCode::Unspecified(e) => {
                refines.push(MetaMetadata::from_epub_title_code(e, &id));
            }
        }
        let base = DcMetadata {
            core: DublinCoreElement::Title,
            id: Some(Cow::Owned(id)),
            value: Some(Cow::Borrowed(title.text.as_str())),
        };
        MetadataGrouping { base, refines }
    }

    fn from_identifier(identifier: &'a Identifier) -> Self {
        let base = DcMetadata {
            core: DublinCoreElement::Identifier,
            id: Some(Cow::Borrowed(IDENTIFIER_ID)),
            value: Some(Cow::Borrowed(identifier.text.as_str())),
        };
        let refinement = MetaMetadata::from_onix_product_id(identifier.code, IDENTIFIER_ID);
        MetadataGrouping {
            base,
            refines: vec![refinement],
        }
    }

    fn from_contributor(contributor: &'a Contributor, is_creator: bool, i: usize) -> Self {
        let id = if is_creator {
            format!("creator{}", i)
        } else {
            format!("contributor{}", i)
        };

        let mut refines = Vec::new();

        match contributor.code {
            ContributorRole::Marc(m) => {
                refines.push(MetaMetadata::from_marc_relator_code(m, &id));
            }
            ContributorRole::Onix(o) => {
                refines.push(MetaMetadata::from_onix_contributor_code(o, &id));
                if let Some(m) = o.map_code() {
                    refines.push(MetaMetadata::from_marc_relator_code(m, &id));
                }
            }
        }

        let base = if is_creator {
            DcMetadata {
                core: DublinCoreElement::Creator,
                id: Some(Cow::Owned(id)),
                value: Some(Cow::Borrowed(&contributor.name)),
            }
        } else {
            DcMetadata {
                core: DublinCoreElement::Contributor,
                id: Some(Cow::Owned(id)),
                value: Some(Cow::Borrowed(&contributor.name)),
            }
        };

        MetadataGrouping { base, refines }
    }

    fn from_lang(lang: &'a str) -> Self {
        let base = DcMetadata {
            core: DublinCoreElement::Language,
            id: None,
            value: Some(Cow::Borrowed(lang)),
        };
        MetadataGrouping {
            base,
            refines: Vec::new(),
        }
    }
}

// an itemref in the spine
pub(crate) struct SpineElement<'a> {
    id: Option<&'a str>,
    idref: Cow<'a, str>,
    linear: Option<bool>,
    properties: Vec<&'a str>,
}

// an entry in the manifest
#[derive(Debug)]
pub(crate) struct ManifestItem<'a> {
    href: &'a str,
    id: Cow<'a, str>,
    media_type: &'static str,
    properties: Vec<&'a str>,
}

impl<'a> ManifestItem<'a> {
    fn from_epub_content(src: &'a EpubContent, i: usize) -> Self {
        let id = format!("contents_{}", i);
        let properties = if src.includes_svg {
            vec!["svg"]
        } else {
            Vec::new()
        };
        ManifestItem {
            href: src.output_path.to_str().unwrap(),
            id: Cow::Owned(id),
            media_type: MimeType::Xhtml.to_str(),
            properties,
        }
    }

    fn from_epub_resource(src: &'a EpubResource) -> Self {
        let id = format!("resource_{}", Uuid::new_v4().to_simple());
        ManifestItem {
            href: src.output_path.to_str().unwrap(),
            id: Cow::Owned(id),
            media_type: src.mimetype.to_str(),
            properties: Vec::new(),
        }
    }

    fn from_cover_image(src: &'a EpubResource) -> Self {
        ManifestItem {
            href: src.output_path.to_str().unwrap(),
            id: Cow::Borrowed(COVER_IMAGE_ID),
            media_type: src.mimetype.to_str(),
            properties: vec!["cover-image"],
        }
    }

    fn from_css(src: &'a EpubResource) -> Self {
        ManifestItem {
            href: src.output_path.to_str().unwrap(),
            id: Cow::Borrowed(CSS_ID),
            media_type: src.mimetype.to_str(),
            properties: Vec::new(),
        }
    }
}

// representation of an abstract epub container
struct Container<'a> {
    files: Vec<(&'a Path, &'a [u8])>,
}

impl<'a> Container<'a> {
    fn to_epub(&self) -> Result<Vec<u8>, EpubBundlingError> {
        if self.files.is_empty() {
            return Err(EpubBundlingError::EmptyContainer);
        }
        let buf = Vec::new();
        let w = Cursor::new(buf);
        let mut zipper = ZipWriter::new(w);
        let uncompressed =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let compressed = zip::write::FileOptions::default();

        // first add an uncompressed mimetype file so this can be recognised
        zipper.start_file("mimetype", uncompressed)?;
        zipper.write_all(b"application/epub+zip")?;

        // now add the container file to point reading systems to the files
        zipper.add_directory("META-INF", compressed)?;

        let c = ContainerInfo {
            document_path: OPF_PATH,
        };
        let container_file = c.render();

        zipper.start_file("META-INF/container.xml", compressed)?;
        zipper.write_all(container_file.as_bytes())?;

        for (filepath, contents) in self.files.iter() {
            if let Some(p) = filepath.to_str() {
                zipper.start_file(p, compressed)?;
                zipper.write_all(&contents)?;
            } else {
                return Err(EpubBundlingError::NonUnicodeFilePath(filepath.into()));
            }
        }

        let result = zipper.finish().map(|cursor| cursor.into_inner())?;

        Ok(result)
    }
}

// information for the container file
struct ContainerInfo<'a> {
    document_path: &'a str,
}

impl<'a> ContainerInfo<'a> {
    fn render(&self) -> String {
        let pre = concat!(
            r#"<?xml version="1.0"?>"#,
            "\n",
            r#"<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">"#,
            "\n\t<rootfiles>",
            "\n\t\t<rootfile full-path="
        );

        let post = concat!(
            " media-type=\"application/oebps-package+xml\" />\n",
            "\n\t</rootfiles>",
            "\n</container>"
        );

        let mut out = String::with_capacity(pre.len() + post.len() + self.document_path.len() + 2);
        out.push_str(pre);
        out.push('"');
        out.push_str(&self.document_path);
        out.push('"');
        out.push_str(post);
        out
    }
}

// for autogenerated toc
struct NavInfo<'a> {
    toc_title: &'a str,
    stylesheet: Option<&'a str>,
    entry_list: String,
}

impl<'a> NavInfo<'a> {
    fn render(&self) -> String {
        let mut out = String::from(
            r#"<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">"#,
        );
        out.push_str("\n\t<head>\n");
        out.push_str(&format!("\t\t<title>{}</title>\n", self.toc_title));
        out.push_str("\t\t<meta charset=\"utf-8\"></meta>\n");
        if let Some(ref stylesheet) = self.stylesheet {
            out.push_str(&format!(
                "\t\t<link rel=\"stylesheet\" href=\"{}\" type=\"text/css\"></link>\n",
                stylesheet
            ));
        }
        out.push_str("\t</head>\n");
        out.push_str("\t<body>\n");
        out.push_str("\t\t<nav epub:type=\"toc\">\n");
        out.push_str("\t\t\t<h1>");
        out.push_str(&self.toc_title);
        out.push_str("</h1>\n");
        out.push_str(&self.entry_list);
        out.push_str("\t\t</nav>\n");
        out.push_str("\t</body>\n");
        out.push_str("</html>");
        out
    }
}

pub(crate) trait EpubBundler {
    fn bundle_epub(&self) -> Result<Vec<u8>, EpubBundlingError>;
    fn get_metadata<'a>(&'a self) -> (Vec<DcMetadata<'a>>, Vec<MetaMetadata<'a>>);
    fn get_manifest_items<'a>(&'a self) -> Result<Vec<ManifestItem<'a>>, EpubBundlingError>;
    fn get_spine_items<'a, 'b>(
        manifest: &'a [ManifestItem<'a>],
    ) -> Result<Vec<SpineElement<'b>>, EpubBundlingError>;
    fn get_nav(&self) -> Result<String, EpubBundlingError>;
    fn generate_opf(&self) -> Result<String, EpubBundlingError>;
}

impl EpubBundler for EpubSource {
    fn bundle_epub(&self) -> Result<Vec<u8>, EpubBundlingError> {
        let mut files = Vec::new();
        let opf = self.generate_opf()?;

        let opf_path = Path::new(OPF_PATH);
        let nav = self.get_nav()?;
        let nav_path = Path::new(NAV_PATH);

        files.push((opf_path, opf.as_bytes()));
        files.push((nav_path, nav.as_bytes()));

        for item in self.contents.iter() {
            let path = item.output_path.as_path();
            files.push((path, item.data.as_bytes()));
        }

        for item in self.resources.iter() {
            files.push((&item.output_path, &item.data));
        }

        if let Some(ref ci) = self.cover_image {
            files.push((&ci.output_path, &ci.data));
        }

        if let Some(ref css) = self.css {
            files.push((&css.output_path, &css.data));
        }

        let container = Container { files };

        let epub = container.to_epub()?;
        Ok(epub)
    }

    fn get_metadata<'a>(&'a self) -> (Vec<DcMetadata<'a>>, Vec<MetaMetadata<'a>>) {
        let mut groupings = Vec::new();
        let mut meta = Vec::new();

        if self.title.is_empty() {
            let base = DcMetadata {
                core: DublinCoreElement::Title,
                id: None,
                value: Some(Cow::Borrowed("Untitled")),
            };
            let i = MetadataGrouping {
                base,
                refines: Vec::new(),
            };
            groupings.push(i);
        } else {
            for (i, title) in self.title.iter().enumerate() {
                let grouped = MetadataGrouping::from_title(title, i);
                groupings.push(grouped);
            }
        }
        for (i, creator) in self.creators.iter().enumerate() {
            let grouped = MetadataGrouping::from_contributor(creator, true, i);
            groupings.push(grouped);
        }
        for (i, contributor) in self.contributors.iter().enumerate() {
            let grouped = MetadataGrouping::from_contributor(contributor, false, i);
            groupings.push(grouped);
        }

        match self.identifier {
            Some(ref i) => {
                let grouped = MetadataGrouping::from_identifier(i);
                groupings.push(grouped);
            }
            None => {
                let u = Uuid::new_v4();
                let urn = u.to_urn().to_string();
                let base = DcMetadata {
                    core: DublinCoreElement::Identifier,
                    id: Some(Cow::Borrowed(IDENTIFIER_ID)),
                    value: Some(Cow::Owned(urn)),
                };
                let i = MetadataGrouping {
                    base,
                    refines: vec![MetaMetadata::from_onix_product_id(
                        OnixProductIdentifier::I22,
                        IDENTIFIER_ID,
                    )],
                };
                groupings.push(i);
            }
        }

        match self.lang {
            Some(ref l) => {
                let grouped = MetadataGrouping::from_lang(l);
                groupings.push(grouped);
            }
            None => {
                let base = DcMetadata {
                    core: DublinCoreElement::Language,
                    id: None,
                    value: Some(Cow::Borrowed("en")),
                };
                let i = MetadataGrouping {
                    base,
                    refines: Vec::new(),
                };
                groupings.push(i);
            }
        }

        match self.last_modification {
            Some(ref m) => {
                let tag = MetaMetadata::from_last_modified_date(m);
                meta.push(tag);
            }
            None => {
                let now = time::now_utc();
                let tag = MetaMetadata::from_last_modified_date(&now);
                meta.push(tag);
            }
        }

        let mut dc = Vec::new();
        for mut group in groupings.into_iter() {
            dc.push(group.base);
            meta.append(&mut group.refines);
        }

        (dc, meta)
    }

    fn get_manifest_items<'a>(&'a self) -> Result<Vec<ManifestItem<'a>>, EpubBundlingError> {
        let mut items = Vec::new();
        for (i, item) in self.contents.iter().enumerate() {
            items.push(ManifestItem::from_epub_content(item, i));
        }
        for item in self.resources.iter() {
            items.push(ManifestItem::from_epub_resource(item));
        }
        if let Some(ref item) = self.cover_image {
            items.push(ManifestItem::from_cover_image(item));
        }
        if let Some(ref item) = self.css {
            items.push(ManifestItem::from_css(item));
        }

        if items.is_empty() {
            return Err(EpubBundlingError::EmptyManifest);
        }

        // need to add nav, even though this isn't generated yet
        let nav = ManifestItem {
            href: NAV_PATH,
            id: Cow::Borrowed(NAV_ID),
            media_type: MimeType::Xhtml.to_str(),
            properties: vec!["nav"],
        };
        items.push(nav);
        Ok(items)
    }

    fn get_spine_items<'a, 'b>(
        manifest: &'a [ManifestItem<'a>],
    ) -> Result<Vec<SpineElement<'b>>, EpubBundlingError> {
        let mut items = Vec::new();
        for item in manifest.iter() {
            if item.media_type == "application/xhtml+xml" && item.href != NAV_PATH {
                let s = SpineElement {
                    id: None,
                    idref: Cow::Owned(item.id.to_string()),
                    linear: None,
                    properties: Vec::new(),
                };
                items.push(s);
            }
        }
        if items.is_empty() {
            Err(EpubBundlingError::EmptySpine)
        } else {
            Ok(items)
        }
    }

    fn get_nav(&self) -> Result<String, EpubBundlingError> {
        let tocs = self
            .contents
            .iter()
            .filter_map(|x| x.toc_entry.as_ref().map(|e| (x.output_path.as_path(), e)));

        let mut toc_entries = TEManager::new();
        for (p, e) in tocs {
            toc_entries.add_new_entry(p, e.level, e.title.clone());
        }
        let rendered_toc = toc_entries.render();
        let stylesheet = match self.css {
            Some(ref css) => Some(css.output_path.to_str().unwrap()),
            None => None,
        };

        let n = NavInfo {
            toc_title: "Contents",
            entry_list: rendered_toc,
            stylesheet,
        };
        let rendered = n.render();
        Ok(rendered)
    }

    fn generate_opf(&self) -> Result<String, EpubBundlingError> {
        let (dc_metadata, meta_metadata) = self.get_metadata();
        let manifest_elements = self.get_manifest_items()?;
        let spine_elements = Self::get_spine_items(&manifest_elements)?;
        let mut opf = String::new();
        opf.push_str(r#"<package version="3.0" unique-identifier=""#);
        opf.push_str(IDENTIFIER_ID);
        opf.push_str(r#"" xmlns="http://www.idpf.org/2007/opf" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/">"#);
        opf.push_str("\n  <metadata>\n");
        for item in dc_metadata.iter() {
            opf.push_str("    <");
            opf.push_str(&item.core.as_tagname());
            if let Some(ref id) = item.id {
                opf.push_str(&format!(" id=\"{}\"", id));
            }
            opf.push_str(">");
            if let Some(ref val) = item.value {
                opf.push_str(&val);
            }
            opf.push_str(&format!("</{}>\n", item.core.as_tagname()));
        }
        for item in meta_metadata.iter() {
            opf.push_str("    <meta property=\"");
            opf.push_str(&item.property);
            opf.push_str("\"");
            if let Some(dir) = item.dir {
                opf.push_str(&format!(" dir=\"{}\"", dir));
            }
            if let Some(id) = item.id {
                opf.push_str(&format!(" id=\"{}\"", id));
            }
            if let Some(ref refines) = item.refines {
                opf.push_str(&format!(" refines=\"{}\"", refines));
            }
            if let Some(scheme) = item.scheme {
                opf.push_str(&format!(" scheme=\"{}\"", scheme));
            }
            opf.push_str(">");
            opf.push_str(&item.value);
            opf.push_str("</meta>\n");
        }
        opf.push_str("  </metadata>\n");
        opf.push_str("  <manifest>");
        for item in manifest_elements.iter() {
            opf.push_str(&format!("\n    <item id=\"{}\"", item.id));
            opf.push_str(&format!(" media-type=\"{}\"", item.media_type));
            opf.push_str(&format!(" href=\"{}\"", item.href));
            if !item.properties.is_empty() {
                opf.push_str(" properties=\"");
                for p in item.properties.iter() {
                    opf.push_str(&p);
                    opf.push(' ');
                }
                opf.pop();
                opf.push('"');
            }
            opf.push('>');
            opf.push_str("</item>");
        }
        opf.push_str("\n  </manifest>");
        opf.push_str("\n  <spine>");
        for item in spine_elements.iter() {
            opf.push_str(&format!("<itemref idref=\"{}\"", item.idref));
            if let Some(id) = item.id {
                opf.push_str(&format!(" id=\"{}\"", id));
            }
            match item.linear {
                Some(true) => {
                    opf.push_str(" linear=\"yes\"");
                }
                Some(false) => {
                    opf.push_str(" linear=\"no\"");
                }
                _ => {}
            }
            if !item.properties.is_empty() {
                opf.push_str(" properties=\"");
                for p in item.properties.iter() {
                    opf.push_str(&p);
                    opf.push(' ');
                }
                opf.pop();
                opf.push('"');
            }
            opf.push('>');
            opf.push_str("</itemref>");
        }
        opf.push_str("\n\t</spine>\n");
        opf.push_str("</package>");
        Ok(opf)
    }
}

#[derive(Debug)]
struct TEManager {
    entries: Vec<TE>,
}

impl TEManager {
    fn new() -> Self {
        TEManager {
            entries: Vec::new(),
        }
    }

    fn render(&self) -> String {
        let mut output = String::from("<ol>");
        for t in self.get_top_level_entries().into_iter() {
            output.push('\n');
            output.push_str(&t.render(self));
        }
        output.push_str("\n</ol>");
        output
    }

    fn get_children_from_idx(&self, entry_idx: usize) -> Result<Vec<&TE>, ()> {
        let entry = self.entries.get(entry_idx).ok_or(())?;
        let indices = match entry.get_children_indices() {
            None => return Err(()),
            Some(indices) => indices,
        };
        let children = indices
            .iter()
            .map(|x| self.entries.get(*x).ok_or(()))
            .collect::<Result<Vec<_>, ()>>()?;
        Ok(children)
    }

    fn get_top_level_entries(&self) -> Vec<&TE> {
        self.entries
            .iter()
            .filter(|e| e.get_parent_idx() == None)
            .collect()
    }

    fn add_new_entry<S: ToString, P: AsRef<Path>>(&mut self, href: P, level: usize, text: S) {
        let idx = self.entries.len();
        let parent_idx = self.entries.iter().rposition(|x| x.get_level() < level);
        if let Some(i) = parent_idx {
            let parent = self.entries.get(i).unwrap();
            let new_parent = parent.with_child(idx);
            self.entries[i] = new_parent;
        }
        self.entries.push(TE::Leaf {
            idx,
            level,
            parent: parent_idx,
            href: href.as_ref().to_path_buf(),
            text: text.to_string(),
        });
    }
}

#[derive(Debug, PartialEq)]
enum TE {
    Leaf {
        idx: usize,
        level: usize,
        parent: Option<usize>,
        href: PathBuf,
        text: String,
    },
    Branch {
        idx: usize,
        level: usize,
        parent: Option<usize>,
        href: PathBuf,
        text: String,
        children: Vec<usize>,
    },
}

impl TE {
    fn render(&self, manager: &TEManager) -> String {
        match self {
            TE::Leaf { href, text, .. } => {
                format!("<li>\n<a href=\"{}\">{}</a>\n</li>", href.display(), text)
            }
            TE::Branch {
                href, text, idx, ..
            } => {
                let mut s = format!("<li>\n<a href=\"{}\">{}</a>\n<ol>\n", href.display(), text);

                if let Ok(children) = manager.get_children_from_idx(*idx) {
                    for child in children.into_iter() {
                        s.push('\t');
                        s.push_str(&child.render(manager));
                        s.push('\n');
                    }
                } else {
                    panic!("Branch element without children... {:?}", idx)
                }
                s.push_str("</ol>\n</li>");
                s
            }
        }
    }

    fn with_child(&self, child_idx: usize) -> Self {
        match self {
            TE::Branch {
                idx,
                level,
                parent,
                href,
                text,
                children,
            } => {
                let mut new_children = children.to_vec();
                new_children.push(child_idx);
                TE::Branch {
                    idx: *idx,
                    level: *level,
                    parent: *parent,
                    href: href.clone(),
                    text: text.clone(),
                    children: new_children,
                }
            }
            TE::Leaf {
                idx,
                level,
                parent,
                href,
                text,
            } => TE::Branch {
                idx: *idx,
                level: *level,
                parent: *parent,
                href: href.clone(),
                text: text.clone(),
                children: vec![child_idx],
            },
        }
    }

    fn get_children_indices(&self) -> Option<&[usize]> {
        match self {
            TE::Branch { children, .. } => Some(children),
            TE::Leaf { .. } => None,
        }
    }

    fn get_level(&self) -> usize {
        match self {
            TE::Branch { level, .. } => *level,
            TE::Leaf { level, .. } => *level,
        }
    }

    fn get_parent_idx(&self) -> Option<usize> {
        match self {
            TE::Branch { parent, .. } => *parent,
            TE::Leaf { parent, .. } => *parent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_opf() -> Result<(), Box<dyn Error>> {
        let ci = EpubResource {
            output_path: PathBuf::from("cover.jpg"),
            data: Vec::new(),
            mimetype: MimeType::Jpeg,
        };

        let mut epub = EpubSource::new();
        epub.set_title("The Everything Book")?;
        epub.set_subtitle("With a subtitle")?;
        epub.add_author("Some Guy")?;
        epub.add_author("Another Guy")?;
        epub.add_editor("Some Editor")?;
        epub.add_author_of_foreword("A.N. Academic")?;
        epub.set_isbn("123456789")?;
        epub.set_language("en")?;
        epub.set_cover_image(ci)?;

        let content = EpubContent::new("Hello world");
        epub.add_content(content)?;

        let opf = epub.generate_opf()?;

        // remove date modified line

        let opf = opf
            .split("\n")
            .filter(|line| {
                if line
                    .trim()
                    .starts_with("<meta property=\"dcterms:modified\" id=\"last_modification\"")
                {
                    false
                } else {
                    true
                }
            })
            .map(|l| l.to_string())
            .collect::<Vec<String>>()
            .join("\n");

        let expected = "<package version=\"3.0\" unique-identifier=\"main_identifier\" xmlns=\"http://www.idpf.org/2007/opf\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:dcterms=\"http://purl.org/dc/terms/\">\n  <metadata>\n    <dc:title id=\"title0\">The Everything Book</dc:title>\n    <dc:title id=\"title1\">With a subtitle</dc:title>\n    <dc:creator id=\"creator0\">Some Guy</dc:creator>\n    <dc:creator id=\"creator1\">Another Guy</dc:creator>\n    <dc:contributor id=\"contributor0\">Some Editor</dc:contributor>\n    <dc:contributor id=\"contributor1\">A.N. Academic</dc:contributor>\n    <dc:identifier id=\"main_identifier\">123456789</dc:identifier>\n    <dc:language>en</dc:language>\n    <meta property=\"title-type\" refines=\"#title0\">main</meta>\n    <meta property=\"title-type\" refines=\"#title1\">subtitle</meta>\n    <meta property=\"role\" refines=\"#creator0\" scheme=\"marc:relators\">aut</meta>\n    <meta property=\"role\" refines=\"#creator1\" scheme=\"marc:relators\">aut</meta>\n    <meta property=\"role\" refines=\"#contributor0\" scheme=\"marc:relators\">edt</meta>\n    <meta property=\"role\" refines=\"#contributor1\" scheme=\"onix:codelist17\">a23</meta>\n    <meta property=\"role\" refines=\"#contributor1\" scheme=\"marc:relators\">aui</meta>\n    <meta property=\"identifier-type\" refines=\"#main_identifier\" scheme=\"onix:codelist5\">i15</meta>\n  </metadata>\n  <manifest>\n    <item id=\"contents_0\" media-type=\"application/xhtml+xml\" href=\"2216321107127430384.xhtml\"></item>\n    <item id=\"cover_image\" media-type=\"image/jpeg\" href=\"cover.jpg\" properties=\"cover-image\"></item>\n    <item id=\"mainnav\" media-type=\"application/xhtml+xml\" href=\"toc.xhtml\" properties=\"nav\"></item>\n  </manifest>\n  <spine><itemref idref=\"contents_0\"></itemref>\n\t</spine>\n</package>";
        assert_eq!(opf, expected);

        Ok(())
    }

    #[test]
    fn test_toc_stuff() {
        let mut toc_entries = TEManager::new();
        toc_entries.add_new_entry("a.xhtml", 1, "Part 1"); // idx 0
        toc_entries.add_new_entry("b.xhtml", 2, "Chapter 1"); // idx 1
        toc_entries.add_new_entry("c.xhtml", 1, "Part 2"); // idx 2
        assert_eq!(
            toc_entries
                .get_top_level_entries()
                .into_iter()
                .map(|e| match e {
                    TE::Branch { text, .. } => text,
                    TE::Leaf { text, .. } => text,
                })
                .collect::<Vec<_>>(),
            vec!["Part 1", "Part 2"]
        );

        let part_1 = toc_entries.entries.get(0).unwrap();
        assert_eq!(
            part_1,
            &TE::Branch {
                idx: 0,
                level: 1,
                parent: None,
                href: PathBuf::from("a.xhtml"),
                text: "Part 1".to_string(),
                children: vec![1]
            }
        );

        let c = toc_entries.entries.get(0).unwrap().get_children_indices();
        assert_eq!(*c.unwrap().first().unwrap(), 1 as usize);
    }
}
