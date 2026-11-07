/**
 * First-run guide (SPEC §11.8): create a project, connect a model, five key
 * shortcuts. A quiet card, not a dialog; dismissing it (or Esc inside it)
 * means it is never shown again. The flag lives in this browser profile, not
 * in any project, so it follows the author rather than the book.
 */
import { useEffect, useState } from "react";

import { openSettings, shellUi } from "./shellUi";

const KEY = "margin.onboarded";

export function wasOnboarded(): boolean {
  try {
    return localStorage.getItem(KEY) === "1";
  } catch {
    return true; // Storage blocked: never nag.
  }
}

function markOnboarded(): void {
  try {
    localStorage.setItem(KEY, "1");
  } catch {
    /* ignore */
  }
}

export const ONBOARDING_SHORTCUTS: [string, string][] = [
  ["Ctrl+K", "Command palette: every action, by name"],
  ["Ctrl+P", "Quick open a scene or bible entry"],
  ["Ctrl+N", "New scene (Ctrl+Alt+N for a chapter)"],
  ["Ctrl+Alt+E", "Export to Word, PDF or Markdown"],
  ["Ctrl+/", "All keyboard shortcuts (rebindable)"],
];

export function Onboarding() {
  const [shown, setShown] = useState(() => !wasOnboarded());
  useEffect(() => {
    const on = () => setShown(true);
    window.addEventListener("margin:onboarding", on);
    return () => window.removeEventListener("margin:onboarding", on);
  }, []);
  if (!shown) return null;
  const dismiss = () => {
    markOnboarded();
    setShown(false);
  };
  return (
    <section
      className="onboarding"
      role="region"
      aria-label="Welcome to Margin"
      onKeyDown={(e) => {
        if (e.key === "Escape") dismiss();
      }}
    >
      <h2>Welcome to Margin</h2>
      <ol>
        <li>
          <strong>Your project is ready.</strong> Add scenes and chapters in the Binder on the left. Everything is saved as plain files in your project folder.
        </li>
        <li>
          <strong>Connect a model (optional).</strong> The writing assistant runs on this computer only. Without a model, every other feature still works.{" "}
          <button className="link" onClick={() => openSettings("models")}>
            Open model settings
          </button>
        </li>
        <li>
          <strong>Five shortcuts to start with.</strong>
          <ul className="onboarding-keys">
            {ONBOARDING_SHORTCUTS.map(([k, d]) => (
              <li key={k}>
                <kbd>{k}</kbd> {d}
              </li>
            ))}
          </ul>
        </li>
      </ol>
      <div className="onboarding-actions">
        <button className="primary" onClick={dismiss} autoFocus>
          Got it
        </button>
        <button onClick={() => shellUi.open("cheatSheet")}>Show all shortcuts</button>
        <span className="muted">You can bring this back from the command palette: “Show the welcome guide”.</span>
      </div>
    </section>
  );
}

/** Show the guide again (palette command). */
export function resetOnboarding(): void {
  try {
    localStorage.removeItem(KEY);
  } catch {
    /* ignore */
  }
  window.dispatchEvent(new Event("margin:onboarding"));
}
