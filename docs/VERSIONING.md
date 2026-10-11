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

- [x] Verificación local del candidato `1.0.0`: `cargo fmt --all -- --check`, `cargo test --all-targets` y `cargo build --release` pasaron el 2026-10-11. El workflow de CI sobre el commit final todavía debe quedar verde antes de publicar.
- [x] Manual de instalación, configuración, keybindings, operaciones y limitaciones actualizado; `LICENSE` MIT presente y metadatos Cargo alineados. README y [`CHANGELOG.md`](../CHANGELOG.md) describen la versión estable.
- [x] Validar la TUI en terminales amplias y estrechas, todos los estilos de paneles, live reload, selección múltiple, ayuda y errores/notificaciones. Revisada en PTY de 120×36 y 60×14; la recarga en vivo, Space/comma, modal de ayuda y notificación/log de TOML malformado funcionaron. Tests de buffer verifican los glifos para los cinco estilos con separadores/bordes.
- [x] Verificar en entornos descartables las operaciones de listado, updates y desinstalación para los proveedores habilitados; confirmar autenticación sudo, cancelación y refresco tras éxito. Uninstall/refresco pasó para los seis proveedores globales; updates globales de Pacman, AUR/yay, Flatpak, Cargo, uv y pip y Cargo/npm/Bun project add/remove/update pasaron en entornos temporales. El 2026-10-11 `yay -Qua` detectó `hello 1.0-1 -> 2.12.1-2`; `yay -S --aur --needed --noconfirm --sudoflags -S -- hello` construyó el PKGBUILD AUR y actualizó la base Pacman privada en un user namespace. Ese mismo día el prompt de Omni autenticó con sudo real y ejecutó con éxito `sudo ... sh -c true`; no se hizo ninguna mutación de paquetes. Los caminos de cancelación y operaciones de paquetes usaron helpers/raíces descartables, como se detalla en [`PROVIDER_SUPPORT.md`](PROVIDER_SUPPORT.md). Esta prueba confirma el prompt y la autenticación del host, no una desinstalación Pacman/AUR real con sudo.
- [x] Revisar amenazas de comandos personalizados: consultas y actualizaciones usan argv; la plantilla `uninstall_cmd` puede ejecutar shell, se advierte explícitamente y sus valores interpolados se escapan. Resultados y límites del análisis en [`SECURITY_REVIEW.md`](SECURITY_REVIEW.md).
- [x] Auditar las dependencias bloqueadas contra avisos conocidos con RustSec actualizado el 2026-10-11: 217 dependencias bloqueadas, 0 vulnerabilidades conocidas y 0 avisos (base RustSec actualizada el 2026-10-11).
- [x] Documentar soporte y límites de cada fuente para tamaño, motivo de instalación, dependencias, resolución de versiones y updates; ningún dato desconocido se debe inventar. Ver [`PROVIDER_SUPPORT.md`](PROVIDER_SUPPORT.md), que también anota los smoke checks de solo lectura hechos en el host.
- [x] Revisar limitaciones conocidas e issues bloqueantes: no hay issues abiertos en GitHub al 2026-10-11 y los límites intencionales de metadatos/gestores están documentados en [`PROVIDER_SUPPORT.md`](PROVIDER_SUPPORT.md). La autenticación real del prompt de sudo también pasó; véase la evidencia y su alcance en el gate manual anterior.
- [x] Crear y verificar un artefacto instalable en un prefijo temporal para `0.1.0-beta.1` (ver historial de la validación anterior).
- [x] Crear y verificar el paquete instalable `omni-1.0.0` en un prefijo temporal el 2026-10-11; `omni --version` respondió `omni 1.0.0` y `omni --help` terminó correctamente.
- [x] Completar la revisión final de limitaciones sobre el candidato. Metadatos y cobertura por proveedor están resumidos en [`PROVIDER_SUPPORT.md`](PROVIDER_SUPPORT.md); los límites de comandos personalizados y privilegios, en [`SECURITY_REVIEW.md`](SECURITY_REVIEW.md).
- [x] Alinear `Cargo.toml`, `Cargo.lock`, README y notas de release en `1.0.0`.
- [ ] Crear y verificar el tag anotado `v1.0.0` sobre el commit aprobado y publicar la release.

---

## 2. Reglas para Crear Tags
1. Todo tag debe ser anotado y creado a través del flujo de GitButler/documentado, no con mutaciones Git directas.
2. Se crea un tag al completar y superar los criterios de aceptación y auditorías de cada Milestone.
3. El tag debe coincidir con el campo `version` en `Cargo.toml`.
