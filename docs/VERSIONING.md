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

### Criterios obligatorios para publicar `v1.0.0`

- [ ] Compilar con `cargo build --release` y pasar `cargo test` en CI sobre una versión soportada de Rust.
- [x] Manual de instalación, configuración, keybindings, operaciones y limitaciones actualizado; `LICENSE` MIT presente y metadatos Cargo alineados.
- [ ] Validar la TUI en terminales amplias y estrechas, todos los estilos de paneles, live reload, selección múltiple, ayuda y errores/notificaciones.
- [ ] Verificar en un entorno descartable las operaciones reales de listado, updates y desinstalación para cada proveedor habilitado; confirmar flujo de sudo/doas, cancelación y refresco tras éxito.
- [x] Revisar amenazas de comandos personalizados: consultas y actualizaciones usan argv; la plantilla `uninstall_cmd` puede ejecutar shell, se advierte explícitamente y sus valores interpolados se escapan. Resultados y límites del análisis en [`SECURITY_REVIEW.md`](SECURITY_REVIEW.md).
- [x] Auditar las dependencias bloqueadas contra avisos conocidos; el escaneo RustSec del 2026-10-10 no encontró vulnerabilidades conocidas. Repetirlo antes del release.
- [x] Documentar soporte y límites de cada fuente para tamaño, motivo de instalación, dependencias, resolución de versiones y updates; ningún dato desconocido se debe inventar. Ver [`PROVIDER_SUPPORT.md`](PROVIDER_SUPPORT.md), que también anota los smoke checks de solo lectura hechos en el host.
- [ ] Cerrar las limitaciones que afecten seguridad/corrección y resolver issues bloqueantes.
- [x] Crear y verificar un artefacto instalable en un prefijo temporal: `cargo package --allow-dirty --offline` y `cargo install --path target/package/omni-0.1.0-beta.1 --root <prefijo-temporal> --locked --offline`; desde allí funcionaron `omni --version` y `omni --help`, y el prefijo se eliminó al terminar.
- [ ] Alinear `Cargo.toml`, `Cargo.lock`, README y notas de release en `1.0.0`; crear tag anotado solo sobre el commit aprobado.

---

## 2. Reglas para Crear Tags
1. Todo tag debe ser anotado y creado a través del flujo de GitButler/documentado, no con mutaciones Git directas.
2. Se crea un tag al completar y superar los criterios de aceptación y auditorías de cada Milestone.
3. El tag debe coincidir con el campo `version` en `Cargo.toml`.
