import { defineModule, editorHost, sdkVersion, workbench } from "@margin/sdk";

import { isSpeaking, speak, stopSpeaking } from "./speech";

function readAloud(whole: boolean) {
  const h = editorHost.active();
  if (!h) return workbench.notify({ message: "Open a scene to read it aloud.", timeoutMs: 3000 });
  const sel = h.selectedText().trim();
  const text = whole || !sel ? h.documentText() : sel;
  if (!text.trim()) return workbench.notify({ message: "Nothing to read here.", timeoutMs: 3000 });
  const ok = speak(text, { onEnd: () => workbench.notify({ message: "Finished reading.", timeoutMs: 2000, key: "readaloud" }) });
  workbench.notify({
    message: ok ? "Reading aloud. Run “Stop reading” from the palette to stop." : "No Windows voice installed on this computer can be used (online voices are not used).",
    timeoutMs: ok ? 3000 : 6000,
    key: "readaloud",
  });
}

export default defineModule({
  id: "readaloud",
  name: "Read aloud",
  description: "Read the selection or the scene aloud with the voices installed in Windows, for proofreading.",
  sdkVersion,
  activate: () => stopSpeaking,
  contributes: {
    commands: [
      { id: "readaloud.selection", title: "Read selection aloud (or the scene if nothing is selected)", category: "Writing", keybinding: "Ctrl+Alt+R", run: () => readAloud(false) },
      { id: "readaloud.scene", title: "Read the whole scene aloud", category: "Writing", run: () => readAloud(true) },
      {
        id: "readaloud.stop",
        title: "Stop reading",
        category: "Writing",
        run: () => {
          if (isSpeaking()) stopSpeaking();
        },
      },
    ],
  },
});
