# Omni (`omni`)

> Unified TUI Package Manager for Linux (Arch / Pacman, AUR, Flatpak, Cargo, uv, pip).

## 🎯 Visión
Una herramienta TUI rápida y ligera escrita en Rust que centraliza el inventario de software instalado en el sistema, permitiendo auditar espacio, filtrar por ecosistema y gestionar desinstalaciones sin tener que recordar la sintaxis de cada gestor individual.

---

## 🗺️ Roadmap & Milestones

### Milestone 1: Core Engine & Multi-Provider Discovery (MVP Backend)
- [ ] Definir el trait `PackageManager` unificado:
  - `name() -> &str`
  - `is_available() -> bool`
  - `list_installed() -> Result<Vec<Package>>`
  - `uninstall_cmd(pkg) -> Command`
- [ ] Implementar adaptadores iniciales:
  - **Pacman / Yay**: consulta mediante `pacman -Qi` o lectura directa de base de datos local.
  - **Flatpak**: consulta mediante `flatpak list --app --columns=application,name,version,size`.
  - **Cargo**: binarios instalados globalmente (`~/.cargo/bin` o `cargo install --list`).
  - **uv / pip**: herramientas globales (`uv tool list` y `pip list`).
- [ ] Normalización de tamaños en disco (bytes -> KiB / MiB / GiB).

### Milestone 2: TUI Layout & Interfaz Ratatui
- [ ] Arquitectura async con `tokio` (carga en paralelo de cada gestor sin congelar la UI).
- [ ] Layout principal:
  - **Barra superior / Header**: Buscador global en tiempo real y contador de paquetes totales.
  - **Panel lateral / Tabs de filtro**: Filtro rápido por tipo (`All`, `Pacman`, `AUR`, `Flatpak`, `Cargo`, `Python`).
  - **Tabla central**: Lista de paquetes con Nombre, Versión, Origen y Tamaño en disco.
  - **Panel inferior / Inspector**: Detalles del paquete seleccionado (descripción, dependencias, ruta).
- [ ] Paleta de colores personalizable (soporte para tema Gruvbox por defecto).

### Milestone 3: Búsqueda, Ordenamiento & Acciones
- [ ] Búsqueda difusa / incremental por nombre y descripción.
- [ ] Ordenar por tamaño en disco (identificar qué paquetes consumen más espacio).
- [ ] Modal de confirmación para desinstalación (`d` o `x`).
- [ ] Ejecución de desinstalación en un subproceso interactivo o flotante.

### Milestone 4: Modo CLI & Exportación
- [ ] Subcomando CLI para consultas sin interfaz TUI: `omni list --sort size --json`.
- [ ] Detección de paquetes huérfanos / no utilizados.
