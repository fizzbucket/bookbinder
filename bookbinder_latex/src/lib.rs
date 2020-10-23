//! This crate provides support, with various options, for transforming a `BookSrc` into a latex file or a pdf generated from that file.
//#![deny(dead_code)]
#![deny(unreachable_patterns)]
#![deny(unused_extern_crates)]
//#![deny(unused_imports)]
#![deny(unused_qualifications)]
//#![deny(clippy::all)]
//#![deny(missing_docs)]
//#![deny(missing_debug_implementations)]

use bookbinder_ast::Metadata;
use temp_file_name::TempFilePath;
use bookbinder_common::MimeTypeHelper;
use std::path::{PathBuf};
use bookbinder_ast::{BookEvent, BookSrc, TextHeaderOptions, SemanticRole};
use extended_pulldown::{Event, Tag};
mod preamble_options;
pub use preamble_options::{PreambleOptions, PaperSize};
use bookbinder_ast::helpers::{BookEventIteratorHelper, CollatedHeader, LatexMarker, CollatedImage};


/// Options with a prerendered preamble;
/// normally derived from `PreambleOptions`.
#[derive(Debug, Default, Clone)]
pub struct OptionsWithRenderedPreamble {
 	publisher_imprint_logo: Option<PathBuf>,
	header_format: TextHeaderOptions,
	preamble: String,
	page_identifier: Option<String>,
	contributor_identifier: Option<String>,
	latex_secnumdepth: LatexSecNumDepth,
	include_toc: bool
}

impl OptionsWithRenderedPreamble {
	/// Set the logo of a publisher to use on the titlepage
	pub fn set_publisher_logo(&mut self, mut p: PathBuf) -> Result<&mut Self, String> {

		p = p.canonicalize()
			.map_err(|e| format!("Error canonicalizing path ({}): {}", p.display(), e.to_string()))?;

		if p.is_svg() {
			let svg = std::fs::read_to_string(&p)
				.map_err(|e| format!("Error reading svg path ({}): {}", p.display(), e.to_string()))?;
			let png_path = svg.temp_file_path(Some("bookbinder"), "png");
			if png_path.exists() {
				p = png_path;
			} else {
				let png = bookbinder_common::convert_svg_file_to_png(&p, Some(150))
					.map_err(|e| format!("Error converting svg file to png ({}): {}", p.display(), e.to_string()))?;
				std::fs::write(&png_path, png)
					.map_err(|e| format!("Error writing new png path ({}): {}", p.display(), e.to_string()))?;
				p = png_path;
			}
		};
		if p.exists() && p.is_valid_logo_image() {
			self.publisher_imprint_logo = Some(p);
			Ok(self)
		} else {
			Err(format!("Error setting logo path: {}", p.display()))
		}
	}

	/// Do not show any given chapter's title -- rely instead
	/// on its label.
	/// For example, `Chapter 1: Wolves Attack!` would be
	/// represented as `Chapter 1`
	pub fn suppress_chapter_titles(&mut self) -> &mut Self {
		self.header_format.suppress_chapter_titles();
		self
	}

	/// Do not label chapters as such in headings; i.e. use only the
	/// chapter title.
	/// For example, `Chapter 1: Wolves Attack!` would be
	/// represented as `Wolves Attack!`
	pub fn suppress_chapter_label(&mut self) -> &mut Self {
		self.header_format.suppress_chapter_labels();
		self
	}

	/// Indicate chapters only by using a numerical indication,
	/// in whatever format.
	/// For example, `Chapter 1: Wolves Attack!` would be
	/// represented as `1`,
	/// or `I` if `use_roman_numerals_for_chapter_labels` was called
	pub fn only_number_chapters(&mut self) -> &mut Self {
		self.header_format.only_number_chapters();
		self
	}

	/// Include a table of contents
	pub fn include_toc(&mut self) -> &mut Self {
		self.include_toc = true;
		self
	}

	/// Set the secnumdepth
	pub fn set_secnumdepth(&mut self, secnumdepth: LatexSecNumDepth) -> &mut Self {
		self.latex_secnumdepth = secnumdepth;
		self
	}

