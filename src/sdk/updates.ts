/**
 * Update check client. The check is off unless the author turns it on in
 * Settings → Updates or presses "Check now": it is the one place Margin
 * contacts a non-local address (see `src-tauri/src/update.rs`).
 */
import { commands } from "./bindings";
import { unwrap } from "./story";

export const updates = {
  check: () => unwrap(commands.updateCheck()),
  /** Open a release page (the backend only accepts this project's own pages). */
  openRelease: (url: string) => unwrap(commands.updateOpenRelease(url)),
};
