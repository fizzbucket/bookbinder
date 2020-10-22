use bookbinder::deserialization::{create_pdf_from_json, create_epub_from_json};
use std::env;
use std::error::Error;
use std::io::{self, Read, Write};

fn main() -> Result<(), Box<dyn Error>> {
	let mut json = String::new();
    let mut stdin = io::stdin();
    stdin.read_to_string(&mut json)?;

	let output = if env::args()
		.any(|x| x == "-epub")
	{
		create_epub_from_json(&json)
	} else {
		create_pdf_from_json(&json)
	}?;

	io::stdout()
		.write_all(&output)?;
	Ok(())
}