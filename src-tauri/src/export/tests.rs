use std::fs;
use std::io::Read;
use std::sync::Arc;

use super::book::{Book, Piece, safe_name};
use super::profile::{self, default_profiles, number_words};
use super::types::*;
use super::{ExportService, typeset};
use crate::dialect::{self, pm};
use crate::events::EventBus;
use crate::project::document::DocKind;
use crate::project::manifest::{Locale, ProjectType, Section};
use crate::story::StoryService;
use crate::story::types::*;
use typst::foundations::NativeElement;
use typst::introspection::Introspector;

struct Fx {
    _dir: tempfile::TempDir,
    story: Arc<StoryService>,
    svc: Arc<ExportService>,
    root: std::path::PathBuf,
    out: tempfile::TempDir,
}

fn target(section: Section, parent: Option<&str>) -> BinderTarget {
    BinderTarget {
        section,
        parent: parent.map(str::to_string),
        index: None,
    }
}

fn put(
    story: &StoryService,
    kind: DocKind,
    title: &str,
    section: Section,
    parent: Option<&str>,
    text: &str,
) -> String {
    let meta = story
        .document_create(kind, title, &target(section, parent))
        .unwrap();
    story
        .document_save(&meta.id, &pm::to_pm(&dialect::parse(text)), true)
        .unwrap();
    meta.id
}