	pub(crate) fn set_running_footers_from_metadata(&mut self, metadata: Metadata<'_>) -> &mut Self {
		let page_identifier = metadata.get_short_title().to_string();
		let authors = if let Some(authors) = metadata.get_authors() {
			let authors = authors.split(" and ")
				.map(|n| n.to_uppercase())
				.collect::<Vec<String>>()
				.join(" and ");
			Some(authors)
		} else {
			None
		};

		self.page_identifier = Some(page_identifier);
		self.contributor_identifier = authors;
		self
	}
}


/// The numbering depth to use for section headings
#[derive(Debug, Clone, Copy)]
pub enum LatexSecNumDepth {
	/// number only parts
	Part,
	/// number chapters and higher
	Chapter,
	/// number sections and higher
	Section,
	/// number subsections and higher
	Subsection,
	/// number subsubsections and higher
	Subsubsection,
	/// number paragraphs and higher
	Paragraph,
	/// number all section headings
	Subparagraph
}

impl LatexSecNumDepth {
	pub(crate) fn as_counter(&self) -> isize {
		use LatexSecNumDepth::*;
		match self {
			Part => -1,
			Chapter => 0,
			Section => 1,
			Subsection => 2,
			Subsubsection => 3,
			Paragraph => 4,
			Subparagraph => 5
		}
	}
}

impl From<i32> for LatexSecNumDepth {
	fn from(i: i32) -> Self {
		
		use LatexSecNumDepth::*;
		match i {
			i if i < 0 => Part,
			0 => Chapter,
			1 => Section,
			2 => Subsection,
			3 => Subsubsection,
			4 => Paragraph,
			_ => Subparagraph
		}
	}
}

impl Default for LatexSecNumDepth {
	fn default() -> Self {
		LatexSecNumDepth::Chapter
	}
}




#[derive(Debug)]
enum Matter {
	Main,
	Front,
	Back
}

/// simple commands to write standard LaTeX items,
/// such as beginning an environment or issuing a command.
///
/// Having this as a seperate trait eliminates a good deal of redundancy
trait LaTexOps {
	fn begin_environment(&mut self, env_name: &str);
	fn end_environment(&mut self, env_name: &str);
	fn set_counter(&mut self, counter: &str, value: isize);
	fn step_counter(&mut self, counter: &str);
}

impl LaTexOps for String {
	fn begin_environment(&mut self, env_name: &str) {
		if !self.ends_with('\n') {
			self.push('\n');
		}
		self.push_str("\\begin{");
		self.push_str(env_name);
		self.push_str("}\n");
	}

	fn end_environment(&mut self, env_name: &str) {
		if !self.ends_with('\n') {
			self.push('\n');
		}
		self.push_str("\\end{");
		self.push_str(env_name);
		self.push_str("}\n");
	}

	fn set_counter(&mut self, counter: &str, value: isize) {
		if !self.is_empty() && !self.ends_with('\n') {
			self.push('\n');
		}
		self.push_str("\\setcounter{");
		self.push_str(counter);
		self.push_str("}{");
		self.push_str(&value.to_string());
		self.push_str("}\n");

	}

	fn step_counter(&mut self, counter: &str) {
		if !self.ends_with('\n') {
			self.push('\n');
		}
		self.push_str("\\stepcounter{");
		self.push_str(counter);
		self.push_str("}\n");
	}
}

#[derive(Debug, Default)]
struct LatexWriter {
	output: String,
	current_matter: Option<Matter>,
	current_division: Option<SemanticRole>,
	expected_epigraphs_count: usize,
	seen_epigraphs: usize,
	expected_appendices_count: usize,
	seen_appendices: usize,
	in_code: bool,
	mainmatter_toggled: bool,
	chapter_count: usize,
	frontmatter_had_toc_contents: bool,
	publisher_imprint_logo: Option<PathBuf>,
	include_toc: bool
}


impl LatexWriter {

