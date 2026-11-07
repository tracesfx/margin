import { describe, expect, it } from "vitest";

import type { ExportProfile, TemplateInfo, TemplateSettings } from "@margin/sdk";

import { changeFormat, clampNumber, customised, duplicate, formatLabel, separatorFor, spreadAt, trimId, usesTemplate, withTemplate, withTrim } from "./model";

const settings = (over: Partial<TemplateSettings> = {}): TemplateSettings => ({
  font: "Garamond",
  sizePt: 11,
  leadingEm: 0.65,
  indentEm: 1.5,
  paragraphSpaceEm: 0,
  justify: true,
  hyphenate: true,
  chapterOpening: "plain",
  ornament: "* * *",
  runningHeads: true,
  pageNumbers: true,
  rectoStarts: true,
  widowOrphan: true,
  trimWidthIn: 5.5,
  trimHeightIn: 8.5,
  ...over,
});

const profile = (over: Partial<ExportProfile> = {}): ExportProfile => ({
  id: "a",
  name: "Print",
  format: "pdf",
  includeFrontMatter: true,
  includeBackMatter: true,
  titlePage: true,
  toc: false,
  copyright: "",
  dedication: "",
  epigraph: "",
  epigraphBy: "",
  numbering: "numeric",
  chapterLabel: "Chapter",
  sceneSeparator: "* * *",
  template: "classic",
  settings: settings(),
  splitChapters: false,
  italics: "underscores",
  manuscriptFont: "Times New Roman",
  contact: { name: "", address: "", email: "", phone: "" },
  outputDir: null,
  fileName: null,
  ...over,
});

const templates: TemplateInfo[] = [
  { id: "classic", name: "Classic", settings: settings() },
  { id: "modern", name: "Modern", settings: settings({ font: "Georgia", indentEm: 0, trimWidthIn: 6, trimHeightIn: 9 }) },
];

describe("export model", () => {
  it("knows which formats use the typeset template", () => {
    expect(usesTemplate("pdf")).toBe(true);
    expect(usesTemplate("docxStyled")).toBe(false);
    expect(formatLabel("markdown")).toBe("Markdown");
  });

  it("recognises trim sizes and sets them", () => {
    expect(trimId(settings())).toBe("5.5x8.5");
    expect(trimId(settings({ trimWidthIn: 4.25, trimHeightIn: 7 }))).toBe("custom");
    const p = withTrim(profile(), "6x9");
    expect(p.settings.trimWidthIn).toBe(6);
    expect(p.settings.trimHeightIn).toBe(9);
    expect(withTrim(profile(), "custom")).toEqual(profile());
  });

  it("a template replaces the settings and keeps the rest", () => {
    const p = withTemplate(profile({ name: "Mine" }), templates, "modern");
    expect(p.template).toBe("modern");
    expect(p.settings.font).toBe("Georgia");
    expect(p.name).toBe("Mine");
    expect(withTemplate(profile(), templates, "missing").template).toBe("classic");
  });

  it("lists what the author changed from the template", () => {
    expect(customised(profile(), templates)).toEqual([]);
    const p = profile({ settings: settings({ sizePt: 12, justify: false }) });
    expect(customised(p, templates).sort()).toEqual(["justify", "sizePt"]);
  });

  it("duplicates with a fresh id and a free name", () => {
    const a = profile();
    const b = duplicate(a, [a]);
    expect(b.id).not.toBe(a.id);
    expect(b.name).toBe("Print copy");
    expect(duplicate(a, [a, b]).name).toBe("Print copy 2");
    b.settings.sizePt = 99;
    expect(a.settings.sizePt).toBe(11);
  });

  it("switches the scene separator with the format", () => {
    expect(separatorFor("docxManuscript", "* * *")).toBe("#");
    expect(separatorFor("markdown", "#")).toBe("***");
    expect(separatorFor("pdf", "#")).toBe("* * *");
    expect(separatorFor("text", "~")).toBe("~");
    expect(changeFormat(profile(), "docxManuscript").sceneSeparator).toBe("#");
  });

  it("clamps numbers", () => {
    expect(clampNumber(20, 6, 16, 11)).toBe(16);
    expect(clampNumber(Number.NaN, 6, 16, 11)).toBe(11);
  });

  it("pairs pages as printed spreads: page 1 alone on the right", () => {
    expect(spreadAt(0, true)).toEqual([0]);
    expect(spreadAt(1, true)).toEqual([1, 2]);
    expect(spreadAt(2, true)).toEqual([1, 2]);
    expect(spreadAt(3, true)).toEqual([3, 4]);
    expect(spreadAt(4, false)).toEqual([4]);
  });
});
