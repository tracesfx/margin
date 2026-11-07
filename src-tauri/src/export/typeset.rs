//! Typst markup for a book (SPEC §12.2). The manuscript stays clean: this
//! turns the export model plus a template into one generated source file,
//! compiled by `world.rs`. The same markup feeds the PDF and the preview.

use super::book::{Book, Piece, plain};
use super::types::{ChapterOpening, ExportProfile, TemplateSettings};
use crate::dialect::{Block, Inline, Mark};
use std::cell::RefCell;

/// Escape text for Typst markup. Every ASCII punctuation mark except the
/// straight quotes (which Typst turns into curly ones) gets a backslash, so
/// nothing in a manuscript can be read as markup, and `[`/`(` after a styled
/// run can never turn into a call.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        if c.is_ascii_punctuation() && c != '\'' && c != '"' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// A Typst string literal.
pub fn lit(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn inches(v: f64) -> String {
    format!("{:.3}in", v)
}

fn font_list(name: &str) -> String {
    let mut fonts = vec![lit(name)];
    for f in ["Libertinus Serif", "New Computer Modern"] {
        if f != name {
            fonts.push(lit(f));
        }
    }
    format!("({})", fonts.join(", "))
}

/// Fonts that ship inside Margin (Typst's own and the OFL book fonts in
/// `src-tauri/fonts/`), so a book looks the same on any computer.
pub fn bundled(font: &str) -> bool {
    matches!(
        font,
        "Libertinus Serif"
            | "New Computer Modern"
            | "DejaVu Sans Mono"
            | "EB Garamond"
            | "Crimson Text"
            | "Source Serif 4"
    )
}

/// Bundled fonts that have real small-capital glyphs. Crimson Text does
/// not, so it gets scaled capitals like any other font.
pub fn has_small_caps(font: &str) -> bool {
    bundled(font) && font != "Crimson Text" && font != "DejaVu Sans Mono"
}

struct Cx<'a> {
    t: &'a TemplateSettings,
    real_sc: bool,
    /// Source map for the Page view: the document being written and the
    /// index of its next text block. `None` = no markers (exports).
    marks: Option<RefCell<(String, u32)>>,
}

impl Cx<'_> {
    /// A comment Typst ignores, naming the text block that follows.
    fn mark(&self, out: &mut String) {
        if let Some(m) = &self.marks {
            let mut m = m.borrow_mut();
            out.push_str(&format!("/*{MARK}{}:{}*/", m.0, m.1));
            m.1 += 1;
        }
    }
    /// A comment ending the previous text block (headings, breaks).
    fn mark_none(&self, out: &mut String) {
        if self.marks.is_some() {
            out.push_str(&format!("/*{MARK}-*/"));
        }
    }
    fn enter(&self, doc: &str) {
        if let Some(m) = &self.marks {
            *m.borrow_mut() = (doc.to_string(), 0);
        }
    }
}

/// Prefix of a source-map comment (`/*§<doc>:<index>*/`).
pub const MARK: &str = "§";

/// Where each text block starts in the markup: (offset after the marker,
/// document id, text block index). Offsets before the first marker, and
/// after a "none" marker, belong to no text block.
pub fn source_map(markup: &str) -> Vec<(usize, Option<(String, u32)>)> {
    let open = format!("/*{MARK}");
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = markup[at..].find(&open) {
        let start = at + i;
        let body = start + open.len();
        let Some(j) = markup[body..].find("*/") else {
            break;
        };
        let label = &markup[body..body + j];
        let end = body + j + 2;
        let target = label
            .split_once(':')
            .and_then(|(d, n)| Some((d.to_string(), n.parse().ok()?)));
        out.push((end, target));
        at = end;
    }
    out
}

