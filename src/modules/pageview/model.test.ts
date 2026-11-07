import { describe, expect, it } from "vitest";

import { pageFromClick, pagesAround } from "./model";

describe("page view", () => {
  it("scales a click on the shown image back to points on the page", () => {
    // A 396 x 612 pt page shown at 198 x 306 px.
    expect(pageFromClick(99, 153, 198, 306, 396, 612)).toEqual({ x: 198, y: 306 });
    expect(pageFromClick(0, 0, 198, 306, 396, 612)).toEqual({ x: 0, y: 0 });
  });
  it("survives an image with no size yet", () => {
    expect(pageFromClick(10, 10, 0, 0, 396, 612)).toEqual({ x: 0, y: 0 });
  });
  it("lists the pages still to fetch", () => {
    expect(pagesAround(4, { 0: {}, 2: {} })).toEqual([1, 3]);
    expect(pagesAround(0, {})).toEqual([]);
    expect(pagesAround(2, { 0: {}, 1: {} })).toEqual([]);
  });
});
