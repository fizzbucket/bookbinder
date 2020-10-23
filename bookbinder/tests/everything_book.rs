use bookbinder::deserialization::{create_epub_from_json, create_pdf_from_json};

static JSON_BOOK: &str = include_str!("everything_book.json");
static EXPECTED_PDF: &[u8] = include_bytes!("test.pdf");

#[test]
fn create_epub() {
    let epub = create_epub_from_json(JSON_BOOK).unwrap();
    // epub files will have extensive use of uuids, especially for ids, so it's hard to do a direct comparison with
    // expected output. Instead we just check validity.
    let mut tmp_path = std::env::temp_dir();
    tmp_path.push("test.epub");
    std::fs::write(&tmp_path, &epub).unwrap();
    bookbinder_common::epubcheck(tmp_path).unwrap();
}

#[test]
fn create_pdf() {
    // we can't compare pdf files directly because of metadata such as creation date;
    // instead we check that the contents of each page are the same.

    let pdf = create_pdf_from_json(JSON_BOOK).unwrap();
    let pdf_document = lopdf::Document::load_mem(&pdf).unwrap();
    let expected_document = lopdf::Document::load_mem(EXPECTED_PDF).unwrap();

    let pdf_pages = pdf_document
        .get_pages()
        .into_iter()
        .map(|(_, id)| pdf_document.get_page_content(id).unwrap());
    let expected_pages = expected_document
        .get_pages()
        .into_iter()
        .map(|(_, id)| expected_document.get_page_content(id).unwrap());
    let pages = pdf_pages.zip(expected_pages);
    for (p1, p2) in pages {
        assert_eq!(p1, p2);
    }
}
