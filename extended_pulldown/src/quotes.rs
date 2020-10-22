use std::borrow::Cow;
use std::collections::VecDeque;
use lazy_static::lazy_static;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};


#[derive(Debug)]
enum TextComponent<'a> {
	LeftQuote,
	LeftQuoteOrApostrophe,
	RightQuote,
	RightQuoteOrApostrophe,
	NonQuote(Cow<'a, str>),
	Apostrophe,
	Break
}

#[derive(Default)]
struct Replacer<'a> {
	sources: Vec<&'a str>,
	current_str: Option<std::str::Chars<'a>>,
	next_char: Option<char>,
	components: VecDeque<TextComponent<'a>>,
	str_buffer: Option<String>,
	in_double_quote: bool,
}

static SUBSTITUTE_BREAK: TextComponent = TextComponent::NonQuote(Cow::Borrowed("\n"));

impl <'a> Replacer<'a> {

	fn new() -> Self {
		Replacer {
			sources: Vec::new(),
			current_str: None,
			next_char: None,
			in_double_quote: false,
			components: VecDeque::new(),
			str_buffer: None,
		}
	}

	fn next_from_current_str(&mut self) -> Option<char> {
		match &mut self.current_str {
			Some(c) => {
				match c.next() {
					Some(c) => Some(c),
					None => {
						self.current_str = None;
						None
					}
				}
			},
			None => None
		}
	}

	fn next(&mut self) -> Option<char> {
		if self.next_char.is_some() {
			self.next_char.take()
		} else if let Some(c) = self.next_from_current_str() {
			Some(c)
		} else if !self.sources.is_empty() {
			self.current_str = Some(self.sources.remove(0).chars());
			if self.components.is_empty() {
				match self.str_buffer {
					Some(_) => Some('\n'),
					None => self.next()
				}
			} else {
				Some('\n')
			}
		} else {
			None
		}
	}

	#[inline]
	fn next_char(&mut self) -> Option<char> {
		if let Some(c) = self.next_char {
			Some(c)
		} else {
			let n = self.next();
			self.next_char = n;
			n
		}
	}

	fn add_str(&mut self, s: &'a str) {
		self.components.reserve(s.len() / 20);
		self.sources.push(s);
	}

	#[inline]
	fn prev_char(&self) -> Option<char> {
		match self.str_buffer {
			Some(ref b) => b.chars().rev().next(),
			None => None
		}
	}

	#[inline]
	fn push_to_buffer(&mut self, c: char) {
		match self.str_buffer.as_mut() {
			Some(b) => b.push(c),
			None => {
				self.str_buffer = Some(c.to_string())
			}
		}
	}

	#[inline]
	fn pop_from_buffer(&mut self) -> Option<char> {
		match self.str_buffer.as_mut() {
			Some(b) => b.pop(),
			None => None
		}
	}

	#[inline]
	fn push_buffer_to_components(&mut self) {
		if let Some(buffer) = std::mem::take(&mut self.str_buffer) {
			self.components.push_back(TextComponent::NonQuote(buffer.into()))
		}
	}

	fn step(&mut self) -> bool {

		let next = match self.next() {
			Some(n) => n,
			None => {
				self.push_buffer_to_components();
				return false
			}
		};

		match next {
			'\n' => {
				self.push_buffer_to_components();
				self.components.push_back(TextComponent::Break);
			},
			'.' => {
				if self.prev_char() == Some('.') && self.next_char() == Some('.') {
					self.next_char = None;
					self.pop_from_buffer();
					self.push_to_buffer('…');
				} else {
					self.push_to_buffer('.');
				}
			},
			'-' => {
				match self.prev_char() {
					Some('-') => {
						self.pop_from_buffer();
						self.push_to_buffer('–');
					},
					Some('–') => {
						self.pop_from_buffer();
						self.push_to_buffer('—');
					},
					_ => self.push_to_buffer('-')
				}
			},
			'“' => {
				self.in_double_quote = true;
				self.push_to_buffer('“');
			},
			'”' => {
				self.in_double_quote = false;
				self.push_to_buffer('”');
			},
			'"' => {
				if self.in_double_quote {
					self.in_double_quote = false;
					self.push_to_buffer('”');
				} else {
					self.in_double_quote = true;
					self.push_to_buffer('“');
				}
			},
			'‘' => {
				self.push_buffer_to_components();
				self.components.push_back(TextComponent::LeftQuote);
			},
			'’' => {
				self.push_buffer_to_components();
				self.components.push_back(TextComponent::RightQuote);
			},
			'\'' => {
				self.handle_straight_single_quote();
			},
			other => {
				self.push_to_buffer(other);
			}
		};
		true
	}

