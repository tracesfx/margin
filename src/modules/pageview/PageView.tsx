/**
 * The Page view (SPEC §12.2): the chapter you are writing, typeset with a
 * print profile, shown beside the text. Read-only. Click a line and the
 * editor jumps to the paragraph it came from.
 *
 * It follows the open scene, and typesets again a moment after the text is
 * saved. Typesetting runs on the Rust side and never touches the editor, so
 * typing is not affected.
 */
import { useEffect, useMemo, useRef, useState } from "react";

import {
  editorHost,
  events,
  exporter,
  uiState,
  useProject,
  useWorkbench,
  workbench,
  type ExportProfile,
  type PreviewInfo,
  type PreviewPage,
  type WorkspaceViewProps,
} from "@margin/sdk";

import { pageFromClick, pagesAround } from "./model";

const WINDOW = 6;
const PROFILE_KEY = "pageview.profile";

const dataUrl = (svg: string) => `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
const text = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** The scene the author is writing: the focused document tab, else any open one. */
function useWritingDoc(): string | undefined {
  // Remembered while the active tabs are not documents (the Page view itself, Settings…).
  const [last, setLast] = useState<string | undefined>();
  const current = useWorkbench((s) => {
    const docView = s.viewFor("document");
    const order = [s.focusedPane, ...s.panes.map((p) => p.id)];
    for (const id of order) {
      const pane = s.panes.find((p) => p.id === id);
      const tab = pane?.tabs.find((t) => t.id === pane.activeTabId);
      if (tab?.resourceId && tab.viewId === docView) return tab.resourceId;
    }
    return undefined;
  });
  if (current && current !== last) setLast(current);
  return current ?? last;
}

export function PageView({ active }: WorkspaceViewProps) {
  const project = useProject();
  const docId = useWritingDoc();
  const [profiles, setProfiles] = useState<ExportProfile[]>([]);
  const [profileId, setProfileId] = useState<string | undefined>(() => uiState.get<string>(PROFILE_KEY));
  const [tick, setTick] = useState(0);
  const [info, setInfo] = useState<PreviewInfo | null>(null);
  const [pages, setPages] = useState<Record<number, PreviewPage>>({});
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const keyRef = useRef<string | null>(null);

  // Print profiles: the ones that make a typeset book.
  useEffect(() => {
    if (!active || !project) return;
    exporter
      .profiles()
      .then((all) => setProfiles(all.filter((p) => p.format === "pdf")))
      .catch((e: unknown) => setError(text(e)));
  }, [active, project?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const profile = useMemo(() => profiles.find((p) => p.id === profileId) ?? profiles[0], [profiles, profileId]);
  const profileJson = useMemo(() => JSON.stringify(profile ?? null), [profile]);

  // Typeset again after text is saved or the binder changes.
  useEffect(() => {
    if (!active) return;
    const bump = () => setTick((t) => t + 1);
    const offs = [events.on("doc.saved", bump), events.on("binder.changed", bump)];
    return () => offs.forEach((o) => o());
  }, [active]);

  useEffect(() => {
    if (!active || !docId || !profile) return;
    let cancelled = false;
    const t = setTimeout(() => {
      setBusy(true);
      exporter
        .pageView(docId, profile)
        .then((next) => {
          if (cancelled) return;
          if (next.key !== keyRef.current) {
            keyRef.current = next.key;
            setPages({});
          }
          setInfo(next);
          setError(null);
        })
        .catch((e: unknown) => {
          if (cancelled) return;
          // Notes and the like are not in the book: say so and keep the last pages.
          setError(text(e));
        })
        .finally(() => !cancelled && setBusy(false));
    }, 500);
    return () => {
      cancelled = true;
      clearTimeout(t);
    };
    // The profile is compared through its JSON.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, docId, profileJson, tick]);

  // Fetch the pages in windows.
  const missing = info ? pagesAround(info.pageCount, pages) : [];
  useEffect(() => {
    if (!info || missing.length === 0) return;
    let cancelled = false;
    exporter
      .pageViewPages(info.key, Math.min(...missing), WINDOW)
      .then((got) => !cancelled && setPages((prev) => ({ ...prev, ...Object.fromEntries(got.map((g) => [g.index, g])) })))
      .catch(() => {
        /* A newer typesetting replaced this one; the next render fetches again. */
      });
    return () => {
      cancelled = true;
    };
  }, [info, missing.join(",")]); // eslint-disable-line react-hooks/exhaustive-deps

  async function jump(e: React.MouseEvent<HTMLImageElement>, index: number) {
    if (!info) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const at = pageFromClick(e.clientX - rect.left, e.clientY - rect.top, rect.width, rect.height, info.widthPt ?? 0, info.heightPt ?? 0);
    try {
      const hit = await exporter.pageViewHit(info.key, index, at.x, at.y);
      if (!hit) return;
      // The writing happens in the left pane; the Page view sits beside it.
      editorHost.openAt(hit.doc, { block: hit.block, start: 0, end: 0 }, { pane: "left" });
    } catch (err) {
      workbench.notify({ message: `Could not find that text: ${text(err)}`, timeoutMs: 4000 });
    }
  }

  if (!project) return null;
  return (
    <div className="pageview" aria-label="Page view">
      <div className="pageview-bar">
        {profiles.length > 1 && (
          <label className="pageview-profile">
            <span className="muted">Layout</span>
            <select
              value={profile?.id ?? ""}
              onChange={(e) => {
                setProfileId(e.target.value);
                uiState.set(PROFILE_KEY, e.target.value);
              }}
            >
              {profiles.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </label>
        )}
        <span className="muted pageview-count" aria-live="polite">
          {busy ? "Typesetting…" : info ? `${info.pageCount} ${info.pageCount === 1 ? "page" : "pages"} in this chapter` : ""}
        </span>
      </div>
      {error && (
        <p className="export-warn" role="status">
          {error}
        </p>
      )}
      {info?.warnings.map((w) => (
        <p key={w} className="export-warn">
          {w}
        </p>
      ))}
      {!docId && <p className="muted pageview-empty">Open a scene to see its pages here.</p>}
      {profiles.length === 0 && docId && <p className="muted pageview-empty">This project has no PDF export profile. Add one in Export.</p>}
      <div className="pageview-scroll">
        {info &&
          Array.from({ length: info.pageCount }, (_, i) => (
            <figure key={i} className="pageview-page" style={{ aspectRatio: `${info.widthPt} / ${info.heightPt}` }}>
              {pages[i] ? (
                <img
                  alt={`Page ${i + 1} of this chapter. Click a line to go to that paragraph.`}
                  src={dataUrl(pages[i].svg)}
                  onClick={(e) => void jump(e, i)}
                  draggable={false}
                />
              ) : (
                <div className="export-page-wait" />
              )}
            </figure>
          ))}
      </div>
    </div>
  );
}
