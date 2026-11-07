//! Book formatting and export (SPEC §12).
//!
//! The Story Data Service gives a read-only view of the binder
//! (`book_source`); the exporters turn it into files. Nothing here changes
//! the manuscript. Output files are written atomically.

pub mod book;
pub mod docx;
pub mod markdown;
pub mod pageview;
pub mod profile;
pub mod types;
pub mod typeset;
pub mod world;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ::typst_layout::PagedDocument;

use crate::error::{MarginError, Result};
use crate::events::{DomainEvent, EventBus};
use crate::project::atomic::atomic_write;
use crate::story::StoryService;
use book::{Book, safe_name};
use types::*;

/// The last typeset book, so the preview can page through it without
/// compiling again for every page.
struct Cached {
    key: String,
    doc: Arc<PagedDocument>,
}

/// The last chapter typeset for the Page view, with what is needed to map a
/// click back to the text.
struct PageCached {
    key: String,
    doc: Arc<PagedDocument>,
    world: Arc<world::BookWorld>,
    map: Vec<(usize, Option<(String, u32)>)>,
}

pub struct ExportService {
    story: Arc<StoryService>,
    bus: Arc<EventBus>,
    cache: Mutex<Option<Cached>>,
    page_cache: Mutex<Option<PageCached>>,
}

fn key_of(markup: &str) -> String {
    let mut h = DefaultHasher::new();
    markup.hash(&mut h);
    format!("{:016x}", h.finish())
}

impl ExportService {
    pub fn new(story: Arc<StoryService>, bus: Arc<EventBus>) -> Arc<Self> {
        Arc::new(ExportService {
            story,
            bus,
            cache: Mutex::new(None),
            page_cache: Mutex::new(None),
        })
    }

    pub fn profiles(&self) -> Result<Vec<ExportProfile>> {
        profile::load_profiles(&self.story.project_root()?)
    }

    pub fn profile_save(&self, p: ExportProfile) -> Result<Vec<ExportProfile>> {
        profile::upsert_profile(&self.story.project_root()?, p)
    }

    pub fn profile_delete(&self, id: &str) -> Result<Vec<ExportProfile>> {
        profile::remove_profile(&self.story.project_root()?, id)
    }

    pub fn templates(&self) -> Vec<TemplateInfo> {
        match self.story.project_root() {
            Ok(root) => profile::list_templates(&root),
            Err(_) => profile::builtin_templates(),
        }
    }

    pub fn template_save(
        &self,
        id: Option<String>,
        name: &str,
        settings: TemplateSettings,
    ) -> Result<Vec<TemplateInfo>> {
        profile::save_template(&self.story.project_root()?, id, name, settings)
    }

    pub fn template_delete(&self, id: &str) -> Result<Vec<TemplateInfo>> {
        profile::delete_template(&self.story.project_root()?, id)
    }

    pub fn fonts(&self) -> Vec<String> {
        world::font_families()
    }

    fn book_for(&self, p: &ExportProfile) -> Result<Book> {
        let src = self.story.book_source()?;
        Ok(Book::build(&src, p))
    }

    fn font_warnings(&self, p: &ExportProfile) -> Vec<String> {
        let font = p.settings.font.trim();
        if p.format != BookFormat::Pdf || font.is_empty() || typeset::bundled(font) {
            return Vec::new();
        }
        if world::font_families().iter().any(|f| f == font) {
            vec![format!(
                "{font} is a system font. Check that its licence allows embedding it in a PDF."
            )]
        } else {
            vec![format!(
                "{font} is not installed here, so the book is set in Libertinus Serif."
            )]
        }
    }

    fn typeset(&self, book: &Book, p: &ExportProfile) -> Result<(String, Arc<PagedDocument>)> {
        let markup = typeset::markup(book, p);
        let key = key_of(&markup);
        if let Ok(g) = self.cache.lock()
            && let Some(c) = g.as_ref()
            && c.key == key
        {
            return Ok((key, c.doc.clone()));
        }
        let doc = Arc::new(world::compile(markup)?);
        if let Ok(mut g) = self.cache.lock() {
            *g = Some(Cached {
                key: key.clone(),
                doc: doc.clone(),
            });
        }
        Ok((key, doc))
    }

