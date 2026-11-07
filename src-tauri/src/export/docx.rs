//! DOCX export (SPEC §12.4): Standard Manuscript Format and a styled
//! document. Semantic styles become named Word styles in both, so the
//! manuscript can be restyled in Word with one change per style.

use std::io::Cursor;

use docx_rs::*;

use super::book::{Book, Piece, surname};
use super::types::{BookFormat, ExportProfile};
use crate::dialect::{Block, Inline, Mark};
use crate::error::{MarginError, Result};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Smf,
    Styled,
}

// Style ids. Names are what Word shows.
const BODY: (&str, &str) = ("BodyText", "Body Text");
const FIRST: (&str, &str) = ("FirstParagraph", "First Paragraph");
const CHAPTER_LABEL: (&str, &str) = ("ChapterLabel", "Chapter Number");
const CHAPTER: (&str, &str) = ("ChapterTitle", "Chapter Title");
const PART: (&str, &str) = ("PartTitle", "Part Title");
const SCENE: (&str, &str) = ("SceneBreak", "Scene Break");
const QUOTE: (&str, &str) = ("BlockQuote", "Block Quote");
const EPIGRAPH: (&str, &str) = ("Epigraph", "Epigraph");
const ATTRIB: (&str, &str) = ("EpigraphAttribution", "Epigraph Attribution");
const LETTER: (&str, &str) = ("Letter", "Letter");
const TEXTMSG: (&str, &str) = ("TextMessage", "Text Message");
const VERSE: (&str, &str) = ("Verse", "Verse");
const CENTERED: (&str, &str) = ("Centered", "Centered");
const MATTER: (&str, &str) = ("MatterTitle", "Matter Title");
const BOOK_TITLE: (&str, &str) = ("BookTitle", "Book Title");
const BYLINE: (&str, &str) = ("Byline", "Byline");
const LIST: (&str, &str) = ("ListItem", "List Item");
const SMALL_PRINT: (&str, &str) = ("FrontMatterText", "Front Matter Text");

fn heading_style(level: u8) -> (String, String) {
    (format!("Heading{level}"), format!("heading {level}"))
}

struct Look {
    font: String,
    /// Half-points.
    size: usize,
    line: i32,
    /// First-line indent of body paragraphs.
    indent: i32,
    /// Space after paragraphs.
    after: u32,
    justify: bool,
}

fn fonts(name: &str) -> RunFonts {
    RunFonts::new()
        .ascii(name)
        .hi_ansi(name)
        .east_asia(name)
        .cs(name)
}

fn spacing(line: i32, before: u32, after: u32) -> LineSpacing {
    LineSpacing::new()
        .line_rule(LineSpacingType::Auto)
        .line(line)
        .before(before)
        .after(after)
}

fn style(id: (&str, &str), look: &Look) -> Style {
    Style::new(id.0, StyleType::Paragraph)
        .name(id.1)
        .q_format(true)
        .fonts(fonts(&look.font))
        .size(look.size)
}