	fn parse(&mut self) {
		while self.step() {}
	}

	fn handle_straight_single_quote(&mut self) {
		use TextComponent::*;

		self.push_buffer_to_components();
		let next_char = self.next_char();
		let previous_component = match self.components.back() {
			Some(Break) => Some(&SUBSTITUTE_BREAK),
			x => x
		};

		match (previous_component, next_char) {
			(Some(Break), _) => unreachable!(),
			(None, Some(_)) => {
				self.components.push_back(LeftQuote);
			},
			(Some(_), None) => {
				self.components.push_back(RightQuote);
			},
			(None, None) => {
				self.components.push_back(Apostrophe);
			},
			(Some(LeftQuote), _) => {
				self.components.push_back(Apostrophe);
			},
			(Some(LeftQuoteOrApostrophe), _) => {
				let p = self.components.back_mut().unwrap();
				*p = LeftQuote;
				self.components.push_back(Apostrophe);
			},
			(Some(RightQuoteOrApostrophe), _) | (Some(RightQuote), _) => {
				let p = self.components.back_mut().unwrap();
				*p = Apostrophe;
				self.components.push_back(RightQuote)
			},
			(Some(Apostrophe), _) => {
				self.components.push_back(RightQuote)
			},

			(Some(NonQuote(s)), next) => {
				let pc = s.chars().rev().next().unwrap();
				// we just need to do a little handling of the special case "'n'",
				// since that isn't quotation but two apostrophes

				if s == "n" {
					let penultimate = match self.components.len() {
						1 => None,
						i => {
							let n = self.components.get_mut(i-2);
							match n {
								Some(LeftQuoteOrApostrophe) => n,
								_ => None
							}
						}
					};
					let next_is_space = match next {
						Some(' ') | Some('\n') => true,
						_ => false
					};

					if next_is_space {
						if let Some(penultimate) = penultimate {
							*penultimate = Apostrophe;
							self.components.push_back(Apostrophe);
							return;
						}
					}
				}

				match (pc, next) {
					(c, Some(d)) if c.is_alphanumeric() && d.is_alphanumeric() => {
						self.components.push_back(Apostrophe);
					},
					('“', Some(_)) => self.components.push_back(LeftQuote),
					(_, Some('”')) => self.components.push_back(RightQuote),
					(' ', _) => self.components.push_back(LeftQuoteOrApostrophe),
					('\n', _) => self.components.push_back(LeftQuoteOrApostrophe),
					(c, Some(' ')) if c.is_alphanumeric() => {
						self.components.push_back(RightQuoteOrApostrophe)
					},
					(c, Some('\n')) if c.is_alphanumeric() => {
						self.components.push_back(RightQuoteOrApostrophe)
					},
					(_, Some(' ')) => self.components.push_back(RightQuote),
					(_, Some('\n')) => self.components.push_back(RightQuote),
					(c, Some(d)) if d.is_alphanumeric() => {
						match c {
							'…' | '—' | '–' => self.components.push_back(LeftQuote),
							c if c.is_ascii_punctuation() => self.components.push_back(LeftQuote),
							_ => self.components.push_back(Apostrophe)
						}
					},
					(c, Some(d)) if c.is_alphanumeric() => {
						match d {
							'…' | '—' | '–' => {
								self.components.push_back(RightQuote)
							},
							c if c.is_ascii_punctuation() => {
								self.components.push_back(RightQuote)
							},
							_ => self.components.push_back(Apostrophe)
						}
					},
					(c, Some(d)) if c.is_ascii_punctuation() => {
						match d {
							'…' | '—' | '–' => {
								self.components.push_back(RightQuote);
							},
							c if c.is_ascii_punctuation() => {
								self.components.push_back(RightQuote);
							},
							_ => {
								self.components.push_back(Apostrophe);
							}
						}
					},
					_ => {
						self.components.push_back(Apostrophe);
					}
				}
			}
		}
	}

