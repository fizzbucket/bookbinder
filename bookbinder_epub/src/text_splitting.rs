use crate::svg_titlepage_generator::{
    FontData, SizedText, TitleEvent, TypeSetter, HORIZONTAL_MARGIN, WIDTH,
};
use crate::paragraph_breaker::{total_fit, Item};

const MAX_LINE_WIDTH: i32 = WIDTH - (HORIZONTAL_MARGIN * 2);

fn to_line_items<'a, 'b, I>(
    text: I,
    font_data: &FontData<'a, 'b>,
    italic_font_data: &FontData<'a, 'b>,
    setter: &TypeSetter<'_>,
    italic_setter: &TypeSetter<'_>,
) -> Vec<Item<SizedText<'a>>>
where
    I: Iterator<Item = TitleEvent<'a>>,
{
    let space_width = SizedText::new(&font_data, &setter, " ").width;
    let italic_space_width = SizedText::new(&italic_font_data, &italic_setter, " ").width;

    let mut items = Vec::new();

    // we use the trick set out in *Breaking Paragraphs into Lines*
    // (SOFTWARE-PRACTICE AND EXPERIENCE, VOL. 11,  1119-1184 (1981))
    // p. 1140 to get centred text

    let s3 = space_width * 3;
    let s6 = space_width * 6;
    let is3 = italic_space_width * 3;
    let is6 = italic_space_width * 6;

    let empty_box = SizedText::new(&font_data, &setter, "");

    for item in text {
        match item {
            TitleEvent::Text(t) => {
                let t = t.trim().to_uppercase();
                for word in t.split(' ').map(|s| s.to_string()) {
                    let sized = SizedText::new(&font_data, &setter, word);
                    items.push(Item::Box {
                        width: sized.width,
                        data: sized,
                    });
                    items.push(Item::Glue {
                        width: 0,
                        stretch: s3,
                        shrink: 0,
                    });
                    items.push(Item::Penalty {
                        width: 0,
                        penalty: 0,
                        flagged: false,
                    });
                    items.push(Item::Glue {
                        width: space_width,
                        stretch: -s6,
                        shrink: 0,
                    });
                    items.push(Item::Box {
                        width: 0,
                        data: empty_box.clone(),
                    });
                    items.push(Item::Penalty {
                        width: 0,
                        penalty: 10_000,
                        flagged: false,
                    });
                    items.push(Item::Glue {
                        width: 0,
                        stretch: s3,
                        shrink: 0,
                    });
                }
            }
            TitleEvent::Emphasised(t) => {
                let t = t.trim().to_uppercase();
                for word in t.split(' ').map(|s| s.to_string()) {
                    let sized = SizedText::new(&italic_font_data, &italic_setter, word);
                    items.push(Item::Box {
                        width: sized.width,
                        data: sized,
                    });
                    items.push(Item::Glue {
                        width: 0,
                        stretch: is3,
                        shrink: 0,
                    });
                    items.push(Item::Penalty {
                        width: 0,
                        penalty: 0,
                        flagged: false,
                    });
                    items.push(Item::Glue {
                        width: space_width,
                        stretch: -is6,
                        shrink: 0,
                    });
                    items.push(Item::Box {
                        width: 0,
                        data: empty_box.clone(),
                    });
                    items.push(Item::Penalty {
                        width: 0,
                        penalty: 10_000,
                        flagged: false,
                    });
                    items.push(Item::Glue {
                        width: 0,
                        stretch: is3,
                        shrink: 0,
                    });
                }
            }
        }
    }

    items.pop();
    items.pop();
    items.pop();
    items.pop();
    items.pop();
    items.pop();
    items.push(Item::Glue {
        width: 0,
        stretch: is3,
        shrink: 0,
    });
    items.push(Item::Penalty {
        width: 0,
        penalty: -10_000,
        flagged: false,
    });

    items
}

#[derive(Debug, Clone)]
pub(crate) enum SizedTextOrSpace<'a> {
    Text(SizedText<'a>),
    Space(usize),
}

fn greedy_fallback<'a>(items: Vec<Item<SizedText<'a>>>) -> Vec<Vec<SizedTextOrSpace<'a>>> {
    let mut lines: Vec<Vec<SizedTextOrSpace>> = Vec::new();
    let mut current_line: Vec<SizedTextOrSpace> = Vec::new();

    let mut current_line_width = 0;

    let space_width = items
        .iter()
        .filter_map(|item| match item {
            Item::Glue { width, .. } => {
                let width = *width;
                if width > 0 {
                    Some(width)
                } else {
                    None
                }
            }
            _ => None,
        })
        .next()
        .unwrap_or(10);

    let text = items.into_iter().filter_map(|item| match item {
        Item::Box { data, .. } if !data.text.is_empty() => Some(data),
        _ => None,
    });

    for item in text {
        let width = if !current_line.is_empty() {
            space_width + item.width
        } else {
            item.width
        };

        if width + current_line_width > MAX_LINE_WIDTH {
            lines.push(current_line);
            current_line_width = item.width;
            current_line = vec![SizedTextOrSpace::Text(item)];
        } else {
            current_line.push(SizedTextOrSpace::Space(space_width as usize));
            current_line.push(SizedTextOrSpace::Text(item));
            current_line_width += width;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

pub(crate) fn split_text<'a, 'b, I>(
    text: I,
    font_data: &FontData<'a, 'b>,
    italic_font_data: &FontData<'a, 'b>,
    setter: &TypeSetter<'_>,
    italic_setter: &TypeSetter<'_>,
) -> Vec<Vec<SizedTextOrSpace<'a>>>
where
    I: Iterator<Item = TitleEvent<'a>>,
{
    let items = to_line_items(text, font_data, italic_font_data, setter, italic_setter);
    let lengths = std::iter::repeat(700).take(items.len()).collect::<Vec<_>>();
    let threshold = 8.0;
    let looseness = 0;
    let breakpoints = total_fit(&items, &lengths, threshold, looseness);
    if breakpoints.is_empty() {
        eprintln!("Couldn't fit nicely");
        return greedy_fallback(items);
    };

    let mut processed = Vec::new();
    let mut current_line = Vec::new();

    let breakpoints = breakpoints.into_iter().map(|b| b.index).collect::<Vec<_>>();

    for (i, item) in items.into_iter().enumerate() {
        if breakpoints.contains(&i) {
            match current_line.last() {
                None => {}
                Some(SizedTextOrSpace::Space(_)) => {
                    current_line.pop();
                    processed.push(std::mem::take(&mut current_line));
                }
                Some(_) => {
                    processed.push(std::mem::take(&mut current_line));
                }
            }
        } else {
            match item {
                Item::Box { data, .. } => {
                    if !data.text.is_empty() {
                        current_line.push(SizedTextOrSpace::Text(data));
                    }
                }
                Item::Glue { width, .. } if width > 0 => {
                    current_line.push(SizedTextOrSpace::Space(width as usize));
                }
                _ => {}
            }
        }
    }

    if let Some(SizedTextOrSpace::Space(_)) = current_line.last() {
        current_line.pop();
    }

    if !current_line.is_empty() {
        processed.push(current_line);
    }

    processed
}
