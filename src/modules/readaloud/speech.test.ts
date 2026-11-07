import { describe, expect, it } from "vitest";

import { chunkText, localVoices } from "./speech";

describe("read aloud", () => {
  it("uses only voices that run on this computer", () => {
    const v = [
      { localService: true, lang: "en-US", name: "Microsoft David" },
      { localService: false, lang: "en-US", name: "Microsoft Aria Online (Natural)" },
    ];
    expect(localVoices(v).map((x) => x.name)).toEqual(["Microsoft David"]);
  });
  it("chunks by whole sentences", () => {
    const t = "One. Two! Three? " + "Word ".repeat(30).trim() + ".";
    const chunks = chunkText(t, 40);
    expect(chunks.every((c) => c.length <= 40 || !c.includes(". "))).toBe(true);
    expect(chunks.join(" ").replace(/\s+/g, " ")).toBe(t);
  });
  it("splits at paragraph breaks and ignores blanks", () => {
    expect(chunkText("First paragraph\n\nSecond one", 10)).toEqual(["First paragraph", "Second one"]);
    expect(chunkText("  ")).toEqual([]);
  });
});
