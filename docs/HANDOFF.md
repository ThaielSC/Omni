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

### Ramas Virtuales Apiladas (Stack actual en `but status`):
```text
╭┄ at [feat/e2e-test-suite]       <-- (Tope del stack: 96 pruebas E2E)
┊●   ovt test(config): add isolated configuration module unit test
┊●   mln test: add comprehensive 96-case E2E test suite and fixtures
┊│
┊├┄ mo [feat/models-config]        <-- (Milestone 1 implementado)
┊●   zqw feat(core): implement models, byte formatter, and TOML configuration
┊│
┊├┄ do [feat/docs]                 <-- (Base del stack: Especificaciones y docs)
┊●   vqm docs: add comprehensive README with architecture, specs, and usage
┊●   ksq docs(versioning): add SemVer criteria and release tagging policy
┊●   pzk docs: add comprehensive specifications, roadmap, and UI/UX design
├╯
┴ 8289bf3 (common base) first commit
```

- **Tag actual de Git**: `v0.1.0-alpha.1` (sobre Milestone 1).

---

## 5. 🚀 QUÉ ESTÁ HECHO Y QUÉ FALTA (Siguientes Pasos Exactos)

### ✅ Lo que ya está completado (Milestone 1):
1. `src/model.rs`: Estructuras completas de datos (`UnifiedPackage`, `PackageSource`, `SortMode`, etc.).
2. `src/utils/format.rs`: Formateo de bytes a KiB/MiB/GiB.
3. `src/config.rs`: Motor completo de configuración TOML (1,460 líneas probadas).
4. `tests/e2e_cli_test.rs`: Arnés de 96 pruebas E2E que validan la suite.
5. 29/29 pruebas unitarias pasando con `cargo test`.

---

### ⏳ Lo que falta para la Versión Alpha Funcional:

#### Paso 1: Implementar los Proveedores (Milestone 2 - `feat/providers`)
Crear el trait `PackageManagerProvider` en `src/provider/mod.rs` e implementar los adaptadores:
1. `src/provider/pacman.rs`: Ejecuta `pacman -Qi` o lee base de datos local para paquetes de sistema.
2. `src/provider/yay.rs`: Detecta paquetes foráneos de AUR con `pacman -Qm` o `yay -Qm`.
3. `src/provider/flatpak.rs`: Ejecuta `flatpak list --app --columns=application,name,version,size`.
4. `src/provider/cargo.rs`: Lee `~/.cargo/.crates2.json` o `cargo install --list`.
5. `src/provider/uv.rs`: Ejecuta `uv tool list`.
6. `src/provider/pip.rs`: Ejecuta `pip list --format=json`.
7. `src/provider/custom.rs`: Carga y parsea gestores dinámicos desde `~/.config/omni/managers.toml` según [`docs/MANAGERS_SPEC.md`](docs/MANAGERS_SPEC.md).

> **Al terminar**: Commitear y apilar la rama con:
> ```bash
> but commit -b feat/providers -m "feat(providers): implement native and custom package providers" <ids...>
> but move feat/providers --above feat/e2e-test-suite
> git tag -a v0.1.0-alpha.2 -m "v0.1.0-alpha.2: Milestone 2 completed"
> ```

#### Paso 2: Implementar la TUI en Ratatui (Milestone 3 - `feat/tui`)
Usar `ratatui` + `crossterm` + `tokio`:
1. `src/ui/layout.rs`: División de pantalla en 3 columnas verticales con proporciones configurables (`ratio = [1, 2, 3]`).
2. `src/ui/app.rs`: Bucle de eventos, navegación `h/j/k/l`, filtro en tiempo real.
3. `src/ui/statusline.rs`: Barra de estado inferior con badges `NOR`, `/SEARCH`, `DEL?`.
4. `src/ui/help.rs`: Modal flotante activable con `?`.
5. `src/ui/modal.rs`: Confirmación de desinstalación previa a ejecutar comandos destructivos.
6. `src/main.rs`: Conectar la carga asíncrona de proveedores con la interfaz de Ratatui.

> **Al terminar**: Commitear y apilar con:
> ```bash
> but commit -b feat/tui -m "feat(ui): implement 3-column Vim/Helix layout, statusline, and help modal" <ids...>
> but move feat/tui --above feat/providers
> git tag -a v0.1.0-beta.1 -m "v0.1.0-beta.1: Milestone 3 completed - Functional TUI Alpha"
> ```

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
