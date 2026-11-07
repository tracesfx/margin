/**
 * Export workspace state: the project's profiles, the built-in templates and
 * the last result. Edits save after a short pause (no Save button, no
 * confirmation; SPEC §2.5); the file is `exports/profiles.yaml`.
 */
import { create } from "zustand";

import { events, exporter, pickExportDir, workbench, type ExportProfile, type ExportResult, type TemplateInfo } from "@margin/sdk";

import { duplicate } from "./model";

interface ExportStore {
  profiles: ExportProfile[];
  templates: TemplateInfo[];
  fonts: string[];
  selectedId: string | null;
  loaded: boolean;
  error: string | null;
  running: boolean;
  result: ExportResult | null;
  runError: string | null;
}

export const useExportStore = create<ExportStore>(() => ({
  profiles: [],
  templates: [],
  fonts: [],
  selectedId: null,
  loaded: false,
  error: null,
  running: false,
  result: null,
  runError: null,
}));

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
const saveTimers = new Map<string, ReturnType<typeof setTimeout>>();

export async function loadExport(): Promise<void> {
  try {
    const [profiles, templates] = await Promise.all([exporter.profiles(), exporter.templates()]);
    useExportStore.setState((s) => ({
      profiles,
      templates,
      loaded: true,
      error: null,
      selectedId: profiles.some((p) => p.id === s.selectedId) ? s.selectedId : (profiles[0]?.id ?? null),
    }));
    // Installed fonts take a moment to list; the form works without them.
    void exporter.fonts().then((fonts) => useExportStore.setState({ fonts })).catch(() => {});
  } catch (e) {
    useExportStore.setState({ loaded: true, error: message(e) });
  }
}

export function selectProfile(id: string): void {
  useExportStore.setState({ selectedId: id, result: null, runError: null });
}

/** Replace a profile in the list now, save it a moment later. */
export function editProfile(p: ExportProfile): void {
  useExportStore.setState((s) => ({ profiles: s.profiles.map((x) => (x.id === p.id ? p : x)) }));
  clearTimeout(saveTimers.get(p.id));
  saveTimers.set(
    p.id,
    setTimeout(() => {
      saveTimers.delete(p.id);
      exporter.saveProfile(p).catch((e: unknown) => useExportStore.setState({ error: message(e) }));
    }, 500),
  );
}

export async function addProfile(from?: ExportProfile): Promise<void> {
  const { profiles, selectedId } = useExportStore.getState();
  const base = from ?? profiles.find((p) => p.id === selectedId) ?? profiles[0];
  if (!base) return;
  const p = duplicate(base, profiles);
  try {
    const all = await exporter.saveProfile(p);
    useExportStore.setState({ profiles: all, selectedId: p.id, result: null, runError: null });
  } catch (e) {
    useExportStore.setState({ error: message(e) });
  }
}

/** Delete a profile. It comes back from the same place a file does: Undo in the toast. */
export async function removeProfile(id: string): Promise<void> {
  const { profiles } = useExportStore.getState();
  const gone = profiles.find((p) => p.id === id);
  if (!gone) return;
  try {
    const all = await exporter.deleteProfile(id);
    useExportStore.setState((s) => ({ profiles: all, selectedId: s.selectedId === id ? (all[0]?.id ?? null) : s.selectedId }));
    workbench.notify({
      message: `Removed "${gone.name}"`,
      action: {
        label: "Undo",
        run: () => {
          void exporter.saveProfile(gone).then((back) => useExportStore.setState({ profiles: back, selectedId: gone.id }));
        },
      },
    });
  } catch (e) {
    useExportStore.setState({ error: message(e) });
  }
}

/** Export with a profile. Asks for a folder when the profile has none. */
export async function runExport(p: ExportProfile): Promise<void> {
  if (useExportStore.getState().running) return;
  let dir: string | null = null;
  if (!p.outputDir) {
    dir = await pickExportDir();
    if (!dir) return;
  }
  useExportStore.setState({ running: true, result: null, runError: null });
  try {
    const result = await exporter.run(p, dir);
    useExportStore.setState({ result });
    const where = result.files.length === 1 ? result.files[0] : `${result.files.length} files`;
    workbench.notify({ message: `Exported ${where}` });
  } catch (e) {
    useExportStore.setState({ runError: message(e) });
  } finally {
    useExportStore.setState({ running: false });
  }
}

let wired = false;
/** Keep the workspace honest when the project changes under it. */
export function startExport(): () => void {
  if (wired) return () => {};
  wired = true;
  const offs = [
    events.on("project.opened", () => void loadExport()),
    events.on("project.closed", () => useExportStore.setState({ profiles: [], selectedId: null, loaded: false, result: null })),
  ];
  return () => {
    wired = false;
    offs.forEach((off) => off());
  };
}
