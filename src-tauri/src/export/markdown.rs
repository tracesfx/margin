//! Markdown and plain-text export (SPEC §12.4, first in the build order).

use super::book::{Book, Piece, safe_name};
use super::types::{BookFormat, ExportProfile, ItalicStyle};
use crate::dialect::{self, Block, Document, Inline, Mark};

pub struct OutFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Groups of pieces, one per output file. One group when not splitting.
fn groups(book: &Book, split: bool) -> Vec<(String, Vec<&Piece>)> {
    let has_chapters = book.body.iter().any(|p| matches!(p, Piece::Chapter { .. }));
    if !split || !has_chapters {
        let all = book
            .front
            .iter()
            .chain(&book.body)
            .chain(&book.back)
            .collect();
        return vec![(String::new(), all)];
    }
    let mut out: Vec<(String, Vec<&Piece>)> = Vec::new();
    if !book.front.is_empty() {
        out.push(("Front matter".into(), book.front.iter().collect()));
    }
    let mut cur: Option<(String, Vec<&Piece>)> = None;
    let mut parts: Vec<&Piece> = Vec::new();
    for p in &book.body {
        match p {
            Piece::Part { .. } => {
                if let Some(c) = cur.take() {
                    out.push(c);
                }
                parts.push(p);
            }
            Piece::Chapter { .. } => {
                if let Some(c) = cur.take() {
                    out.push(c);
                }
                let mut v = std::mem::take(&mut parts);
                v.push(p);
                cur = Some((p.heading_line().unwrap_or_else(|| "Chapter".into()), v));
            }
            _ => match cur.as_mut() {
                Some((_, v)) => v.push(p),
                None => parts.push(p),
            },
        }
    }
    if let Some(c) = cur.take() {
        out.push(c);
    }
    if !parts.is_empty() {
        out.push(("Untitled".into(), parts));
    }
    if !book.back.is_empty() {
        out.push(("Back matter".into(), book.back.iter().collect()));
    }
    out
}

fn file_for(i: usize, count: usize, base: &str, label: &str, ext: &str) -> String {
    if count == 1 {
        format!("{base}.{ext}")
    } else {
        let width = count.to_string().len().max(2);
        format!("{:0width$} - {}.{ext}", i + 1, safe_name(label))
    }
}

pub fn export(book: &Book, p: &ExportProfile, base: &str) -> Vec<OutFile> {
    let text = p.format == BookFormat::Text;
    let ext = p.format.extension();
    let gs = groups(book, p.split_chapters);
    let count = gs.len();
    gs.iter()
        .enumerate()
        .map(|(i, (label, pieces))| {
            let first = i == 0;
            let body = if text {
                to_text(book, p, pieces, first)
            } else {
                to_markdown(book, p, pieces, first)
            };
            OutFile {
                name: file_for(i, count, base, label, ext),
                bytes: body.into_bytes(),
            }
        })
        .collect()
}

// --- Markdown -------------------------------------------------------------------

fn md_blocks(blocks: &[Block]) -> String {
    dialect::serialize(&Document {
        blocks: blocks.to_vec(),
    })
    .trim_end()
    .to_string()
}

fn to_markdown(book: &Book, p: &ExportProfile, pieces: &[&Piece], first: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    if first {
        let mut head = format!("---\ntitle: {}\n", yaml(&book.title));
        if !book.author.is_empty() {
            head.push_str(&format!("author: {}\n", yaml(&book.author)));
        }
        head.push_str("---");
        parts.push(head);
        if p.title_page {
            front_text(p, &mut parts);
        }
    }
    for piece in pieces {
        match piece {
            Piece::Part { title } => parts.push(format!("# {}", md_line(title))),
            Piece::Chapter { .. } => {
                if let Some(h) = piece.heading_line() {
                    parts.push(format!("## {}", md_line(&h)));
                }
            }
            Piece::SceneBreak => parts.push(p.scene_separator.clone()),
            Piece::Blocks(b) => parts.push(md_blocks(b)),
            Piece::Matter { title, blocks } => {
                if let Some(t) = title {
                    parts.push(format!("## {}", md_line(t)));
                }
                parts.push(md_blocks(blocks));
            }
        }
    }
    parts.retain(|s| !s.is_empty());
    let mut s = parts.join("\n\n");
    s.push('\n');
    s
}

