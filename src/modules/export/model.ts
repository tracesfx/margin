/**
 * Pure helpers for the Export workspace (SPEC §12): labels, trim sizes,
 * and the profile edits. No IPC here, so it is all unit-tested.
 */
import type { BookFormat, ExportProfile, TemplateInfo, TemplateSettings } from "@margin/sdk";

export const FORMATS: { id: BookFormat; label: string; hint: string }[] = [
  { id: "docxManuscript", label: "Word: standard manuscript", hint: "For agents and editors: Times New Roman or Courier, double-spaced, header with your name and page." },
  { id: "pdf", label: "PDF: typeset book", hint: "A print-ready layout with your template, fonts embedded, bookmarks." },
  { id: "docxStyled", label: "Word: styled document", hint: "Named Word styles for each kind of paragraph, real footnotes, a contents list." },
  { id: "markdown", label: "Markdown", hint: "Plain files with scene breaks and styles kept." },
  { id: "text", label: "Plain text", hint: "No formatting. Choose how italics show." },
];

export const formatLabel = (f: BookFormat) => FORMATS.find((x) => x.id === f)?.label ?? f;

/** Whether the typeset template applies (pages, fonts, trim). */
export const usesTemplate = (f: BookFormat) => f === "pdf";
export const isDocx = (f: BookFormat) => f === "docxManuscript" || f === "docxStyled";
export const isPlain = (f: BookFormat) => f === "markdown" || f === "text";

export const TRIMS: { id: string; label: string; w: number; h: number }[] = [
  { id: "5x8", label: "5 × 8 in", w: 5, h: 8 },
  { id: "5.5x8.5", label: "5.5 × 8.5 in", w: 5.5, h: 8.5 },
  { id: "6x9", label: "6 × 9 in", w: 6, h: 9 },
  { id: "a5", label: "A5 (148 × 210 mm)", w: 5.827, h: 8.268 },
];

export function trimId(s: Pick<TemplateSettings, "trimWidthIn" | "trimHeightIn">): string {
  const t = TRIMS.find((x) => Math.abs(x.w - (s.trimWidthIn ?? 0)) < 0.01 && Math.abs(x.h - (s.trimHeightIn ?? 0)) < 0.01);
  return t?.id ?? "custom";
}

export function withTrim(p: ExportProfile, id: string): ExportProfile {
  const t = TRIMS.find((x) => x.id === id);
  if (!t) return p;
  return { ...p, settings: { ...p.settings, trimWidthIn: t.w, trimHeightIn: t.h } };
}

/** Choose a template: its settings replace the profile's. */
export function withTemplate(p: ExportProfile, templates: TemplateInfo[], id: string): ExportProfile {
  const t = templates.find((x) => x.id === id);
  return t ? { ...p, template: id, settings: { ...t.settings } } : p;
}

/** Which settings differ from the template the profile started from. */
export function customised(p: ExportProfile, templates: TemplateInfo[]): string[] {
  const t = templates.find((x) => x.id === p.template);
  if (!t) return [];
  return (Object.keys(t.settings) as (keyof TemplateSettings)[]).filter((k) => t.settings[k] !== p.settings[k]);
}

let counter = 0;
/** A new profile from an existing one (or a default), with a fresh id. */
export function duplicate(p: ExportProfile, existing: ExportProfile[]): ExportProfile {
  const names = new Set(existing.map((x) => x.name));
  let name = `${p.name} copy`;
  for (let n = 2; names.has(name); n++) name = `${p.name} copy ${n}`;
  counter += 1;
  return { ...p, id: `p${Date.now().toString(36)}${counter}`, name, settings: { ...p.settings }, contact: { ...p.contact } };
}

/** The scene separator a format starts with when the author changes format. */
export function separatorFor(f: BookFormat, current: string): string {
  if (f === "docxManuscript") return "#";
  if (f === "markdown") return "***";
  if (current === "#" || current === "***") return "* * *";
  return current;
}

export function changeFormat(p: ExportProfile, f: BookFormat): ExportProfile {
  return { ...p, format: f, sceneSeparator: separatorFor(f, p.sceneSeparator) };
}

export const clampNumber = (v: number, lo: number, hi: number, fallback: number) =>
  Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : fallback;

/** Which pages show together. Page 1 is a right-hand page, so a spread is (2,3), (4,5)… */
export function spreadAt(index: number, spread: boolean): number[] {
  if (!spread) return [index];
  if (index === 0) return [0];
  const left = index % 2 === 1 ? index : index - 1;
  return [left, left + 1];
}
