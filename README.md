# Omni

Gestor de paquetes para terminal: un solo inventario para Pacman, AUR, Flatpak, Cargo, uv, pip y dependencias de proyectos. Diseñado para navegar con el teclado y mostrar solo lo necesario.

> **Versión candidata:** 1.0.0. La release estable se publicará al integrar esta rama y crear el tag `v1.0.0`.

![Vista principal de Omni con datos de ejemplo](docs/images/omni-overview.png)

*La captura usa paquetes ficticios. [Ver demo animada (GIF)](docs/images/omni-demo.gif) o [abrir la captura de la ayuda](docs/images/omni-help.png).*

## Funciones

- Inventario unificado, búsqueda, ordenación y detalles por paquete.
- Oculta dependencias por defecto; `e` permite mostrarlas.
- Actualizaciones en segundo plano, selección múltiple y acciones de actualizar o desinstalar con confirmación.
- Dependencias locales de proyectos Cargo, npm y Bun.
- Paneles, colores, bordes y teclas personalizables con TOML y recarga en vivo.
- CLI para listar paquetes en texto o JSON; logs persistentes para diagnosticar errores.

## Instalación

### Desde una copia del repositorio con Cargo

Requiere Rust estable y Cargo. Clona el repositorio e instala el ejecutable desde su `Cargo.toml`:

```sh
git clone https://github.com/ThaielSC/Omni.git
cd Omni
cargo install --path . --locked
omni
```

Esto compila e instala Omni en el directorio de binarios de Cargo (normalmente `~/.cargo/bin`). **No publica el proyecto en crates.io ni requiere una cuenta allí.** Para reinstalar o actualizar desde una copia más reciente:

```sh
cargo install --path . --locked --force
```

### Descargas precompiladas para Linux x86-64

Al publicar `v1.0.0`, GitHub Releases incluirá:

- `Omni-1.0.0-x86_64.AppImage`
- `omni-1.0.0-x86_64-unknown-linux-gnu.tar.gz`
- `SHA256SUMS` para verificar ambas descargas

El AppImage se distribuye como un solo archivo; el tarball contiene el binario, la licencia y este README. Los pasos y requisitos están en [formatos de distribución](docs/DISTRIBUTION.md).

## Uso rápido

Omni se inicia como usuario normal. Los gestores que quieras consultar deben estar instalados y disponibles en `PATH`. En Arch, `pacman-contrib` proporciona `checkupdates`; algunas comprobaciones de actualizaciones requieren herramientas opcionales del gestor.

| Tecla | Acción |
| --- | --- |
| `h` / `l`, `Tab` | Cambiar de panel |
| `j` / `k`, flechas | Moverse por la lista |
| `/` | Buscar por nombre o descripción |
| `u` | Mostrar paquetes con actualizaciones disponibles |
| `Space` / `,` | Seleccionar un paquete / limpiar la selección |
| `U` / `d` | Actualizar / revisar desinstalación |
| `e` | Incluir u ocultar dependencias |
| `c` / `C` | Copiar nombre del paquete / contenido de Details |
| `?` | Abrir la ayuda de teclas |
| `q` | Salir |

La ayuda dentro de la aplicación muestra el mapa completo de teclas activas y se puede filtrar con `/`.

Para usar el inventario sin abrir la TUI:

```sh
omni --list
omni --list --json
omni --list --manager flatpak --sort size
```

## Configuración y seguridad

Los archivos de usuario están en `~/.config/omni/`:

- `config.toml`: paneles, comportamiento e integraciones de proyectos.
- `theme.toml`: colores y estilos.
- `keymap.toml`: atajos.
- `managers.toml`: gestores personalizados.

Omni solicita confirmación antes de modificar paquetes. Si una operación requiere privilegios del sistema, la app pide la contraseña en un prompt enmascarado y la pasa directamente al proceso; no hace falta iniciar Omni con `sudo`. Los comandos de gestores personalizados pueden ejecutar acciones arbitrarias: revisa su configuración antes de habilitarlos.

Para consultar los logs: `omni --logs`. La ruta se obtiene con `omni --log-path`; por defecto usa `~/.local/state/omni/omni.log`.

## Desarrollo

```sh
cargo fmt --all -- --check
cargo test --all-targets
cargo build --release
```

Las pruebas de operaciones usan entornos de ejemplo; revisa la cobertura y sus límites antes de ejecutar acciones reales.

## Documentación

- [Cobertura y límites por proveedor](docs/PROVIDER_SUPPORT.md)
- [Formatos de distribución](docs/DISTRIBUTION.md)
- [Revisión de seguridad](docs/SECURITY_REVIEW.md)
- [Versionado y releases](docs/VERSIONING.md)
- [Changelog](CHANGELOG.md)
- [Licencia MIT](LICENSE)
