import { describe, expect, it } from "vitest";

import { DEFAULT_SETTINGS } from "@margin/sdk";

import { dueForCheck, shouldAnnounce } from "./logic";

const DAY = 24 * 3600 * 1000;

describe("update check", () => {
  it("is off by default and has never run", () => {
    expect(DEFAULT_SETTINGS.updateCheck).toBe(false);
    expect(DEFAULT_SETTINGS.updateCheckedAt).toBe("");
  });
  it("is due when never checked, after a day, or if the clock moved back", () => {
    const now = Date.parse("2026-11-08T12:00:00Z");
    expect(dueForCheck("", now)).toBe(true);
    expect(dueForCheck("garbage", now)).toBe(true);
    expect(dueForCheck(new Date(now - DAY + 60_000).toISOString(), now)).toBe(false);
    expect(dueForCheck(new Date(now - DAY).toISOString(), now)).toBe(true);
    expect(dueForCheck(new Date(now + 3 * DAY).toISOString(), now)).toBe(true);
  });
  it("announces a newer version once", () => {
    expect(shouldAnnounce(true, "0.2.0", "")).toBe(true);
    expect(shouldAnnounce(true, "0.2.0", "0.2.0")).toBe(false);
    expect(shouldAnnounce(false, "0.1.0", "")).toBe(false);
    expect(shouldAnnounce(true, null, "")).toBe(false);
  });
});
