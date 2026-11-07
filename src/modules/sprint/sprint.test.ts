import { beforeEach, describe, expect, it } from "vitest";

import { clock, remainingMs, sprintWords, startSprint, stopSprint, summaryLine, useSprint } from "./store";

describe("sprint", () => {
  beforeEach(() => stopSprint());

  it("counts words from the start of the sprint", () => {
    startSprint(10, 120, 1000);
    const s = useSprint.getState().sprint!;
    expect(sprintWords(s, 120)).toBe(0);
    expect(sprintWords(s, 350)).toBe(230);
    expect(sprintWords(s, 100)).toBe(0);
  });
  it("counts down and stops at zero", () => {
    startSprint(1, 0, 0);
    const s = useSprint.getState().sprint!;
    expect(remainingMs(s, 30_000)).toBe(30_000);
    expect(remainingMs(s, 90_000)).toBe(0);
  });
  it("formats the clock and the summary", () => {
    expect(clock(61_000)).toBe("1:01");
    expect(clock(0)).toBe("0:00");
    expect(summaryLine(25, 500)).toBe("Sprint done: 500 words in 25 min (1,200 words/hour).");
    expect(summaryLine(10, 1)).toContain("1 word in");
  });
});