///
/// `real_sc`: the font has small-capital glyphs (the bundled ones do). Any other
/// font gets capitals one size down, so a lead-in never silently stays lowercase.
fn inline(content: &[Inline], real_sc: bool) -> String {
    let mut out = String::new();
    for i in content {
        match i {
            Inline::Text { text, marks } => {
                let text = if marks.contains(&Mark::NoBreak) {
                    text.replace(' ', "\u{00A0}")
                } else {
                    text.clone()
                };
                if text.is_empty() {
                    continue;
                }
                let fake_sc = !real_sc && marks.contains(&Mark::SmallCaps);
                let mut s = if fake_sc {
                    format!("#sc({})", lit(&text))
                } else {
                    esc(&text)
                };
                // Innermost first; the mark order is canonical already.
                for m in marks.iter().rev() {
                    s = match m {
                        Mark::Italic => format!("#emph[{s}]"),
                        Mark::Bold => format!("#strong[{s}]"),
                        Mark::SmallCaps if !fake_sc => format!("#smallcaps[{s}]"),
                        Mark::Link { href } => format!("#link({})[{s}]", lit(href)),
                        Mark::SmallCaps | Mark::NoBreak | Mark::Comment { .. } => s,
                    };
                }
                out.push_str(&s);
            }
            Inline::HardBreak => out.push_str("#linebreak()"),
            Inline::Footnote(inner) => {
                out.push_str(&format!("#footnote[{}]", inline(inner, real_sc)))
            }
        }
    }
    out
}

/// Small capitals over roughly the first line.
fn smallcaps_lead(content: &[Inline], words: usize) -> Vec<Inline> {
    let mut left = words;
    let mut out = Vec::new();
    for i in content {
        match i {
            Inline::Text { text, marks } if left > 0 => {
                let mut head = String::new();
                let mut rest = String::new();
                let mut in_word = false;
                for c in text.chars() {
                    if left == 0 {
                        rest.push(c);
                        continue;
                    }
                    head.push(c);
                    if c.is_whitespace() {
                        if in_word {
                            left -= 1;
                            in_word = false;
                        }
                    } else {
                        in_word = true;
                    }
                }
                // A word running to the end of this run counts when the run ends the paragraph.
                if in_word && left > 0 {
                    // keep counting in the next run
                }
                let mut m = marks.clone();
                m.push(Mark::SmallCaps);
                if !head.is_empty() {
                    out.push(Inline::text(head, m));
                }
                if !rest.is_empty() {
                    out.push(Inline::text(rest, marks.clone()));
                }
            }
            other => out.push(other.clone()),
        }
    }
    out
}

/// The first letter enlarged. Leading quotation marks stay small.
fn large_initial(content: &[Inline], real_sc: bool) -> String {
    let mut done = false;
    let mut out = String::new();
    for i in content {
        match i {
            Inline::Text { text, marks } if !done => {
                let mut lead = String::new();
                let mut chars = text.chars().peekable();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() {
                        break;
                    }
                    lead.push(c);
                    chars.next();
                }
                let Some(first) = chars.next() else {
                    out.push_str(&inline(std::slice::from_ref(i), real_sc));
                    continue;
                };
                let rest: String = chars.collect();
                done = true;
                let wrap = |s: String| inline(&[Inline::text(s, marks.clone())], real_sc);
                if !lead.is_empty() {
                    out.push_str(&wrap(lead));
                }
                out.push_str(&format!(
                    "#text(size: 2.3em, weight: \"bold\")[{}]",
                    esc(&first.to_string())
                ));
                if !rest.is_empty() {
                    out.push_str(&wrap(rest));
                }
            }
            other => out.push_str(&inline(std::slice::from_ref(other), real_sc)),
        }
    }
    out
}

struct Flow {
    /// The next paragraph is the first of a section: no indent.
    first: bool,
    /// The next paragraph opens a chapter: apply the chapter opening style.
    opening: bool,
}

