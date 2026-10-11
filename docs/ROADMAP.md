# Omni roadmap

Omni is a Rust package inventory and management TUI that unifies system and language package managers. Milestone names and release tags follow [`VERSIONING.md`](VERSIONING.md).

## Milestones

### M1 — Core models and configuration (`v0.1.0-alpha.1`)

- [x] Shared package model, source/install-reason types, sorting and byte formatting.
- [x] TOML configuration for layout, theme, keymaps, behavior, and custom managers.
- [x] Unit and adversarial tests for parsing and formatting boundaries.

### M2 — Providers and headless inventory (`v0.1.0-alpha.2`)

- [x] Providers for pacman, AUR/yay, Flatpak, Cargo, uv, pip, and declarative custom managers.
- [x] `omni --list`, manager filters, sorting, text and JSON output.
- [x] Missing providers are skipped cleanly; custom uninstall commands are represented as configuration.
- [x] End-to-end CLI coverage.

### M3 — Interactive TUI (`v0.1.0-beta.1`)

- [x] Ratatui three-pane layout: managers, packages, details; Vim/Helix navigation and statusline.
- [x] Search, sorting, focused package view, help modal, live configuration reload.
- [x] Configurable panel borders/backgrounds and Gruvbox-oriented theme.
- [x] Package dependency subpanel and dependency detail selection.
- [x] Copy the selected package name or the text displayed in Details to the system clipboard without blocking the event loop.

### M4 — Safe package operations (`v0.1.0-rc.1`)

- [x] Confirm before uninstall; display the exact command that will run.
- [x] Build native commands with argument vectors; quote custom-manager template values.
- [x] Stream subprocess output into Details, keep it visible after completion, and refresh inventory after every completed operation (including partial failures).
- [x] Background update discovery for Pacman, AUR, Flatpak, uv, pip, and global Cargo when `cargo-update` is installed.
- [x] Read-only update discovery for Cargo/npm/Bun project dependencies, with manager-specific commands to update selected packages.
- [x] Opt-in update discovery and argv-based update commands for custom managers.
- [x] Enrich package metadata for pip, Flatpak runtime requirements, and custom manager JSON/info/size commands where available.
- [x] Persist user-visible notices and operation summaries in a private, rotating log; expose `omni --logs` and `omni --log-path`.
- [x] Document actual metadata coverage and known limitations for every built-in, custom, and project-local provider in [`PROVIDER_SUPPORT.md`](PROVIDER_SUPPORT.md). A read-only smoke check on the current Arch host confirmed installed inventories for Pacman, AUR, Flatpak, Cargo, uv, and pip; it does not substitute for mutation tests.
- [x] Manually exercise manager and privilege-helper paths in disposable environments. Uninstall/refresh passed for all six built-in global providers; global updates for Pacman, AUR/yay, Flatpak, Cargo, uv, and pip and Cargo, npm, and Bun project add/remove/update passed. A real `yay -Qua` discovery and AUR PKGBUILD build/update passed in a private Pacman root under a user namespace. On 2026-10-11 the real Omni sudo prompt authenticated with host credentials and completed a harmless custom-manager command (`sudo ... sh -c true`) successfully; no package was changed. See [`PROVIDER_SUPPORT.md`](PROVIDER_SUPPORT.md) for scope and limits.
- [ ] Finish the stable-release gates in [`VERSIONING.md`](VERSIONING.md): perform the final pre-release checks, update version/release notes to `1.0.0`, and create the annotated release tag on the approved commit. CI, command review, dependency audit, TUI acceptance, disposable provider operations, isolated AUR update, and real sudo prompt authentication are documented in [`SECURITY_REVIEW.md`](SECURITY_REVIEW.md), [`PROVIDER_SUPPORT.md`](PROVIDER_SUPPORT.md), and [`VERSIONING.md`](VERSIONING.md).

## Post-1.0 / optional follow-up

- Dependency metadata and dependency-to-installed-package resolution for providers beyond Pacman.
- Orphan detection and safer package cleanup recommendations.
- Accessibility and narrow-terminal review, including configurable panel minimum widths.
- Additional providers contributed through `managers.toml` examples.

## Verification

GitHub Actions runs `cargo test --all-targets` and `cargo build --release` for pushes and pull requests. Run `cargo fmt --all -- --check` locally before submitting. Automated tests validate parsers, command construction, config, and CLI behavior; they do not run real uninstall operations. Manual uninstall verification must use disposable packages or a throwaway environment.
