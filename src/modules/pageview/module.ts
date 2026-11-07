import { PAGE_VIEW, defineModule, sdkVersion, useWorkbench } from "@margin/sdk";

import { PageView } from "./PageView";

/** Open the Page view in the right-hand pane, beside the text. */
function showPageView(): void {
  const ws = useWorkbench.getState();
  if (!ws.panes.some((p) => p.id === "right")) ws.toggleSplit();
  useWorkbench.getState().openView(PAGE_VIEW, undefined, { pane: "right" });
}

export default defineModule({
  id: "pageview",
  name: "Page view",
  description: "The chapter you are writing, typeset as it will print, beside the text. Click a line to jump to the paragraph.",
  sdkVersion,
  contributes: {
    workspaceViews: [{ id: PAGE_VIEW, title: "Page view", component: PageView }],
    commands: [{ id: "pageview.show", title: "Show the Page view beside the text", category: "View", keybinding: "Ctrl+Alt+L", run: showPageView }],
  },
});
