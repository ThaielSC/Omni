# HANDOFF MAESTRO: Continuación de Desarrollo de Omni (`omni`)

> **Para el próximo Agente / Modelo**: Este documento contiene **TODO** lo que necesitas saber para continuar el proyecto sin perder contexto. Lee este archivo antes de tocar cualquier código.

---

## 1. 📌 ¿Qué es Omni?
**Omni** es un gestor universal de paquetes para Linux en terminal (TUI) escrito en **Rust**.
Centraliza la visualización, auditoría de espacio en disco y desinstalación de aplicaciones provenientes de múltiples gestores:
- Gestores de sistema: **`pacman`**, **`yay` (AUR)**, **`flatpak`**.
- Gestores de herramientas/lenguajes: **`cargo`**, **`uv`**, **`pip`**.
- Gestores dinámicos vía TOML: Soporte para cualquier gestor definido por el usuario (`~/.config/omni/managers.toml`) como `npm`, `gem`, `brew`, `nix`.

---

## 2. 🏛️ Filosofía de UI/UX (Estricta - Inspirada en Helix / Vim / Yazi)
1. **3 Columnas Verticales**:
   - **Columna 1 (Managers)**: Lista con `All` y cada gestor disponible (`Pacman`, `AUR`, `Flatpak`, `Cargo`, `Python`) con sus conteos en vivo.
   - **Columna 2 (Packages)**: Lista de paquetes del gestor enfocado con su tamaño en disco (`4.8 MiB`, `1.2 GiB`).
   - **Columna 3 (Details)**: Inspector con descripción, versión, licencia, dependencias y archivos.
2. **Navegación Espacial**:
   - `h` / `l`: Cambiar entre columnas (Managers ↔ Packages ↔ Details).
   - `j` / `k`: Subir y bajar dentro de la lista enfocada.
   - `1` .. `6`: Salto numérico directo a cada pestaña de gestor.
   - `s`: Alternar ordenación (`Size ▼` descendente / `Name ▲`).
   - `/`: Activar búsqueda interactiva / filtro fuzzy.
   - `d` o `x`: Desinstalar (abre modal de confirmación con comando y espacio liberado).
   - `?`: Modal flotante de ayuda. `Esc` o `q` para cerrar.
3. **Statusline Inferior**:
   - Bloque modal invertido a la izquierda: ` NOR `, ` /SEARCH `, ` DEL? `.
   - Contador central: `[All] 1/1420`.
   - Estadísticas a la derecha: `Size: 14.8 GiB  utf-8  100%`.
4. **Zero Clutter**: No mostrar teclas ni ayudas permanentes en la pantalla principal.

---

## 3. 📂 Mapa de Archivos de Documentación (`docs/`)
Revisa estos archivos para especificaciones exhaustivas:
- [`docs/ROADMAP.md`](docs/ROADMAP.md): Hitos del proyecto (M1 a M4).
- [`docs/UI_UX_SPEC.md`](docs/UI_UX_SPEC.md): Diseños ASCII exactos de las pantallas, modales y paleta Gruvbox Material.
- [`docs/CONFIGURATION_SPEC.md`](docs/CONFIGURATION_SPEC.md): Esquema TOML de `~/.config/omni/config.toml`, `theme.toml` y `keymap.toml`.
- [`docs/MANAGERS_SPEC.md`](docs/MANAGERS_SPEC.md): Formato declarativo para gestores externos en `managers.toml`.
- [`docs/VERSIONING.md`](docs/VERSIONING.md): Esquema SemVer y política de tags de Git (`v0.1.0-alpha.1`, etc.).
- [`docs/GITBUTLER_BRANCHES.md`](docs/GITBUTLER_BRANCHES.md): Estructura de ramas virtuales de GitButler.

---

## 4. 🛠️ Estado Actual del Repositorio y GitButler

El usuario gestiona el repositorio con **GitButler** (`but`).
⚠️ **REGLA CRÍTICA**: **NUNCA ejecutes `git add`, `git commit` o `git checkout`**. Todas las operaciones de control de versiones deben hacerse exclusivamente con el binario **`but`**.

