/**
 * The book's pages as they will print (SPEC §12.2): the same Typst layout
 * the PDF is made from, shown as SVG, one page or a spread at a time. Pages
 * are fetched a window at a time, so a long book costs no more than a short one.
 */
import { useEffect, useMemo, useRef, useState } from "react";

import { exporter, type ExportProfile, type PreviewInfo, type PreviewPage } from "@margin/sdk";

import { spreadAt } from "./model";

const WINDOW = 4;

const dataUrl = (svg: string) => `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;

export function Preview({ profile, active }: { profile: ExportProfile; active: boolean }) {
  const [info, setInfo] = useState<PreviewInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [page, setPage] = useState(0);
  const [spread, setSpread] = useState(true);
  const [pages, setPages] = useState<Record<number, PreviewPage>>({});
  const keyRef = useRef<string | null>(null);

  // Typeset again a moment after the last change. Nothing runs while hidden.
  const settingsKey = useMemo(() => JSON.stringify(profile), [profile]);
  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    const t = setTimeout(() => {
      setBusy(true);
      exporter
        .preview(profile)
        .then((next) => {
          if (cancelled) return;
          if (next.key !== keyRef.current) {
            keyRef.current = next.key;
            setPages({});
          }
          setInfo(next);
          setError(null);
          setPage((p) => Math.min(p, Math.max(0, next.pageCount - 1)));
        })
        .catch((e: unknown) => !cancelled && setError(e instanceof Error ? e.message : String(e)))
        .finally(() => !cancelled && setBusy(false));
    }, 700);
    return () => {
      cancelled = true;
      clearTimeout(t);
    };
    // The profile is read through its JSON, so a typed character does not retrigger needlessly.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settingsKey, active]);

  const shown = info ? spreadAt(page, spread).filter((i) => i < info.pageCount) : [];
  const need = shown.filter((i) => !pages[i]);
  useEffect(() => {
    if (!info || need.length === 0) return;
    const start = Math.min(...need);
    let cancelled = false;
    exporter
      .previewPages(info.key, start, WINDOW)
      .then((got) => {
        if (cancelled) return;
        setPages((prev) => ({ ...prev, ...Object.fromEntries(got.map((g) => [g.index, g])) }));
      })
      .catch((e: unknown) => !cancelled && setError(e instanceof Error ? e.message : String(e)));
    return () => {
      cancelled = true;
    };
  }, [info, need.join(",")]); // eslint-disable-line react-hooks/exhaustive-deps

  const step = spread ? 2 : 1;
  const last = info ? info.pageCount - 1 : 0;
  const move = (d: number) => setPage((p) => Math.min(last, Math.max(0, p + d)));
  const aspect = info && info.widthPt && info.heightPt ? info.widthPt / info.heightPt : 5.5 / 8.5;

  return (
    <div className="export-preview" aria-label="Page preview">
      <div className="export-preview-bar">
        <button onClick={() => setPage(0)} disabled={page === 0} aria-label="First page">
          ⏮
        </button>
        <button onClick={() => move(-step)} disabled={page === 0} aria-label="Previous">
          ◀
        </button>
        <span className="export-pages" aria-live="polite">
          {info ? (shown.length > 1 ? `Pages ${shown[0]! + 1}–${shown[1]! + 1}` : `Page ${(shown[0] ?? 0) + 1}`) : "—"} of {info?.pageCount ?? "—"}
        </span>
        <button onClick={() => move(step)} disabled={!info || page >= last} aria-label="Next">
          ▶
        </button>
        <button onClick={() => setPage(last)} disabled={!info || page >= last} aria-label="Last page">
          ⏭
        </button>
        <label className="export-check">
          <input type="checkbox" checked={spread} onChange={(e) => setSpread(e.target.checked)} />
          <span>Spreads</span>
        </label>
        {busy && <span className="muted">Typesetting…</span>}
      </div>
      {error && (
        <p className="export-error" role="alert">
          {error}
        </p>
      )}
      {info?.warnings.map((w) => (
        <p key={w} className="export-warn">
          {w}
        </p>
      ))}
      <div className="export-sheet" style={{ ["--page-aspect" as string]: String(aspect) }}>
        {shown.map((i) => (
          <div key={i} className="export-page" data-page={i + 1}>
            {pages[i] ? <img alt={`Page ${i + 1}`} src={dataUrl(pages[i].svg)} /> : <div className="export-page-wait" />}
          </div>
        ))}
        {!info && !error && <p className="muted">Typesetting the book…</p>}
      </div>
    </div>
  );
}