    /// Typeset the book and report its size. Pages come from [`Self::preview_pages`].
    pub fn preview(&self, p: &ExportProfile) -> Result<PreviewInfo> {
        let book = self.book_for(p)?;
        let (key, doc) = self.typeset(&book, p)?;
        let (w, h) = doc
            .pages()
            .first()
            .map(|pg| (pg.frame.width().to_pt(), pg.frame.height().to_pt()))
            .unwrap_or((0.0, 0.0));
        Ok(PreviewInfo {
            key,
            page_count: doc.pages().len() as u32,
            width_pt: w,
            height_pt: h,
            warnings: self.font_warnings(p),
        })
    }

    /// Typeset the chapter (or part, or matter document) holding `doc_id` for the Page view.
    pub fn page_view(&self, doc_id: &str, p: &ExportProfile) -> Result<PreviewInfo> {
        let whole = self.book_for(p)?;
        let book = whole
            .only_section_of(doc_id)
            .ok_or_else(|| MarginError::NotFound("this document is not part of the book".into()))?;
        // Only the pages of the text itself: no title page, contents or copyright.
        let p = ExportProfile {
            title_page: false,
            toc: false,
            copyright: String::new(),
            dedication: String::new(),
            epigraph: String::new(),
            epigraph_by: String::new(),
            ..p.clone()
        };
        let markup = typeset::markup_with(&book, &p, true);
        let key = key_of(&markup);
        let cached = self
            .page_cache
            .lock()
            .ok()
            .and_then(|g| g.as_ref().filter(|c| c.key == key).map(|c| c.doc.clone()));
        let doc = match cached {
            Some(d) => d,
            None => {
                let map = typeset::source_map(&markup);
                let (doc, world) = world::compile_with_world(markup)?;
                let doc = Arc::new(doc);
                if let Ok(mut g) = self.page_cache.lock() {
                    *g = Some(PageCached {
                        key: key.clone(),
                        doc: doc.clone(),
                        world: Arc::new(world),
                        map,
                    });
                }
                doc
            }
        };
        let (w, h) = doc
            .pages()
            .first()
            .map(|pg| (pg.frame.width().to_pt(), pg.frame.height().to_pt()))
            .unwrap_or((0.0, 0.0));
        Ok(PreviewInfo {
            key,
            page_count: doc.pages().len() as u32,
            width_pt: w,
            height_pt: h,
            warnings: self.font_warnings(&p),
        })
    }

    pub fn page_view_pages(&self, key: &str, start: u32, count: u32) -> Result<Vec<PreviewPage>> {
        let g = self
            .page_cache
            .lock()
            .map_err(|_| MarginError::Internal("page view cache lock poisoned".into()))?;
        match g.as_ref() {
            Some(c) if c.key == key => Ok(svg_pages(&c.doc, start, count)),
            _ => Err(MarginError::NotFound(
                "this page view is out of date".into(),
            )),
        }
    }

    /// The text block under a click on a page (`x`, `y` in points from the page's top left).
    pub fn page_view_hit(&self, key: &str, page: u32, x: f64, y: f64) -> Result<Option<PageHit>> {
        let g = self
            .page_cache
            .lock()
            .map_err(|_| MarginError::Internal("page view cache lock poisoned".into()))?;
        let c = match g.as_ref() {
            Some(c) if c.key == key => c,
            _ => {
                return Err(MarginError::NotFound(
                    "this page view is out of date".into(),
                ));
            }
        };
        Ok(pageview::offset_at(&c.doc, &c.world, page as usize, x, y)
            .and_then(|o| pageview::block_at(&c.map, o)))
    }

