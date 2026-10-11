# Sistema de Configuración y Personalización para Omni (`omni`)

Inspirado en la ergonomía y modularidad de **Yazi, Helix y Zellij**:
- Archivo de configuración: `~/.config/omni/config.toml` (o `theme.toml` + `keymap.toml`).
- Formato **TOML** limpio y legible.
- Soporte para **Live Reload** o recarga instantánea.

---

## 1. Personalización de Paneles y Layout (`[layout]`)

Permite a los usuarios ajustar las proporciones de las 3 columnas y su visibilidad según el ancho del monitor o gusto personal:

```toml
# ~/.config/omni/config.toml

[layout]
# Proporción de ancho de las 3 columnas: [Managers, Packages, Details]
# Por defecto: 1 parte Managers, 2 partes Packages, 3 partes Details
ratio = [ 1, 2, 3 ]

# Mostrar u ocultar paneles específicos por defecto
show_managers = true
show_details  = true

# Decoración de paneles: marco completo o divisores discretos.
# Opciones: "rounded", "plain", "thick", "double", "vertical", "none"
# El valor predeterminado usa fondos distintos y no dibuja líneas.
border_style = "none"

# Relleno interno y espacios de las celdas
padding = 1
```

Los cambios en `config.toml`, `theme.toml`, `keymap.toml` y `managers.toml` se detectan mientras Omni está abierto. El nuevo estado se aplica sin reiniciar; al cambiar `managers.toml`, el inventario se vuelve a consultar en segundo plano. Si un archivo todavía está incompleto o contiene TOML inválido, Omni conserva la configuración anterior y muestra un aviso temporal en la statusline. Los archivos indicados con `--config` también se observan, junto con el `managers.toml` del mismo directorio.

---

## 2. Personalización de Temas y Colores (`[theme]`)

Permite definir paletas personalizadas (soporta Gruvbox Material, Catppuccin, Tokyo Night, Nord, etc.) y personalizar cada componente visual de la TUI:

```toml
# ~/.config/omni/theme.toml (o dentro de config.toml)

[theme]
# Tema predeterminado ("gruvbox-material", "catppuccin-mocha", "tokyonight", "custom")
name = "gruvbox-material"

[theme.colors]
# Colores base
bg             = "#282828"
fg             = "#d4be98"
border         = "#5a524c"
border_focused = "#d8a657"   # Borde amarillo cálido para la columna con el foco actual

# Fondos individuales para distinguir paneles cuando border_style = "none"
panel_managers_bg = "#2e2b2a"
panel_packages_bg = "#282828"
panel_details_bg  = "#32302f"

# Franja de selección activa
selection_bg   = "#45403d"
selection_fg   = "#d4be98"
cursor_bar     = "#d8a657"   # Color de la barra indicadora vertical "▍"
# En border_style = "none", esta barra identifica el panel enfocado.

# Statusline estilo Helix
status_bar_bg  = "#32302f"
status_bar_fg  = "#d4be98"
mode_normal_bg = "#a9b665"   # Verde para bloque 'NOR'
mode_normal_fg = "#282828"
mode_search_bg = "#e78a4e"   # Naranja para bloque '/SEARCH'
mode_search_fg = "#282828"
mode_alert_bg  = "#ea6962"   # Rojo para bloque 'DEL?'
mode_alert_fg  = "#282828"

# Badges de cada Gestor de Paquetes
badge_pacman  = { fg = "#7daea3", bold = true }  # Azul suave
badge_aur     = { fg = "#e78a4e", bold = true }  # Naranja
badge_flatpak = { fg = "#d3869b", bold = true }  # Magenta/Violeta
badge_cargo   = { fg = "#ea6962", bold = true }  # Rojo óxido
badge_python  = { fg = "#d8a657", bold = true }  # Amarillo cálido

# Rangos de tamaño de paquetes (Alertas visuales de espacio)
size_heavy    = { fg = "#ea6962" }  # > 500 MiB (Rojo)
size_medium   = { fg = "#e78a4e" }  # 50 - 500 MiB (Naranja)
size_light    = { fg = "#a9b665" }  # < 50 MiB (Verde)
```

---

## 3. Personalización Completa de Atajos de Teclado (`[keymap]`)

Permite reasignar cualquier acción a teclas individuales o compuestas (chords):

```toml
# ~/.config/omni/keymap.toml (o dentro de config.toml)

[keymap]

# --- Navegación entre Columnas ---
focus_left   = [ "h", "<Left>" ]
focus_right  = [ "l", "<Right>" ]
focus_next   = [ "<Tab>" ]
focus_prev   = [ "<BackTab>" ]

# --- Navegación dentro de Listas ---
down         = [ "j", "<Down>" ]
up           = [ "k", "<Up>" ]
page_down    = [ "<C-d>", "<PageDown>" ]
page_up      = [ "<C-u>", "<PageUp>" ]
top          = [ "g", "<Home>" ]
bottom       = [ "G", "<End>" ]

# --- Saltos directos a Gestores (Column 1) ---
goto_all     = "1"
goto_pacman  = "2"
goto_aur     = "3"
goto_flatpak = "4"
goto_cargo   = "5"
goto_python  = "6"

# --- Búsqueda y Filtros ---
search       = "/"
clear_search = "<Esc>"
sort_toggle  = "s"            # Alterna Size ▼ / Name ▲ / Date

# --- Acciones ---
uninstall    = [ "d", "x" ]   # Abre confirmación de desinstalación
reload       = "r"            # Rescan de paquetes en segundo plano
help         = "?"            # Abre modal de ayuda
quit         = [ "q", "<C-c>" ]

# --- Toggle de Paneles ---
toggle_managers = "M"         # Oculta/Muestra panel izquierdo
toggle_details  = "D"         # Oculta/Muestra panel derecho para pantalla completa
```

---

## 4. Opciones de Comportamiento (`[behavior]`)

```toml
[behavior]
# Confirmación antes de desinstalar ("always", "never")
confirm_uninstall = true

# Comando personalizado para sudo si el usuario prefiere doas o pkexec
sudo_command = "sudo"         # "sudo", "doas", etc.

# Carga en segundo plano de tamaños profundos
preload_disk_sizes = true
```
