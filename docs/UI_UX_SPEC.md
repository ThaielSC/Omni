# Diseño de Pantallas UI/UX para Omni (`omni`)
**Filosofía Helix / Vim: Minimalismo puro, 3 paneles verticales y cero ruido visual.**

---

## 🧘 Principios de Diseño
1. **Zero Clutter (Sin ruido)**: No se muestran listas de atajos ni ayudas en la pantalla principal. La interfaz está completamente limpia.
2. **Help flotante con `?`**: Una ventana modal centrada que se abre al pulsar `?` y se cierra con `Esc` o `q`.
3. **Statusline clásica de Vim / Helix**: Al pie de la pantalla, con modo (`NORMAL` / `SEARCH`), contador de elementos, indicador de posición y estado de filtro.
4. **Navegación de 3 columnas (Paneles Verticales)**:
   - **Columna 1 (Gestores)**: `All`, `Pacman`, `AUR (Yay)`, `Flatpak`, `Cargo`, `Python (uv/pip)`.
   - **Columna 2 (Paquetes)**: Lista de paquetes del gestor enfocado.
   - **Columna 3 (Detalles)**: Información contextual bajo demanda.
5. **Navegación espacial `h / j / k / l`**:
   - `h` y `l` cambian de columna (como moverse entre divisiones en Helix/Vim).
   - `j` y `k` suben y bajan en la columna enfocada.
6. **Vista enfocada**: Las dependencias identificadas se ocultan al iniciar; `e` alterna `DEPS OFF` / `DEPS ON`. Los paquetes sin clasificación fiable permanecen visibles.

## Decoración y separación de paneles

`[layout].border_style` admite `none` (predeterminado, fondos distintos), `vertical` (solo separadores entre paneles), `rounded`, `plain`, `thick` y `double`. En `[theme.colors]`, `panel_managers_bg`, `panel_packages_bg` y `panel_details_bg` personalizan los fondos individuales.

Con `border_style = "none"`, el panel enfocado se distingue además con una barra vertical `▍` en el borde izquierdo, coloreada por `theme.colors.cursor_bar`. El espacio de esa barra se reserva en los tres paneles para que el contenido no se desplace al cambiar el foco. La marca es deliberadamente estrecha: hace evidente el foco sin añadir marcos ni ruido visual. En los otros estilos, el foco se comunica mediante el borde y el título del panel.

El modal de ayuda mantiene un marco redondeado aunque la vista principal use paneles sin bordes. Es compacto, agrupa los atajos por sección con títulos de color y separadores horizontales, y destaca las teclas en bloques. `/` filtra la guía mientras se escribe; el pie recuerda cómo cerrar el modal.

---

## 📺 Pantalla 1: Vista Principal (Modo NORMAL)

```
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
│              │  eza                   4.2M   │ Line-oriented search tool that recursively searches the│
│              │  starship             11.4M   │ current directory for a regex pattern.                 │
│              │  hyprland             18.9M   │                                                        │
│              │  kitty                24.1M   │ Dependencies:                                          │
│              │  neovim               31.2M   │ - glibc                                                │
│              │  nushell              16.8M   │ - pcre2                                                │
│              │  git                  14.5M   │                                                        │
│              │                               │ Files (top 5):                                         │
│              │                               │ /usr/bin/rg                                            │
│              │                               │ /usr/share/man/man1/rg.1.gz                            │
└──────────────┴───────────────────────────────┴────────────────────────────────────────────────────────┘
 NOR  [All] 1/1420                                                          Size: 14.8 GiB  utf-8  100% 
```

> **Detalles de la Status Bar:**
> - Bloque izquierdo: Modo actual invertido ` NOR ` (color verde o amarillo Gruvbox).
> - Filtro/Contexto activo: `[All]` y posición `1/1420`.
> - Bloque derecho: Tamaño acumulado en disco `Size: 14.8 GiB`, codificación `utf-8` y porcentaje de scroll.

---

## 📺 Pantalla 2: Filtrado por Gestor (Foco en Cargo)

Al pulsar `j` en la primera columna para seleccionar **Cargo** y luego `l` para pasar a la lista:

```
┌──────────────┬───────────────────────────────┬────────────────────────────────────────────────────────┐
│ MANAGERS     │ PACKAGES (Cargo)              │ DETAILS                                                │
│              │                               │                                                        │
│ All    1,420 │ ▍zellij               28.4M   │ zellij 0.45.1                                          │
│ Pacman 1,150 │  yazi                 12.3M   │                                                        │
│ AUR       18 │  rio                  35.0M   │ Source:       cargo install                            │
│ Flatpak    4 │  bat                   6.5M   │ Size:         28.42 MiB                                │
│▍Cargo     12 │  eza                   4.2M   │ Binary:       ~/.cargo/bin/zellij                      │
│ Python    36 │  starship             11.4M   │ Installed:    2026-09-02                               │
│              │  cargo-binstall        3.1M   │                                                        │
│              │  cargo-update          2.8M   │ Description:                                           │
│              │  bottom               10.2M   │ A terminal workspace with batteries included           │
│              │  tokei                 3.9M   │                                                        │
│              │  dust                  4.5M   │ Crates.io:                                             │
│              │  git-delta             7.1M   │ https://crates.io/crates/zellij                        │
└──────────────┴───────────────────────────────┴────────────────────────────────────────────────────────┘
 NOR  [Cargo] 1/12                                                           Size: 129.4 MiB  utf-8   8% 
```

---

## 📺 Pantalla 3: Búsqueda en Vivo (`/`)

Al presionar `/`, la status bar se transforma en el prompt de búsqueda al estilo Helix/Vim. Los paneles se filtran en tiempo real:

