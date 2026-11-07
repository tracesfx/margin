/**
 * The Export workspace (SPEC §12): profiles on the left, the selected
 * profile's settings in the middle, and the typeset pages on the right for
 * PDF profiles. Exports run in the background; the page stays usable.
 */
import { useEffect } from "react";

import { pickExportDir, useProject, type WorkspaceViewProps } from "@margin/sdk";

import { ProfileForm } from "./ProfileForm";
import { Preview } from "./Preview";
import { formatLabel, usesTemplate } from "./model";
import { addProfile, editProfile, loadExport, removeProfile, runExport, selectProfile, useExportStore } from "./store";

export function ExportHome({ active }: WorkspaceViewProps) {
  const project = useProject();
  const s = useExportStore();
  const profile = s.profiles.find((p) => p.id === s.selectedId) ?? null;

  useEffect(() => {
    if (active && project) void loadExport();
  }, [active, project?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!project) return <div className="export-home"><p className="muted export-empty">Open a project to export it.</p></div>;

  const chooseFolder = async () => {
    const dir = await pickExportDir();
    if (dir && profile) editProfile({ ...profile, outputDir: dir });
  };

  return (
    <div className="export-home">
      <nav className="export-list" aria-label="Export profiles">
        <h2>Export</h2>
        <ul>
          {s.profiles.map((p) => (
            <li key={p.id}>
              <button className={p.id === s.selectedId ? "active" : ""} onClick={() => selectProfile(p.id)}>
                <span className="insights-nav-title">{p.name}</span>
                <span className="insights-nav-desc">{formatLabel(p.format)}</span>
              </button>
            </li>
          ))}
        </ul>
        <div className="export-list-actions">
          <button onClick={() => void addProfile()} disabled={!profile}>
            Duplicate
          </button>
          <button onClick={() => profile && void removeProfile(profile.id)} disabled={!profile || s.profiles.length < 2}>
            Remove
          </button>
        </div>
        <p className="muted export-note">Profiles are saved with the project, in exports/profiles.yaml.</p>
      </nav>

      <div className="export-main">
        {s.error && (
          <p className="export-error" role="alert">
            {s.error}
          </p>
        )}
        {!s.loaded && <p className="muted export-empty">Loading…</p>}
        {profile && (
          <>
            <div className="export-actions">
              <button className="primary" disabled={s.running} onClick={() => void runExport(profile)}>
                {s.running ? "Exporting…" : "Export"}
              </button>
              {s.result && (
                <span className="export-result" role="status">
                  Exported {s.result.words.toLocaleString("en-US")} words{s.result.pages ? ` in ${s.result.pages} pages` : ""}:{" "}
                  {s.result.files.map((f) => (
                    <code key={f}>{f}</code>
                  ))}
                </span>
              )}
              {s.runError && (
                <span className="export-error" role="alert">
                  {s.runError}
                </span>
              )}
            </div>
            {s.result?.warnings.map((w) => (
              <p key={w} className="export-warn">
                {w}
              </p>
            ))}
            <div className="export-split">
              <ProfileForm profile={profile} templates={s.templates} fonts={s.fonts} onChange={editProfile} onChooseFolder={() => void chooseFolder()} />
              {usesTemplate(profile.format) && <Preview profile={profile} active={active} />}
            </div>
          </>
        )}
      </div>
    </div>
  );
}
