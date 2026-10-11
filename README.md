# Omni (`omni`)

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](#)
[![Version](https://img.shields.io/badge/version-v0.1.0--alpha.2-orange)](#)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Theme: Gruvbox Material](https://img.shields.io/badge/theme-Gruvbox%20Material-yellow)](#)

> **Omni** es un gestor de paquetes universal en terminal (TUI) escrito en **Rust**. Centraliza, audita y gestiona el software instalado en tu sistema a través de múltiples gestores de paquetes (`pacman`, `yay`/AUR, `flatpak`, `cargo`, `uv`, `pip`) o cualquier gestor personalizado definido por el usuario mediante un archivo declarativo TOML.

Inspirado en la filosofía y ergonomía de **Helix, Vim y Yazi**: diseño minimalista sin ruido visual, navegación espacial en 3 columnas (`h/j/k/l`), statusline clásica y ayuda flotante contextual (`?`).

---

## ✨ Características Principales

- 📦 **Multi-Gestor Unificado**: Consulta en paralelo paquetes de sistema (`pacman`, AUR vía `yay`, `flatpak`) y gestores de herramientas de lenguajes (`cargo`, `uv`, `pip`).
- 🧩 **Extensible vía `managers.toml`**: Soporte para agregar cualquier gestor de paquetes externo (`npm`, `brew`, `gem`, `nix`) de forma declarativa sin tocar código Rust.
- 🧘 **Filosofía Helix / Vim**:
  - Navegación espacial limpia mediante `h / j / k / l`.
  - Cero ruido visual en pantalla (sin indicadores fijos molestos).
  - Statusline inferior con indicador modal, contexto, posición y estado de dependencias.
  - Overlay de ayuda flotante activable con `?` y descartable con `Esc` o `q`.
- 📊 **Auditoría de Espacio**: Ordenamiento instantáneo por tamaño (`Size ▼`) con alertas visuales por rangos de peso para identificar qué aplicaciones consumen más disco.
- 🎨 **Tema Gruvbox Material**: Paleta cálida, limpia y consistente con entornos de terminal modernos.
- 🧹 **Vista enfocada**: Oculta por defecto las dependencias que el gestor identifica como tales; `e` permite incluirlas. Los elementos sin clasificación siguen visibles.
- 🛡️ **Acciones seguras**: La desinstalación aún no está habilitada en esta alpha; `d` / `x` informa que no se realizó ningún cambio.
- ⚡ **Carga en segundo plano**: Los inventarios se leen fuera del bucle de eventos para mantener la navegación fluida.

---

## 🖥️ Arquitectura de la Interfaz (3 Columnas)

```text
┌──────────────┬───────────────────────────────┬────────────────────────────────────────────────────────┐
│ MANAGERS     │ PACKAGES                      │ DETAILS                                                │
│              │                               │                                                        │
│▍All    1,420 │ ▍ripgrep               4.8M   │ ripgrep 14.1.0                                         │
│ Pacman 1,150 │  helix                42.1M   │                                                        │
│ AUR       18 │  zellij               28.4M   │ Source:       pacman (extra)                           │
│ Flatpak    4 │  yazi                 12.3M   │ Size:         4.82 MiB                                 │
│ Cargo     12 │  rio                  35.0M   │ License:      MIT OR Unlicense                         │
│ Python    36 │  python-numpy        118.2M   │ Installed:    2026-08-14                               │
│              │  obs-studio            1.2G   │ URL:          https://github.com/BurntSushi/ripgrep    │
│              │  uv                   22.0M   │                                                        │
│              │  bat                   6.5M   │ Description:                                           │
│              │  eza                   4.2M   │ Line-oriented regex search tool                        │
│              │                               │                                                        │
│              │                               │ Dependencies: glibc, pcre2                             │
└──────────────┴───────────────────────────────┴────────────────────────────────────────────────────────┘
 NOR  [All] 1/1420 DEPS OFF                                                Size: 14.8 GiB  utf-8
```

---

## ⌨️ Atajos de Teclado (Keybindings)

| Tecla | Acción |
|:---|:---|
| **`h` / `l`** | Cambiar el foco entre las 3 columnas (*Managers* ↔ *Packages* ↔ *Details*) |
| **`j` / `k`** o **`↓` / `↑`** | Moverse hacia abajo / arriba en la columna activa |
| **`g` / `G`** | Ir al inicio / final de la lista |
| **`1` .. `6`** | Saltar directamente a un gestor de paquetes (`1`: All, `2`: Pacman, etc.) |
| **`/`** | Activar la búsqueda interactiva (fuzzy search) |
| **`s`** | Alternar ordenación: Tamaño descendente (`Size ▼`) ↔ Nombre (`Name ▲`) |
| **`e`** | Alternar entre ocultar (`DEPS OFF`) e incluir (`DEPS ON`) dependencias detectadas |
| **`d`** o **`x`** | Informa que la desinstalación todavía no está disponible en esta alpha |
| **`r`** | Recargar / reescanear paquetes en segundo plano |
| **`?`** | Mostrar / ocultar ventana flotante de ayuda |
| **`q`** o **`Ctrl-C`** | Salir de la aplicación |

---

## ⚙️ Personalización (`~/.config/omni/`)

Omni es 100% personalizable a través de archivos TOML en `~/.config/omni/`:

- **Layout y Proporciones (`config.toml`)**:
  ```toml
  [layout]
  ratio = [ 1, 2, 3 ]          # Proporción de ancho de las 3 columnas
  border_style = "none"        # also: "vertical", "rounded", "plain", "thick", "double"
  show_managers = true
  show_details = true
  ```
- **Temas y Colores (`theme.toml`)**: Permite ajustar colores de selección, badges, statusline y fondos por panel (`panel_managers_bg`, `panel_packages_bg`, `panel_details_bg`).
- **Atajos (`keymap.toml`)**: Reasignación completa de teclas de navegación y acciones.
- **Gestores Personalizados (`managers.toml`)**: Define nuevos gestores con `list_cmd`, `format` (regex o json) y `uninstall_cmd`.

---

## 📂 Documentación del Proyecto

Toda la especificación técnica y de diseño está organizada en la carpeta [`docs/`](docs/):

- 🗺️ **[ROADMAP.md](docs/ROADMAP.md)**: Hitos, fases y entregables del desarrollo.
- 🎨 **[UI_UX_SPEC.md](docs/UI_UX_SPEC.md)**: Especificación completa de las pantallas, layout y paleta visual Gruvbox Material.
- ⚙️ **[CONFIGURATION_SPEC.md](docs/CONFIGURATION_SPEC.md)**: Manual del sistema de personalización TOML.
- 🧩 **[MANAGERS_SPEC.md](docs/MANAGERS_SPEC.md)**: Guía y ejemplos para definir gestores externos en `managers.toml`.
- 🏷️ **[VERSIONING.md](docs/VERSIONING.md)**: Política SemVer 2.0.0 y criterios de etiquetado de versiones (`git tag`).
- 🌿 **[GITBUTLER_BRANCHES.md](docs/GITBUTLER_BRANCHES.md)**: Estructura de ramas apiladas (stacked virtual branches) en GitButler.

---

## 🚀 Compilación y Pruebas

### Requisitos:
- **Rust** (edición 2021+, toolchain estable)
- `cargo`

```bash
# Clonar el repositorio
git clone https://github.com/ThaielSC/omni.git
cd omni

# Ejecutar la suite de pruebas E2E (96 casos de prueba)
cargo test --test e2e_cli_test

# Ejecutar pruebas unitarias internas
cargo test

# Compilar en modo Release
cargo build --release
```

El binario resultante se encontrará en `./target/release/omni`.

---

## 📄 Licencia

Este proyecto está bajo la Licencia **MIT**. Consulta el archivo `LICENSE` para más detalles.
