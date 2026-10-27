# Agent Gibbon

[한국어](README.md) · [Website](https://agentgibbon.com/en/) · [Download](https://github.com/egoing/agent-gibbon/releases/latest)

An open-source floating Markdown note app for macOS, created and updated by AI agents through a local CLI. Keep coding progress, checklists, and research notes beside your work without moving keyboard focus away from your editor.

## Design system

The [design system and visual mockups](docs/design-system.md) define shared terminology, components, styles, and behavior (Korean explanations with English terms). Update the document and affected mockups when UI rules change, and include a link in the delivery summary. Adopted design and implementation status are tracked separately.

## Install

Download the Apple Silicon (`aarch64`) or Intel (`x64`) DMG from GitHub Releases and move Agent Gibbon to Applications. Apple distribution signing and notarization are not configured; macOS may block the downloaded application.

## Quick start

```sh
"/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon" open --id work --markdown '# Current task'
"/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon" update --id work --markdown 'Review complete.'
"/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon" inspect --id work
"/Applications/Agent Gibbon.app/Contents/MacOS/agent-gibbon" close --id work
```

`open` creates or shows a note. `update` preserves focus and hidden state. `close` hides the note rather than terminating the app. The CLI returns JSON with `ok` indicating success.

Give your agent the [CLI skill](.agents/skills/agent-gibbon/SKILL.md), or ask it: “Use Agent Gibbon to keep a checklist in note ID work. Update it when a step finishes. Do not reopen a note I have hidden.” The agent needs permission to run local shell commands; the app does not include an AI model.

## Features and scope

- Multiple note IDs, independent colors, Markdown tables, code, and checklists.
- Floating windows with manual editing, copying, folding, and pinning.
- Optional source-file connection using `--file`; external changes require an explicit update.
- Local CLI communication without a web server, account, or cloud sync.
- Current source saves notes locally; source features can be newer than the latest downloadable release.
- Published v0.2.7 assets cover macOS. Source now supports Windows x64 packaging; future version tags build macOS ARM64/Intel DMGs and a Windows installer together. Linux packaging is not provided.
- See the [automated release guide](docs/releases.md) for version bumps, tag-triggered releases, and build-only manual runs.

## Build from source

Requires macOS, Rust stable, Node.js/npm, Xcode Command Line Tools, and the macOS SDK.

```sh
git clone https://github.com/egoing/agent-gibbon.git
cd agent-gibbon
npm ci
npm run build
./scripts/agent-gibbon open --id sample --markdown '# Hello'
```

The app uses Tauri 2, Rust, and the system WebView. See the [Korean README](README.md) for full CLI behavior, development instance isolation, local storage, and verification details.

## Diagnostics

Official release builds may send application error diagnostics through Sentry. Current source disables Sentry for development/tests and builds without an embedded production DSN. Already published binaries retain their release behavior. See [telemetry documentation](README.md#sentry-오류-수집).

## License

[MIT](LICENSE). Commercial use, modification, and redistribution are permitted subject to the license notice.

## Windows development

See the [Windows development and build guide](docs/windows.md) for prerequisites, isolated development CLI, NSIS packaging, and platform limitations. Validation evidence is recorded in [verification](docs/verification.md).
