# Changelog

This file records notable changes to Omni. Versions follow [Semantic Versioning](docs/VERSIONING.md).

## [1.0.0] - Unreleased

Candidate for the first stable release; it will be published with the `v1.0.0` tag.

### Added

- Unified inventory for Pacman, AUR/yay, Flatpak, Cargo tools, uv tools, pip packages, and declaratively configured managers.
- Optional direct dependency management for Cargo, npm, and Bun projects.
- Three-panel Ratatui TUI with keyboard navigation, search, sorting, contextual help, and live configuration reload.
- Customizable themes, keybindings, backgrounds, borders, and panel proportions through TOML.
- Optional dependency classification, multi-selection, and package details.
- Explicit update and uninstall actions with confirmation, process output in Details, and inventory refresh.
- Masked sudo password prompt; the password is sent only through stdin, is never logged, and is zeroized after use.
- Background update checks for global and project packages.
- Clipboard copying for package names and visible Details contents.
- Non-interactive text/JSON inventory CLI and persistent diagnostic logs with private permissions.

### Security and limitations

- Omni runs as a normal user; only operations that require elevated privileges invoke the privilege helper after confirmation.
- Custom manager `uninstall_cmd` templates run user-configured shell commands and must be treated as trusted code.
- Package size, install reason, dependencies, and update coverage vary by manager; known limitations are documented in [`docs/PROVIDER_SUPPORT.md`](docs/PROVIDER_SUPPORT.md).
