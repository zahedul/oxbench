# Oxbench

A personal, offline developer toolkit written in Rust with [egui](https://github.com/emilk/egui).

## Layout

- `crates/oxbench-core`: pure tool logic, with no GUI dependency, fully unit-tested
- `crates/oxbench-app`: the egui desktop shell and one UI file per tool

## Commands

| Command | Purpose |
|---|---|
| `cargo run -p oxbench-app` | Run the app |
| `cargo test` | Run all tests |
| `cargo clippy --all-targets -- -D warnings` | Lint |
| `cargo fmt` | Format |
| `cargo build --release` | Optimised binary in `target/release/oxbench` |
| `OXBENCH_SCREENSHOT_DIR=/tmp/shots cargo test -p oxbench-app render_screenshots -- --ignored` | Render the UI to PNGs (both themes) for review |

## Add a tool

1. Logic and tests in `crates/oxbench-core/src/<tool>.rs`
2. UI in `crates/oxbench-app/src/tools/<tool>.rs`
3. One entry in `crates/oxbench-app/src/registry.rs`

## Credits

Bundled fonts in `assets/fonts/`, each with its licence file:

- [Inter](https://rsms.me/inter/) (SIL Open Font License 1.1)
- [JetBrains Mono](https://www.jetbrains.com/lp/mono/) (SIL Open Font License 1.1)
- [Phosphor Icons](https://phosphoricons.com/) (MIT)
