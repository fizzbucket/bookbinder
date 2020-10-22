This book is intended for testing purposes. It includes a variety of constructs and text events such that, when rendered to a particular format, most possibilities are tried (here is a footnote, here are stacked blockquotes, here is a header with an embedded link, etc).

It is not necessarily for use in automated testing, so much as to provide a final sanity check to be looked over by human eyes, checking that no unexpected regressions have appeared.

So we might as well begin!

### Escaping text

Many special characters will need to be managed. These include the greater-than, less-than and ampersand (for HTML). These are, respectively, <, > and &.

Even more characters will need to be escaped for LaTeX; some of these are Unicode chars perhaps better expressed with TeX constructs ('…', '–', '—', '-'). Others are control characters: '&', '%', '$', '#', '_', '{', '}', '[' and ']'. And some just might be difficult and so should probably be escaped just in case: '~', '^', '\\'.