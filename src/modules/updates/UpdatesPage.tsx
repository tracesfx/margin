import { settings, updates, useSettings } from "@margin/sdk";

import { checkForUpdates, useUpdates } from "./store";

export function UpdatesPage() {
  const auto = useSettings((s) => s.updateCheck);
  const lastChecked = useSettings((s) => s.updateCheckedAt);
  const { status, checking, error } = useUpdates();
  const url = status?.url;
  return (
    <section>
      <h2>Updates</h2>
      <p>
        Margin never connects to the internet on its own. This is the one exception, and it is yours to switch on: it asks GitHub whether a newer version has been published. Nothing is
        sent but what any web request reveals (your computer's address and the name “Margin-update-check”): no text, no project, no version number. Nothing is downloaded or installed;
        you open the page and download the installer yourself.
      </p>
      <label className="check">
        <input type="checkbox" checked={auto} onChange={(e) => settings.update({ updateCheck: e.target.checked })} />
        <span>Check for updates once a day</span>
      </label>
      <p>
        <button onClick={() => void checkForUpdates(false)} disabled={checking}>
          {checking ? "Checking…" : "Check now"}
        </button>{" "}
        <span className="muted" role="status">
          {error
            ? error
            : status
              ? status.newer
                ? `Margin ${status.latest} is available (you have ${status.current}).`
                : status.latest
                  ? `You have the latest version (${status.current}).`
                  : `No release has been published yet (you have ${status.current}).`
              : lastChecked
                ? `Last checked ${new Date(lastChecked).toLocaleString()}.`
                : "Not checked yet."}
        </span>
      </p>
      {status?.newer && url && (
        <p>
          <button className="primary" onClick={() => void updates.openRelease(url).catch(() => undefined)}>
            Open the download page
          </button>
        </p>
      )}
    </section>
  );
}
