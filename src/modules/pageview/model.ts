/** Page view arithmetic, kept apart from the component so it can be tested. */

/** A click inside a page image, in points from the page's top left. */
export function pageFromClick(
  px: number,
  py: number,
  shownWidth: number,
  shownHeight: number,
  pageWidthPt: number,
  pageHeightPt: number,
): { x: number; y: number } {
  if (shownWidth <= 0 || shownHeight <= 0) return { x: 0, y: 0 };
  return { x: (px / shownWidth) * pageWidthPt, y: (py / shownHeight) * pageHeightPt };
}

/**
 * Which pages still need fetching: all of them, in order, until each is
 * loaded. A chapter is short, so there is no windowing by scroll position.
 */
export function pagesAround(pageCount: number, have: Record<number, unknown>): number[] {
  const out: number[] = [];
  for (let i = 0; i < pageCount; i++) if (!have[i]) out.push(i);
  return out;
}
