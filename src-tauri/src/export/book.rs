//! The export model: the binder flattened into pieces every exporter walks
//! the same way, so chapter numbering and scene breaks agree across formats.

use super::profile::number_words;
use super::types::{ExportProfile, Numbering};
use crate::dialect::{self, Block, Document, Inline, Mark};
use crate::project::document::DocKind;
use crate::project::manifest::Section;
use crate::story::book::{BookItem, BookSource};

#[derive(Debug, Clone)]
pub enum Piece {
    Part {
        title: String,
    },
    Chapter {
        /// "Chapter Three", when numbering is on.
        label: Option<String>,
        title: Option<String>,
    },
    SceneBreak,
    Blocks(Vec<Block>),
    /// A front- or back-matter document.
    Matter {
        title: Option<String>,
        blocks: Vec<Block>,
    },
}

impl Piece {
    /// The single line used for contents lists, bookmarks and file names.
    pub fn heading_line(&self) -> Option<String> {
        match self {
            Piece::Part { title } => Some(title.clone()),
            Piece::Chapter { label, title } => match (label, title) {
                (Some(l), Some(t)) => Some(format!("{l}: {t}")),
                (Some(l), None) => Some(l.clone()),
                (None, Some(t)) => Some(t.clone()),
                (None, None) => None,
            },
            Piece::Matter { title, .. } => title.clone(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Book {
    pub title: String,
    pub author: String,
    pub front: Vec<Piece>,
    pub body: Vec<Piece>,
    pub back: Vec<Piece>,
    pub words: u32,
}

fn strip_comments(content: &mut Vec<Inline>) {
    for i in content.iter_mut() {
        match i {
            Inline::Text { marks, .. } => marks.retain(|m| !matches!(m, Mark::Comment { .. })),
            Inline::Footnote(inner) => strip_comments(inner),
            Inline::HardBreak => {}
        }
    }
    // Runs that differed only by a comment mark now match.
    let runs = std::mem::take(content);
    *content = dialect::ast::merge(runs);
}

/// Comments are the author's own notes; they are not part of the book.
fn clean(doc: &Document) -> Vec<Block> {
    let mut d = doc.clone();
    d.for_each_text_block_mut(&mut strip_comments);
    d.blocks
}

fn chapter_title(
    title: &str,
    label: &str,
    n: u32,
    how: Numbering,
) -> (Option<String>, Option<String>) {
    let title = title.trim();
    let title = (!title.is_empty()).then(|| title.to_string());
    let lead = label.trim().to_lowercase();
    // "Chapter 1" as a title already says it; do not number it twice.
    let titled_like_label = title
        .as_deref()
        .is_some_and(|t| !lead.is_empty() && t.to_lowercase().starts_with(&lead));
    let num = match how {
        Numbering::None => None,
        _ if titled_like_label => None,
        Numbering::Numeric => Some(n.to_string()),
        Numbering::Words => Some(number_words(n)),
    };
    let label = num.map(|num| {
        if label.trim().is_empty() {
            num
        } else {
            format!("{} {num}", label.trim())
        }
    });
    (label, title)
}

impl Book {
    pub fn build(src: &BookSource, p: &ExportProfile) -> Book {
        let mut front = Vec::new();
        let mut body = Vec::new();
        let mut back = Vec::new();
        let mut words = 0u32;
        let mut chapter_no = 0u32;
        let mut need_break = false;

        for item in &src.items {
            match item.section {
                Section::FrontMatter | Section::BackMatter => {
                    let wanted = if item.section == Section::FrontMatter {
                        p.include_front_matter
                    } else {
                        p.include_back_matter
                    };
                    if !wanted {
                        continue;
                    }
                    let out = if item.section == Section::FrontMatter {
                        &mut front
                    } else {
                        &mut back
                    };
                    out.push(matter(item));
                }
                Section::Manuscript => {
                    words = words.saturating_add(dialect::word_count(&item.ast));
                    match item.kind {
                        DocKind::Part => {
                            body.push(Piece::Part {
                                title: item.title.trim().to_string(),
                            });
                            need_break = false;
                        }
                        DocKind::Chapter => {
                            chapter_no += 1;
                            let (label, title) = chapter_title(
                                &item.title,
                                &p.chapter_label,
                                chapter_no,
                                p.numbering,
                            );
                            body.push(Piece::Chapter { label, title });
                            need_break = false;
                        }
                        _ => {}
                    }
                    let blocks = clean(&item.ast);
                    if !blocks.is_empty() {
                        if need_break && !matches!(item.kind, DocKind::Part | DocKind::Chapter) {
                            body.push(Piece::SceneBreak);
                        }
                        body.push(Piece::Blocks(blocks));
                        need_break = true;
                    }
                }
                _ => {}
            }
        }
        Book {
            title: src.title.clone(),
            author: src.author.clone(),
            front,
            body,
            back,
            words,
        }
    }
}

fn matter(item: &BookItem) -> Piece {
    let title = item.title.trim();
    Piece::Matter {
        title: (!title.is_empty()).then(|| title.to_string()),
        blocks: clean(&item.ast),
    }
}

/// Plain text of inline content with footnote marks left out.
pub fn plain(content: &[Inline]) -> String {
    let mut out = String::new();
    for i in content {
        match i {
            Inline::Text { text, .. } => out.push_str(text),
            Inline::HardBreak => out.push('\n'),
            Inline::Footnote(_) => {}
        }
    }
    out
}

/// "Smith" from "Jane Q. Smith"; empty if no author.
pub fn surname(author: &str) -> String {
    author.split_whitespace().last().unwrap_or("").to_string()
}

/// A file-system-safe base name.
pub fn safe_name(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if "<>:\"/\\|?*".contains(c) || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect();
    let t = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let t = t.trim_end_matches(['.', ' ']).to_string();
    if t.is_empty() { "Untitled".into() } else { t }
}
