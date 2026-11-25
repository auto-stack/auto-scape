# AutoScape

AutoScape is the native browser application for the Auto Scape Engine (ASE). This Windows source build opens the seven-region browser with the native Gallery, including Counter, Clock, and the original 013 Todo. It supports local `app://` navigation and read-only AutoWeb over HTTPS.

## Build and run on Windows

Use Git, PowerShell 7, and Rust with the Windows MSVC toolchain. Clone into a parent directory reserved for this application's pinned source checkouts. From the application checkout, run:

```powershell
pwsh -NoProfile -File scripts/setup-upstreams.ps1
cargo build --locked
cargo run --locked
```

Cargo uses the sibling repositories in [upstreams.json](upstreams.json), including AutoUI's Iced widget and GPU patches. The setup script verifies each exact commit and tracked working tree. It also checks the original Todo source and tutorial bytes, which must match the Gallery's registered digests. A local mirror can supply the same commits with `-MirrorRoot <parent-of-mirrors>`. This initial revision requires that mirror while the AutoLang, AutoUI and AutoWeb pins are awaiting publication. Keep the checked-out source siblings alongside the app for this source-build prototype. The binary can run from another current directory.

The browser saves favorites in its user profile. `--config-home <dir>` or `AUTOSCAPE_HOME` isolates that profile. Todo uses in-memory state by default. Create an empty directory and launch with `--todo-data <absolute-dir>` to persist Todo there. Use the same directory on the next launch.

## Verify

The integration suite exercises Gallery input and previews, browser navigation, and a loopback HTTPS server with an isolated trust root. Run the window cases serially on a Windows desktop. Provider preparation is a separate step.

```powershell
cargo build --locked
./target/debug/auto-scape.exe --prep-only
$env:ACE_NODE_DESKTOP_PREPARED_RUSTC = rustc --version
$env:AUTO_UI_ASE_M0_E2E = '1'
cargo test --locked --tests -- --test-threads=1
```

## Source ownership

This repository owns the launched application, shell, address bar, tabs, bookmarks, and Gallery screens. AutoLang supplies ACE and ASE execution. AutoUI supplies the AGE-backed native view, input and focus contracts, local package sessions, Gallery source verification, and the AutoWeb client binding. AutoWeb and AutoDown supply protocol and document support.

The browser shell, native launcher, and regression tests originate from the MIT-licensed AutoUI prototype at `74bb8d5fcef2e523a446625dca7444cab57b0371` (AutoLang PLAN-1128). See [LICENSE](LICENSE).
