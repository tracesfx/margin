import { defineModule, sdkVersion } from "@margin/sdk";

import { UpdatesPage } from "./UpdatesPage";
import { checkForUpdates, startUpdateCheck } from "./store";

export default defineModule({
  id: "updates",
  name: "Update check",
  description: "Optionally checks GitHub once a day for a newer Margin. Off by default; sends no project data.",
  sdkVersion,
  activate: startUpdateCheck,
  contributes: {
    settingsPages: [{ id: "updates", title: "Updates", component: UpdatesPage }],
    commands: [{ id: "updates.check", title: "Check for updates now", category: "Help", run: () => void checkForUpdates(false) }],
  },
});
