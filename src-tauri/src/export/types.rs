//! Export profiles, templates and results (SPEC §12). These cross the IPC
//! boundary, so they are all `specta::Type`.

use serde::{Deserialize, Serialize};
use specta::Type;

pub const PROFILES_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum BookFormat {
    Markdown,
    Text,
    /// Standard Manuscript Format, for submissions.
    DocxManuscript,
    /// Semantic styles become named Word styles.
    DocxStyled,
    Pdf,
}

impl BookFormat {
    pub fn extension(self) -> &'static str {
        match self {
            BookFormat::Markdown => "md",
            BookFormat::Text => "txt",
            BookFormat::DocxManuscript | BookFormat::DocxStyled => "docx",
            BookFormat::Pdf => "pdf",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Numbering {
    /// Chapter titles only.
    None,
    /// "Chapter 3"
    Numeric,
    /// "Chapter Three"
    Words,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ItalicStyle {
    Underscores,
    Asterisks,
    Plain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ChapterOpening {
    Plain,
    SmallCapsLead,
    /// An enlarged first letter (a raised initial; sunken drop caps are not built).
    LargeInitial,
}

/// Everything a template decides (SPEC §12.2). The manuscript stays clean.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSettings {
    pub font: String,
    pub size_pt: f64,
    /// Extra space between lines, in em (Typst "leading").
    pub leading_em: f64,
    /// First-line indent, in em. 0 = block paragraphs.
    pub indent_em: f64,
    /// Space between paragraphs, in em (used with no indent).
    pub paragraph_space_em: f64,
    pub justify: bool,
    pub hyphenate: bool,
    pub chapter_opening: ChapterOpening,
    pub ornament: String,
    pub running_heads: bool,
    pub page_numbers: bool,
    /// Chapters and parts start on a right-hand page.
    pub recto_starts: bool,
    pub widow_orphan: bool,
    pub trim_width_in: f64,
    pub trim_height_in: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    /// Built in (cannot be changed) or saved by the author in `templates/`.
    pub builtin: bool,
    pub settings: TemplateSettings,
}

pub const TEMPLATE_SCHEMA_VERSION: u32 = 1;

/// One saved template: `templates/<id>.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TemplateFile {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub settings: TemplateSettings,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct Contact {
    pub name: String,
    pub address: String,
    pub email: String,
    pub phone: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportProfile {
    pub id: String,
    pub name: String,
    pub format: BookFormat,
    pub include_front_matter: bool,
    pub include_back_matter: bool,
    pub title_page: bool,
    pub toc: bool,
    /// Copyright page text. Empty = no copyright page.
    pub copyright: String,
    pub dedication: String,
    pub epigraph: String,
    pub epigraph_by: String,
    pub numbering: Numbering,
    pub chapter_label: String,
    /// Printed between scenes (the manuscript's `***`). SMF uses `#`.
    pub scene_separator: String,
    /// Built-in template id (`classic`, `modern`, `literary`, `nonfiction`).
    pub template: String,
    pub settings: TemplateSettings,
    /// Markdown/TXT: one file per chapter.
    pub split_chapters: bool,
    pub italics: ItalicStyle,
    /// Standard Manuscript Format: `Times New Roman` or `Courier New`.
    pub manuscript_font: String,
    pub contact: Contact,
    /// Folder the file goes to. `None` = ask each time.
    pub output_dir: Option<String>,
    /// File name without the extension. `None` = the book's title.
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesFile {
    pub schema_version: u32,
    pub profiles: Vec<ExportProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub profile_id: String,
    pub files: Vec<String>,
    pub words: u32,
    /// Pages, for PDF.
    pub pages: Option<u32>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewInfo {
    /// Identifies this typesetting; pass it back to fetch pages.
    pub key: String,
    pub page_count: u32,
    pub width_pt: f64,
    pub height_pt: f64,
    pub warnings: Vec<String>,
}

/// A text block of the manuscript, found by clicking a page.
pub use super::pageview::PageHit;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPage {
    pub index: u32,
    pub svg: String,
}