fn yaml(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn md_line(s: &str) -> String {
    s.replace('\n', " ")
}

/// Dedication, epigraph and copyright text from the profile, as paragraphs.
fn front_text(p: &ExportProfile, parts: &mut Vec<String>) {
    if !p.dedication.trim().is_empty() {
        parts.push(p.dedication.trim().to_string());
    }
    if !p.epigraph.trim().is_empty() {
        let mut e = p.epigraph.trim().to_string();
        if !p.epigraph_by.trim().is_empty() {
            e.push_str(&format!("\n— {}", p.epigraph_by.trim()));
        }
        parts.push(e);
    }
    if !p.copyright.trim().is_empty() {
        parts.push(p.copyright.trim().to_string());
    }
}

// --- Plain text -------------------------------------------------------------------

struct TextCtx {
    italics: ItalicStyle,
    notes: Vec<String>,
}

fn inline_text(content: &[Inline], cx: &mut TextCtx) -> String {
    let mut out = String::new();
    for i in content {
        match i {
            Inline::Text { text, marks } => {
                let em = marks.contains(&Mark::Italic);
                let mark = match (em, cx.italics) {
                    (true, ItalicStyle::Underscores) => "_",
                    (true, ItalicStyle::Asterisks) => "*",
                    _ => "",
                };
                if mark.is_empty() || text.trim().is_empty() {
                    out.push_str(text);
                } else {
                    // Keep the marks tight against the words.
                    let lead = text.len() - text.trim_start().len();
                    let trail = text.len() - text.trim_end().len();
                    out.push_str(&text[..lead]);
                    out.push_str(mark);
                    out.push_str(text.trim());
                    out.push_str(mark);
                    out.push_str(&text[text.len() - trail..]);
                }
            }
            Inline::HardBreak => out.push('\n'),
            Inline::Footnote(inner) => {
                let note = inline_text(inner, cx);
                cx.notes.push(note);
                out.push_str(&format!("[{}]", cx.notes.len()));
            }
        }
    }
    out
}

fn indent(s: &str, by: &str) -> String {
    s.lines()
        .map(|l| {
            if l.is_empty() {
                String::new()
            } else {
                format!("{by}{l}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn text_blocks(blocks: &[Block], cx: &mut TextCtx, sep: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for b in blocks {
        match b {
            Block::Paragraph(c) => out.push(inline_text(c, cx)),
            Block::Heading { content, .. } => out.push(inline_text(content, cx)),
            Block::SceneBreak => out.push(sep.to_string()),
            Block::BlockQuote(inner) => out.push(indent(&text_blocks(inner, cx, sep), "    ")),
            Block::Div { class, content } => {
                let inner = text_blocks(content, cx, sep);
                out.push(match class.as_str() {
                    "epigraph" | "letter" | "textmsg" | "verse" => indent(&inner, "    "),
                    "attribution" => format!("    — {inner}"),
                    _ => inner,
                });
            }
            Block::BulletList { items, .. } => {
                let lines: Vec<String> = items
                    .iter()
                    .map(|it| format!("- {}", text_blocks(it, cx, sep).replace('\n', "\n  ")))
                    .collect();
                out.push(lines.join("\n"));
            }
            Block::OrderedList { start, items, .. } => {
                let lines: Vec<String> = items
                    .iter()
                    .enumerate()
                    .map(|(n, it)| {
                        format!(
                            "{}. {}",
                            *start as usize + n,
                            text_blocks(it, cx, sep).replace('\n', "\n   ")
                        )
                    })
                    .collect();
                out.push(lines.join("\n"));
            }
        }
    }
    out.join("\n\n")
}

fn to_text(book: &Book, p: &ExportProfile, pieces: &[&Piece], first: bool) -> String {
    let mut cx = TextCtx {
        italics: p.italics,
        notes: Vec::new(),
    };
    let mut parts: Vec<String> = Vec::new();
    if first && p.title_page {
        parts.push(book.title.to_uppercase());
        if !book.author.is_empty() {
            parts.push(format!("by {}", book.author));
        }
        front_text(p, &mut parts);
    }
    for piece in pieces {
        match piece {
            Piece::Part { title } => parts.push(title.to_uppercase()),
            Piece::Chapter { .. } => {
                if let Some(h) = piece.heading_line() {
                    parts.push(h);
                }
            }
            Piece::SceneBreak => parts.push(p.scene_separator.clone()),
            Piece::Blocks(b) => parts.push(text_blocks(b, &mut cx, &p.scene_separator)),
            Piece::Matter { title, blocks } => {
                if let Some(t) = title {
                    parts.push(t.to_uppercase());
                }
                parts.push(text_blocks(blocks, &mut cx, &p.scene_separator));
            }
        }
    }
    if !cx.notes.is_empty() {
        let list: Vec<String> = cx
            .notes
            .iter()
            .enumerate()
            .map(|(i, n)| format!("[{}] {n}", i + 1))
            .collect();
        parts.push("NOTES".into());
        parts.push(list.join("\n"));
    }
    parts.retain(|s| !s.is_empty());
    let mut s = parts.join("\n\n");
    s.push('\n');
    s
}
