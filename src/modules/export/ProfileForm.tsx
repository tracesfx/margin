/**
 * One export profile's settings (SPEC §12.5). Only what the chosen format
 * uses is shown. Every change saves itself.
 */
import type { ReactNode } from "react";

import type { BookFormat, ChapterOpening, ExportProfile, ItalicStyle, Numbering, TemplateInfo, TemplateSettings } from "@margin/sdk";

import { deleteTemplate, renameTemplate, saveTemplateAs, updateTemplate } from "./store";
import { FORMATS, TRIMS, changeFormat, clampNumber, customised, isDocx, isPlain, trimId, usesTemplate, withTemplate, withTrim } from "./model";

interface Props {
  profile: ExportProfile;
  templates: TemplateInfo[];
  fonts: string[];
  onChange: (p: ExportProfile) => void;
  onChooseFolder: () => void;
}

function Row({ label, children, hint }: { label: string; children: ReactNode; hint?: string }) {
  return (
    <label className="export-row">
      <span className="export-label">{label}</span>
      <span className="export-control">{children}</span>
      {hint && <span className="export-hint muted">{hint}</span>}
    </label>
  );
}

function Check({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="export-check">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      <span>{label}</span>
    </label>
  );
}

const OPENINGS: { id: ChapterOpening; label: string }[] = [
  { id: "plain", label: "Plain" },
  { id: "smallCapsLead", label: "Small-caps lead-in" },
  { id: "largeInitial", label: "Large first letter" },
];

