# Omni

Omni es una aplicación de terminal escrita en Rust para consultar y gestionar paquetes instalados mediante varios gestores desde una sola interfaz. Su TUI está pensada para teclado, con navegación inspirada en Vim y Helix, tres paneles y una statusline compacta.

> **Estado:** versión temprana `0.1.0-beta.1`. La interfaz y los proveedores principales ya existen, pero el proyecto sigue en desarrollo. Consulta [limitaciones conocidas](#limitaciones-conocidas) antes de usar operaciones de mantenimiento.

## Contenido

- [Características](#características)
- [Interfaz](#interfaz)
- [Requisitos](#requisitos)
- [Instalación y ejecución](#instalación-y-ejecución)
- [Atajos](#atajos)
- [Comprobación y actualización de paquetes](#comprobación-y-actualización-de-paquetes)
- [Selección y desinstalación](#selección-y-desinstalación)
- [Configuración](#configuración)
- [Gestores personalizados](#gestores-personalizados)
- [Desarrollo y pruebas](#desarrollo-y-pruebas)
- [Documentación](#documentación)
- [Limitaciones conocidas](#limitaciones-conocidas)

## Características

- Inventario unificado de Pacman, AUR mediante `yay`, Flatpak, Cargo, `uv`, `pip` y gestores definidos por el usuario.
- Integración configurable con dependencias locales de proyectos Cargo, npm y Bun.
- Omite de forma predeterminada los paquetes que el proveedor identifica como dependencias; se pueden incluir bajo demanda.
- Filtrado por nombre o descripción, orden por nombre, tamaño o fecha, y panel de detalles.
- Vista de dependencias del paquete seleccionado cuando el proveedor dispone de esos datos.
- Comprobación automática de actualizaciones en segundo plano tras mostrar el inventario inicial, además de consulta manual cuando se necesite.
- Selección múltiple y desinstalación con revisión previa de los paquetes y comandos.
- Salida de los procesos de desinstalación visible en el panel de detalles.
- Personalización de paneles, separadores, colores, ventanas emergentes y atajos mediante TOML, con recarga en vivo.
- Modo de inventario no interactivo con salida de texto o JSON.

## Interfaz

La vista principal prioriza la información solicitada y distribuye el espacio en tres paneles:

```text
┌──────────────┬────────────────────────────┬──────────────────────────────┐
│ MANAGERS     │ PACKAGES                   │ DETAILS                      │
│              │                            │                              │
│ All          │ ripgrep              4.8M  │ ripgrep 14.1.0               │
│ Pacman       │ helix               42.1M  │ Source: Pacman                │
│ AUR          │ zellij              28.4M  │ Size: 4.82 MiB                │
│ Flatpak      │ obs-studio            1.2G │ Description: ...              │
│ Cargo        │                            │                              │
│ Python       │                            │                              │
└──────────────┴────────────────────────────┴──────────────────────────────┘
 NOR  [All] 1/142  DEPS OFF                         SIZE 14.8 GiB
```

Los paneles pueden distinguirse con fondos, separadores o bordes. El indicador de foco ayuda a identificar el panel activo. La ayuda contextual se abre con `?`; no hay una fila permanente de atajos ocupando espacio.

## Requisitos

- Linux y una terminal compatible con Unicode y colores.
- Rust estable y Cargo para compilar desde el código fuente.
- Los gestores que se quieran consultar deben estar instalados y disponibles en `PATH`.
- Para consultar actualizaciones de repositorios Arch se necesita `checkupdates`, incluido en `pacman-contrib`.
- La consulta de actualizaciones AUR usa `yay`.

Omni se inicia como usuario normal. No necesita privilegios de administrador para listar paquetes; una desinstalación del sistema puede solicitar la contraseña de `sudo` después de la confirmación.

## Instalación y ejecución

### Compilar desde el código fuente

```sh
git clone https://github.com/ThaielSC/Omni.git
cd Omni
cargo run
```

El comando sin argumentos abre la TUI. Para compilar una versión optimizada:

```sh
cargo build --release
./target/release/omni
```

### Inventario en modo no interactivo

```sh
cargo run -- --list
cargo run -- --list --json
cargo run -- --list --manager pacman
cargo run -- --list --manager flatpak --sort size
```

Las opciones de orden son `name`, `size` y `date`. `--manager` admite gestores individuales y los grupos implementados, como `aur`/`yay` y `python`. Ejecuta `omni --help` para ver las opciones disponibles.

## Atajos

Los atajos predeterminados se pueden reasignar; la ventana `?` muestra las acciones y teclas activas.

| Tecla | Acción |
| --- | --- |
| `h` / `l`, flechas | Cambiar el foco entre paneles |
| `j` / `k`, flechas | Mover la selección o recorrer los detalles |
| `Tab` / `Shift-Tab` | Cambiar al panel siguiente o anterior |
| `g` / `G` | Ir al primer o último elemento |
| `1`–`6` | Elegir All, Pacman, AUR, Flatpak, Cargo o Python |
| `/` | Buscar por nombre o descripción |
| `Esc` | Limpiar el filtro de búsqueda |
| `s` | Alternar el orden por tamaño, nombre y fecha |
| `e` | Mostrar u ocultar paquetes clasificados como dependencias |
| `u` | Consultar actualizaciones y mostrar los paquetes actualizables; repetir para volver al inventario |
| `Space` | Seleccionar o quitar de la selección el paquete actual y avanzar |
| `,` | Limpiar toda la selección de paquetes |
| `c` | Copiar el nombre del paquete bajo el cursor al portapapeles |
| `U` | Actualizar los paquetes seleccionados o, si no hay selección, el paquete bajo el cursor |
| `a` | Agregar una dependencia cuando está activa una categoría de proyecto |
| `i` | Abrir o cerrar la lista de dependencias del paquete actual |
| `d` / `x` | Revisar y confirmar la desinstalación del paquete o selección |
| `r` | Recargar el inventario; en la vista de actualizaciones también repite la consulta |
| `?` | Abrir la ayuda; `/` filtra sus atajos |
| `q` / `Ctrl-C` | Salir |

## Comprobación y actualización de paquetes

Después de mostrar el inventario inicial, Omni busca actualizaciones en segundo plano. La lista completa de paquetes aparece primero; al terminar la consulta, los paquetes actualizables reciben un indicador `↑` amarillo y la versión disponible aparece en Details. La consulta no bloquea la navegación. Pulsa `u` para filtrar la lista a los paquetes actualizables y otra vez para volver al inventario completo. `r` vuelve a consultar tras recargar los paquetes.

La detección disponible depende del gestor y de sus herramientas:

- **Pacman:** `checkupdates` (paquete `pacman-contrib`), que usa una base temporal para no alterar la base local sincronizada.
- **AUR:** `yay -Qua`.
- **Flatpak:** actualizaciones de aplicaciones disponibles desde los remotos configurados.
- **uv y pip:** versiones nuevas para las herramientas instaladas por cada gestor.
- **Cargo global:** requiere el subcomando opcional `cargo install-update` del paquete `cargo-update`.
- **Proyectos locales:** Cargo consulta con `cargo update --dry-run`; npm y Bun usan sus consultas `outdated`. Omni filtra a dependencias directas del proyecto. Los nombres de ejecutables se pueden configurar en `[manager.projects]`.
- **Managers personalizados:** la comprobación es optativa y requiere `update_check_cmd` y `update_format` en `managers.toml`.

Pulsa `U` para actualizar los paquetes seleccionados o, si no hay selección, el paquete bajo el cursor. Solo se eligen como objetivos paquetes con una actualización detectada. Omni muestra una confirmación con los objetivos y comandos; la salida de la operación se transmite al panel Details y permanece ahí al finalizar. Las actualizaciones se pueden solicitar en lote.

La actualización está disponible para los proveedores nativos y de proyecto que tienen una operación definida; los managers personalizados requieren `update_cmd`. En Arch, la confirmación de Pacman avisa que `pacman -Syu` puede actualizar también otros paquetes desactualizados: Arch no admite actualizaciones parciales. AUR se actualiza con `yay` limitado a los objetivos detectados, y sus scripts de compilación se ejecutan como usuario. Las actualizaciones de proyecto pueden cambiar manifiestos y lockfiles según el gestor; revisa la confirmación antes de aceptar.

## Selección y desinstalación

Pulsa `Space` sobre los paquetes que quieras incluir. La statusline indica cuántos están seleccionados. Pulsa `,` para desmarcarlos todos. Con `d` o `x`, Omni presenta una confirmación con los objetivos y comandos de desinstalación; con `U`, presenta la confirmación de actualización. Si no hay selección, ambas acciones usan el paquete bajo el cursor.

Omni no requiere iniciar toda la aplicación con `sudo`. Si una operación del sistema necesita privilegios, primero se confirma la acción y luego se solicita la contraseña en un prompt enmascarado en la parte inferior. La contraseña se envía por la entrada estándar del proceso y se elimina del búfer de memoria tras el uso. `Esc` cancela el prompt.

La salida del proceso permanece visible en el panel de detalles al terminar. Al salir de ese panel o pulsar `r`, Omni cierra esa salida y actualiza el inventario. Mientras el proceso siga ejecutándose, su salida no se descarta.

Los comandos de gestores personalizados se consideran confiables y pueden ejecutar acciones arbitrarias. Revisa `managers.toml` antes de usarlos.

## Configuración

Omni busca su configuración en `~/.config/omni/`:

```text
~/.config/omni/
├── config.toml    # Layout y comportamiento
├── theme.toml     # Colores y estilos
├── keymap.toml    # Atajos
└── managers.toml  # Gestores personalizados
```

Ejemplo reducido:

```toml
# config.toml
[layout]
ratio = [1, 2, 3]
show_managers = true
show_details = true
border_style = "none"        # none, vertical, rounded, plain, thick, double
popup_border_style = "rounded"

[behavior]
confirm_uninstall = true
sudo_command = "sudo"

[manager.projects]
enabled = true
cargo = true
npm = true
bun = true
```

```toml
# keymap.toml
[keymap]
toggle_updates = "u"
toggle_package_selection = " "
copy_package_name = "c"
uninstall = ["d", "x"]
reload = "r"
help = "?"
```

Los cambios válidos se aplican mientras la aplicación está abierta. Si un archivo queda temporalmente inválido durante la edición, Omni mantiene la última configuración válida y muestra una notificación. La referencia completa de opciones está en [CONFIGURATION_SPEC.md](docs/CONFIGURATION_SPEC.md).

El atajo `c` copia el nombre del paquete bajo el cursor. En Linux hace falta un helper de portapapeles: Omni prueba `wl-copy` (paquete `wl-clipboard`), `xclip` y `xsel`, en ese orden. El atajo puede reasignarse con `copy_package_name`.

### Gestores de proyectos

Al abrir Omni dentro de un proyecto compatible, las dependencias directas pueden aparecer bajo una fila identificada como `<carpeta>[Cargo]`, `<carpeta>[npm]` o `<carpeta>[bun]`. Cargo se reconoce por `Cargo.toml`; npm y Bun por `package.json` y los archivos del gestor. La integración está habilitada para los tres por defecto y se configura en `[manager.projects]`:

```toml
[manager.projects]
enabled = true
detect_ancestors = true
cargo = true
npm = false
bun = true
cargo_binary = "cargo"
npm_binary = "npm"
bun_binary = "bun"
```

En la categoría del proyecto, `a` abre un prompt para agregar una dependencia usando `cargo add <paquete>`, `npm install <paquete>` o `bun add <paquete>` desde la raíz del proyecto. Sobre una dependencia seleccionada, `d` o `x` la quita con `cargo remove`, `npm uninstall` o `bun remove`. Omni ejecuta estas acciones solo tras una solicitud explícita; pasa argumentos separados sin shell y muestra la salida en Details. El gestor subyacente actualiza el manifiesto y su lockfile según sus reglas. Cada ecosistema puede desactivarse independientemente.

`detect_ancestors = false` limita la detección al directorio actual. Los nombres de ejecutables se pueden cambiar con `cargo_binary`, `npm_binary` y `bun_binary`. El atajo contextual `a` se puede reasignar con `add_project_package` en `keymap.toml`.

Se puede indicar un archivo de configuración alternativo con `--config RUTA`. Omni también busca `managers.toml` junto a ese archivo.

## Gestores personalizados

Se pueden definir proveedores adicionales con `[[manager]]` en `managers.toml`. El formato admite salida JSON y texto parseado con expresiones regulares, además de comandos para listar, consultar detalles, medir tamaño y desinstalar.

Ejemplo ilustrativo para paquetes globales de npm:

```toml
[[manager]]
id = "npm"
name = "NPM Global"
badge = "[NPM]"
binary = "npm"
list_cmd = "npm list -g --depth=0 --json"
format = "json"
json_map_path = "/dependencies"
json_version = "/version"
uninstall_cmd = "npm uninstall -g {name}"
need_sudo = false
```

Los comandos configurados se ejecutan en el sistema. Define solo gestores y comandos que entiendas y sean de confianza. Consulta [MANAGERS_SPEC.md](docs/MANAGERS_SPEC.md) para el esquema y sus reglas.

## Desarrollo y pruebas

```sh
cargo check
cargo test
```

El proyecto usa la edición 2024 de Rust. La TUI se construye con Ratatui y Crossterm; los proveedores están separados en adaptadores dentro de `src/provider/`. Las pruebas emplean datos de ejemplo y no ejecutan desinstalaciones reales.

## Logs

Omni conserva los avisos breves de la status bar y el resultado de operaciones en un log persistente. No guarda la salida completa de comandos ni datos de autenticación.

- `omni --logs`: muestra las últimas 500 entradas.
- `omni --log-path`: imprime la ruta del archivo.
- Ubicación por defecto: `${XDG_STATE_HOME}/omni/omni.log`, o `~/.local/state/omni/omni.log` si `XDG_STATE_HOME` no está configurado.

El archivo rota al superar 1 MiB, conserva una copia `.log.1` y se crea con permisos privados en sistemas Unix.

## Documentación

- [Roadmap y hitos](docs/ROADMAP.md)
- [Especificación de UI/UX](docs/UI_UX_SPEC.md)
- [Configuración, temas y atajos](docs/CONFIGURATION_SPEC.md)
- [Gestores personalizados](docs/MANAGERS_SPEC.md)
- [Política de versiones y tags](docs/VERSIONING.md)
- [Flujo de GitButler](docs/GITBUTLER_BRANCHES.md)
- [Guía de continuación](docs/HANDOFF.md)

## Limitaciones conocidas

- La consulta de actualizaciones cubre Pacman, AUR, Flatpak, uv, pip, Cargo global (si está instalado `cargo-update`), managers personalizados configurados y proyectos Cargo/npm/Bun. La actualización de cada gestor sigue las reglas de sus lockfiles y rangos.
- La clasificación entre paquetes explícitos y dependencias depende de la información que entregue cada proveedor.
- Los metadatos de dependencias no están disponibles de forma uniforme entre todos los gestores.
- La consulta de updates de herramientas Cargo global requiere el subcomando opcional `cargo install-update`; sin él se informa como no compatible. Los managers personalizados requieren `update_check_cmd` y `update_cmd` configurados para consultar y ejecutar updates.
- Las actualizaciones de proyecto cambian manifiestos/lockfiles según la política del gestor; revisa el comando mostrado en la confirmación antes de aceptar.
- Una suite automatizada no reemplaza la validación manual de cada gestor y método de autorización en un entorno descartable; esa revisión sigue siendo necesaria antes de una versión estable.
- El proyecto usa licencia MIT; consulta [LICENSE](LICENSE).
