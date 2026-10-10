# Estructura de Virtual Branches en GitButler para Omni

Esta estructura organiza las ramas virtuales apiladas (stacked branches) para GitButler:

## 1. `feat/docs-spec` (Base de Especificación)
- `docs/ROADMAP.md`
- `docs/UI_UX_SPEC.md`
- `docs/CONFIGURATION_SPEC.md`
- `docs/MANAGERS_SPEC.md`
- `docs/GITBUTLER_BRANCHES.md`

## 2. `feat/models-and-config` (Milestone 1: Core & Config)
- `Cargo.toml` / `Cargo.lock`
- `.gitignore`
- `src/lib.rs`
- `src/main.rs`
- `src/model.rs`
- `src/config.rs`
- `src/utils/`

## 3. `feat/e2e-test-suite` (Milestone 1 Test Harness)
- `tests/e2e_cli_test.rs`
- `tests/common/`
- `tests/fixtures/`

## 4. `feat/providers` (Milestone 2: Package Managers)
- `src/provider/mod.rs`
- `src/provider/pacman.rs`
- `src/provider/yay.rs`
- `src/provider/flatpak.rs`
- `src/provider/cargo.rs`
- `src/provider/uv.rs`
- `src/provider/pip.rs`
- `src/provider/custom.rs` (Soporte `managers.toml`)

## 5. `feat/tui-interface` (Milestone 3: Ratatui 3-Column UI)
- `src/ui/mod.rs`
- `src/ui/app.rs`
- `src/ui/layout.rs` (3 columnas con proporciones configurables)
- `src/ui/theme.rs` (Gruvbox Material)
- `src/ui/statusline.rs` (Helix / Vim statusline)
- `src/ui/help.rs` (Modal de ayuda flotante con `?`)
- `src/ui/modal.rs` (Confirmación de desinstalación)
