# Omni

Omni es una aplicación de terminal escrita en Rust para consultar y gestionar paquetes instalados mediante varios gestores desde una sola interfaz. Su TUI está pensada para teclado, con navegación inspirada en Vim y Helix, tres paneles y una statusline compacta.

> **Estado:** versión temprana `0.1.0-beta.1`. La interfaz y los proveedores principales ya existen, pero el proyecto sigue en desarrollo. Consulta [limitaciones conocidas](#limitaciones-conocidas) antes de usar operaciones de mantenimiento.

## Contenido

- [Características](#características)
- [Interfaz](#interfaz)
- [Requisitos](#requisitos)
- [Instalación y ejecución](#instalación-y-ejecución)
- [Atajos](#atajos)
- [Comprobación de actualizaciones](#comprobación-de-actualizaciones)
- [Selección y desinstalación](#selección-y-desinstalación)
- [Configuración](#configuración)
- [Gestores personalizados](#gestores-personalizados)
- [Desarrollo y pruebas](#desarrollo-y-pruebas)
- [Documentación](#documentación)
- [Limitaciones conocidas](#limitaciones-conocidas)

## Características

- Inventario unificado de Pacman, AUR mediante `yay`, Flatpak, Cargo, `uv`, `pip` y gestores definidos por el usuario.
- Omite de forma predeterminada los paquetes que el proveedor identifica como dependencias; se pueden incluir bajo demanda.
- Filtrado por nombre o descripción, orden por nombre, tamaño o fecha, y panel de detalles.
- Vista de dependencias del paquete seleccionado cuando el proveedor dispone de esos datos.
- Comprobación de actualizaciones bajo demanda, sin exploración automática durante el inicio.
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
| `i` | Abrir o cerrar la lista de dependencias del paquete actual |
| `d` / `x` | Revisar y confirmar la desinstalación del paquete o selección |
| `r` | Recargar el inventario; en la vista de actualizaciones también repite la consulta |
| `?` | Abrir la ayuda; `/` filtra sus atajos |
| `q` / `Ctrl-C` | Salir |

## Comprobación de actualizaciones

Omni no consulta actualizaciones al iniciar. Pulsa `u` para solicitar la comprobación; se ejecuta en segundo plano y la lista pasa a mostrar los paquetes para los que se encontró una versión más reciente. La statusline muestra el estado de carga y la cantidad de resultados. Pulsa `u` otra vez para volver al inventario normal.

Cobertura actual:

- **Repositorios de Arch:** usa `checkupdates`, que comprueba versiones disponibles con una base de datos temporal y evita actualizar la base normal de sincronización de Pacman. Si falta, instala `pacman-contrib`.
- **AUR:** usa `yay -Qua` si `yay` está instalado.
- **Otros gestores:** la comprobación de actualizaciones todavía no está implementada.

Esta función solo informa de versiones disponibles; Omni no instala actualizaciones.

## Selección y desinstalación

Pulsa `Space` sobre los paquetes que quieras incluir. La statusline indica cuántos están seleccionados. Con `d` o `x`, Omni presenta una confirmación con los objetivos y comandos; si no hay selección, la acción se aplica al paquete bajo el cursor.

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
```

```toml
# keymap.toml
[keymap]
toggle_updates = "u"
toggle_package_selection = " "
uninstall = ["d", "x"]
reload = "r"
help = "?"
```

Los cambios válidos se aplican mientras la aplicación está abierta. Si un archivo queda temporalmente inválido durante la edición, Omni mantiene la última configuración válida y muestra una notificación. La referencia completa de opciones está en [CONFIGURATION_SPEC.md](docs/CONFIGURATION_SPEC.md).

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

## Documentación

- [Roadmap y hitos](docs/ROADMAP.md)
- [Especificación de UI/UX](docs/UI_UX_SPEC.md)
- [Configuración, temas y atajos](docs/CONFIGURATION_SPEC.md)
- [Gestores personalizados](docs/MANAGERS_SPEC.md)
- [Política de versiones y tags](docs/VERSIONING.md)
- [Flujo de GitButler](docs/GITBUTLER_BRANCHES.md)
- [Guía de continuación](docs/HANDOFF.md)

## Limitaciones conocidas

- La comprobación de actualizaciones solo cubre Pacman y AUR actualmente.
- La clasificación entre paquetes explícitos y dependencias depende de la información que entregue cada proveedor.
- Los metadatos de dependencias no están disponibles de forma uniforme entre todos los gestores.
- Una suite automatizada no reemplaza la validación manual de cada gestor y método de autorización en un entorno descartable; esa revisión sigue siendo necesaria antes de una versión estable.
- El proyecto todavía no incluye un archivo de licencia. No redistribuyas el código como si tuviera una licencia específica hasta que se añada una.
