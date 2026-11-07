import { create } from "zustand";

import { settings, updates, useSettingsStore, workbench, type UpdateStatus } from "@margin/sdk";

import { dueForCheck, shouldAnnounce } from "./logic";

interface UpdatesState {
  status: UpdateStatus | null;
  checking: boolean;
  error: string | null;
}

export const useUpdates = create<UpdatesState>(() => ({ status: null, checking: false, error: null }));

/** Run one check. `announce` shows the notice for a version not announced before. */
export async function checkForUpdates(announce: boolean): Promise<void> {
  if (useUpdates.getState().checking) return;
  useUpdates.setState({ checking: true, error: null });
  try {
    const status = await updates.check();
    useUpdates.setState({ status });
    const s = settings.get();
    settings.update({ updateCheckedAt: status.checkedAt });
    if (announce && shouldAnnounce(status.newer, status.latest, s.updateNotified)) {
      settings.update({ updateNotified: status.latest ?? "" });
      const url = status.url;
      workbench.notify({
        message: `Margin ${status.latest} is available (you have ${status.current}).`,
        timeoutMs: 0,
        key: "update",
        ...(url ? { action: { label: "Open download page", run: () => void updates.openRelease(url).catch(() => undefined) } } : {}),
      });
    }
  } catch (e) {
    useUpdates.setState({ error: e instanceof Error ? e.message : String(e) });
  } finally {
    useUpdates.setState({ checking: false });
  }
}

/**
 * The daily check, only if the author turned it on. Waits for the settings to
 * load and for startup to settle; does nothing at all otherwise.
 */
export function startUpdateCheck(): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let done = false;
  const consider = () => {
    const st = useSettingsStore.getState();
    if (done || !st.loaded) return;
    done = true;
    if (!st.settings.updateCheck || !dueForCheck(st.settings.updateCheckedAt, Date.now())) return;
    timer = setTimeout(() => void checkForUpdates(true), 20_000);
  };
  consider();
  const off = useSettingsStore.subscribe(consider);
  return () => {
    off();
    clearTimeout(timer);
  };
}
