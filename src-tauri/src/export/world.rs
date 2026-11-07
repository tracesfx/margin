//! A minimal Typst `World`: one generated source file, bundled and system
//! fonts, no packages and no file access (SPEC §12.2).

use std::sync::OnceLock;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontStore;
use typst_layout::PagedDocument;

use crate::error::{MarginError, Result};

/// Fonts are found once per run: scanning the system takes a moment.
static FONTS: OnceLock<FontStore> = OnceLock::new();

fn fonts() -> &'static FontStore {
    FONTS.get_or_init(|| {
        let mut store = FontStore::new();
        store.extend(typst_kit::fonts::embedded());
        store.extend(book_fonts());
        store.extend(typst_kit::fonts::system());
        store
    })
}

/// The OFL book fonts that ship with Margin (licences sit beside them in
/// `src-tauri/fonts/`). Compiled in, so no file is read at run time.
fn book_fonts() -> impl Iterator<Item = (Font, typst::text::FontInfo)> {
    const FILES: &[&[u8]] = &[
        include_bytes!("../../fonts/eb-garamond/EBGaramond-Regular.ttf"),
        include_bytes!("../../fonts/eb-garamond/EBGaramond-Italic.ttf"),
        include_bytes!("../../fonts/eb-garamond/EBGaramond-Bold.ttf"),
        include_bytes!("../../fonts/eb-garamond/EBGaramond-BoldItalic.ttf"),
        include_bytes!("../../fonts/crimson-text/CrimsonText-Regular.ttf"),
        include_bytes!("../../fonts/crimson-text/CrimsonText-Italic.ttf"),
        include_bytes!("../../fonts/crimson-text/CrimsonText-Bold.ttf"),
        include_bytes!("../../fonts/crimson-text/CrimsonText-BoldItalic.ttf"),
        include_bytes!("../../fonts/source-serif-4/SourceSerif4-Regular.otf"),
        include_bytes!("../../fonts/source-serif-4/SourceSerif4-It.otf"),
        include_bytes!("../../fonts/source-serif-4/SourceSerif4-Bold.otf"),
        include_bytes!("../../fonts/source-serif-4/SourceSerif4-BoldIt.otf"),
    ];
    FILES.iter().flat_map(|data| {
        Font::iter(Bytes::new(*data)).map(|font| {
            let info = font.info().clone();
            (font, info)
        })
    })
}

/// Family names that can be used for a book, bundled ones first.
pub fn font_families() -> Vec<String> {
    let mut seen: Vec<String> = fonts()
        .book()
        .families()
        .map(|(name, _)| name.to_string())
        .collect();
    seen.sort();
    seen.dedup();
    seen
}

pub struct BookWorld {
    library: LazyHash<Library>,
    main: FileId,
    source: Source,
}

impl BookWorld {
    pub fn new(markup: String) -> Result<Self> {
        let path = VirtualPath::new("book.typ")
            .map_err(|e| MarginError::Internal(format!("typesetting setup failed: {e}")))?;
        let main = RootedPath::new(VirtualRoot::Project, path).intern();
        Ok(BookWorld {
            library: LazyHash::new(Library::default()),
            main,
            source: Source::new(main, markup),
        })
    }
}

impl World for BookWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        fonts().book()
    }
    fn main(&self) -> FileId {
        self.main
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main {
            Ok(self.source.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }
    fn file(&self, _id: FileId) -> FileResult<Bytes> {
        Err(FileError::AccessDenied)
    }
    fn font(&self, index: usize) -> Option<Font> {
        fonts().font(index)
    }
    fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
        None
    }
}

pub fn compile(markup: String) -> Result<PagedDocument> {
    let world = BookWorld::new(markup)?;
    let out = typst::compile::<PagedDocument>(&world);
    out.output.map_err(|errs| {
        let msg = errs
            .iter()
            .take(3)
            .map(|e| e.message.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        MarginError::Internal(format!("typesetting failed: {msg}"))
    })
}

/// Like [`compile`], but keeps the world so spans can be mapped back to the
/// generated source (the Page view's click-to-source).
pub fn compile_with_world(markup: String) -> Result<(PagedDocument, BookWorld)> {
    let world = BookWorld::new(markup)?;
    let out = typst::compile::<PagedDocument>(&world);
    match out.output {
        Ok(doc) => Ok((doc, world)),
        Err(errs) => {
            let msg = errs
                .iter()
                .take(3)
                .map(|e| e.message.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            Err(MarginError::Internal(format!("typesetting failed: {msg}")))
        }
    }
}