fn blocks(cx: &Cx, bs: &[Block], flow: &mut Flow, out: &mut String, flush: bool) {
    for b in bs {
        match b {
            Block::Paragraph(c) => {
                cx.mark(out);
                let body = if flow.opening {
                    match cx.t.chapter_opening {
                        ChapterOpening::Plain => inline(c, cx.real_sc),
                        ChapterOpening::SmallCapsLead => inline(&smallcaps_lead(c, 5), cx.real_sc),
                        ChapterOpening::LargeInitial => large_initial(c, cx.real_sc),
                    }
                } else {
                    inline(c, cx.real_sc)
                };
                if flush || flow.first || flow.opening {
                    out.push_str(&format!("#par(first-line-indent: 0pt)[{body}]\n\n"));
                } else {
                    out.push_str(&format!("{body}\n\n"));
                }
                flow.first = false;
                flow.opening = false;
            }
            Block::Heading { level, content } => {
                cx.mark(out);
                let size = match level {
                    2 => "1.25em",
                    3 => "1.1em",
                    _ => "1em",
                };
                out.push_str(&format!(
                    "#block(above: 1.6em, below: 0.9em, sticky: true)[#text(size: {size}, weight: \"bold\")[{}]]\n\n",
                    inline(content, cx.real_sc)
                ));
                flow.first = true;
            }
            Block::SceneBreak => {
                cx.mark_none(out);
                out.push_str("#scene-break()\n\n");
                flow.first = true;
                flow.opening = false;
            }
            Block::BlockQuote(inner) => {
                out.push_str("#block(above: 1em, below: 1em, inset: (left: 1.6em, right: 1.6em))[\n#set par(first-line-indent: 0pt)\n#text(size: 0.95em)[\n");
                let mut f = Flow {
                    first: true,
                    opening: false,
                };
                blocks(cx, inner, &mut f, out, true);
                out.push_str("]\n]\n\n");
                flow.first = true;
            }
            Block::Div { class, content } => {
                let mut f = Flow {
                    first: true,
                    opening: false,
                };
                let (open, close) = match class.as_str() {
                    "epigraph" => (
                        "#block(above: 1.4em, below: 0.4em, inset: (left: 18%))[\n#set par(first-line-indent: 0pt, justify: false)\n#emph[\n",
                        "]\n]\n\n",
                    ),
                    "attribution" => (
                        "#block(above: 0pt, below: 1.4em, inset: (left: 18%))[\n#set par(first-line-indent: 0pt, justify: false)\n— ",
                        "\n]\n\n",
                    ),
                    "letter" => (
                        "#block(above: 1em, below: 1em, inset: (left: 1.6em, right: 1.6em))[\n#set par(first-line-indent: 0pt)\n#text(size: 0.95em)[\n",
                        "]\n]\n\n",
                    ),
                    "textmsg" => (
                        "#block(above: 0.8em, below: 0.8em, inset: (left: 1.6em, right: 1.6em))[\n#set par(first-line-indent: 0pt, justify: false)\n#text(font: (\"DejaVu Sans Mono\",), size: 0.82em)[\n",
                        "]\n]\n\n",
                    ),
                    "verse" => (
                        "#block(above: 1em, below: 1em, inset: (left: 2em))[\n#set par(first-line-indent: 0pt, justify: false)\n",
                        "]\n\n",
                    ),
                    "center" => (
                        "#align(center)[\n#set par(first-line-indent: 0pt, justify: false)\n",
                        "]\n\n",
                    ),
                    _ => ("#[\n", "]\n\n"),
                };
                out.push_str(open);
                blocks(cx, content, &mut f, out, true);
                out.push_str(close);
                flow.first = true;
            }
            Block::BulletList { items, .. } | Block::OrderedList { items, .. } => {
                let ordered = matches!(b, Block::OrderedList { .. });
                let start = if let Block::OrderedList { start, .. } = b {
                    *start
                } else {
                    1
                };
                let mut body = Vec::new();
                for item in items {
                    let mut s = String::new();
                    let mut f = Flow {
                        first: true,
                        opening: false,
                    };
                    blocks(cx, item, &mut f, &mut s, true);
                    body.push(format!("[{}]", s.trim_end()));
                }
                if ordered {
                    out.push_str(&format!("#enum(start: {start}, {})\n\n", body.join(", ")));
                } else {
                    out.push_str(&format!("#list({})\n\n", body.join(", ")));
                }
                flow.first = true;
            }
        }
    }
}

