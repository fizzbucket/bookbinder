use std::borrow::Cow;
use crate::{TitlePageContributorRole, ContributorSource};

/// The metadata of a particular book
#[derive(Debug, Default, Clone)]
pub struct Metadata<'a> {
	do_not_assert_moral_rights: bool,
	is_not_first_publication: bool,
	/// The main title of this work
	pub title: Cow<'a, str>,
	pub(crate) shorttitle: Option<Cow<'a, str>>,
	/// A subtitle
	pub subtitle: Option<Cow<'a, str>>,
	/// The authors of this work
	pub authors: Vec<Cow<'a, str>>,
	/// The editors of this work
	pub editors: Vec<Cow<'a, str>>,
	/// The translators of this work
	pub translators: Vec<Cow<'a, str>>,
	/// Authors of forewords to this work
	pub foreword_authors: Vec<Cow<'a, str>>,
	/// Authors of introductions to this work
	pub introduction_authors: Vec<Cow<'a, str>>,
	/// Authors of afterword to this work
	pub afterword_authors: Vec<Cow<'a, str>>,
	/// Authors of introductions to and in-text notes or commentary on this work 
	pub introduction_and_notes_authors: Vec<Cow<'a, str>>,
	cover_designer: Option<Cow<'a, str>>,
	author_photo_copyright_holder: Option<Cow<'a, str>>,
	paperback_isbn: Option<Cow<'a, str>>,
	hardback_isbn: Option<Cow<'a, str>>,
	/// The isbn of this work as an epub
	pub epub_isbn: Option<Cow<'a, str>>,
	publisher_name: Option<Cow<'a, str>>,
	publisher_address: Option<Cow<'a, str>>,
	publisher_url: Option<Cow<'a, str>>,
	print_loc: Option<Cow<'a, str>>,
	copyright_statement: Option<Cow<'a, str>>,
}

impl <'a> Metadata<'a> {