export function ProfileForm({ profile: p, templates, fonts, onChange, onChooseFolder }: Props) {
  const set = (patch: Partial<ExportProfile>) => onChange({ ...p, ...patch });
  const setT = (patch: Partial<TemplateSettings>) => onChange({ ...p, settings: { ...p.settings, ...patch } });
  const num = (v: string, lo: number, hi: number, fallback: number) => clampNumber(Number.parseFloat(v), lo, hi, fallback);
  const f: BookFormat = p.format;
  const typeset = usesTemplate(f);
  const changed = customised(p, templates);
  const saved = templates.find((t) => t.id === p.template && !t.builtin);

  return (
    <form className="export-form" onSubmit={(e) => e.preventDefault()} aria-label="Export settings">
      <section>
        <h3>Profile</h3>
        <Row label="Name">
          <input value={p.name} onChange={(e) => set({ name: e.target.value })} />
        </Row>
        <Row label="Format" hint={FORMATS.find((x) => x.id === f)?.hint}>
          <select value={f} onChange={(e) => onChange(changeFormat(p, e.target.value as BookFormat))}>
            {FORMATS.map((x) => (
              <option key={x.id} value={x.id}>
                {x.label}
              </option>
            ))}
          </select>
        </Row>
        <Row label="Folder" hint={p.outputDir ? undefined : "Asked each time you export."}>
          <span className="export-folder">
            <code title={p.outputDir ?? ""}>{p.outputDir ?? "—"}</code>
            <button type="button" onClick={onChooseFolder}>
              Choose…
            </button>
            {p.outputDir && (
              <button type="button" onClick={() => set({ outputDir: null })}>
                Ask each time
              </button>
            )}
          </span>
        </Row>
        <Row label="File name" hint="Without the extension. Empty uses the book's title.">
          <input value={p.fileName ?? ""} placeholder="Book title" onChange={(e) => set({ fileName: e.target.value || null })} />
        </Row>
      </section>

      <section>
        <h3>Contents</h3>
        {f !== "docxManuscript" && (
          <div className="export-checks">
            <Check label="Front matter" checked={p.includeFrontMatter} onChange={(v) => set({ includeFrontMatter: v })} />
            <Check label="Back matter" checked={p.includeBackMatter} onChange={(v) => set({ includeBackMatter: v })} />
            <Check label="Title page" checked={p.titlePage} onChange={(v) => set({ titlePage: v })} />
            {(typeset || f === "docxStyled") && <Check label="Table of contents" checked={p.toc} onChange={(v) => set({ toc: v })} />}
          </div>
        )}
        {f === "docxManuscript" && (
          <div className="export-checks">
            <Check label="Front matter" checked={p.includeFrontMatter} onChange={(v) => set({ includeFrontMatter: v })} />
            <Check label="Back matter" checked={p.includeBackMatter} onChange={(v) => set({ includeBackMatter: v })} />
          </div>
        )}
        <p className="muted export-note">Scenes and documents switched off in the binder are never exported. Comments are not exported.</p>
        {f !== "docxManuscript" && (
          <>
            <Row label="Copyright page">
              <textarea rows={3} value={p.copyright} placeholder="Copyright © 2026 …" onChange={(e) => set({ copyright: e.target.value })} />
            </Row>
            <Row label="Dedication">
              <textarea rows={2} value={p.dedication} onChange={(e) => set({ dedication: e.target.value })} />
            </Row>
            <Row label="Epigraph">
              <textarea rows={2} value={p.epigraph} onChange={(e) => set({ epigraph: e.target.value })} />
            </Row>
            {p.epigraph.trim() && (
              <Row label="Epigraph by">
                <input value={p.epigraphBy} onChange={(e) => set({ epigraphBy: e.target.value })} />
              </Row>
            )}
          </>
        )}
      </section>

      <section>
        <h3>Chapters and scenes</h3>
        <Row label="Chapter numbers">
          <select value={p.numbering} onChange={(e) => set({ numbering: e.target.value as Numbering })}>
            <option value="numeric">Chapter 3</option>
            <option value="words">Chapter Three</option>
            <option value="none">Titles only</option>
          </select>
        </Row>
        {p.numbering !== "none" && (
          <Row label="Word before the number">
            <input value={p.chapterLabel} onChange={(e) => set({ chapterLabel: e.target.value })} />
          </Row>
        )}
        {!typeset && (
          <Row label="Scene break" hint={f === "docxManuscript" ? "The standard is a centred #." : undefined}>
            <input value={p.sceneSeparator} onChange={(e) => set({ sceneSeparator: e.target.value })} />
          </Row>
        )}
        {isPlain(f) && <Check label="One file per chapter" checked={p.splitChapters} onChange={(v) => set({ splitChapters: v })} />}
        {f === "text" && (
          <Row label="Italics">
            <select value={p.italics} onChange={(e) => set({ italics: e.target.value as ItalicStyle })}>
              <option value="underscores">_underscores_</option>
              <option value="asterisks">*asterisks*</option>
              <option value="plain">Not marked</option>
            </select>
          </Row>
        )}
      </section>

      {f === "docxManuscript" && (
        <section>
          <h3>Manuscript</h3>
          <Row label="Font">
            <select value={p.manuscriptFont} onChange={(e) => set({ manuscriptFont: e.target.value })}>
              <option>Times New Roman</option>
              <option>Courier New</option>
            </select>
          </Row>
          <Row label="Your name">
            <input value={p.contact.name} onChange={(e) => set({ contact: { ...p.contact, name: e.target.value } })} />
          </Row>
          <Row label="Address">
            <textarea rows={2} value={p.contact.address} onChange={(e) => set({ contact: { ...p.contact, address: e.target.value } })} />
          </Row>
          <Row label="Email">
            <input value={p.contact.email} onChange={(e) => set({ contact: { ...p.contact, email: e.target.value } })} />
          </Row>
          <Row label="Phone">
            <input value={p.contact.phone} onChange={(e) => set({ contact: { ...p.contact, phone: e.target.value } })} />
          </Row>
          <p className="muted export-note">The first page gets this block and a rounded word count; the header reads "Surname / TITLE / page".</p>
        </section>
      )}

      {isDocx(f) && f === "docxStyled" && (
        <section>
          <h3>Look</h3>
          <Row label="Font">
            <input list="export-fonts" value={p.settings.font} onChange={(e) => setT({ font: e.target.value })} />
          </Row>
          <Row label="Size (pt)">
            <input type="number" min={8} max={18} step={0.5} value={p.settings.sizePt ?? ""} onChange={(e) => setT({ sizePt: num(e.target.value, 8, 18, 11) })} />
          </Row>
          <Row label="Paragraph indent (em)">
            <input type="number" min={0} max={4} step={0.1} value={p.settings.indentEm ?? ""} onChange={(e) => setT({ indentEm: num(e.target.value, 0, 4, 1.5) })} />
          </Row>
          <Check label="Justified" checked={p.settings.justify} onChange={(v) => setT({ justify: v })} />
        </section>
      )}

      {typeset && (
        <section>
          <h3>Template</h3>
          <Row label="Template" hint={changed.length ? `Changed from ${templates.find((t) => t.id === p.template)?.name ?? "the template"}: ${changed.join(", ")}.` : undefined}>
            <span className="export-folder">
              <select value={p.template} onChange={(e) => onChange(withTemplate(p, templates, e.target.value))}>
                {templates.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.builtin ? t.name : `${t.name} (saved)`}
                  </option>
                ))}
                {!templates.some((t) => t.id === p.template) && <option value={p.template}>Removed template</option>}
              </select>
              {changed.length > 0 && (
                <button type="button" onClick={() => onChange(withTemplate(p, templates, p.template))}>
                  Reset
                </button>
              )}
            </span>
          </Row>
          <Row label="Saved templates" hint={saved ? "Saved in this project's templates folder." : "Keep these settings as a template for other profiles."}>
            <span className="export-folder">
              <button type="button" onClick={() => void saveTemplateAs(p)}>
                Save as template…
              </button>
              {saved && changed.length > 0 && (
                <button type="button" onClick={() => void updateTemplate(p)}>
                  Update “{saved.name}”
                </button>
              )}
              {saved && (
                <>
                  <button type="button" onClick={() => void renameTemplate(saved.id)}>
                    Rename…
                  </button>
                  <button type="button" onClick={() => void deleteTemplate(saved.id)}>
                    Delete
                  </button>
                </>
              )}
            </span>
          </Row>
          <Row label="Trim size">
            <span className="export-folder">
              <select value={trimId(p.settings)} onChange={(e) => onChange(withTrim(p, e.target.value))}>
                {TRIMS.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.label}
                  </option>
                ))}
                <option value="custom">Custom</option>
              </select>
              <input aria-label="Width in inches" type="number" min={3} max={12} step={0.125} value={p.settings.trimWidthIn ?? ""} onChange={(e) => setT({ trimWidthIn: num(e.target.value, 3, 12, 5.5) })} />
              ×
              <input aria-label="Height in inches" type="number" min={4} max={14} step={0.125} value={p.settings.trimHeightIn ?? ""} onChange={(e) => setT({ trimHeightIn: num(e.target.value, 4, 14, 8.5) })} />
              in
            </span>
          </Row>
          <Row label="Font" hint="Libertinus Serif and New Computer Modern come with Margin. Another font is used if it is installed.">
            <input list="export-fonts" value={p.settings.font} onChange={(e) => setT({ font: e.target.value })} />
          </Row>
          <datalist id="export-fonts">
            {fonts.map((x) => (
              <option key={x} value={x} />
            ))}
          </datalist>
          <Row label="Size (pt)">
            <input type="number" min={8} max={16} step={0.5} value={p.settings.sizePt ?? ""} onChange={(e) => setT({ sizePt: num(e.target.value, 8, 16, 11) })} />
          </Row>
          <Row label="Line spacing (em)">
            <input type="number" min={0.3} max={1.5} step={0.05} value={p.settings.leadingEm ?? ""} onChange={(e) => setT({ leadingEm: num(e.target.value, 0.3, 1.5, 0.65) })} />
          </Row>
          <Row label="Paragraph indent (em)">
            <input type="number" min={0} max={4} step={0.1} value={p.settings.indentEm ?? ""} onChange={(e) => setT({ indentEm: num(e.target.value, 0, 4, 1.5) })} />
          </Row>
          {p.settings.indentEm === 0 && (
            <Row label="Space between paragraphs (em)">
              <input type="number" min={0} max={3} step={0.1} value={p.settings.paragraphSpaceEm ?? ""} onChange={(e) => setT({ paragraphSpaceEm: num(e.target.value, 0, 3, 0.8) })} />
            </Row>
          )}
          <Row label="Chapter opening">
            <select value={p.settings.chapterOpening} onChange={(e) => setT({ chapterOpening: e.target.value as ChapterOpening })}>
              {OPENINGS.map((o) => (
                <option key={o.id} value={o.id}>
                  {o.label}
                </option>
              ))}
            </select>
          </Row>
          <Row label="Scene break mark">
            <input value={p.settings.ornament} onChange={(e) => setT({ ornament: e.target.value })} />
          </Row>
          <div className="export-checks">
            <Check label="Justified" checked={p.settings.justify} onChange={(v) => setT({ justify: v })} />
            <Check label="Hyphenation (US English)" checked={p.settings.hyphenate} onChange={(v) => setT({ hyphenate: v })} />
            <Check label="Running heads" checked={p.settings.runningHeads} onChange={(v) => setT({ runningHeads: v })} />
            <Check label="Page numbers" checked={p.settings.pageNumbers} onChange={(v) => setT({ pageNumbers: v })} />
            <Check label="Chapters start on right-hand pages" checked={p.settings.rectoStarts} onChange={(v) => setT({ rectoStarts: v })} />
            <Check label="Avoid widows and orphans" checked={p.settings.widowOrphan} onChange={(v) => setT({ widowOrphan: v })} />
          </div>
        </section>
      )}
    </form>
  );
}
