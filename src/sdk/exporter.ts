/**
 * Export client (SPEC §12): profiles, templates, the preview and the export
 * itself. Wraps the generated IPC bindings like `story` and `memory` do. The
 * work runs on the Rust side and never changes the manuscript.
 */
import { open } from "@tauri-apps/plugin-dialog";

import { commands, type ExportProfile } from "./bindings";
import { unwrap } from "./story";
import { workbench } from "./workbench";

/** The workspace view that hosts export. */
export const EXPORT_VIEW = "export.home";

let lastDir: string | null = null;

/**
 * Ask for the folder an export goes to (the native picker, not a
 * confirmation). `__marginExportDir` lets the CDP driver answer for it.
 */
export async function pickExportDir(): Promise<string | null> {
  const forced = (window as { __marginExportDir?: string }).__marginExportDir;
  const dir = forced ?? (await open({ directory: true, multiple: false, defaultPath: lastDir ?? undefined, title: "Export to…" }));
  if (typeof dir !== "string") return null;
  lastDir = dir;
  return dir;
}

export const exporter = {
  profiles: () => unwrap(commands.exportProfiles()),
  saveProfile: (profile: ExportProfile) => unwrap(commands.exportProfileSave(profile)),
  deleteProfile: (id: string) => unwrap(commands.exportProfileDelete(id)),
  templates: () => unwrap(commands.exportTemplates()),
  fonts: () => unwrap(commands.exportFonts()),
  /** Export into `dir`; `null` uses the profile's own folder. */
  run: (profile: ExportProfile, dir: string | null) => unwrap(commands.exportRun(profile, dir)),
  /** Typeset the book (PDF profiles) and say how many pages it has. */
  preview: (profile: ExportProfile) => unwrap(commands.exportPreview(profile)),
  previewPages: (key: string, start: number, count: number) => unwrap(commands.exportPreviewPages(key, start, count)),
};

export function openExport(): void {
  workbench.openView(EXPORT_VIEW);
}