fn preamble(book: &Book, p: &ExportProfile, has_parts: bool) -> String {
    let t = &p.settings;
    let (w, h) = (t.trim_width_in, t.trim_height_in);
    let (inside, outside, top, bottom) = (w * 0.16, w * 0.12, h * 0.09, h * 0.10);
    let spacing = if t.indent_em > 0.0 {
        t.leading_em
    } else {
        t.paragraph_space_em.max(t.leading_em)
    };
    let costs = if t.widow_orphan {
        ""
    } else {
        ", costs: (widow: 0%, orphan: 0%)"
    };
    let author = if book.author.is_empty() {
        String::new()
    } else {
        format!(", author: {}", lit(&book.author))
    };
    let recto = if t.recto_starts {
        "{ pagebreak(weak: true); context { if calc.even(here().page()) { page(header: none, footer: none)[] } } }"
    } else {
        "pagebreak(weak: true)"
    };
    let _ = has_parts;
    format!(
        r##"#set document(title: {title}{author})
#set text(font: {fonts}, size: {size}pt, lang: "en", region: "us", hyphenate: {hyph}{costs})
#set par(justify: {just}, leading: {lead}em, spacing: {spacing}em, first-line-indent: {indent}em)
#set page(width: {w}, height: {h}, margin: (inside: {inside}, outside: {outside}, top: {top}, bottom: {bottom}), header: none, footer: none)
#show heading: none
#show link: set text(fill: rgb("#1a4a7a"))
#let sc(s) = for c in s.clusters() {{ if lower(c) == c and upper(c) != c {{ text(size: 0.8em)[#upper(c)] }} else {{ c }} }}
#let recto() = {recto}
#let scene-break() = block(above: 1.1em, below: 1.1em, width: 100%)[#align(center)[#text(tracking: 0.1em)[{ornament}]]]
#let chapter-open(lvl, label, title, toc) = {{
  recto()
  heading(level: lvl, outlined: true)[#toc]
  v(14%)
  align(center)[
    #set par(first-line-indent: 0pt, justify: false)
    #if label != none {{ text(size: 0.9em, tracking: 0.14em)[#upper(label)]; parbreak() }}
    #if title != none {{ text(size: 1.7em, weight: "bold")[#title] }}
  ]
  v(2.2em)
}}
#let part-open(title) = {{
  recto()
  heading(level: 1, outlined: true)[#title]
  v(30%)
  align(center)[
    #set par(first-line-indent: 0pt, justify: false)
    #text(size: 2.2em, weight: "bold")[#title]
  ]
  pagebreak(weak: true)
}}
#let matter-open(title) = {{
  recto()
  if title != none {{
    heading(level: 1, outlined: true)[#title]
    v(14%)
    align(center)[#text(size: 1.5em, weight: "bold")[#title]]
    v(2em)
  }}
}}

"##,
        title = lit(&book.title),
        author = author,
        fonts = font_list(&t.font),
        size = t.size_pt,
        hyph = t.hyphenate,
        costs = costs,
        just = t.justify,
        lead = t.leading_em,
        spacing = spacing,
        indent = t.indent_em,
        w = inches(w),
        h = inches(h),
        inside = inches(inside),
        outside = inches(outside),
        top = inches(top),
        bottom = inches(bottom),
        recto = recto,
        ornament = esc(&t.ornament),
    )
}

fn title_pages(book: &Book, p: &ExportProfile, out: &mut String) {
    if p.title_page {
        out.push_str(&format!(
            "#page(header: none, footer: none)[\n#set par(first-line-indent: 0pt, justify: false)\n#align(center + horizon)[\n#text(size: 2.6em, weight: \"bold\")[{}]\n",
            esc(&book.title)
        ));
        if !book.author.is_empty() {
            out.push_str(&format!(
                "#v(2em)\n#text(size: 1.3em)[{}]\n",
                esc(&book.author)
            ));
        }
        out.push_str("]\n]\n\n");
    }
    if !p.copyright.trim().is_empty() {
        let lines: Vec<String> = p.copyright.lines().map(|l| esc(l.trim())).collect();
        out.push_str(&format!(
            "#page(header: none, footer: none)[\n#set par(first-line-indent: 0pt, justify: false)\n#set text(size: 0.85em)\n#align(bottom + center)[{}]\n]\n\n",
            lines.join(" \\\n")
        ));
    }
    if !p.dedication.trim().is_empty() {
        let lines: Vec<String> = p.dedication.lines().map(|l| esc(l.trim())).collect();
        out.push_str(&format!(
            "#page(header: none, footer: none)[\n#set par(first-line-indent: 0pt, justify: false)\n#align(center + horizon)[#emph[{}]]\n]\n\n",
            lines.join(" \\\n")
        ));
    }
    if !p.epigraph.trim().is_empty() {
        let lines: Vec<String> = p.epigraph.lines().map(|l| esc(l.trim())).collect();
        let by = if p.epigraph_by.trim().is_empty() {
            String::new()
        } else {
            format!(
                "\n#v(0.8em)\n#align(right)[— {}]",
                esc(p.epigraph_by.trim())
            )
        };
        out.push_str(&format!(
            "#page(header: none, footer: none)[\n#set par(first-line-indent: 0pt, justify: false)\n#align(center + horizon)[#block(width: 70%)[#emph[{}]{}]]\n]\n\n",
            lines.join(" \\\n"),
            by
        ));
    }
}

/// The whole book as Typst source.
pub fn markup(book: &Book, p: &ExportProfile) -> String {
    markup_with(book, p, false)
}

