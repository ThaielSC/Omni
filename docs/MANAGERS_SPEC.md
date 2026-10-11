# Especificación del Motor Genérico de Gestores (`managers.toml`) para Omni

Permite extender Omni a **cualquier gestor de paquetes presente o futuro** (como `brew`, `nix`, `guix`, `gem`, `npm`, `cargo`, `pacman`, etc.) sin tener que recompilar la aplicación ni escribir código en Rust.

---

## 1. Archivo de Configuración: `~/.config/omni/managers.toml`

Omni incluye adaptadores nativos optimizados, pero si se define una tabla `[[manager]]` en `managers.toml`, Omni la registra dinámicamente como un nuevo proveedor soportado.

### Ejemplo Completo de Definición:

```toml
# ~/.config/omni/managers.toml

# --- Gestor de Herramientas Python: uv ---
[[manager]]
id = "uv"
name = "UV Tools"
badge = "[UV ]"
color = "#d8a657"            # Amarillo Gruvbox
binary = "uv"                # Comando verificado con `which`

# Comando para listar paquetes instalados
list_cmd = "uv tool list"

# Formato de salida del comando de listado:
# Opciones soportadas: "line_regex", "json", "tsv", "csv"
format = "line_regex"
# Expresión regular con grupos nombrados (?P<name>...), (?P<version>...), (?P<path>...)
parse_regex = '^(?P<name>[a-zA-Z0-9_\-]+)\s+v(?P<version>[0-9\.]+)\s+\((?P<path>.*)\)$'

# Comando opcional para calcular tamaño en disco
size_cmd = "du -sb {path}"   # {path} o {name} inyectado desde la captura

# Comando de inspección / detalles
info_cmd = "uv tool show {name}"

# Comando de desinstalación
uninstall_cmd = "uv tool uninstall {name}"
need_sudo = false

# Updates son opt-in. Los argumentos se separan en argv (sin shell).
update_check_cmd = "outdated --json"
update_format = "json"                 # json o line_regex
update_json_array_path = "/updates"
update_json_name = "/name"
update_json_latest_version = "/latest"
update_cmd = "update {name} --to {latest_version}"


# --- Gestor de Paquetes Ruby: Gem ---
[[manager]]
id = "gem"
name = "Ruby Gems"
badge = "[GEM]"
color = "#ea6962"
binary = "gem"
list_cmd = "gem list --local"
format = "line_regex"
parse_regex = '^(?P<name>[a-zA-Z0-9_\-]+)\s+\((?P<version>[0-9\., ]+)\)'
uninstall_cmd = "gem uninstall {name}"
need_sudo = false


# --- Homebrew en Linux (Linuxbrew) ---
[[manager]]
id = "brew"
name = "Homebrew"
badge = "[BRW]"
color = "#e78a4e"
binary = "brew"
list_cmd = "brew info --installed --json=v2"
format = "json"
# Rutas JSONPointer / JQ para extraer los campos
json_array_path = "/formulae"
json_name       = "/name"
json_version    = "/versions/stable"
json_size       = "/installed/0/installed_as_dependency"
json_desc       = "/desc"
uninstall_cmd   = "brew uninstall {name}"
need_sudo       = false


# --- NPM Global Packages ---
[[manager]]
id = "npm"
name = "NPM Global"
badge = "[NPM]"
color = "#ea6962"
binary = "npm"
list_cmd = "npm list -g --depth=0 --json"
format = "json"
json_map_path   = "/dependencies"
json_version    = "/version"
uninstall_cmd   = "npm uninstall -g {name}"
need_sudo       = false
```

---

## 2. Definición del Modelo en Rust (`src/provider/custom.rs`)

En Rust, el trait `PackageManagerProvider` se implementa de forma genérica con la estructura `CustomManager`:

> **Seguridad:** `uninstall_cmd` es una línea de shell ejecutable, no una descripción declarativa inocua. Omni escapa los valores interpolados `{name}` y `{version}`, pero el resto de la plantilla se ejecuta tal cual. Trata `managers.toml` como código confiable; con `need_sudo = true`, la orden se ejecuta con privilegios elevados.

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct CustomManagerConfig {
    pub id: String,
    pub name: String,
    pub badge: String,
    pub color: Option<String>,
    pub binary: String,
    pub list_cmd: String,
    pub format: OutputFormat,         // LineRegex, Json, Tsv, Csv
    pub parse_regex: Option<String>,
    pub json_array_path: Option<String>,
    pub json_name: Option<String>,
    pub json_version: Option<String>,
    pub json_desc: Option<String>,
    pub size_cmd: Option<String>,
    pub info_cmd: Option<String>,
    pub uninstall_cmd: String,
    pub update_check_cmd: Option<String>,
    pub update_format: Option<OutputFormat>,
    pub update_parse_regex: Option<String>,
    pub update_json_array_path: Option<String>,
    pub update_json_name: Option<String>,
    pub update_json_latest_version: Option<String>,
    pub update_cmd: Option<String>,
    #[serde(default)]
    pub need_sudo: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    LineRegex,
    Json,
    Tsv,
    Csv,
}
```

---

## 3. Ventajas de esta Arquitectura
1. **Extensibilidad infinita**: Si un usuario usa `pipx`, `nix-env`, `asdf`, `go install`, `npm`, `composer` o `guix`, simplemente añade un bloque `[[manager]]` en `managers.toml`.
2. **Zero-code plugins**: Los usuarios pueden compartir snippets de configuración de sus gestores favoritos en la comunidad.
3. **Robustez**: Si el binario (`binary = "..."`) no está en el `$PATH`, Omni desactiva el gestor automáticamente sin errores.

---

## 4. Gestores de dependencias locales de proyectos

Los gestores globales configurados en `managers.toml` son independientes de la integración opcional de proyectos. Esta última se controla desde `~/.config/omni/config.toml` con `[manager.projects]`:

```toml
[manager.projects]
enabled = true
cargo = true
npm = true
bun = true
```

Los valores predeterminados habilitan la detección de Cargo, npm y Bun. `enabled = false` deshabilita por completo la integración local; los booleanos individuales optan por ecosistema. Omni reconoce Cargo mediante `Cargo.toml` y proyectos JavaScript mediante `package.json`, y clasifica npm/Bun según los metadatos del proyecto. En el panel Managers la categoría lleva el nombre de la carpeta del proyecto y el gestor entre corchetes, por ejemplo `omni[Cargo]`, `website[npm]` o `website[bun]`.

La lista local representa dependencias directas declaradas por el proyecto, no un inventario global de paquetes. Para agregar una dependencia, Omni solicita el nombre y ejecuta `cargo add <paquete>`, `npm install <paquete>` o `bun add <paquete>` con el directorio de trabajo en la raíz detectada. Los argumentos se pasan directamente al proceso; no se interpolan en una shell. No se ejecuta ningún comando de instalación al iniciar ni al explorar. La salida queda disponible en Details. El gestor de origen es quien modifica el manifiesto y lockfile y quien aplica sus scripts o reglas de resolución.

Desactiva con `[manager.projects]` los ecosistemas que no quieras que Omni detecte. Las operaciones de proyecto no necesitan `sudo` y se ejecutan con los permisos del usuario actual. Revisa el nombre del paquete antes de confirmar: Cargo, npm y Bun descargan código y pueden ejecutar scripts de instalación conforme a la configuración propia de cada herramienta.