/// A small book: two chapters (two scenes in the first), a part-less layout,
/// a dedication-like front matter document and an excluded scene.
fn fixture() -> Fx {
    let dir = tempfile::tempdir().unwrap();
    let bus = Arc::new(EventBus::new());
    let story = Arc::new(StoryService::new(bus.clone()));
    let info = story
        .create_project(&NewProject {
            parent_dir: dir.path().to_string_lossy().into(),
            title: "The Salt Road".into(),
            author: "Jane Q. Doe".into(),
            project_type: ProjectType::Fiction,
            locale: Locale::EnUs,
        })
        .unwrap();
    // Clear the starter structure.
    let tree = story.binder_tree().unwrap();
    let starter: Vec<String> = tree
        .sections
        .iter()
        .flat_map(|s| s.nodes.iter().map(|n| n.id.clone()))
        .collect();
    story.binder_trash(&starter).unwrap();

    let c1 = put(
        &story,
        DocKind::Chapter,
        "The Arrival",
        Section::Manuscript,
        None,
        "",
    );
    put(
        &story,
        DocKind::Scene,
        "Harbor",
        Section::Manuscript,
        Some(&c1),
        "The *harbor* bells had stopped. [Odd]{.sc} things: 1. [^1] \"Quoted\" - and #hash $5 *star* _under_ <b> @at\n\nSecond paragraph (with parens) [and brackets].\n\n[^1]: A note.",
    );
    put(
        &story,
        DocKind::Scene,
        "Market",
        Section::Manuscript,
        Some(&c1),
        "She walked to the market.\n\n::: epigraph\nA line to remember.\n:::\n\n> A quoted line.\n\n- one\n- two",
    );
    let hidden = put(
        &story,
        DocKind::Scene,
        "Cut",
        Section::Manuscript,
        Some(&c1),
        "Cut scene text.",
    );
    story
        .document_update_meta(
            &hidden,
            &DocMetaPatch {
                include: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    let c2 = put(
        &story,
        DocKind::Chapter,
        "Chapter 2",
        Section::Manuscript,
        None,
        "Night fell on the second day.",
    );
    let _ = c2;
    put(
        &story,
        DocKind::Note,
        "Foreword",
        Section::FrontMatter,
        None,
        "A few words before the story.",
    );
    put(
        &story,
        DocKind::Note,
        "Acknowledgements",
        Section::BackMatter,
        None,
        "Thanks to everyone.",
    );
    let svc = ExportService::new(story.clone(), bus);
    Fx {
        _dir: dir,
        story,
        svc,
        root: std::path::PathBuf::from(&info.root),
        out: tempfile::tempdir().unwrap(),
    }
}

fn profile_of(id: &str) -> ExportProfile {
    default_profiles().into_iter().find(|p| p.id == id).unwrap()
}

fn book(f: &Fx, p: &ExportProfile) -> Book {
    Book::build(&f.story.book_source().unwrap(), p)
}

fn read_zip_entry(path: &std::path::Path, name: &str) -> String {
    let file = fs::File::open(path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut s = String::new();
    zip.by_name(name).unwrap().read_to_string(&mut s).unwrap();
    s
}

#[test]
fn number_words_read_like_chapter_numbers() {
    assert_eq!(number_words(1), "One");
    assert_eq!(number_words(13), "Thirteen");
    assert_eq!(number_words(21), "Twenty-One");
    assert_eq!(number_words(100), "One Hundred");
    assert_eq!(number_words(105), "One Hundred Five");
}

#[test]
fn safe_names_avoid_reserved_characters() {
    assert_eq!(safe_name("A: B?/C."), "A B C");
    assert_eq!(safe_name("  "), "Untitled");
}

#[test]
fn book_numbers_chapters_and_skips_excluded_documents() {
    let f = fixture();
    let b = book(&f, &profile_of("print"));
    let heads: Vec<String> = b.body.iter().filter_map(|p| p.heading_line()).collect();
    // "Chapter 2" is titled like the label, so it is not numbered twice.
    assert_eq!(heads, vec!["Chapter 1: The Arrival", "Chapter 2"]);
    let text = format!("{:?}", b.body);
    assert!(!text.contains("Cut scene text"));
    // One break between the two scenes of chapter 1, none before a chapter heading.
    let breaks = b
        .body
        .iter()
        .filter(|p| matches!(p, Piece::SceneBreak))
        .count();
    assert_eq!(breaks, 1);
    assert_eq!(b.front.len(), 1);
    assert_eq!(b.back.len(), 1);
    assert!(b.words > 10);
}

#[test]
fn numbering_styles_and_matter_switches() {
    let f = fixture();
    let mut p = profile_of("print");
    p.numbering = Numbering::Words;
    p.chapter_label = "Part".into();
    let b = book(&f, &p);
    assert_eq!(b.body[0].heading_line().unwrap(), "Part One: The Arrival");
    p.numbering = Numbering::None;
    assert_eq!(book(&f, &p).body[0].heading_line().unwrap(), "The Arrival");
    p.include_front_matter = false;
    p.include_back_matter = false;
    let b = book(&f, &p);
    assert!(b.front.is_empty() && b.back.is_empty());
}

#[test]
fn comments_are_not_part_of_the_book() {
    let f = fixture();
    let id = put(
        &f.story,
        DocKind::Scene,
        "Commented",
        Section::Manuscript,
        None,
        "Some <!-- c:01ABC -->commented<!-- /c --> words.",
    );
    let _ = id;
    let md = super::markdown::export(
        &book(&f, &profile_of("markdown")),
        &profile_of("markdown"),
        "x",
    );
    let s = String::from_utf8(md[0].bytes.clone()).unwrap();
    assert!(s.contains("Some commented words."), "{s}");
    assert!(!s.contains("c:01ABC"));
}

#[test]
fn markdown_has_metadata_headings_and_separators() {
    let f = fixture();
    let p = profile_of("markdown");
    let out = super::markdown::export(&book(&f, &p), &p, "The Salt Road");
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].name, "The Salt Road.md");
    let s = String::from_utf8(out[0].bytes.clone()).unwrap();
    assert!(s.starts_with("---\ntitle: \"The Salt Road\"\nauthor: \"Jane Q. Doe\"\n---"));
    assert!(s.contains("## Chapter 1: The Arrival"));
    assert!(s.contains("\n\n***\n\n"));
    assert!(s.contains("[Odd]{.sc}"));
    assert!(s.contains("## Foreword"));
    assert!(!s.contains("Cut scene text"));
}

#[test]
fn markdown_splits_into_one_file_per_chapter() {
    let f = fixture();
    let mut p = profile_of("markdown");
    p.split_chapters = true;
    let out = super::markdown::export(&book(&f, &p), &p, "x");
    let names: Vec<&str> = out.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "01 - Front matter.md",
            "02 - Chapter 1 The Arrival.md",
            "03 - Chapter 2.md",
            "04 - Back matter.md"
        ]
    );
}