	pub(crate) fn new<S: Into<Cow<'a, str>>>(title: S) -> Self {
		Metadata {
			title: title.into(),
			..Default::default()
		}
	}

	/// Get either the short title or the full title as a fallback
	pub fn get_short_title(&self) -> &str{
		self.shorttitle.as_deref()
			.unwrap_or(&self.title)
	}

	pub(crate) fn get_subtitle(&'a self) -> Option<&'a str> {
		match &self.subtitle {
			Some(Cow::Owned(s)) => Some(s.as_ref()),
			Some(Cow::Borrowed(s)) => Some(s),
			None => None
		}
	}

	pub(crate) fn get_title(&'a self) -> &'a str {
		match &self.title {
			Cow::Borrowed(t) => t,
			Cow::Owned(t) => t.as_ref()
		}
	}

	pub(crate) fn has_no_introduction_authors(&self) -> bool {
		self.introduction_authors.is_empty() && self.introduction_and_notes_authors.is_empty()
	}

	/// get a string of all the authors with in the format `Name, Name and Name`
	pub fn get_authors(&self) -> Option<String> {
		match self.authors.len() {
			0 => None,
			1 => self.authors.first().map(|n| n.to_string()),
			l => {
				let mut o = String::new();
				for (i, n) in self.authors.iter().enumerate() {
					if i == l {
						o.push_str(" and ");
						o.push_str(n);
					} else if i == 0 {
						o.push_str(n);
					} else {
						o.push_str(", ");
						o.push_str(n);
					}
				}
				Some(o)
			}
		}
	}

	/// Get a string of all authors, in the format `A, B and C`,
	/// where all names are uppercased
	pub fn uppercased_authors(&self) -> Option<String> {
		let authors = self.authors.iter()
			.into_iter()
			.map(|a| a.to_uppercase())
			.collect::<Vec<_>>();
		match authors.len() {
			0 => None,
			1 => Some(authors.first().unwrap().clone()),
			i => {
				let last = authors.last().unwrap();
				let pre = authors.iter()
					.cloned()
					.take(i-1)
					.collect::<Vec<_>>()
					.join(", ");
				Some(format!("{} and {}", pre, last))
			}
		}
	}

	pub(crate) fn get_copyright_page_text(&self) -> String {
		let year = bookbinder_common::get_current_year();

		// get an optional value from metadata;
		// optionally, wrap this in a prefix or suffix
		macro_rules! parse_optional_value {
			($pre:expr, $field:ident, $post:expr) => {
				if let Some(ref x) = self.$field {
					Some(format!("{}{}{}", $pre, x, $post))
				} else {
					None
				}
			};
			($pre:expr, $field:ident) => {
				if let Some(ref x) = self.$field {
					Some(format!("{}{}", $pre, x))
				} else {
					None
				}
			};
			($field:ident) => {
				if let Some(ref x) = self.$field {
					Some(x)
				} else {
					None
				}
			};
		}

		let copyright_statement = parse_optional_value!(copyright_statement);
		let publisher_name = parse_optional_value!(publisher_name);
		let publisher_address = parse_optional_value!(publisher_address);
		let publisher_url = parse_optional_value!(publisher_url);
		let cover_designer = parse_optional_value!("Cover design by ", cover_designer);
		let author_photo_copyright_holder = parse_optional_value!("Author photo © ", author_photo_copyright_holder);
		let print_loc = parse_optional_value!("Printed in ", print_loc);

		macro_rules! get_isbn {
			($field:ident, $name:expr) => {
				self.$field
					.as_ref()
					.map(|i| bookbinder_common::display_isbn(i, Some($name)))
					.transpose()
					.ok()
					.flatten()

			};
		}

		let epub_isbn = get_isbn!(epub_isbn, "epub");
		let hardback_isbn = get_isbn!(hardback_isbn, "hardback");
		let paperback_isbn = get_isbn!(paperback_isbn, "paperback");


		let copyright_statement = copyright_statement
			.map(|s| s.to_string())
			.or_else(|| {
				let authors = self.get_authors();
				if let Some(authors) = authors {
					Some(format!("Copyright © {} {}", &year, authors))
				} else {
					None
				}
			});

		let mut text = String::new();

		
		if let Some(ref copyright_statement) = copyright_statement {
			text.push_str(copyright_statement);
			text.push_str("  \n");
		};

		if !self.do_not_assert_moral_rights {
			match self.authors.len() {
				0 => {},
				1 => {
					text.push_str("The author's moral rights have been asserted");
					text.push_str("  \n");
				},
				_ => {
					text.push_str("The moral rights of the authors have been asserted");
					text.push_str("  \n");
				}
			}
		};
		if !text.is_empty() {
			text.push_str("\n\n* * *\n\n");
		}

		let pub_statement = match (self.is_not_first_publication, publisher_name) {
			(false, Some(publisher_name)) => format!("First published {} by {}", &year, publisher_name),
			(false, None) => format!("First published {}", &year),
			(true, Some(publisher_name)) =>  format!("This edition published {} by {}", &year, publisher_name),
			(true, None) => format!("This edition published {}", &year),
		};
		text.push_str(&pub_statement);
		text.push_str("  \n");
		if let Some(ref publisher_address) = publisher_address {
			text.push_str(publisher_address);
			if publisher_url.is_some() {
				text.push_str("  \n");
			}
		}
		if let Some(ref x) = publisher_url {
			text.push_str("```");
			text.push_str(x);
			text.push_str("```");
		}
		
		text.push_str("\n\n* * *\n\n");

		macro_rules! push_misc {
			($misc:ident) => {
				if let Some(ref x) = $misc {
					text.push_str(x);
					text.push_str("  \n");
				}
			};
		}

		push_misc!(epub_isbn);
		push_misc!(hardback_isbn);
		push_misc!(paperback_isbn);
		push_misc!(cover_designer);
		push_misc!(author_photo_copyright_holder);

		if text.ends_with("  \n") {
			text.drain(text.len()-3..);
		}

		if let Some(ref print_loc) = print_loc {
			if !text.ends_with("*\n\n") {
				text.push_str("\n\n* * *\n\n");
			}
			text.push_str(print_loc);
		}

		if text.ends_with("\n\n* * *\n\n") {
			text.drain(text.len()-9..);
		}
		text
	}



	pub(crate) fn get_titlepage_contributors(&'a self) -> Vec<(TitlePageContributorRole, Vec<Cow<'static, str>>)> {
		let mut contributors = Vec::new();

		macro_rules! add_contributors {
			($role:expr, $field:ident) => {
				if !self.$field.is_empty() {
					let names = self.$field.iter()
						.map(|s| match s {
							Cow::Owned(s) => Cow::Owned(s.clone()),
							Cow::Borrowed(s) => Cow::Owned(s.to_string())
						})
						.collect();
					contributors.push(($role, names));
				}
			};
		}

		use TitlePageContributorRole::*;

		add_contributors!(Author, authors);
		add_contributors!(Editor, editors);
		add_contributors!(Translator, translators);
		add_contributors!(ForewordAuthor, foreword_authors);
		add_contributors!(AfterwordAuthor, afterword_authors);
		add_contributors!(IntroductionAuthor, introduction_authors);
		add_contributors!(IntroductionAndNotesAuthor, introduction_and_notes_authors);

		contributors
	}
}



macro_rules! metadata_add_contributor {
	($fnname:ident, $field:ident, $d:meta) => {
		#[$d]
		pub fn $fnname<A: ContributorSource<'a>>(&mut self, name: A) -> &mut Self {
			if name.is_single_name() {
				self.$field.push(name.to_single().unwrap());
			} else {
				self.$field.append(&mut name.to_vec());
			};
			self
		}
	};
}


