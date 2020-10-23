use lazy_static::lazy_static;
use pulldown_cmark::CowStr;
use regex::Captures;
use regex::{Regex, RegexSet};
use std::borrow::Cow;

static SUPERSCRIPT: &str = r#"\^([[:alnum:]]+)\^"#;
static SUBSCRIPT: &str = r#"~([[:alnum:]]+)~"#;

lazy_static! {
    pub(crate) static ref REGGIE: RegexSet = RegexSet::new(&[SUPERSCRIPT, SUBSCRIPT]).unwrap();
    static ref SUPERSCRIPT_REGEX: Regex = Regex::new(SUPERSCRIPT).unwrap();
    static ref SUBSCRIPT_REGEX: Regex = Regex::new(SUBSCRIPT).unwrap();
}

macro_rules! replace_script {
    ($class:expr, $fnname:ident) => {
        fn $fnname(caps: &Captures) -> String {
            let mut out = String::from("<span class=\"");
            out.reserve($class.len() + 2);
            out.push_str($class);
            out.push_str("\">");
            let mut matches = caps.iter();
            matches.next(); // skip match 0
            for mat in matches {
                if let Some(mat) = mat {
                    out.push_str(mat.as_str());
                }
            }

            out.push_str("</span>");
            out
        }
    };
}

replace_script!("superscript", replace_superscript);
replace_script!("subscript", replace_subscript);

fn replace<'a>(
    text: Cow<'a, str>,
    regex: &Regex,
    replace_func: fn(&Captures) -> String,
) -> Cow<'a, str> {
    match regex.replace_all(&text, replace_func) {
        Cow::Borrowed(_) => text,
        Cow::Owned(o) => Cow::Owned(o),
    }
}

pub(crate) fn disambiguate_sub_and_superscript(text: CowStr<'_>) -> Cow<'_, str> {
    let mut text = match text {
        CowStr::Boxed(b) => Cow::Owned(b.to_string()),
        CowStr::Inlined(i) => Cow::Owned(i.to_string()),
        CowStr::Borrowed(b) => Cow::Borrowed(b),
    };
    text = replace(text, &SUPERSCRIPT_REGEX, replace_superscript);
    text = replace(text, &SUBSCRIPT_REGEX, replace_subscript);
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reggie_testing() {
        let a = REGGIE.is_match("Plain and ^super^");
        let b = REGGIE.is_match("Plain and ~sub~");
        let c = REGGIE.is_match("Plain and ~in valid~");
        let d = REGGIE.is_match("Plain and ^in valid^");
        let e = REGGIE.is_match("Plain^ and unmatched ~");
        assert!(a);
        assert!(b);
        assert!(!c);
        assert!(!d);
        assert!(!e);
    }

    #[test]
    fn sub_and_super_byte_boundaries() {
        let text = "~2~ ‘$’ Hello’^21^’ go on";
        let processed = disambiguate_sub_and_superscript(text.into());
        assert_eq!(processed, "<span class=\"subscript\">2</span> ‘$’ Hello’<span class=\"superscript\">21</span>’ go on");
    }

    #[test]
    fn sub_and_superscript() {
        let text = r"H~2~O 25^th^ Hello^not super^. Goodbye~not_sub~.";
        let processed = disambiguate_sub_and_superscript(text.into());
        let expected = "H<span class=\"subscript\">2</span>O 25<span class=\"superscript\">th</span> Hello^not super^. Goodbye~not_sub~.";
        assert_eq!(expected, processed);
    }
}
