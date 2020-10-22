use bookbinder::deserialization::{create_epub_from_json, create_pdf_from_json, create_ast_from_json};

static JSON_BOOK: &str = include_str!("everything_book.json");
static PAMELA: &str = include_str!("pamela.json");

#[test]
fn create_epub() {
	let epub = create_epub_from_json(JSON_BOOK).unwrap();
	std::fs::write("test.epub", &epub).unwrap();
}

#[test]
fn create_pdf() {
	let pdf = create_pdf_from_json(JSON_BOOK).unwrap();
	std::fs::write("test.pdf", &pdf).unwrap();
}

#[test]
fn create_long_epub() {
	let epub = create_epub_from_json(PAMELA).unwrap();
	std::fs::write("pamela.epub", &epub).unwrap();
}

#[test]
fn create_long_pdf() {
	let pdf = create_pdf_from_json(PAMELA).unwrap();
	std::fs::write("pamela.pdf", &pdf).unwrap();
}