macro_rules! metadata_add_bulk_crate_contributor {
	($fnname:ident, $field:ident) => {
		pub(crate) fn $fnname<I, S>(&mut self, names: I) -> &mut Self
		where
			I: IntoIterator<Item=S>,
			S: Into<Cow<'a, str>>
		{
			for n in names.into_iter() {
				self.$field.push(n.into());
			}
			self
		}
	};
}

macro_rules! add_metadata_value {
	($fnname:ident, $field:ident, $d:meta) => {
		#[$d]
		pub fn $fnname<S: Into<Cow<'a, str>>>(&mut self, value: S) -> &mut Self {
			self.$field = Some(value.into());
			self
		}
	};
}

macro_rules! meta_bool {
	($fnname:ident, $field:ident, $d:meta) => {
		#[$d]
		pub fn $fnname(&mut self) -> &mut Self {
			self.$field = true;
			self
		}
	};
}

impl <'a> Metadata<'a> {
	metadata_add_contributor!(author, authors, doc="Add an author or authors");
	metadata_add_contributor!(translator, translators, doc="Add a translator or translators");
	metadata_add_contributor!(editor, editors, doc="Add an editor or editors");
	add_metadata_value!(cover_designer, cover_designer, doc="Name the cover designer of this work");
	add_metadata_value!(shorttitle, shorttitle, doc="Set the short title, a briefer version of the title used where a full title might be unneccesarily verbose");
	add_metadata_value!(subtitle, subtitle, doc="Set the work's subtitle");
	add_metadata_value!(author_photo_copyright_holder, author_photo_copyright_holder, doc="Name the copyright holder in any author photograph included in the work or on the covers");
	add_metadata_value!(paperback_isbn, paperback_isbn, doc="An isbn for the paperback edition of this book");
	add_metadata_value!(hardback_isbn, hardback_isbn, doc="An isbn for the hardback edition of this book");
	add_metadata_value!(epub_isbn, epub_isbn, doc="An isbn for the epub edition of this book");
	add_metadata_value!(publisher, publisher_name, doc="The name of the publisher of this book");
	add_metadata_value!(publisher_address, publisher_address, doc="An address for the publisher");
	add_metadata_value!(publisher_url, publisher_url, doc="A url for the publisher");
	add_metadata_value!(print_location, print_loc, doc="State where the work will be printed");
	add_metadata_value!(copyright_statement, copyright_statement, doc="Set a custom copyright statement");
	meta_bool!(do_not_assert_moral_rights, do_not_assert_moral_rights, doc="Set this flag if you do not wish to assert the moral rights of the author on the copyright page");
	meta_bool!(is_not_first_publication, is_not_first_publication, doc="Set this flag if this is not the book's first publication");
	metadata_add_bulk_crate_contributor!(add_foreword_authors, foreword_authors);
	metadata_add_bulk_crate_contributor!(add_afterword_authors, afterword_authors);
	metadata_add_bulk_crate_contributor!(add_introduction_authors, introduction_authors);
}