fn define_styles(mut docx: Docx, look: &Look, mode: Mode) -> Docx {
    let body_align = if look.justify {
        AlignmentType::Both
    } else {
        AlignmentType::Left
    };
    let single = if mode == Mode::Smf { 240 } else { look.line };
    let styles = vec![
        style(BODY, look)
            .align(body_align)
            .line_spacing(spacing(look.line, 0, look.after))
            .indent(
                None,
                Some(SpecialIndentType::FirstLine(look.indent)),
                None,
                None,
            ),
        style(FIRST, look)
            .align(body_align)
            .line_spacing(spacing(look.line, 0, look.after)),
        style(CHAPTER_LABEL, look)
            .align(AlignmentType::Center)
            .line_spacing(spacing(
                single,
                if mode == Mode::Smf { 2880 } else { 1800 },
                120,
            ))
            .size(look.size + 4)
            .outline_lvl(0),
        style(CHAPTER, look)
            .align(AlignmentType::Center)
            .line_spacing(spacing(single, 0, 480))
            .size(look.size + 8)
            .bold()
            .outline_lvl(0),
        style(PART, look)
            .align(AlignmentType::Center)
            .line_spacing(spacing(single, 2880, 720))
            .size(look.size + 12)
            .bold()
            .outline_lvl(0),
        style(SCENE, look)
            .align(AlignmentType::Center)
            .line_spacing(spacing(look.line, 0, look.after)),
        style(QUOTE, look)
            .align(body_align)
            .line_spacing(spacing(look.line, 0, look.after))
            .indent(Some(720), None, Some(720), None),
        style(EPIGRAPH, look)
            .italic()
            .align(AlignmentType::Left)
            .line_spacing(spacing(single, 0, 120))
            .indent(Some(1440), None, None, None),
        style(ATTRIB, look)
            .align(AlignmentType::Left)
            .line_spacing(spacing(single, 0, 240))
            .indent(Some(1440), None, None, None),
        style(LETTER, look)
            .align(AlignmentType::Left)
            .line_spacing(spacing(look.line, 0, look.after))
            .indent(Some(720), None, Some(720), None),
        style(TEXTMSG, look)
            .align(AlignmentType::Left)
            .line_spacing(spacing(single, 0, 120))
            .indent(Some(720), None, Some(720), None),
        style(VERSE, look)
            .align(AlignmentType::Left)
            .line_spacing(spacing(look.line, 0, look.after))
            .indent(Some(720), None, None, None),
        style(CENTERED, look)
            .align(AlignmentType::Center)
            .line_spacing(spacing(look.line, 0, look.after)),
        style(MATTER, look)
            .align(AlignmentType::Center)
            .line_spacing(spacing(single, 1440, 480))
            .size(look.size + 6)
            .bold()
            .outline_lvl(0),
        style(BOOK_TITLE, look)
            .align(AlignmentType::Center)
            .line_spacing(spacing(single, 0, 240))
            .size(look.size + 16)
            .bold(),
        style(BYLINE, look)
            .align(AlignmentType::Center)
            .line_spacing(spacing(single, 0, 240))
            .size(look.size + 2),
        style(LIST, look)
            .align(AlignmentType::Left)
            .line_spacing(spacing(look.line, 0, 60))
            .indent(Some(720), Some(SpecialIndentType::Hanging(360)), None, None),
        style(SMALL_PRINT, look)
            .align(AlignmentType::Center)
            .line_spacing(spacing(single, 0, 120))
            .size(look.size.saturating_sub(2)),
    ];
    for s in styles {
        docx = docx.add_style(s);
    }
    for level in 2u8..=4 {
        let (id, name) = heading_style(level);
        let s = Style::new(&id, StyleType::Paragraph)
            .name(&name)
            .q_format(true)
            .fonts(fonts(&look.font))
            .size(look.size + 6 - (level as usize - 2) * 2)
            .bold()
            .align(AlignmentType::Left)
            .line_spacing(spacing(single, 240, 120))
            .outline_lvl(level as usize - 1);
        docx = docx.add_style(s);
    }
    docx
}

// --- Inline content ----------------------------------------------------------------

struct Cx {
    font: String,
    size: usize,
}

fn text_run(text: &str, marks: &[Mark], cx: &Cx) -> Run {
    let small_caps = marks.contains(&Mark::SmallCaps);
    let shown = if small_caps {
        text.to_uppercase()
    } else {
        text.to_string()
    };
    let nb = marks.contains(&Mark::NoBreak);
    let shown = if nb {
        shown.replace(' ', "\u{00A0}")
    } else {
        shown
    };
    let mut run = Run::new().add_text(shown).fonts(fonts(&cx.font));
    // Word has small caps, docx-rs does not: capitals one size down.
    run = run.size(if small_caps {
        cx.size * 85 / 100
    } else {
        cx.size
    });
    if marks.contains(&Mark::Italic) {
        run = run.italic();
    }
    if marks.contains(&Mark::Bold) {
        run = run.bold();
    }
    run
}

