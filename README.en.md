# Jajan

[한국어](README.md) · [Website](https://jajan.app/en/) · [Download](https://github.com/egoing/agent-gibbon/releases/latest)

An open-source floating Markdown note app for macOS, created and updated by AI agents through a local CLI. Keep coding progress, checklists, and research notes beside your work without moving keyboard focus away from your editor.

## Design system

The [design system and visual mockups](docs/design-system.md) define shared terminology, components, styles, and behavior (Korean explanations with English terms). Update the document and affected mockups when UI rules change, and include a link in the delivery summary. Adopted design and implementation status are tracked separately.

## Install

Download the Apple Silicon (`aarch64`) or Intel (`x64`) DMG from GitHub Releases and move Jajan to Applications. Version 0.3.5 is Developer ID signed and unnotarized. macOS may block the first launch.

## Quick start

```sh
"/Applications/Jajan.app/Contents/MacOS/jajan" open --id work --markdown '# Current task'
"/Applications/Jajan.app/Contents/MacOS/jajan" update --id work --markdown 'Review complete.'
"/Applications/Jajan.app/Contents/MacOS/jajan" inspect --id work
"/Applications/Jajan.app/Contents/MacOS/jajan" close --id work
```

`open` creates or shows a note. `update` preserves focus and hidden state. `close` hides the note rather than terminating the app. The CLI returns JSON with `ok` indicating success.

**Launch at login:** First launch proposes “로그인 시 자잔 자동 실행” checked, but OS registration happens only after Continue. Existing users are not enrolled automatically. Toggle the checkbox in **Settings → 로그인 자동 실행** to apply the choice immediately. App preference and OS approval are shown separately; Windows registration is reported as approval unknown, with a button to open Startup Apps. Login launch restores note visibility without opening the manager or replaying terminal commands. This feature requires an installed macOS 13+ app bundle or the Windows NSIS desktop app; development and verification builds cannot register. See the [design and verification scope](docs/verification/login-autostart-20261120.md).

Give your agent the [CLI skill](.agents/skills/jajan/SKILL.md), or ask it: “Use Jajan to keep a checklist in note ID work. Update it when a step finishes. Do not reopen a note I have hidden.” The agent needs permission to run local shell commands; the app does not include an AI model.

## Features and scope

- Multiple note IDs, independent colors, Markdown tables, code, and checklists.
- Memo Windows with manual editing, copying, folding, and pinning.
- Compact Toast and Action Bar feedback outside each Memo Window; messages wrap within its width without reducing content space.
- Optional source-file connection using `--file`; external changes require an explicit update.
- Local CLI communication without a web server, account, or cloud sync.
- Current source saves notes locally; source features can be newer than the latest downloadable release.
- Published v0.2.7 assets cover macOS. Source now supports Windows x64 packaging; future version tags build macOS ARM64/Intel DMGs and a Windows installer together. Linux packaging is not provided.
- See the [automated release guide](docs/releases.md) for version bumps, tag-triggered releases, and build-only manual runs.

## Memo connections

Use the plug button in a text, capture/image, terminal, or web memo, move onto another memo, and click. Enter an optional label and choose arrows toward the clicked memo, at both ends, or none. Escape or Cancel exits without saving. No feature activation environment variable is required.

Connections are stored locally. Lines appear while any memo has focus and hide during related movement or hiding. Reconnecting the same pair updates its settings. While a memo drag is held, its lines stay hidden even when the pointer stops. After release, they reappear once the latest endpoint positions and SVG update are confirmed. Editing existing lines by clicking them is tracked separately. See the [design system](docs/design-system.md) for implementation and verification limits.

## Build from source

Requires macOS, Rust 1.91+ stable, Node.js 22.13+/pnpm 11.25.0, Xcode Command Line Tools, and the macOS SDK.

```sh
git clone https://github.com/egoing/agent-gibbon.git
cd jajan
npm install -g pnpm@11.25.0
pnpm install --frozen-lockfile
pnpm run build
./scripts/jajan open --id sample --markdown '# Hello'
```

The app uses Tauri 2, Rust, and the system WebView. See the [Korean README](README.md) for full CLI behavior, development instance isolation, local storage, and verification details.

## Diagnostics

Official release builds may send application error diagnostics through Sentry. Current source disables Sentry for development/tests and builds without an embedded production DSN. Already published binaries retain their release behavior. See [telemetry documentation](README.md#sentry-오류-수집).

## License

[MIT](LICENSE). Commercial use, modification, and redistribution are permitted subject to the license notice.

## Windows development

See the [Windows development and build guide](docs/windows.md) for prerequisites, isolated development CLI, NSIS packaging, and platform limitations. Validation evidence is recorded in [verification](docs/verification.md).

Package build/dev/check scripts share intermediate Cargo artifacts under the Git common directory (`jajan/build-cache`) while keeping final binaries in each worktree’s `src-tauri/target`. `pnpm run dev:info` reports both paths. Set `CARGO_BUILD_BUILD_DIR` for an isolated cache. Direct Cargo commands do not apply this launcher policy. Cross-worktree build scripts can recompile the application and concurrent builds can wait on the shared lock; see the [measurements and limits](docs/verification/shared-build-cache-pnpm-20261114.md).
