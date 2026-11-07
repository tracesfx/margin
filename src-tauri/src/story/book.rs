//! The book as the exporters see it (SPEC §12): the binder read in order,
//! front matter, manuscript and back matter, parsed to the Markdown AST.
//! Read-only; the exporters never touch project files except to write
//! their own output.

use std::fs;

use super::service::{OpenProject, StoryService};
use crate::dialect::{self, Document};
use crate::error::Result;
use crate::project::document::{DocFile, DocKind};
use crate::project::manifest::{BinderEntry, Locale, Section};

#[derive(Debug, Clone)]
pub struct BookItem {
    pub id: String,
    pub kind: DocKind,
    pub title: String,
    pub section: Section,
    /// Nesting depth in the binder (0 = top level of its section).
    pub depth: u32,
    pub pov: Option<String>,
    pub ast: Document,
}

#[derive(Debug, Clone)]
pub struct BookSource {
    pub title: String,
    pub author: String,
    pub locale: Locale,
    /// Front matter, manuscript, back matter, each in binder order. Documents
    /// marked "not included" are left out together with everything inside them.
    pub items: Vec<BookItem>,
}

fn walk(
    p: &OpenProject,
    section: Section,
    entries: &[BinderEntry],
    depth: u32,
    out: &mut Vec<BookItem>,
) -> Result<()> {
    for e in entries {
        let Some(d) = p.docs.get(&e.id) else {
            // Unreadable file: its children still export.
            walk(p, section, &e.children, depth, out)?;
            continue;
        };
        if !d.fm.include {
            continue;
        }
        let text = fs::read_to_string(&d.path)?;
        let body = DocFile::parse(&text, &d.path)?.body;
        out.push(BookItem {
            id: e.id.clone(),
            kind: d.fm.kind(),
            title: d.fm.title.clone(),
            section,
            depth,
            pov: d.fm.pov.clone(),
            ast: dialect::parse(&body),
        });
        walk(p, section, &e.children, depth + 1, out)?;
    }
    Ok(())
}

impl StoryService {
    /// Everything an exporter needs, in one consistent read.
    pub fn book_source(&self) -> Result<BookSource> {
        self.read(|p| {
            let mut items = Vec::new();
            for s in [
                Section::FrontMatter,
                Section::Manuscript,
                Section::BackMatter,
            ] {
                walk(p, s, p.manifest.binder.section(s), 0, &mut items)?;
            }
            Ok(BookSource {
                title: p.manifest.title.clone(),
                author: p.manifest.author.clone(),
                locale: p.manifest.locale,
                items,
            })
        })
    }
}