```
┌──────────────┬───────────────────────────────┬────────────────────────────────────────────────────────┐
│ MANAGERS     │ PACKAGES (Search: "rip")      │ DETAILS                                                │
│              │                               │                                                        │
│ All        3 │ ▍ripgrep (pacman)      4.8M   │ ripgrep 14.1.0                                         │
│ Pacman     1 │  ripgrep-all (aur)     8.2M   │                                                        │
│ AUR        1 │  python-rip (pip)      1.1M   │ Source:       pacman (extra)                           │
│ Flatpak    0 │                               │ Size:         4.82 MiB                                 │
│ Cargo      0 │                               │ Description:                                           │
│ Python     1 │                               │ Fast line-oriented regex search tool                   │
└──────────────┴───────────────────────────────┴────────────────────────────────────────────────────────┘
 /rip_                                                                         3 matches  [Esc] Cancel 
```

---

## 📺 Pantalla 4: Modal Flotante de Ayuda (`?`)

Al presionar `?` en cualquier momento, se suspende la vista y aparece una ventana modal limpia con bordes redondeados o rectos Gruvbox:

```
┌──────────────┬───────────────────────────────┬────────────────────────────────────────────────────────┐
│ MANAGERS     │ PACKAGES                      │ DETAILS                                                │
│              │ ┌────────────────────── Keyboard Shortcuts ──────────────────────┐                    │
│ All    1,420 │ │                                                                │                    │
│ Pacman 1,150 │ │  Navigation                                                    │                    │
│ AUR       18 │ │    h / l            Switch between Columns (Managers/List/Info)│                    │
│ Flatpak    4 │ │    j / k            Move selection Down / Up                   │                    │
│ Cargo     12 │ │    g / G            Jump to top / bottom of list               │                    │
│ Python    36 │ │                                                                │                    │
│              │ │  Search & Filters                                              │                    │
│              │ │    /                Search packages (regex / fuzzy)            │                    │
│              │ │    s                Toggle sort: Size ▼ / Name ▲ / Date        │                    │
│              │ │    1..6             Jump directly to Manager tab               │                    │
│              │ │                                                                │                    │
│              │ │  Actions                                                       │                    │
│              │ │    d / x            Uninstall package (with confirmation)      │                    │
│              │ │    r                Reload / rescan packages in background     │                    │
│              │ │    ?                Toggle this help screen                    │                    │
│              │ │    q / <Esc>        Quit                                       │                    │
│              │ │                                                                │                    │
│              │ │                    Press [Esc] or [q] to close                 │                    │
│              │ └────────────────────────────────────────────────────────────────┘                    │
└──────────────┴───────────────────────────────┴────────────────────────────────────────────────────────┘
 NOR  [All] 1/1420                                                          Size: 14.8 GiB  utf-8  100% 
```

---

## 📺 Pantalla 5: Modal de Confirmación de Desinstalación (`d` o `x`)

```
┌──────────────┬───────────────────────────────┬────────────────────────────────────────────────────────┐
│ MANAGERS     │ PACKAGES                      │ DETAILS                                                │
│              │        ┌────────────── Confirm Uninstall ──────────────┐                               │
│ All    1,420 │        │                                               │                               │
│ Pacman 1,150 │        │  Uninstall 'helix' (42.1 MiB)?                │                               │
│ AUR       18 │        │                                               │                               │
│ Flatpak    4 │        │  Manager:   pacman                            │                               │
│ Cargo     12 │        │  Action:    sudo pacman -Rns helix            │                               │
│ Python    36 │        │  Freed:     42.1 MiB                          │                               │
│              │        │                                               │                               │
│              │        │         [Enter] Confirm     [Esc] Cancel      │                               │
│              │        └───────────────────────────────────────────────┘                               │
└──────────────┴───────────────────────────────┴────────────────────────────────────────────────────────┘
 DEL? Confirm uninstall: helix                                                     Freed: 42.1 MiB       
```

---

## 🎨 Paleta y Atributos Visuales (Gruvbox Material)

| Elemento | Color ANSI / Hex | Estilo |
|---|---|---|
| **Líneas divisorias** | `#5a524c` | Bordes delgados y limpios |
| **Columna activa (foco)** | Título en `#d8a657` (Amarillo) | Resaltado sutil |
| **Elemento seleccionado** | Fondo `#45403d`, barra `▍` en `#d8a657` | Barra sólida recta |
| **Texto general** | `#d4be98` / `#ddc7a1` | Crema cálido de Helix |
| **Status Bar ` NOR `** | Fondo `#a9b665` (Verde), texto `#282828` | Estilo bloque invertido Helix |
| **Status Bar ` /search `** | Fondo `#e78a4e` (Naranja), texto `#282828` | Modo búsqueda |
| **Status Bar ` DEL? `** | Fondo `#ea6962` (Rojo), texto `#282828` | Alerta de desinstalación |
# Dependencias y operaciones de paquete

- `i` abre una sublista inferior de dependencias del paquete actual. `j/k` navega por esa sublista; el panel Details presenta los datos del paquete de dependencia cuando está presente en el inventario. Si el proveedor no publica dependencias, Omni lo indica sin inventar datos.
- `d/x` sobre un paquete o dependencia abre una confirmación con el nombre, origen y comando exacto. Al aceptar, Details se convierte temporalmente en consola; stdout y stderr aparecen en vivo. `j/k`, `g/G` permiten revisar el log y `Esc` lo cierra después de finalizar.
- Al terminar correctamente, Omni vuelve a consultar el inventario. Antes de ejecutar una operación con `sudo`, Omni restaura temporalmente el terminal para que sudo presente su prompt habitual; la contraseña nunca pasa por Omni. Luego la desinstalación se ejecuta sin interacción y los errores de permisos quedan visibles en la consola.
