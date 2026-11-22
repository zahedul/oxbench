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

## Desktop integration

- **Global shortcut**: ⇧⌘Space on macOS, Ctrl+Alt+Space on Windows and Linux (X11), brings
  Oxbench to the front from any app.
- **Menu bar icon** (system tray on Windows) with a menu to open any tool. Not on Linux.
- **Update check**: at launch, asks GitHub for the latest release and links to it when newer.
  Nothing is downloaded or installed automatically.

Each can be switched off from the gear menu at the top right.

## Packaging

Install the packager once with `cargo install cargo-packager --locked`, then:

```sh
cd crates/oxbench-app
cargo packager --release              # every format for this platform
cargo packager --release --formats dmg
```

Installers land in `target/packages/`. Its settings are in `[package.metadata.packager]` in
`crates/oxbench-app/Cargo.toml`.

| Platform | Formats |
|---|---|
| macOS | `.app`, `.dmg` |
| Windows | NSIS `-setup.exe` |
| Linux | `.deb`, `.AppImage` |

### Releases

Push a version tag (for example `git tag v0.2.0 && git push origin v0.2.0`) and the
`Release` workflow builds the installers on all three platforms and attaches them to a
**draft** GitHub release. Publish the draft once it has been checked. Bump `version` in the
root `Cargo.toml` first so the app's update check sees the new number.

### macOS signing and notarisation

Builds are **ad-hoc signed** (`signing-identity = "-"`). They run on the Mac that built them;
on other Macs, Gatekeeper blocks the first launch until it is allowed in
System Settings → Privacy & Security (or with `xattr -dr com.apple.quarantine Oxbench.app`).

To sign and notarise for every Mac, with an Apple Developer ID:

- **Locally**: set `signing-identity` to the certificate's name, such as
  `"Developer ID Application: Your Name (TEAMID)"`, and export `APPLE_ID`,
  `APPLE_PASSWORD` (an app-specific password) and `APPLE_TEAM_ID` before running the packager.
- **In CI**: add the repository secrets `APPLE_SIGNING_IDENTITY`, `APPLE_CERTIFICATE`
  (base64 of the exported `.p12`), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD`
  and `APPLE_TEAM_ID`. The release workflow uses them when they are present.

## Add a tool

1. Logic and tests in `crates/oxbench-core/src/<tool>.rs`
2. UI in `crates/oxbench-app/src/tools/<tool>.rs`
3. One entry in `crates/oxbench-app/src/registry.rs`

## Credits

Bundled fonts in `assets/fonts/`, each with its licence file:

- [Inter](https://rsms.me/inter/) (SIL Open Font License 1.1)
- [JetBrains Mono](https://www.jetbrains.com/lp/mono/) (SIL Open Font License 1.1)
- [Phosphor Icons](https://phosphoricons.com/) (MIT)

The app icon in `assets/icons/` is drawn from Phosphor's `code` glyph.
