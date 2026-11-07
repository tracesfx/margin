# Bundled book fonts

These fonts ship inside Margin (compiled into the program) so a book looks the same on any computer. All are licensed under the SIL Open Font License 1.1; each folder holds its licence. The installer also copies the licences next to the program (`licenses/fonts/`).

| Family | Used by | Source | Licence file |
|---|---|---|---|
| EB Garamond (Regular, Italic, Bold, Bold Italic) | Classic template | github.com/octaviopardo/EBGaramond12, `fonts/ttf` | `eb-garamond/OFL.txt` |
| Crimson Text (same four styles) | Literary template | github.com/google/fonts, `ofl/crimsontext` | `crimson-text/OFL.txt` |
| Source Serif 4 (Regular, Italic, Bold, Bold Italic; static OTF) | Modern template | github.com/adobe-fonts/source-serif, release 4.005R | `source-serif-4/OFL.md` |

Typst also bundles Libertinus Serif, New Computer Modern and DejaVu Sans Mono (Non-fiction template and fallbacks).

To add a family: put the static TTF/OTF files and the licence here, add them to `book_fonts()` in `src/export/world.rs`, list the name in `bundled()` (and `has_small_caps()` if it has real small capitals) in `src/export/typeset.rs`, and add the licence to `bundle.resources` in `tauri.conf.json`.
