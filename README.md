# bd-neon

Fast, native Kanban GUI for [Beads](https://github.com/gastownhall/beads) issue tracking.
For devs and agents using the `bd` CLI: instant board, keyboard-first navigation, and a glass OLED-friendly UI. Shells out to `bd` for all issue data — no daemon, no lock-in.

## Tech stack

- Rust 2021, eframe 0.36 (egui + glow)
- egui-phosphor icons, egui_commonmark
- notify file-watch, rayon parallel
- chrono + serde / serde_json
- Windows DWM glass (windows + raw-window-handle, winres)
- Strict clippy: pedantic + nursery