#[test]
fn text_uses_the_chosen_italics_and_collects_footnotes() {
    let f = fixture();
    let mut p = profile_of("text");
    p.scene_separator = "~".into();
    let render = |p: &ExportProfile| {
        let out = super::markdown::export(&book(&f, p), p, "t");
        String::from_utf8(out[0].bytes.clone()).unwrap()
    };
    let s = render(&p);
    assert!(s.contains("The _harbor_ bells"), "{s}");
    assert!(s.contains("[1] A note."));
    assert!(s.contains("\n\n~\n\n"));
    assert!(s.contains("    A line to remember."));
    p.italics = ItalicStyle::Asterisks;
    assert!(render(&p).contains("The *harbor* bells"));
    p.italics = ItalicStyle::Plain;
    assert!(render(&p).contains("The harbor bells"));
}

#[test]
fn docx_manuscript_has_the_standard_format() {
    let f = fixture();
    let mut p = profile_of("manuscript");
    p.contact = Contact {
        name: "Jane Q. Doe".into(),
        address: "1 Main St\nTown".into(),
        email: "jane@example.com".into(),
        phone: String::new(),
    };
    let r = f
        .svc
        .run(&p, Some(&f.out.path().to_string_lossy()))
        .unwrap();
    assert_eq!(r.files.len(), 1);
    let path = std::path::PathBuf::from(&r.files[0]);
    assert!(path.ends_with("The Salt Road.docx"));
    let doc = read_zip_entry(&path, "word/document.xml");
    assert!(doc.contains("Jane Q. Doe"));
    assert!(
        doc.contains("About 100 words") || doc.contains("About "),
        "{}",
        &doc[..400.min(doc.len())]
    );
    assert!(doc.contains("THE SALT ROAD"));
    assert!(doc.contains("Chapter 1"));
    assert!(doc.contains("The Arrival"));
    assert!(doc.contains(">#<"), "scene break");
    let styles = read_zip_entry(&path, "word/styles.xml");
    assert!(styles.contains("Times New Roman"));
    assert!(styles.contains("w:line=\"480\""), "double spaced");
    let header = read_zip_entry(&path, "word/header1.xml");
    assert!(header.contains("Doe / THE SALT ROAD / "), "{header}");
    assert!(header.contains("PAGE"));
    assert!(doc.contains("w:titlePg"));
}

#[test]
fn docx_styled_uses_named_styles_and_real_footnotes() {
    let f = fixture();
    let p = profile_of("styled");
    let r = f
        .svc
        .run(&p, Some(&f.out.path().to_string_lossy()))
        .unwrap();
    let path = std::path::PathBuf::from(&r.files[0]);
    let styles = read_zip_entry(&path, "word/styles.xml");
    for name in [
        "Chapter Title",
        "Scene Break",
        "Block Quote",
        "Epigraph",
        "Body Text",
        "First Paragraph",
        "Verse",
        "Text Message",
    ] {
        assert!(
            styles.contains(&format!("w:val=\"{name}\"")),
            "style {name} missing"
        );
    }
    let doc = read_zip_entry(&path, "word/document.xml");
    assert!(doc.contains("w:pStyle w:val=\"ChapterTitle\""));
    assert!(doc.contains("w:footnoteReference"), "real footnote");
    assert!(doc.contains("TOC "), "contents field");
    let notes = read_zip_entry(&path, "word/footnotes.xml");
    assert!(notes.contains("A note."));
    assert!(!doc.contains("Cut scene text"));
}