fn fill(mut p: Paragraph, content: &[Inline], cx: &Cx) -> Paragraph {
    for i in content {
        match i {
            Inline::Text { text, marks } => {
                let link = marks.iter().find_map(|m| match m {
                    Mark::Link { href } => Some(href.clone()),
                    _ => None,
                });
                match link {
                    Some(href) if href.starts_with("http") || href.starts_with("mailto:") => {
                        p = p.add_hyperlink(
                            Hyperlink::new(href, HyperlinkType::External).add_run(
                                text_run(text, marks, cx)
                                    .color("0563C1")
                                    .underline("single"),
                            ),
                        );
                    }
                    _ => p = p.add_run(text_run(text, marks, cx)),
                }
            }
            Inline::HardBreak => p = p.add_run(Run::new().add_break(BreakType::TextWrapping)),
            Inline::Footnote(inner) => {
                let note = fill(Paragraph::new().style("FootnoteText"), inner, cx);
                let mut f = Footnote::new();
                let f = f.add_content(note);
                p = p.add_run(Run::new().add_footnote_reference(f));
            }
        }
    }
    p
}

// --- Blocks ----------------------------------------------------------------------------

struct Out {
    docx: Docx,
    /// The next body paragraph opens a section and is not indented.
    first: bool,
    sep: String,
}

impl Out {
    fn push(&mut self, p: Paragraph) {
        self.docx = std::mem::take(&mut self.docx).add_paragraph(p);
    }

    fn para(&mut self, id: (&str, &str), content: &[Inline], cx: &Cx) {
        self.push(fill(Paragraph::new().style(id.0), content, cx));
    }

    fn line(&mut self, id: (&str, &str), text: &str, cx: &Cx) {
        let run = Run::new().add_text(text).fonts(fonts(&cx.font));
        self.push(Paragraph::new().style(id.0).add_run(run));
    }
}

fn blocks(out: &mut Out, bs: &[Block], cx: &Cx, base: Option<(&str, &str)>) {
    for b in bs {
        match b {
            Block::Paragraph(c) => {
                let id = match base {
                    Some(id) => id,
                    None if out.first => FIRST,
                    None => BODY,
                };
                out.para(id, c, cx);
                if base.is_none() {
                    out.first = false;
                }
            }
            Block::Heading { level, content } => {
                let (id, name) = heading_style((*level).clamp(2, 4));
                out.push(fill(Paragraph::new().style(&id), content, cx).keep_next(true));
                let _ = name;
                out.first = true;
            }
            Block::SceneBreak => {
                let sep = out.sep.clone();
                out.line(SCENE, &sep, cx);
                out.first = true;
            }
            Block::BlockQuote(inner) => blocks(out, inner, cx, Some(QUOTE)),
            Block::Div { class, content } => {
                let id = match class.as_str() {
                    "epigraph" => EPIGRAPH,
                    "attribution" => ATTRIB,
                    "letter" => LETTER,
                    "textmsg" => TEXTMSG,
                    "verse" => VERSE,
                    "center" => CENTERED,
                    _ => BODY,
                };
                blocks(out, content, cx, Some(id));
            }
            Block::BulletList { items, .. } => {
                for item in items {
                    list_item(out, item, "•\t", cx);
                }
            }
            Block::OrderedList { start, items, .. } => {
                for (n, item) in items.iter().enumerate() {
                    list_item(out, item, &format!("{}.\t", *start as usize + n), cx);
                }
            }
        }
    }
}

fn list_item(out: &mut Out, item: &[Block], marker: &str, cx: &Cx) {
    let mut first = true;
    for b in item {
        if let Block::Paragraph(c) = b {
            let mut p = Paragraph::new().style(LIST.0);
            if first {
                p = p.add_run(Run::new().add_text(marker).fonts(fonts(&cx.font)).add_tab());
            }
            out.push(fill(p, c, cx));
            first = false;
        } else {
            blocks(out, std::slice::from_ref(b), cx, None);
        }
    }
}

