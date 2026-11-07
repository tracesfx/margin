//! IPC commands for formatting and export (SPEC §12). Thin: the work is in
//! `export::`, and none of it changes the manuscript.

use std::sync::Arc;

use tauri::State;

use crate::error::{MarginError, Result};
use crate::export::ExportService;
use crate::export::types::*;

pub type Exp = Arc<ExportService>;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| MarginError::Internal(e.to_string()))?
}

macro_rules! cmd {
    ($name:ident($($arg:ident : $ty:ty),*) -> $ret:ty = |$svc:ident| $body:expr) => {
        #[tauri::command]
        #[specta::specta]
        pub async fn $name(svc: State<'_, Exp>, $($arg: $ty),*) -> Result<$ret> {
            let $svc = svc.inner().clone();
            $body
        }
    };
}

cmd!(export_profiles() -> Vec<ExportProfile> = |s| blocking(move || s.profiles()).await);
cmd!(export_profile_save(profile: ExportProfile) -> Vec<ExportProfile> =
    |s| blocking(move || s.profile_save(profile)).await);
cmd!(export_profile_delete(id: String) -> Vec<ExportProfile> =
    |s| blocking(move || s.profile_delete(&id)).await);
cmd!(export_templates() -> Vec<TemplateInfo> = |s| Ok(s.templates()));
cmd!(export_fonts() -> Vec<String> = |s| blocking(move || Ok(s.fonts())).await);
cmd!(export_run(profile: ExportProfile, dir: Option<String>) -> ExportResult =
    |s| blocking(move || s.run(&profile, dir.as_deref())).await);
cmd!(export_preview(profile: ExportProfile) -> PreviewInfo =
    |s| blocking(move || s.preview(&profile)).await);
cmd!(export_preview_pages(key: String, start: u32, count: u32) -> Vec<PreviewPage> =
    |s| blocking(move || s.preview_pages(&key, start, count)).await);
