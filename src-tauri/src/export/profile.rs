//! Built-in templates, default profiles and `exports/profiles.yaml`.

use std::fs;
use std::path::Path;

use super::types::*;
use crate::error::{MarginError, Result};
use crate::project::atomic::atomic_write;

pub const PROFILES_PATH: &str = "exports/profiles.yaml";

fn base() -> TemplateSettings {
    TemplateSettings {
        font: "Libertinus Serif".into(),
        size_pt: 11.0,
        leading_em: 0.65,
        indent_em: 1.5,
        paragraph_space_em: 0.0,
        justify: true,
        hyphenate: true,
        chapter_opening: ChapterOpening::Plain,
        ornament: "* * *".into(),
        running_heads: true,
        page_numbers: true,
        recto_starts: true,
        widow_orphan: true,
        trim_width_in: 5.5,
        trim_height_in: 8.5,
    }
}

pub fn builtin_templates() -> Vec<TemplateInfo> {
    let t = |id: &str, name: &str, settings| TemplateInfo {
        id: id.into(),
        name: name.into(),
        settings,
    };
    vec![
        t(
            "classic",
            "Classic",
            TemplateSettings {
                font: "Garamond".into(),
                chapter_opening: ChapterOpening::SmallCapsLead,
                ..base()
            },
        ),
        t(
            "modern",
            "Modern",
            TemplateSettings {
                font: "Georgia".into(),
                size_pt: 10.5,
                indent_em: 0.0,
                paragraph_space_em: 0.8,
                justify: false,
                hyphenate: false,
                ornament: "—".into(),
                trim_width_in: 6.0,
                trim_height_in: 9.0,
                ..base()
            },
        ),
        t(
            "literary",
            "Literary",
            TemplateSettings {
                font: "Palatino Linotype".into(),
                indent_em: 1.2,
                chapter_opening: ChapterOpening::LargeInitial,
                ornament: "·  ·  ·".into(),
                trim_width_in: 5.0,
                trim_height_in: 8.0,
                ..base()
            },
        ),
        t(
            "nonfiction",
            "Non-fiction",
            TemplateSettings {
                font: "Cambria".into(),
                indent_em: 0.0,
                paragraph_space_em: 0.7,
                recto_starts: false,
                trim_width_in: 6.0,
                trim_height_in: 9.0,
                ..base()
            },
        ),
    ]
}

pub fn template(id: &str) -> Option<TemplateInfo> {
    builtin_templates().into_iter().find(|t| t.id == id)
}

fn profile(id: &str, name: &str, format: BookFormat) -> ExportProfile {
    ExportProfile {
        id: id.into(),
        name: name.into(),
        format,
        include_front_matter: true,
        include_back_matter: true,
        title_page: true,
        toc: false,
        copyright: String::new(),
        dedication: String::new(),
        epigraph: String::new(),
        epigraph_by: String::new(),
        numbering: Numbering::Numeric,
        chapter_label: "Chapter".into(),
        scene_separator: "* * *".into(),
        template: "classic".into(),
        settings: template("classic").map(|t| t.settings).unwrap_or_else(base),
        split_chapters: false,
        italics: ItalicStyle::Underscores,
        manuscript_font: "Times New Roman".into(),
        contact: Contact::default(),
        output_dir: None,
        file_name: None,
    }
}

/// What a project offers before the author saves anything.
pub fn default_profiles() -> Vec<ExportProfile> {
    vec![
        ExportProfile {
            title_page: false,
            include_front_matter: false,
            include_back_matter: false,
            scene_separator: "#".into(),
            ..profile(
                "manuscript",
                "Manuscript for submission",
                BookFormat::DocxManuscript,
            )
        },
        profile("print", "Print book (PDF)", BookFormat::Pdf),
        ExportProfile {
            toc: true,
            ..profile("styled", "Styled Word document", BookFormat::DocxStyled)
        },
        ExportProfile {
            scene_separator: "***".into(),
            ..profile("markdown", "Markdown", BookFormat::Markdown)
        },
        ExportProfile {
            title_page: false,
            ..profile("text", "Plain text", BookFormat::Text)
        },
    ]
}

pub fn load_profiles(root: &Path) -> Result<Vec<ExportProfile>> {
    let path = root.join(PROFILES_PATH);
    match fs::read_to_string(&path) {
        Ok(text) => {
            let file: ProfilesFile = serde_norway::from_str(&text)
                .map_err(|e| MarginError::Parse(format!("{PROFILES_PATH}: {e}")))?;
            Ok(file.profiles)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(default_profiles()),
        Err(e) => Err(e.into()),
    }
}

pub fn save_profiles(root: &Path, profiles: &[ExportProfile]) -> Result<()> {
    let file = ProfilesFile {
        schema_version: PROFILES_SCHEMA_VERSION,
        profiles: profiles.to_vec(),
    };
    let yaml = serde_norway::to_string(&file)?;
    atomic_write(&root.join(PROFILES_PATH), yaml.as_bytes())?;
    Ok(())
}

/// Insert or replace one profile; returns the whole list.
pub fn upsert_profile(root: &Path, p: ExportProfile) -> Result<Vec<ExportProfile>> {
    if p.id.trim().is_empty() || p.name.trim().is_empty() {
        return Err(MarginError::Invalid(
            "a profile needs an id and a name".into(),
        ));
    }
    let mut all = load_profiles(root)?;
    match all.iter_mut().find(|x| x.id == p.id) {
        Some(slot) => *slot = p,
        None => all.push(p),
    }
    save_profiles(root, &all)?;
    Ok(all)
}

pub fn remove_profile(root: &Path, id: &str) -> Result<Vec<ExportProfile>> {
    let mut all = load_profiles(root)?;
    all.retain(|p| p.id != id);
    save_profiles(root, &all)?;
    Ok(all)
}

const ONES: [&str; 20] = [
    "Zero",
    "One",
    "Two",
    "Three",
    "Four",
    "Five",
    "Six",
    "Seven",
    "Eight",
    "Nine",
    "Ten",
    "Eleven",
    "Twelve",
    "Thirteen",
    "Fourteen",
    "Fifteen",
    "Sixteen",
    "Seventeen",
    "Eighteen",
    "Nineteen",
];
const TENS: [&str; 10] = [
    "", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety",
];

/// "One", "Twenty-One", "One Hundred Five".
pub fn number_words(n: u32) -> String {
    match n {
        0..=19 => ONES[n as usize].to_string(),
        20..=99 => {
            let (t, o) = (TENS[(n / 10) as usize], n % 10);
            if o == 0 {
                t.to_string()
            } else {
                format!("{t}-{}", ONES[o as usize])
            }
        }
        100..=999 => {
            let rest = n % 100;
            let head = format!("{} Hundred", ONES[(n / 100) as usize]);
            if rest == 0 {
                head
            } else {
                format!("{head} {}", number_words(rest))
            }
        }
        _ => n.to_string(),
    }
}