	fn finish(&mut self) -> Vec<String> {
		use TextComponent::*;
		let mut in_quote = false;
		let mut collated = Vec::new();
		let mut out = String::new();

		while let Some(next) = self.components.pop_front() {
			match next {
				Break => {
					if !out.is_empty() {
						collated.push(std::mem::take(&mut out));
					}
				},
				Apostrophe => {
					out.push('\'');
				},
				LeftQuote => {
					in_quote = true;
					out.push('‘')
				},
				LeftQuoteOrApostrophe => {
					if in_quote {
						out.push('\'');
					} else {
						let closed_in_future = self.components.iter()
							.take_while(|x| match x {
								LeftQuote => false,
								_ => true
							})
							.any(|x| match x {
								RightQuote | RightQuoteOrApostrophe => true,
								_ => false
							});

						if closed_in_future {
							in_quote = true;
							out.push('‘');
						} else {
							out.push('\'');
						}
					}
				},
				RightQuote => {
					in_quote = false;
					out.push('’');
				},
				RightQuoteOrApostrophe => {
					if in_quote {
						let closed_in_future = self.components.iter()
							.take_while(|x| match x {
								LeftQuote | LeftQuoteOrApostrophe => false,
								_ => true
							})
							.any(|x| match x {
								RightQuote | RightQuoteOrApostrophe => true,
								_ => false
							});
						if closed_in_future {
							out.push('\'');
						} else {
							in_quote = false;
							out.push('’');
						}
					} else {
						out.push('\'');
					}
				},
				NonQuote(s) => out.push_str(&s)
			}
		}
		if !out.is_empty() {
			collated.push(out);
		}
		collated
	}
}

static SIGNIFICANT_CHARS: &[&str] = &[
	"...",
	"---",
	"--",
	"'",
	"\"",
	"“",
	"”",
	"‘",
	"’",
	"^",
	"~"
];


lazy_static! {
	static ref SEARCHER: AhoCorasick = AhoCorasickBuilder::new()
		.auto_configure(SIGNIFICANT_CHARS)
		.match_kind(MatchKind::LeftmostFirst)
		.build(SIGNIFICANT_CHARS);
}

pub(crate) fn convert_quotes_in_text_segment<'a, I>(texts: I) -> Vec<Cow<'a, str>>
where
	I: IntoIterator<Item=Cow<'a, str>>
{

	let mut good = Vec::new();
	let mut bad = Vec::new();
	let mut into_bad = false;

	for i in texts.into_iter() {
		if into_bad {
			bad.push(i);
		} else if SEARCHER.is_match(i.as_ref()) {
			bad.push(i);
			into_bad = true;
		} else {
			good.push(i);
		}
	};

	if !bad.is_empty() {
		let mut replacer = Replacer::new();
		for item in bad.iter() {
			replacer.add_str(&item);
		}
		replacer.parse();
		let fixed = replacer.finish();
		good.reserve(fixed.len());
		for item in fixed.into_iter() {
			good.push(Cow::Owned(item));
		}
	}

	good
}




#[cfg(test)]
mod tests {
	use super::*;

	/// replace straight single and double quotes, as well as `---`, `--` and `...`.
	/// Each instance of `src` should be a single entity within which quotes
	/// can be assumed to begin and end, such as a paragraph,
	/// rather than a full document
	fn replace_quotes_ellipsis_and_dashes(src: &str) -> String {
		let mut replacer = Replacer::new();
		replacer.add_str(src);
		replacer.parse();
		replacer.finish()
			.join("\n")
	}

	#[test]
	fn double_quotes_test() {
		let a = replace_quotes_ellipsis_and_dashes("\"Hello world\"");
		let c = replace_quotes_ellipsis_and_dashes("Hello world");
		let d = replace_quotes_ellipsis_and_dashes("\"Hello world");
		let e = replace_quotes_ellipsis_and_dashes("Hello world...");
		let f = replace_quotes_ellipsis_and_dashes("\"Hello world...\"");
		let g = replace_quotes_ellipsis_and_dashes("\"Hello world\"...");
		let h = replace_quotes_ellipsis_and_dashes("\"Hello world\" \"Goodbye world\"");
		let i = replace_quotes_ellipsis_and_dashes("Hello -- world --- dash");
		let j = replace_quotes_ellipsis_and_dashes("\"Hello -- world --- dash\"");

		assert_eq!(a, "“Hello world”");
		assert_eq!(c, "Hello world");
		assert_eq!(d, "“Hello world");
		assert_eq!(e, "Hello world…");
		assert_eq!(f, "“Hello world…”");
		assert_eq!(g, "“Hello world”…");
		assert_eq!(h, "“Hello world” “Goodbye world”");
		assert_eq!(i, "Hello – world — dash");
		assert_eq!(j,  "“Hello – world — dash”");
	}