fn page_field() -> Run {
    Run::new()
        .add_field_char(FieldCharType::Begin, false)
        .add_instr_text(InstrText::PAGE(InstrPAGE::new()))
        .add_field_char(FieldCharType::Separate, false)
        .add_text("1")
        .add_field_char(FieldCharType::End, false)
}

fn rounded_words(n: u32) -> String {
    let step = if n >= 100_000 { 1000 } else { 100 };
    let r = ((n + step / 2) / step) * step;
    let digits = r.to_string();
    let mut s = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            s.push(',');
        }
        s.push(c);
    }
    s
}

fn short_title(title: &str) -> String {
    let t = title.trim().to_uppercase();
    if t.chars().count() <= 28 {
        return t;
    }
    t.split_whitespace().take(3).collect::<Vec<_>>().join(" ")
}

pub fn export(book: &Book, p: &ExportProfile) -> Result<Vec<u8>> {
    let mode = if p.format == BookFormat::DocxManuscript {
        Mode::Smf
    } else {
        Mode::Styled
    };
    let look = match mode {
        Mode::Smf => Look {
            font: if p.manuscript_font.trim().is_empty() {
                "Times New Roman".into()
            } else {
                p.manuscript_font.clone()
            },
            size: 24,
            line: 480,
            indent: 720,
            after: 0,
            justify: false,
        },
        Mode::Styled => Look {
            font: p.settings.font.clone(),
            size: (p.settings.size_pt * 2.0).round() as usize,
            line: 276,
            indent: (p.settings.indent_em * p.settings.size_pt * 20.0).round() as i32,
            after: (p.settings.paragraph_space_em * p.settings.size_pt * 20.0).round() as u32,
            justify: p.settings.justify,
        },
    };
    let cx = Cx {
        font: look.font.clone(),
        size: look.size,
    };

    let mut docx = Docx::new()
        .default_fonts(fonts(&look.font))
        .default_size(look.size)
        .page_size(12240, 15840)
        .page_margin(
            PageMargin::new()
                .top(1440)
                .bottom(1440)
                .left(1440)
                .right(1440)
                .header(720)
                .footer(720),
        );
    docx = define_styles(docx, &look, mode);
    docx = docx.add_style(
        Style::new("FootnoteText", StyleType::Paragraph)
            .name("footnote text")
            .fonts(fonts(&look.font))
            .size(look.size.saturating_sub(4).max(16)),
    );

    let who = surname(&book.author);
    match mode {
        Mode::Smf => {
            let head = format!(
                "{} / {} / ",
                if who.is_empty() { "Author" } else { &who },
                short_title(&book.title)
            );
            let header = Header::new().add_paragraph(
                Paragraph::new()
                    .align(AlignmentType::Right)
                    .add_run(
                        Run::new()
                            .add_text(head)
                            .fonts(fonts(&look.font))
                            .size(look.size),
                    )
                    .add_run(page_field().fonts(fonts(&look.font)).size(look.size)),
            );
            docx = docx.title_pg().header(header).first_header(Header::new());
        }
        Mode::Styled => {
            let footer = Footer::new().add_paragraph(
                Paragraph::new()
                    .align(AlignmentType::Center)
                    .add_run(page_field().fonts(fonts(&look.font)).size(look.size)),
            );
            docx = docx.footer(footer);
        }
    }

    let mut out = Out {
        docx,
        first: true,
        sep: if p.scene_separator.trim().is_empty() {
            "#".into()
        } else {
            p.scene_separator.clone()
        },
    };

    // --- first page ---
    if mode == Mode::Smf {
        for line in [
            p.contact.name.as_str(),
            p.contact.address.as_str(),
            p.contact.email.as_str(),
            p.contact.phone.as_str(),
        ] {
            for l in line.lines().filter(|l| !l.trim().is_empty()) {
                out.push(
                    Paragraph::new().line_spacing(spacing(240, 0, 0)).add_run(
                        Run::new()
                            .add_text(l.trim())
                            .fonts(fonts(&look.font))
                            .size(look.size),
                    ),
                );
            }
        }
        out.push(
            Paragraph::new()
                .align(AlignmentType::Right)
                .line_spacing(spacing(240, 0, 0))
                .add_run(
                    Run::new()
                        .add_text(format!("About {} words", rounded_words(book.words)))
                        .fonts(fonts(&look.font))
                        .size(look.size),
                ),
        );
        out.push(
            Paragraph::new()
                .style(BOOK_TITLE.0)
                .line_spacing(spacing(240, 3600, 240))
                .add_run(
                    Run::new()
                        .add_text(book.title.to_uppercase())
                        .fonts(fonts(&look.font))
                        .size(look.size)
                        .bold(),
                ),
        );
        if !book.author.is_empty() {
            out.push(
                Paragraph::new().style(BYLINE.0).add_run(
                    Run::new()
                        .add_text(format!("by {}", book.author))
                        .fonts(fonts(&look.font))
                        .size(look.size),
                ),
            );
        }
    } else if p.title_page {
        out.push(
            Paragraph::new()
                .style(BOOK_TITLE.0)
                .line_spacing(spacing(276, 3600, 240))
                .add_run(
                    Run::new()
                        .add_text(&book.title)
                        .fonts(fonts(&look.font))
                        .size(look.size + 16)
                        .bold(),
                ),
        );
        if !book.author.is_empty() {
            out.push(
                Paragraph::new().style(BYLINE.0).add_run(
                    Run::new()
                        .add_text(&book.author)
                        .fonts(fonts(&look.font))
                        .size(look.size + 2),
                ),
            );
        }
        if !p.copyright.trim().is_empty() {
            let mut first = true;
            for l in p.copyright.lines() {
                let mut para = Paragraph::new().style(SMALL_PRINT.0);
                if first {
                    para = para
                        .page_break_before(true)
                        .line_spacing(spacing(276, 7200, 120));
                    first = false;
                }
                out.push(
                    para.add_run(
                        Run::new()
                            .add_text(l)
                            .fonts(fonts(&look.font))
                            .size(look.size.saturating_sub(2)),
                    ),
                );
            }
        }
        if !p.dedication.trim().is_empty() {
            let mut first = true;
            for l in p.dedication.lines() {
                let mut para = Paragraph::new().style(CENTERED.0);
                if first {
                    para = para
                        .page_break_before(true)
                        .line_spacing(spacing(276, 3600, 120));
                    first = false;
                }
                out.push(
                    para.add_run(
                        Run::new()
                            .add_text(l)
                            .italic()
                            .fonts(fonts(&look.font))
                            .size(look.size),
                    ),
                );
            }
        }
        if !p.epigraph.trim().is_empty() {
            let mut first = true;
            for l in p.epigraph.lines() {
                let mut para = Paragraph::new().style(EPIGRAPH.0);
                if first {
                    para = para
                        .page_break_before(true)
                        .line_spacing(spacing(276, 3600, 120));
                    first = false;
                }
                out.push(
                    para.add_run(
                        Run::new()
                            .add_text(l)
                            .italic()
                            .fonts(fonts(&look.font))
                            .size(look.size),
                    ),
                );
            }
            if !p.epigraph_by.trim().is_empty() {
                out.line(ATTRIB, &format!("— {}", p.epigraph_by.trim()), &cx);
            }
        }
        if p.toc {
            out.push(
                Paragraph::new()
                    .style(MATTER.0)
                    .page_break_before(true)
                    .add_run(
                        Run::new()
                            .add_text("Contents")
                            .fonts(fonts(&look.font))
                            .size(look.size + 6)
                            .bold(),
                    ),
            );
            out.docx = std::mem::take(&mut out.docx).add_table_of_contents(
                TableOfContents::with_instr_text("TOC \\o \"1-1\" \\h \\z \\u")
                    .auto()
                    .dirty(),
            );
        }
    }

    // --- the book ---
    let mut started = false;
    let emit = |out: &mut Out, piece: &Piece, started: &mut bool| {
        match piece {
            Piece::Part { title } => {
                out.push(
                    Paragraph::new()
                        .style(PART.0)
                        .page_break_before(true)
                        .add_run(
                            Run::new()
                                .add_text(title)
                                .fonts(fonts(&cx.font))
                                .size(cx.size + 12)
                                .bold(),
                        ),
                );
                out.first = true;
            }
            Piece::Chapter { label, title } => {
                let mut break_pending = true;
                if let Some(l) = label {
                    out.push(
                        Paragraph::new()
                            .style(CHAPTER_LABEL.0)
                            .page_break_before(true)
                            .add_run(
                                Run::new()
                                    .add_text(l)
                                    .fonts(fonts(&cx.font))
                                    .size(cx.size + 4),
                            ),
                    );
                    break_pending = false;
                }
                if let Some(t) = title {
                    let mut para = Paragraph::new().style(CHAPTER.0);
                    if break_pending {
                        para = para.page_break_before(true).line_spacing(spacing(
                            if mode == Mode::Smf { 240 } else { look.line },
                            if mode == Mode::Smf { 2880 } else { 1800 },
                            480,
                        ));
                    }
                    out.push(
                        para.add_run(
                            Run::new()
                                .add_text(t)
                                .fonts(fonts(&cx.font))
                                .size(cx.size + 8)
                                .bold(),
                        ),
                    );
                }
                out.first = true;
            }
            Piece::SceneBreak => {
                let sep = out.sep.clone();
                out.line(SCENE, &sep, &cx);
                out.first = true;
            }
            Piece::Blocks(b) => {
                if mode == Mode::Smf && !*started {
                    // The first chapter's heading starts its own page already.
                }
                blocks(out, b, &cx, None);
            }
            Piece::Matter { title, blocks: b } => {
                if let Some(t) = title {
                    out.push(
                        Paragraph::new()
                            .style(MATTER.0)
                            .page_break_before(true)
                            .add_run(
                                Run::new()
                                    .add_text(t)
                                    .fonts(fonts(&cx.font))
                                    .size(cx.size + 6)
                                    .bold(),
                            ),
                    );
                }
                out.first = true;
                blocks(out, b, &cx, None);
            }
        }
        *started = true;
    };
    if mode == Mode::Styled || p.include_front_matter {
        for piece in &book.front {
            emit(&mut out, piece, &mut started);
        }
    }
    // SMF: the story begins on its own page after the title block.
    if mode == Mode::Smf
        && !matches!(
            book.body.first(),
            Some(Piece::Chapter { .. } | Piece::Part { .. })
        )
    {
        out.push(Paragraph::new().page_break_before(true));
    }
    for piece in &book.body {
        emit(&mut out, piece, &mut started);
    }
    if mode == Mode::Styled || p.include_back_matter {
        for piece in &book.back {
            emit(&mut out, piece, &mut started);
        }
    }
    if mode == Mode::Smf {
        out.push(
            Paragraph::new()
                .align(AlignmentType::Center)
                .line_spacing(spacing(480, 240, 0))
                .add_run(
                    Run::new()
                        .add_text("THE END")
                        .fonts(fonts(&look.font))
                        .size(look.size),
                ),
        );
    }

    let mut buf = Cursor::new(Vec::new());
    out.docx
        .build()
        .pack(&mut buf)
        .map_err(|e| MarginError::Internal(format!("could not build the Word file: {e}")))?;
    Ok(buf.into_inner())
}
