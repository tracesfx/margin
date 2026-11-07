/**
 * Export client (SPEC §12): profiles, templates, the preview and the export
 * itself. Wraps the generated IPC bindings like `story` and `memory` do. The
 * work runs on the Rust side and never changes the manuscript.
 */
import { open } from "@tauri-apps/plugin-dialog";

import { commands, type ExportProfile, type TemplateSettings } from "./bindings";
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
  /** Save a template in the project (`id` of a saved one replaces it). Returns every template. */
  saveTemplate: (id: string | null, name: string, settings: TemplateSettings) => unwrap(commands.exportTemplateSave(id, name, settings)),
  deleteTemplate: (id: string) => unwrap(commands.exportTemplateDelete(id)),
  /** Typeset the chapter holding a document for the Page view (read-only). */
  pageView: (docId: string, profile: ExportProfile) => unwrap(commands.pageView(docId, profile)),
  pageViewPages: (key: string, start: number, count: number) => unwrap(commands.pageViewPages(key, start, count)),
  /** The text block under a click (points from the page's top left), if any. */
  pageViewHit: (key: string, page: number, x: number, y: number) => unwrap(commands.pageViewHit(key, page, x, y)),
  fonts: () => unwrap(commands.exportFonts()),
  /** Export into `dir`; `null` uses the profile's own folder. */
  run: (profile: ExportProfile, dir: string | null) => unwrap(commands.exportRun(profile, dir)),
  /** Typeset the book (PDF profiles) and say how many pages it has. */
  preview: (profile: ExportProfile) => unwrap(commands.exportPreview(profile)),
  previewPages: (key: string, start: number, count: number) => unwrap(commands.exportPreviewPages(key, start, count)),
};

/** The workspace view that shows the book's pages beside the writing. */
export const PAGE_VIEW = "pageview.home";

export function openExport(): void {
  workbench.openView(EXPORT_VIEW);
}
