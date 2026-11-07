import { useEffect, useState } from "react";

import { workbench } from "@margin/sdk";

import { useStats } from "../stats/store";
import { clock, remainingMs, sprintWords, stopSprint, summaryLine, useSprint } from "./store";

export function SprintStatusItem() {
  const sprint = useSprint((s) => s.sprint);
  const sessionWords = useStats((s) => s.summary?.sessionWords ?? 0);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (!sprint) return;
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, [sprint]);

  const left = sprint ? remainingMs(sprint, now) : 0;
  useEffect(() => {
    if (!sprint || left > 0) return;
    workbench.notify({ message: summaryLine(sprint.minutes, sprintWords(sprint, sessionWords)), timeoutMs: 12000, key: "sprint" });
    stopSprint();
  }, [sprint, left, sessionWords]);

  if (!sprint) return null;
  const words = sprintWords(sprint, sessionWords);
  return (
    <button
      className="status-sprint link"
      title="Writing sprint. Click to end it early."
      aria-label={`Writing sprint, ${clock(left)} left, ${words} words so far. Activate to end it.`}
      onClick={() => {
        workbench.notify({ message: summaryLine(Math.max(1, Math.round((now - sprint.startedAt) / 60000)), words), timeoutMs: 8000, key: "sprint" });
        stopSprint();
      }}
    >
      <span aria-live="off">
        Sprint {clock(left)} · {words.toLocaleString("en-US")} words
      </span>
    </button>
  );
}
