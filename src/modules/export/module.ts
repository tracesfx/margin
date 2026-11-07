import { EXPORT_VIEW, defineModule, openExport, sdkVersion } from "@margin/sdk";

import { ExportHome } from "./ExportHome";
import { startExport } from "./store";

export default defineModule({
  id: "export",
  name: "Export",
  description: "Turn the book into a Word manuscript, a typeset PDF, Markdown or plain text, with saved profiles and a page preview.",
  sdkVersion,
  activate: startExport,
  contributes: {
    workspaceViews: [{ id: EXPORT_VIEW, title: "Export", component: ExportHome }],
    exporters: [
      { id: "export.docx-manuscript", title: "Word: standard manuscript" },
      { id: "export.pdf", title: "PDF: typeset book" },
      { id: "export.docx-styled", title: "Word: styled document" },
      { id: "export.markdown", title: "Markdown" },
      { id: "export.text", title: "Plain text" },
    ],
    commands: [{ id: "export.open", title: "Export the book…", category: "File", keybinding: "Ctrl+Alt+E", run: openExport }],
  },
});