	fn write_plain(&mut self, event: Event<'_>) {
		use Event::*;
		use Tag::*;

		match event {
			HardBreak => {
				self.output.push_str("\\\\\n");
			},
			SoftBreak => {
				self.output.push_str("\n");
			},
			Rule => {
				self.output.push_str("\n\\pfbreak{}\n");
			},
			Text(s) => {
				if !self.in_code {
					let escaped = bookbinder_common::escape_to_latex(s.as_ref());
					self.output.push_str(&escaped);
				} else {
					self.output.push_str(&s);
				}
			},
			Code(c) => {
				// we use the ❡ sign as something highly unlikely to be in any code
				let delimiter = if c.contains('❡') {
					let options = "|!®©℗™℠";
					let mut d = None;
					for x in options.chars() {
						if !c.contains(x) {
							d = Some(x);
							break;
						}
					}
					if let Some(x) = d {
						x
					} else {
						panic!("Could not find appropriate delimiter for inline code");
					}
				} else {
					'❡'
				};
				self.output.push_str("\\verb");
				self.output.push(delimiter);
				self.output.push_str(&c);
				self.output.push(delimiter);
			},
			Start(Paragraph) => {
				self.output.push('\n');
			},
			Start(UnindentedParagraph) => {
				self.output.push_str("\n\\noindent ");
			},
			Start(Heading(l)) => {
				match l {
			    	0 | 1 => self.output.push_str("\n\\section{"),
			    	2 => self.output.push_str("\n\\subsection{"),
			    	3 => self.output.push_str("\n\\subsubsection{"),
			    	4 => self.output.push_str("\n\\paragraph{"),
			    	_ => self.output.push_str("\n\\subparagraph{"),
				}
			},
			Start(BlockQuote) => self.output.begin_environment("quote"),
			Start(BlockQuotation) => self.output.begin_environment("quotation"),
			Start(CodeBlock(_)) => {
				self.output.begin_environment("verbatim");
				self.in_code = true;
			},
			Start(List(None)) => self.output.begin_environment("itemize"),
			Start(List(Some(_))) => self.output.begin_environment("enumerate"),
			Start(Item) => self.output.push_str("\\item "),
			Start(Sans) => self.output.push_str("\\textsf{"),
			Start(Emphasis) => self.output.push_str("\\emph{"),
			Start(Strong) => self.output.push_str("\\textbf{"),
			Start(Link(_, url, _)) => {
				self.output.push_str("\\href{");
		    	self.output.push_str(&url);
		    	self.output.push_str("}{");
			},
			Start(Image(_, dest, alt)) => {
				let alt = if !alt.is_empty() {
					Some(alt)
				} else {
					None
				};

				let collated = CollatedImage {
					caption: None,
					dest,
					alt
				};
				if let Ok(p) = collated.get_latex_image_path() {
					self.output.begin_environment("figure");
		            self.output.push_str("\\centering\n");
		            self.output.push_str("\\includegraphics[width=\\textwidth]{");
		            self.output.push_str(&p);
		            self.output.push_str("}\n");
					self.output.end_environment("figure");
				}
			},
			Start(Strikethrough) => self.output.push_str("\\sout{"),
			Start(SmallCaps) => self.output.push_str("\\textsc{"),
			Start(RightAligned) => self.output.begin_environment("flushright"),
			Start(Superscript) => self.output.push_str("\\textsuperscript{"),
			Start(Subscript) => self.output.push_str("\\textsubscript{"),
			Start(Centred) => self.output.begin_environment("center"),
			End(Paragraph) | End(UnindentedParagraph) => {
				if !self.output.ends_with("\n\n") {
					self.output.push('\n');
				}
			}
			End(Heading(_)) => self.output.push_str("}\n"),
			End(BlockQuote) => self.output.end_environment("quote"),
			End(BlockQuotation) => self.output.end_environment("quotation"),
			End(CodeBlock(_)) => {
				self.output.end_environment("verbatim");
				self.in_code = false;
			}
			End(List(None)) => self.output.end_environment("itemize"),
			End(List(Some(_))) => self.output.end_environment("enumerate"),
			End(Item) => self.output.push('\n'),
			End(Image(_, _, _)) => {},
			End(RightAligned) => self.output.end_environment("flushright"),
			End(Centred) => self.output.end_environment("center"),
			End(Sans) | End(Emphasis) | End(Strong) | End(Link(_, _, _)) | End(Strikethrough) | End(SmallCaps) | End(Superscript) | End(Subscript) => {
				self.output.push('}');
			},
			End(TableHead) => {},
			End(TableRow) => {},
			End(TableCell) => {},
			Start(FootnoteDefinition(_)) => {},
			End(FootnoteDefinition(_)) => {},
			End(Table(_)) => {},
			Html(_) => {},
			Start(TableHead) => {},
			Start(TableRow) => {},
			Start(TableCell) => {},
			FootnoteReference(_) => {},
			TaskListMarker(_) => {},
			Start(Table(_)) => {},
			Start(FlattenedFootnote) => {},
			End(FlattenedFootnote) => {}
		}
	}