    pub fn preview_pages(&self, key: &str, start: u32, count: u32) -> Result<Vec<PreviewPage>> {
        let doc = {
            let g = self
                .cache
                .lock()
                .map_err(|_| MarginError::Internal("preview cache lock poisoned".into()))?;
            match g.as_ref() {
                Some(c) if c.key == key => c.doc.clone(),
                _ => return Err(MarginError::NotFound("this preview is out of date".into())),
            }
        };
        Ok(svg_pages(&doc, start, count))
    }

    /// Export with a profile into `dir` (or the profile's own folder).
    pub fn run(&self, p: &ExportProfile, dir: Option<&str>) -> Result<ExportResult> {
        let result = self.run_inner(p, dir);
        let event = match &result {
            Ok(r) => DomainEvent::ExportFinished {
                profile_id: p.id.clone(),
                ok: true,
                files: r.files.clone(),
                error: None,
            },
            Err(e) => DomainEvent::ExportFinished {
                profile_id: p.id.clone(),
                ok: false,
                files: Vec::new(),
                error: Some(e.to_string()),
            },
        };
        self.bus.publish(event);
        result
    }

    fn run_inner(&self, p: &ExportProfile, dir: Option<&str>) -> Result<ExportResult> {
        let dir = dir
            .map(str::to_string)
            .or_else(|| p.output_dir.clone())
            .filter(|d| !d.trim().is_empty())
            .ok_or_else(|| MarginError::Invalid("choose a folder for the export".into()))?;
        let dir = PathBuf::from(dir);
        let book = self.book_for(p)?;
        let base = safe_name(
            p.file_name
                .as_deref()
                .filter(|n| !n.trim().is_empty())
                .unwrap_or(&book.title),
        );
        let mut warnings = self.font_warnings(p);
        let mut pages = None;
        let mut files: Vec<(PathBuf, Vec<u8>)> = Vec::new();

        match p.format {
            BookFormat::Markdown | BookFormat::Text => {
                let out = markdown::export(&book, p, &base);
                // Several files go together in a folder of their own.
                let folder = if out.len() > 1 {
                    dir.join(&base)
                } else {
                    dir.clone()
                };
                for f in out {
                    files.push((folder.join(safe_name(&f.name)), f.bytes));
                }
            }
            BookFormat::DocxManuscript | BookFormat::DocxStyled => {
                if p.format == BookFormat::DocxManuscript && p.contact.name.trim().is_empty() {
                    warnings.push(
                        "The manuscript's contact block is empty. Fill it in under Contact.".into(),
                    );
                }
                files.push((dir.join(format!("{base}.docx")), docx::export(&book, p)?));
            }
            BookFormat::Pdf => {
                let (_, doc) = self.typeset(&book, p)?;
                pages = Some(doc.pages().len() as u32);
                let bytes =
                    typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()).map_err(|e| {
                        let msg = e
                            .iter()
                            .take(2)
                            .map(|d| d.message.to_string())
                            .collect::<Vec<_>>()
                            .join("; ");
                        MarginError::Internal(format!("could not write the PDF: {msg}"))
                    })?;
                files.push((dir.join(format!("{base}.pdf")), bytes));
            }
        }

        let mut written = Vec::new();
        for (path, bytes) in files {
            write_output(&path, &bytes)?;
            written.push(path.to_string_lossy().to_string());
        }
        Ok(ExportResult {
            profile_id: p.id.clone(),
            files: written,
            words: book.words,
            pages,
            warnings,
        })
    }
}

fn svg_pages(doc: &PagedDocument, start: u32, count: u32) -> Vec<PreviewPage> {
    let opts = typst_svg::SvgOptions::default();
    doc.pages()
        .iter()
        .enumerate()
        .skip(start as usize)
        .take(count as usize)
        .map(|(i, page)| PreviewPage {
            index: i as u32,
            svg: typst_svg::svg(page, &opts),
        })
        .collect()
}

/// A finished export replaces the file of the same name, atomically.
fn write_output(path: &Path, bytes: &[u8]) -> Result<()> {
    atomic_write(path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests;
