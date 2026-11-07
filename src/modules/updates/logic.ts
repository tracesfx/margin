/** Update-check decisions, kept apart from the UI so they can be tested. */

const DAY_MS = 24 * 60 * 60 * 1000;

/** Whether a daily check is due: never checked, a bad date, or a day has passed. */
export function dueForCheck(lastIso: string, now: number): boolean {
  const last = Date.parse(lastIso);
  if (!Number.isFinite(last)) return true;
  return now - last >= DAY_MS || last > now + DAY_MS;
}

/** Whether to show a notice for `latest`: it is newer, and was not announced before. */
export function shouldAnnounce(newer: boolean, latest: string | null, notified: string): boolean {
  return newer && latest !== null && latest !== notified;
}
