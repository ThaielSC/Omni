# Política de Versionado y Etiquetado (Versioning & Tagging) para Omni

Omni sigue el estándar **SemVer 2.0.0** (`MAJOR.MINOR.PATCH`) adaptado a las fases del proyecto.

---

## 1. Esquema de Versiones y Criterios

### Fase Pre-Release (Alpha & Beta)
- **`v0.1.0-alpha.1`**: **Milestone 1** (Core Models, Formatter, Config TOML, E2E Test Suite).
- **`v0.1.0-alpha.2`**: **Milestone 2** (Package Providers: Pacman, Yay, Flatpak, Cargo, uv/pip, managers.toml).
- **`v0.1.0-beta.1`**: **Milestone 3** (TUI 3 Columnas Ratatui funcional + Navegación Helix/Vim).
- **`v0.1.0-rc.1`**: **Milestone 4** (Modales de desinstalación, headless CLI `--list --json` y auditoría de seguridad).

### Versión Estable (`v1.0.0` en adelante)
Una vez alcanzada la versión 1.0.0:
- **`PATCH` (ej. `v1.0.1`)**: Correcciones de bugs, optimización de parsers o ajustes menores de UI sin romper compatibilidad.
- **`MINOR` (ej. `v1.1.0`)**: Nuevas características compatibles hacia atrás (ej. soporte nativo para un nuevo gestor, nueva vista en TUI, nuevas opciones en `config.toml`).
- **`MAJOR` (ej. `v2.0.0`)**: Cambios incompatibles (breaking changes) en el esquema TOML, en la interfaz CLI o en los traits públicos.

---

## 2. Reglas para Crear Tags
1. Todo tag debe ser anotado (`git tag -a vX.Y.Z -m "mensaje"`).
2. Se crea un tag al completar y superar los criterios de aceptación y auditorías de cada Milestone.
3. El tag debe coincidir con el campo `version` en `Cargo.toml`.