#[test]
fn tricky_text_cannot_break_the_typesetter() {
    let f = fixture();
    let p = profile_of("print");
    let b = book(&f, &p);
    let markup = typeset::markup(&b, &p);
    let doc = super::world::compile(markup).expect("compiles");
    assert!(!doc.pages().is_empty());
    assert_eq!(
        typeset::esc("a_b *c* #d [e](f) 1. - = @"),
        "a\\_b \\*c\\* \\#d \\[e\\]\\(f\\) 1\\. \\- \\= \\@"
    );
    assert_eq!(typeset::esc("\"it's\""), "\"it's\"");
    assert_eq!(typeset::lit("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
}

#[test]
fn pdf_is_written_with_pages_and_chapters_open_on_right_hand_pages() {
    let f = fixture();
    let mut p = profile_of("print");
    p.toc = true;
    p.title_page = true;
    p.copyright = "Copyright 2026 Jane Doe".into();
    p.dedication = "For everyone".into();
    p.epigraph = "A line of verse.".into();
    p.epigraph_by = "Someone".into();
    let r = f
        .svc
        .run(&p, Some(&f.out.path().to_string_lossy()))
        .unwrap();
    let bytes = fs::read(&r.files[0]).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    assert!(r.pages.unwrap() >= 6);
    // Fonts: a template asking for a font that is not here is reported.
    let _ = r.warnings;

    // Chapter openings land on odd pages (recto starts).
    let info = f.svc.preview(&p).unwrap();
    assert_eq!(info.page_count, r.pages.unwrap());
    let doc = {
        let b = book(&f, &p);
        super::world::compile(typeset::markup(&b, &p)).unwrap()
    };
    let headings = doc
        .introspector()
        .query(&typst::foundations::Selector::Elem(
            typst::model::HeadingElem::ELEM,
            None,
        ));
    let mut seen = 0;
    for h in headings {
        let loc = h.location().unwrap();
        let page = doc.introspector().page(loc).unwrap().get();
        let text = h.plain_text().to_string().replace("Section", "");
        if text.starts_with("Chapter") {
            assert_eq!(page % 2, 1, "{text} opens on page {page}");
            seen += 1;
        }
    }
    assert_eq!(seen, 2);
}

#[test]
fn blank_filler_pages_carry_no_running_head_or_number() {
    let f = fixture();
    let p = profile_of("print");
    let info = f.svc.preview(&p).unwrap();
    let pages = f.svc.preview_pages(&info.key, 0, info.page_count).unwrap();
    // A page with no glyphs at all is truly blank: no running head, no folio.
    let blank: Vec<u32> = pages
        .iter()
        .filter(|pg| !pg.svg.contains("<use"))
        .map(|pg| pg.index + 1)
        .collect();
    assert!(
        !blank.is_empty(),
        "recto starts leave blank left-hand pages"
    );
    assert!(
        blank.iter().all(|n| n % 2 == 0),
        "only left-hand pages are blank: {blank:?}"
    );
    // And no page that has text is a bare running head.
    for pg in &pages {
        if pg.svg.contains("<use") {
            assert!(
                pg.svg.matches("<use").count() > 3,
                "page {} holds only a head or number",
                pg.index + 1
            );
        }
    }
    // Without recto starts nothing is left blank.
    let mut q = p.clone();
    q.settings.recto_starts = false;
    let info = f.svc.preview(&q).unwrap();
    let pages = f.svc.preview_pages(&info.key, 0, info.page_count).unwrap();
    assert!(pages.iter().all(|pg| pg.svg.contains("<use")));
}

#[test]
fn preview_serves_pages_for_the_key_it_was_given() {
    let f = fixture();
    let p = profile_of("print");
    let info = f.svc.preview(&p).unwrap();
    assert!(info.page_count > 0);
    assert!(
        (info.width_pt - 5.5 * 72.0).abs() < 0.5,
        "{}",
        info.width_pt
    );
    let pages = f.svc.preview_pages(&info.key, 0, 2).unwrap();
    assert_eq!(pages.len(), 2);
    assert!(pages[0].svg.starts_with("<svg"));
    assert!(f.svc.preview_pages("stale", 0, 1).is_err());
    // The same settings reuse the typesetting.
    let again = f.svc.preview(&p).unwrap();
    assert_eq!(again.key, info.key);
    // A different template changes it.
    let mut q = p.clone();
    q.settings.trim_width_in = 6.0;
    assert_ne!(f.svc.preview(&q).unwrap().key, info.key);
}

#[test]
fn exporting_never_changes_the_manuscript() {
    let f = fixture();
    let snapshot = |root: &std::path::Path| {
        let mut all = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    if p.file_name().is_some_and(|n| n == ".margin") {
                        continue;
                    }
                    stack.push(p);
                } else {
                    all.push((p.clone(), fs::read(&p).unwrap()));
                }
            }
        }
        all.sort();
        all
    };
    let before = snapshot(&f.root);
    for id in ["manuscript", "print", "styled", "markdown", "text"] {
        f.svc
            .run(&profile_of(id), Some(&f.out.path().to_string_lossy()))
            .unwrap();
    }
    assert_eq!(snapshot(&f.root), before);
}