/// The book as Typst source. With `marks`, every text block is preceded by
/// a comment naming its document and index (see [`source_map`]).
pub fn markup_with(book: &Book, p: &ExportProfile, marks: bool) -> String {
    let t = &p.settings;
    let cx = Cx {
        t,
        real_sc: has_small_caps(&t.font),
        marks: marks.then(|| RefCell::new((String::new(), 0))),
    };
    let has_parts = book.body.iter().any(|x| matches!(x, Piece::Part { .. }));
    let mut out = preamble(book, p, has_parts);
    title_pages(book, p, &mut out);

    if p.toc {
        out.push_str("#page(header: none, footer: none)[\n#set par(first-line-indent: 0pt, justify: false)\n#align(center)[#text(size: 1.5em, weight: \"bold\")[Contents]]\n#v(1.5em)\n#outline(title: none, depth: 2, indent: 1.2em)\n]\n\n");
    }

    let folio = |fmt: &str| {
        if t.page_numbers {
            format!("footer: context align(center)[#counter(page).display(\"{fmt}\")]")
        } else {
            "footer: none".to_string()
        }
    };

    // Front matter documents: roman numerals.
    if !book.front.is_empty() {
        out.push_str(&format!(
            "#set page(numbering: \"i\", header: none, {})\n",
            folio("i")
        ));
        for (piece, doc) in book.front.iter().zip(&book.front_docs) {
            matter(&cx, doc, piece, &mut out);
        }
    }

    // Body: arabic numerals from 1, running heads.
    let head = if t.running_heads {
        format!(
            "header: context {{\n  let p = here().page()\n  if query(heading).any(h => h.location().page() == p) {{ none }} else {{\n    set text(size: 0.85em, tracking: 0.08em)\n    let n = counter(page).get().first()\n    align(center, smallcaps(if calc.even(n) {{ {author} }} else {{ {title} }}))\n  }}\n}}",
            author = lit(if book.author.is_empty() {
                &book.title
            } else {
                &book.author
            }),
            title = lit(&book.title),
        )
    } else {
        "header: none".to_string()
    };
    out.push_str(&format!(
        "#set page(numbering: \"1\", {head}, {})\n",
        folio("1")
    ));
    out.push_str("#counter(page).update(1)\n\n");

    let mut flow = Flow {
        first: true,
        opening: false,
    };
    for (piece, doc) in book.body.iter().zip(&book.body_docs) {
        cx.mark_none(&mut out);
        match piece {
            Piece::Part { title } => {
                out.push_str(&format!("#part-open({})\n\n", lit(title)));
                flow = Flow {
                    first: true,
                    opening: false,
                };
            }
            Piece::Chapter { label, title } => {
                let lvl = if has_parts { 2 } else { 1 };
                let toc = piece.heading_line().unwrap_or_default();
                let opt = |s: &Option<String>| {
                    s.as_ref().map(|s| lit(s)).unwrap_or_else(|| "none".into())
                };
                out.push_str(&format!(
                    "#chapter-open({lvl}, {}, {}, {})\n\n",
                    opt(label),
                    opt(title),
                    lit(&toc)
                ));
                flow = Flow {
                    first: true,
                    opening: true,
                };
            }
            Piece::SceneBreak => {
                out.push_str("#scene-break()\n\n");
                flow.first = true;
                flow.opening = false;
            }
            Piece::Blocks(b) => {
                cx.enter(doc);
                blocks(&cx, b, &mut flow, &mut out, false);
            }
            Piece::Matter { .. } => matter(&cx, doc, piece, &mut out),
        }
    }

    // Back matter.
    for (piece, doc) in book.back.iter().zip(&book.back_docs) {
        matter(&cx, doc, piece, &mut out);
    }
    out
}

fn matter(cx: &Cx, doc: &str, piece: &Piece, out: &mut String) {
    if let Piece::Matter { title, blocks: b } = piece {
        cx.mark_none(out);
        cx.enter(doc);
        let t = title
            .as_ref()
            .map(|t| lit(t))
            .unwrap_or_else(|| "none".into());
        out.push_str(&format!("#matter-open({t})\n\n"));
        let mut flow = Flow {
            first: true,
            opening: false,
        };
        blocks(cx, b, &mut flow, out, false);
    }
}

/// Plain text of the first few words, for tests.
#[allow(dead_code)]
pub fn first_words(content: &[Inline], n: usize) -> String {
    plain(content)
        .split_whitespace()
        .take(n)
        .collect::<Vec<_>>()
        .join(" ")
}
