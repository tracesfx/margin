/**
 * Writing sprints (SPEC §11.4): a timer in the status bar and a count of the
 * words written during it. The count is the session word count now minus
 * what it was at the start, so it follows the stats service and never reads
 * the document. Ending a sprint leaves one quiet notice, not a dialog.
 */
import { create } from "zustand";

export interface Sprint {
  startedAt: number;
  endsAt: number;
  minutes: number;
  startWords: number;
}

interface SprintStore {
  sprint: Sprint | null;
}

export const useSprint = create<SprintStore>(() => ({ sprint: null }));

export function startSprint(minutes: number, sessionWords: number, now = Date.now()): void {
  useSprint.setState({ sprint: { startedAt: now, endsAt: now + minutes * 60_000, minutes, startWords: sessionWords } });
}

export function stopSprint(): void {
  useSprint.setState({ sprint: null });
}

export function sprintWords(s: Sprint, sessionWords: number): number {
  return Math.max(0, sessionWords - s.startWords);
}

export function remainingMs(s: Sprint, now = Date.now()): number {
  return Math.max(0, s.endsAt - now);
}

export function clock(ms: number): string {
  const total = Math.ceil(ms / 1000);
  const m = Math.floor(total / 60);
  const sec = total % 60;
  return `${m}:${String(sec).padStart(2, "0")}`;
}

/** The summary line shown when a sprint ends. */
export function summaryLine(minutes: number, words: number): string {
  const rate = minutes > 0 ? Math.round((words / minutes) * 60) : 0;
  return `Sprint done: ${words.toLocaleString("en-US")} ${words === 1 ? "word" : "words"} in ${minutes} min (${rate.toLocaleString("en-US")} words/hour).`;
}