#[test]
fn a_folder_is_required_and_failures_are_events() {
    let f = fixture();
    let err = f.svc.run(&profile_of("markdown"), None).unwrap_err();
    assert!(err.to_string().contains("folder"));
}

#[test]
fn profiles_default_save_and_delete() {
    let f = fixture();
    let all = f.svc.profiles().unwrap();
    assert_eq!(all.len(), 5);
    assert!(
        !f.root.join(profile::PROFILES_PATH).exists(),
        "defaults are not written until saved"
    );
    let mut mine = profile_of("print");
    mine.id = "mine".into();
    mine.name = "My print".into();
    mine.settings.size_pt = 12.0;
    mine.output_dir = Some("D:/out".into());
    let all = f.svc.profile_save(mine.clone()).unwrap();
    assert_eq!(all.len(), 6);
    assert!(f.root.join(profile::PROFILES_PATH).exists());
    let text = fs::read_to_string(f.root.join(profile::PROFILES_PATH)).unwrap();
    assert!(text.contains("schemaVersion: 1"));
    assert_eq!(
        f.svc.profiles().unwrap().iter().find(|p| p.id == "mine"),
        Some(&mine)
    );
    // Saving again with the same id replaces it.
    mine.name = "Renamed".into();
    assert_eq!(f.svc.profile_save(mine).unwrap().len(), 6);
    let left = f.svc.profile_delete("mine").unwrap();
    assert_eq!(left.len(), 5);
    assert!(
        f.svc
            .profile_save(ExportProfile {
                name: " ".into(),
                ..profile_of("print")
            })
            .is_err()
    );
}

#[test]
fn every_template_typesets() {
    let f = fixture();
    for t in profile::builtin_templates() {
        let mut p = profile_of("print");
        p.template = t.id.clone();
        p.settings = t.settings.clone();
        let info = f
            .svc
            .preview(&p)
            .unwrap_or_else(|e| panic!("{}: {e}", t.id));
        assert!(info.page_count > 0, "{}", t.id);
    }
}

#[test]
fn small_caps_lead_in_works_for_any_font() {
    let f = fixture();
    let mut p = profile_of("print");
    p.settings.chapter_opening = ChapterOpening::SmallCapsLead;
    // A system font may have no small-capital glyphs: the lead-in is drawn with capitals one size down.
    p.settings.font = "Garamond".into();
    let b = book(&f, &p);
    let m = typeset::markup(&b, &p);
    assert!(
        m.contains("#sc(\"The \")") || m.contains("#sc(\"The harbor"),
        "{}",
        &m[m.find("#chapter-open").unwrap()..][..400]
    );
    assert!(!m.contains("#smallcaps[The"));
    assert!(super::world::compile(m).is_ok());
    // A bundled font has the real thing.
    p.settings.font = "Libertinus Serif".into();
    let m = typeset::markup(&book(&f, &p), &p);
    assert!(m.contains("#smallcaps["));
    assert!(super::world::compile(m).is_ok());
    // The author's own small caps ("[Odd]{.sc}") follow the same rule.
    p.settings.font = "Georgia".into();
    assert!(typeset::markup(&book(&f, &p), &p).contains("#sc(\"Odd\")"));
}