	fn write<'a, I: IntoIterator<Item=BookEvent<'a>>>(&mut self, events: I) {
		
		use BookEvent::*;
		use extended_pulldown::Event::*;

		let mut events = events.into_iter();


		macro_rules! drop_until {
			($break:pat) => {
				while let Some(event) = events.next() {
					if let $break = event {
						break;
					}
				}
			};
		}

		while let Some(event) = events.next() {
			match event {
				BeginSemantic(SemanticRole::Epigraph) => {
					self.current_division = Some(SemanticRole::Epigraph);
					if self.seen_epigraphs == 0 {
						self.output.begin_environment("epigraphs");
					}
					self.seen_epigraphs += 1;
					let epigraph_src = events.collate_epigraph();
					
					for event in epigraph_src.text.into_iter() {
						self.write_plain(event);
					}
					if !epigraph_src.source.is_empty() {
						self.output.push_str("\\par\n\\vspace{1em}\\noindent\\epigraphsource{");
						for event in epigraph_src.source.into_iter() {
							self.write_plain(event);
						}
						self.output.push_str("}\n");
					}
					if self.seen_epigraphs == self.expected_epigraphs_count {
						self.output.end_environment("epigraphs");
					} else {
						self.output.push_str("\\bigskip\n");
					}
					self.current_division = None;
				},
				BeginDivisionHeader(is_starred) => {
					let header_src: CollatedHeader<LatexMarker> = events.collate_division_header(is_starred);
					let label_and_title = header_src.reconcile_joined_label_and_title();
					if let Some((label, title)) = label_and_title {
						match self.current_matter {
							Some(Matter::Main) => {
								match self.current_division {
									Some(SemanticRole::Part) => {
										if let Some(title) = title {
											if is_starred {
												self.output.push_str(&format!("\n\\part*{{{}}}", title));
											} else {
												self.output.push_str(&format!("\n\\part{{{}}}", title));
											}
										}
									},
									Some(SemanticRole::Chapter) => {
										match (label, title) {
											(Some(_), Some(title)) => {
												self.output.push_str(&format!("\n\\chapter{{{}}}", title));
											},
											(Some(_), None) => {
												self.output.push_str("\n\\chapter[\\chaptername{} \\thechapter]{}");
											},
											(None, Some(title)) => {
												self.output.step_counter("chapter");
												self.output.push_str("\n\\addcontentsline{toc}{chapter}{\\numberline{\\thechapter} ");
												self.output.push_str(&title);
												self.output.push('}');
												self.output.push_str(&format!("\n\\chapter*{{{}}}", title));
											},
											(None, None) => {
												self.output.step_counter("chapter");
												self.output.push_str("\n\\addcontentsline{toc}{chapter}{\\numberline{\\thechapter}  \\chaptername{} \\thechapter}");
												self.output.push_str("\n\\chapter*{}");				
											}

										}
									},
									_ => {}
								}
							},
							_ => {
								match self.current_division {
									Some(SemanticRole::Appendix) => {
										if self.seen_appendices == 1 {
											self.output.push_str("\\addcontentsline{toc}{part}{Appendices}\n");
											self.output.set_counter("chapter", 0);
											self.output.push_str("\\renewcommand{\\thechapter}{\\Alph{chapter}}\n");
										}
										let label_and_title = header_src.reconcile_joined_label_and_title();
										if let Some((label, title)) = label_and_title {
											let header = match (label, title) {
												(Some(label), Some(title)) => {
													let mut header = String::new();
													header.push_str("\\setchapterlabel{");
													header.push_str(&label);
													header.push_str("}\n");
													header.push_str(&format!("\\chapter{{{}}}\n", title));
													header.push_str("\\unsetchapterlabel\n");
													header
												},
												(None, Some(title)) => {
												let mut header = String::new();
												header.step_counter("chapter");
												header.push_str(&format!("\\chapter*{{{}}}\n", &title));
												header.push_str(&format!("\\addcontentsline{{toc}}{{chapter}}{{\\numberline{{{}}} {}}}", header_src.get_label_number().unwrap_or_default(), title));
												header
												},
												_ => String::new()
											};
											self.output.push('\n');
											self.output.push_str(&header);
											self.output.push('\n');
										}
									},
									Some(SemanticRole::Halftitle) => {
										if let Some(title) = title {
											self.output.push_str(&title);
										}
									},
									Some(SemanticRole::Acknowledgements) => {
										if let Some(title) = title {
											self.output.step_counter("chapter");
											self.output.push_str(&format!("\\chapter*{{{}}}\n", &title));
											self.output.push_str(&format!("\\addcontentsline{{toc}}{{chapter}}{{{}}}", title));
											self.output.push('\n');
										}	
									},
									_ => {

										let authors = match header_src.get_authors() {
											None => None,
											Some((first, None)) => Some(first),
											Some((first, Some(second))) => Some(format!("{} and {}", first, second).into())
										};

										match (label, title, authors) {
											(Some(label), Some(title), Some(authors)) => {
												self.output.push_str(&format!("\n\\ancillaryheader{{{}}}{{{}}}{{{}}}\n", label, title, authors));
											},
											(Some(label), Some(title), None) => {
												self.output.push_str(&format!("\n\\labelledchapter{{{}}}{{{}}}\n", label, title));
											},
											(None, Some(title), Some(authors)) => {
												self.output.push_str(&format!("\n\\unlabelledancillaryheader{{{}}}{{{}}}\n", title, authors));
											},
											(None, Some(title), None) => {
												let mut header = String::new();
												header.step_counter("chapter");
												header.push_str(&format!("\\chapter*{{{}}}\n", &title));
												header.push_str(&format!("\\addcontentsline{{toc}}{{chapter}}{{{}}}\n", title));
												self.output.push_str(&header)
											},
											_ => {}
										};
									}
								}
							}
						}
					}
				},
				BeginTitlePage => {
					let titlepage_src = events.collate_titlepage();
					self.output.begin_environment("titlepage");
					self.output.begin_environment("titlepagetitleblock");
					self.output.push_str("\\titlepagetitle{");
					for event in titlepage_src.title.into_iter() {
						self.write_plain(event);
					}
					self.output.push_str("}\n");
					if let Some(subtitle) = titlepage_src.subtitle {
						self.output.push_str("\\titlepagesubtitle{");
						for event in subtitle.into_iter() {
							self.write_plain(event);
						}
						self.output.push_str("}\n");
					}
					self.output.end_environment("titlepagetitleblock");


					if let Some(contributors) = titlepage_src.contributors {

						self.output.begin_environment("titlepagecontributors");

						for (role, names) in contributors.into_iter() {
							self.output.begin_environment("contributorgroup");
							if let Some(role) = role {
								self.output.push_str(&format!("\n\\contributorintro{{{}}}", role));
							}

							let mut names = names.into_iter()
								.map(bookbinder_common::escape_to_latex)
								.collect::<Vec<_>>();
							match names.len() {
								0 => {},
								1 => {
									self.output.push_str("\n\\ctbname{");
									self.output.push_str(&names.last().unwrap());
									self.output.push('}');
								},
								_ => {
									let last = names.pop().unwrap();
									self.output.push_str("\n\\ctbname{");
									let pre = names.join(", ");
									self.output.push_str(&pre);
									self.output.push('}');
									self.output.push_str(" \\ctband ");
									self.output.push_str("\\ctbname{");
									self.output.push_str(&last);
									self.output.push('}');
								}
							}
							self.output.end_environment("contributorgroup");
						}
						self.output.end_environment("titlepagecontributors");
					}

					if let Some(ref l) = self.publisher_imprint_logo {
						self.output.push_str("\\publisherlogo{");
						self.output.push_str(l.to_str().unwrap());
						self.output.push_str("}\n");
					}
					self.output.end_environment("titlepage");
				},
				Event(Start(Tag::FlattenedFootnote)) => {
					let footnote_events = events.collect_plain_until_end_of_footnote();
					self.output.push_str("\\footnote{");
					let mut footnote_events = footnote_events.into_iter();
					
					#[allow(clippy::while_let_on_iterator)]
					while let Some(event) = footnote_events.next() {
						match event {
							Start(Tag::Paragraph) => {},
							End(Tag::Paragraph) => self.output.push_str("\\par{}"),
							Start(Tag::FlattenedFootnote) => {
								while let Some(event) = footnote_events.next() {
									if matches!(event, End(Tag::FlattenedFootnote)) {
										break
									}
								}
							}
							e => self.write_plain(e)
						}
					}
					self.output.push('}');
				},
				Event(Start(Tag::Image(_, dest, alt))) => {
					let collated_image = events.collate_image(dest, alt);
					if let Ok(p) = collated_image.get_latex_image_path() {
						self.output.begin_environment("figure");
			            self.output.push_str("\\centering\n");
			            self.output.push_str("\\includegraphics[width=\\textwidth]{");
			            self.output.push_str(&p);
			            self.output.push_str("}\n");
						if let Some(caption) = collated_image.caption {
			            	self.output.push_str("\\caption{");
			            	for event in caption.into_iter() {
			            		self.write_plain(event);
			            	}
			            	self.output.push_str("}\n");
						}
						self.output.end_environment("figure");
					}
				},
				BeginFrontmatter => {
					self.current_matter = Some(Matter::Front);
					self.output.push_str("\n\\frontmatter\n");
					self.output.push_str("\n\\suppresschapternumbersintoc\n");
				},
				BeginMainmatter => {
					self.current_matter = Some(Matter::Main);
					self.output.push_str("\n\\clearpage");
					self.output.set_counter("chapter", 0);
					self.output.push_str("\n\\pagenumbering{arabic}");
					self.output.push_str("\n\\restorechapternumbersintoc{}\n");
					//if self.frontmatter_had_toc_contents {
					//	self.output.push_str("\\addtocontents{toc}{{\\bigskip\\par\\noindent\\hfill\\pfbreakdisplay\\hfill\\bigskip\n\n}}");
					//}

					if !self.mainmatter_toggled {
						self.output.push_str("\n\\mainmatter\n");
					}
				},
				BeginBackmatter => {
					self.current_matter = Some(Matter::Back);
					if self.expected_appendices_count == 0 {
					 	self.output.push_str("\n\\suppresschapternumbersintoc\n");
					}
					//self.output.push_str("\\addtocontents{toc}{{\\bigskip\\par\\noindent\\hfill\\pfbreakdisplay\\hfill\\bigskip\n\n}}");
					self.output.push('\n');
				},
				// we don't (yet) know how to handle tables or tasklists
				Event(Start(Tag::Table(_))) => {
					drop_until!(Event(End(Tag::Table(_))));
				},
				// footnotes should have been flattened
				Event(Start(Tag::FootnoteDefinition(_))) => {
					drop_until!(Event(End(Tag::FootnoteDefinition(_))));
				},
				BeginSemantic(role) => {
					self.current_division = Some(role);
					match role {
						SemanticRole::Titlepage => self.output.begin_environment("titlepage"),
						SemanticRole::Copyrightpage => self.output.begin_environment("copyrightpage"),
						SemanticRole::Halftitle => self.output.begin_environment("halftitle"),
						SemanticRole::Dedication => self.output.begin_environment("dedication"),
						SemanticRole::Colophon => self.output.begin_environment("colophon"),
						SemanticRole::Epigraph => unreachable!(),
						SemanticRole::Introduction | SemanticRole::Foreword | SemanticRole::Preface => {
							if !self.mainmatter_toggled {
								self.output.push_str("\n\\mainmatter\n");
								self.output.set_counter("secnumdepth", 0);
								self.output.push_str("\n\\suppresschapternumbersintoc{}\n\\pagenumbering{roman}");
							}
							self.mainmatter_toggled = true;
						},
						SemanticRole::Appendix => {
							self.seen_appendices += 1;
						}
						_ => {}
					}
				},
				EndSemantic(role) => {
					self.current_division = None;
					match role {
						SemanticRole::Titlepage => self.output.end_environment("titlepage"),
						SemanticRole::Copyrightpage => {
							self.output.end_environment("copyrightpage");
							if self.include_toc {
								self.output.push_str("\n\\tableofcontents\n");
							}
						},
						SemanticRole::Halftitle => self.output.end_environment("halftitle"),
						SemanticRole::Dedication => self.output.end_environment("dedication"),
						SemanticRole::Colophon => self.output.end_environment("colophon"),
						SemanticRole::Appendix => {
							if self.seen_appendices == self.expected_appendices_count {
								self.output.push_str("\n\\suppresschapternumbersintoc\n");
							}
						},
						_ => {}
					}
				},
				Event(e) => self.write_plain(e),
				_ => {}
			}
		}
	}
}