### Ramas Virtuales Apiladas (Stack de trabajo en GitButler):
```text
╭┄ at [feat/uninstall]             <-- (Dependencias + desinstalación segura)
┊●   feat(uninstall): manage packages and dependencies from the TUI
┊│
┊├┄ at [feat/tui]                   <-- (TUI + personalización visual)
┊●   feat(ui): add customizable three-pane Ratatui interface
┊│
┊├┄ pr [feat/providers-import]     <-- (Providers, CLI y clasificación de instalación)
┊●   feat(providers): classify package installation reasons
┊●   feat(cli): expose package inventory as JSON and text
┊●   feat(providers): implement native and custom package providers
┊│
┊├┄ te [feat/e2e-test-suite]
┊├┄ mo [feat/models-config]
┊└┄ do [feat/docs]
├╯
┴ 8289bf3 (common base) first commit
```

- **Último tag publicado**: `v0.1.0-beta.1` (TUI interactiva M3).
- **Estado actual**: providers/CLI, TUI y la gestión de dependencias/desinstalación están en el stack virtual y publicados en PRs encadenadas. Quedan en el workspace únicamente cambios M1 de hardening (`src/utils/format.rs`, `tests/m1_challenger_test.rs`) que no se mezclaron en la PR de gestión.

---

## 5. 🚀 QUÉ ESTÁ HECHO Y QUÉ FALTA (Siguientes Pasos Exactos)

### ✅ Lo que ya está completado (Milestone 1):
1. `src/model.rs`: Estructuras completas de datos (`UnifiedPackage`, `PackageSource`, `SortMode`, etc.).
2. `src/utils/format.rs`: Formateo de bytes a KiB/MiB/GiB.
3. `src/config.rs`: Motor completo de configuración TOML (1,460 líneas probadas).
4. `tests/e2e_cli_test.rs`: Arnés de 96 pruebas E2E que validan la suite.
5. 29/29 pruebas unitarias pasando con `cargo test`.

---

### ✅ Milestones implementados

- **Milestone 1 — Core/config**: modelos, configuración TOML, formato de tamaños y fixtures de tests.
- **Milestone 2 — Providers/CLI**: pacman, AUR, Flatpak, Cargo, uv, pip y managers personalizados; CLI de texto/JSON.
- **Milestone 3 — Primera TUI**: `src/ui/` ofrece tres paneles, navegación Vim, búsqueda, ordenamiento, detalles, recarga, ayuda `?` y statusline. El arranque sin flags abre la TUI; `--list` mantiene el modo CLI.
- **Dependencias interactivas**: `i` abre una lista inferior de dependencias disponibles del paquete; `j/k` selecciona y Details muestra el paquete elegido. El mapeo es mejor cuando el proveedor expone metadatos de dependencias (por ahora, Pacman).
- **Desinstalación**: `d/x` prepara el comando de proveedor, presenta confirmación y transmite stdout/stderr al panel Details; al finalizar correctamente actualiza el inventario. Los comandos usan argv para proveedores nativos y escape POSIX de valores para gestores configurables. Si usa `sudo`, Omni restaura momentáneamente el terminal y llama `sudo -v`; la contraseña la gestiona sudo directamente y Omni no la captura ni guarda. La desinstalación posterior corre en modo no interactivo. Otros helpers configurados deben estar autorizados de antemano y aceptar `-n`.
- **Filtro de dependencias**: `e` alterna `DEPS OFF/ON`; por defecto se ocultan solo dependencias detectadas con fiabilidad. Los paquetes `Unknown` se conservan.
- **Decoración visual**: `border_style` admite `none`, `vertical`, `rounded`, `plain`, `thick` y `double`; el tema Gruvbox separa paneles por fondo con `none` como valor por defecto.

### ⏳ Trabajo pendiente para una beta sólida

- Revisar manualmente la TUI en terminales estrechas y confirmar cada proveedor real de desinstalación en una máquina de prueba; la suite automatizada verifica construcción segura de comandos, no ejecuta desinstalaciones reales.
- Antes de publicar `v0.1.0-rc.1`, cerrar los criterios de aceptación/auditoría de [`docs/VERSIONING.md`](docs/VERSIONING.md), alinear versión en Cargo.toml/Cargo.lock y crear tag anotado. `but` no tiene subcomando de tags; usar GitButler Desktop o el mecanismo de tags documentado autorizado.

---

## 6. 🧪 Comandos de Verificación Rápida
Para comprobar que el código compila y pasa todas las pruebas:
```bash
cargo check
cargo test
cargo test --test e2e_cli_test
```
Para ver el estado de GitButler:
```bash
but status
```
EOF
