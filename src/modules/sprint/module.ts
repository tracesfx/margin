import { defineModule, sdkVersion, workbench } from "@margin/sdk";

import { useStats } from "../stats/store";
import { SprintStatusItem } from "./SprintItem";
import { startSprint, stopSprint, useSprint } from "./store";

function start(minutes: number) {
  startSprint(minutes, useStats.getState().summary?.sessionWords ?? 0);
  workbench.notify({ message: `Sprint started: ${minutes} minutes.`, timeoutMs: 2500, key: "sprint" });
}

export default defineModule({
  id: "sprint",
  name: "Writing sprints",
  description: "A timer in the status bar with the words written during it.",
  sdkVersion,
  contributes: {
    statusBarItems: [{ id: "sprint.timer", align: "right", order: -11, component: SprintStatusItem }],
    commands: [
      { id: "sprint.start25", title: "Start a 25-minute sprint", category: "Writing", keybinding: "Ctrl+Alt+T", run: () => start(25) },
      { id: "sprint.start15", title: "Start a 15-minute sprint", category: "Writing", run: () => start(15) },
      { id: "sprint.start10", title: "Start a 10-minute sprint", category: "Writing", run: () => start(10) },
      {
        id: "sprint.stop",
        title: "End the sprint",
        category: "Writing",
        run: () => {
          if (useSprint.getState().sprint) stopSprint();
        },
      },
    ],
  },
});