/// Support for rendering to a tex document
pub trait TexRenderer {
	/// Render to tex with a pregenerated preamble
	fn render_to_tex_standalone(self, logo: Option<PathBuf>, include_toc: bool) -> String;
	/// Render to a fragment of tex without a preamble or document beginning or end
	fn render_to_tex_with_preamble(self, options: OptionsWithRenderedPreamble) -> String;
	/// Render with default options
	fn render_to_tex(self) -> String;
	/// Render with particular options to generate a preamble with
	fn render_to_tex_with_options(self, options: PreambleOptions) -> String;
}


impl TexRenderer for BookSrc<'_> {

	fn render_to_tex_with_options(self, options: PreambleOptions) -> String {
		let options = OptionsWithRenderedPreamble::from(options);
		self.render_to_tex_with_preamble(options)
	}

	fn render_to_tex(self) -> String {
		let options = PreambleOptions::default();
		self.render_to_tex_with_options(options)
	}

	fn render_to_tex_standalone(self, logo: Option<PathBuf>, include_toc: bool) -> String {
		let mut writer = LatexWriter::default();
		writer.expected_epigraphs_count = self.expected_epigraph_count;
		writer.expected_appendices_count = self.expected_appendices_count;
		writer.include_toc = include_toc;
		writer.publisher_imprint_logo = logo;
		writer.write(self.contents);
		writer.output
	}

	fn render_to_tex_with_preamble(mut self, mut options: OptionsWithRenderedPreamble) -> String {
		self.change_headers(options.header_format);
		options.set_running_footers_from_metadata(std::mem::take(&mut self.metadata));
		let imprint_logo = options.publisher_imprint_logo;
		let include_toc = options.include_toc;
		let text = self.render_to_tex_standalone(imprint_logo, include_toc);
		
		let secnumdepth = options.latex_secnumdepth;

		let mut start = options.preamble;
		start.begin_environment("document");
		start.set_counter("secnumdepth", secnumdepth.as_counter());
		if let Some(ref page_identifier) = options.page_identifier {
			start.push_str(&format!("\n\\renewcommand{{\\pageidentifier}}{{{}}}", page_identifier.trim()));
		}
		if let Some(ref contributor_identifier) = options.contributor_identifier {
			start.push_str(&format!("\n\\renewcommand{{\\currentcontributor}}{{{}}}", contributor_identifier.trim()));
		}

		start.push_str(&text);
		start.end_environment("document");

		start
	}
}





/// Support for rendering to a pdf file
pub trait PdfRenderer: TexRenderer + Sized {
	/// Use a pregenerated preamble in rendering
	fn render_to_pdf_with_preamble(self, options: OptionsWithRenderedPreamble) -> Result<Vec<u8>, std::io::Error> {
		let tex = self.render_to_tex_with_preamble(options);
		bookbinder_common::call_latex(&tex)
	}

	/// Generate a preamble and then render using it
	fn render_to_pdf_with_options(self, options: PreambleOptions) -> Result<Vec<u8>, std::io::Error> {
		let tex = self.render_to_tex_with_options(options);
		bookbinder_common::call_latex(&tex)
	}

	/// render with default options
	fn render_to_pdf(self) -> Result<Vec<u8>, std::io::Error> {
		let tex = self.render_to_tex();
		bookbinder_common::call_latex(&tex)
	}

}

impl <T> PdfRenderer for T where T: TexRenderer {
}