	#[test]
	fn double_quotes_fixes_boundary_singles() {
		let k = replace_quotes_ellipsis_and_dashes("'Hello World'");
		assert_eq!(k, "‘Hello World’");
	}

	#[test]
	fn music_with_rocks_in() {
		let a = replace_quotes_ellipsis_and_dashes("Rock 'n' roll!");
		assert_eq!(a, "Rock 'n' roll!");
		let b = replace_quotes_ellipsis_and_dashes("Rock n' roll!");
		assert_eq!(b, "Rock n' roll!");
		let c = replace_quotes_ellipsis_and_dashes("Rockin' rolling!");
		assert_eq!(c, "Rockin' rolling!");
		let d = replace_quotes_ellipsis_and_dashes("Rock 'n' roll!");
		assert_eq!(d, "Rock 'n' roll!");
		let e = replace_quotes_ellipsis_and_dashes("Rock ‘n’ roll!");
		assert_eq!(e, "Rock ‘n’ roll!");
		let f = replace_quotes_ellipsis_and_dashes("Rock ‘n' roll!");
		assert_eq!(f, "Rock ‘n’ roll!");
		let g = replace_quotes_ellipsis_and_dashes("Rock 'n’ roll!");
		assert_eq!(g, "Rock ‘n’ roll!");
	}

	#[test]
	fn single_quotes_text() {
		let a = replace_quotes_ellipsis_and_dashes("Hello world");
		assert_eq!(a, "Hello world");
		let b = replace_quotes_ellipsis_and_dashes("''Tisn't a big deal,' he said.");
		assert_eq!(b, "‘'Tisn't a big deal,’ he said.");
		let c = replace_quotes_ellipsis_and_dashes("Book 'em, Danno.");
		assert_eq!(c, "Book 'em, Danno.");
		let d = replace_quotes_ellipsis_and_dashes("'Book 'em, Danno.'");
		assert_eq!(d, "‘Book 'em, Danno.’");
		let e = replace_quotes_ellipsis_and_dashes("'Got a feeling '21 / is going to be a good year'");
		assert_eq!(e, "‘Got a feeling '21 / is going to be a good year’");
		let f = replace_quotes_ellipsis_and_dashes("'Hello World'");
		assert_eq!(f, "‘Hello World’");
		let g = replace_quotes_ellipsis_and_dashes("She wrote: 'It will be,' etc.");
		assert_eq!(g, "She wrote: ‘It will be,’ etc.");
		let h = replace_quotes_ellipsis_and_dashes("'When Mr. Kurtz,' I continued, severely, 'is General Manager, you won't have the opportunity.'");
		assert_eq!(h, "‘When Mr. Kurtz,’ I continued, severely, ‘is General Manager, you won't have the opportunity.’");
		let i = replace_quotes_ellipsis_and_dashes("'There's your Company's station,' said the Swede, pointing to three wooden barrack-like structures on the rocky slope. 'I will send your things up. Four boxes did you say? So. Farewell.'");
		assert_eq!(i, "‘There\'s your Company\'s station,’ said the Swede, pointing to three wooden barrack-like structures on the rocky slope. ‘I will send your things up. Four boxes did you say? So. Farewell.’");
		let j = replace_quotes_ellipsis_and_dashes("'Top o' the mornin' to ya,' said the Argentinian.");
		assert_eq!(j,  "‘Top o' the mornin' to ya,’ said the Argentinian.");
		let k = replace_quotes_ellipsis_and_dashes("'He does not hear.' 'What! Dead?'");
		assert_eq!(k, "‘He does not hear.’ ‘What! Dead?’");
		let l = replace_quotes_ellipsis_and_dashes("“'The last word he pronounced was–your name.'");
		assert_eq!(l, "“‘The last word he pronounced was–your name.’");
		let m = replace_quotes_ellipsis_and_dashes("He became very cool and collected all at\nonce. 'I am not such a fool as I look, quoth Plato to his disciples,'\nhe said sententiously, emptied his glass with great resolution, and we\nrose.");
		assert_eq!(m, "He became very cool and collected all at\nonce. ‘I am not such a fool as I look, quoth Plato to his disciples,’\nhe said sententiously, emptied his glass with great resolution, and we\nrose.");
		let n = replace_quotes_ellipsis_and_dashes("an air of whispering, 'Come and find out.'
This one was almost featureless, as if still in the making, with names like Gran' Bassam.");
		assert_eq!(n, "an air of whispering, ‘Come and find out.’\nThis one was almost featureless, as if still in the making, with names like Gran' Bassam.");
	}
}