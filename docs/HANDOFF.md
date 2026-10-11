# HANDOFF MAESTRO: Continuación de Desarrollo de Omni (`omni`)

> **Para el próximo Agente / Modelo**: Este documento contiene **TODO** lo que necesitas saber para continuar el proyecto sin perder contexto. Lee este archivo antes de tocar cualquier código.

---

## 1. 📌 ¿Qué es Omni?
**Omni** es un gestor universal de paquetes para Linux en terminal (TUI) escrito en **Rust**.
Centraliza la visualización, auditoría de espacio en disco y desinstalación de aplicaciones provenientes de múltiples gestores:
- Gestores de sistema: **`pacman`**, **`yay` (AUR)**, **`flatpak`**.
- Gestores de herramientas/lenguajes: **`cargo`**, **`uv`**, **`pip`**.
- Gestores dinámicos vía TOML: Soporte para cualquier gestor definido por el usuario (`~/.config/omni/managers.toml`) como `npm`, `gem`, `brew`, `nix`.
- Gestores locales de proyecto: Cargo, npm y Bun configurables en `[manager.projects]`; las filas se muestran como `<carpeta>[Cargo|npm|bun]` y permiten agregar dependencias con `cargo add`, `npm install` o `bun add`.

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
╭┄ at [feat/updates]               <-- (Updates + logging persistente)
┊●   feat(logging): persist notices and expose recent logs
┊●   feat(updates): add native, project, and custom manager update flows
┊│
┊├┄ at [feat/project-managers]      <-- (Dependencias de proyectos)
┊●   feat(projects): manage Cargo/npm/Bun dependencies and show updates
┊│
┊├┄ at [feat/uninstall]             <-- (Dependencias + desinstalación segura)
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

- **Último tag publicado**: `v0.1.0-beta.1` (TUI interactiva M3). No se creó el tag `v0.1.0-rc.1`: siguen pendientes las revisiones manuales y de seguridad de [`docs/VERSIONING.md`](VERSIONING.md).
- **Estado actual**: providers/CLI, TUI, desinstalación, gestores de proyectos, updates y logging están en el stack virtual y publicados como PRs encadenadas (`#1` a `#9`). La PR #9 debe pasar CI y continúa en borrador hasta completar su revisión.
- **Cambios M1 sin commitear**: `src/utils/format.rs`, `tests/common/mod.rs`, `tests/config_test.rs`, `tests/e2e_cli_test.rs`, `tests/m1_challenger_test.rs` y `tests/m1_stress_test.rs`; no agregarlos a una PR de feature sin revisar su procedencia.

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
- **Carga y avisos**: la exploración se indica con un spinner junto al modo en la statusline; los errores de gestores y recarga de configuración se notifican temporalmente en la misma barra. Los paneles no se reemplazan con modales genéricos de carga.
- **Actualizaciones**: después de mostrar el inventario, Omni busca actualizaciones en segundo plano y marca los hallazgos con `↑` amarillo. `u` filtra por paquetes actualizables. La detección cubre Pacman (`checkupdates`), AUR (`yay -Qua`), Flatpak, uv, pip, Cargo global (con `cargo-update`), dependencias de proyecto Cargo/npm/Bun y managers personalizados configurados. `U` actualiza paquetes seleccionados (o el paquete bajo el cursor) y muestra confirmación/comandos y salida en Details. En Arch, el comando `pacman -Syu` puede actualizar también otros paquetes pendientes; la confirmación lo advierte porque las actualizaciones parciales no están soportadas. Los managers personalizados necesitan configurar su consulta y comando de actualización; las consultas de proyectos usan los ejecutables de `[manager.projects]`.
- **Selección múltiple**: `Space` marca el paquete bajo el cursor y avanza a la fila siguiente; `,` limpia la selección. `d/x` desinstala la selección marcada (o el elemento bajo cursor si no hay selección), con una confirmación que enumera los paquetes/comandos.
- **Dependencias interactivas**: `i` abre una lista inferior de dependencias disponibles del paquete; `j/k` selecciona y Details muestra el paquete elegido. El mapeo es mejor cuando el proveedor expone metadatos de dependencias (por ahora, Pacman).
- **Desinstalación**: `d/x` prepara el comando de proveedor, presenta confirmación y transmite stdout/stderr al panel Details; al finalizar correctamente actualiza el inventario. Los comandos usan argv para proveedores nativos y escape POSIX de valores para gestores configurables. La app y el inventario corren como usuario normal. Si una operación usa `sudo`, después de confirmar Omni abre un prompt enmascarado en la fila inferior; envía la contraseña a `sudo -S` solo por stdin y la borra del buffer en memoria con `zeroize`. No se incluye en argumentos, entorno, comando mostrado ni logs. Otros helpers como `doas` no reciben este tratamiento automáticamente y deben poder autorizarse sin prompt interactivo.
- **Consola de operaciones**: stdout/stderr queda visible en Details aunque el proceso termine. Al finalizar, Omni vuelve a consultar el inventario en segundo plano incluso si el comando reportó error (puede haber cambios parciales); la consola permanece visible hasta salir de Details. `r` vuelve a consultar manualmente y no oculta una consola con una operación activa.
- **Filtro de dependencias**: `e` alterna `DEPS OFF/ON`; por defecto se ocultan solo dependencias detectadas con fiabilidad. Los paquetes `Unknown` se conservan.
- **Gestores de proyectos**: `[manager.projects]` en `config.toml` controla la integración local. Cargo, npm y Bun están habilitados por defecto y pueden desactivarse globalmente (`enabled = false`) o uno a uno (`cargo`, `npm`, `bun`). La categoría del proyecto usa `<folder>[Cargo]`, `<folder>[npm]` o `<folder>[bun]`. Agregar una dependencia requiere acción explícita en esa categoría y usa `cargo add <name>`, `npm install <name>` o `bun add <name>` desde la raíz del proyecto; los argumentos se pasan sin shell y la salida se presenta en Details. Omni no instala dependencias automáticamente al iniciar. Estos gestores operan como usuario normal y mantienen el manifiesto/lockfile según sus propias reglas.
- **Decoración visual**: `border_style` admite `none`, `vertical`, `rounded`, `plain`, `thick` y `double`; el tema Gruvbox separa paneles por fondo con `none` como valor por defecto.
- **Logs persistentes**: avisos temporales de statusline y resultados de operaciones se guardan en `${XDG_STATE_HOME}/omni/omni.log` o `~/.local/state/omni/omni.log`. `omni --logs` muestra las últimas 500 líneas y `omni --log-path` imprime la ruta. El log rota al superar 1 MiB; no registra la salida íntegra de procesos ni la contraseña sudo.
- **Managers personalizados con privilegios**: `need_sudo = true` también se aplica al `update_cmd`. Si el helper configurado es `sudo`, Omni solicita la contraseña en el prompt inferior y la pasa solo por stdin; helpers como `doas` se ejecutan en modo no interactivo y requieren autorización externa configurada.

### ⏳ Gates pendientes antes de publicar estable

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
