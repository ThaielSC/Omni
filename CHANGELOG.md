# Changelog

Este archivo recoge los cambios relevantes de Omni. Las versiones siguen [SemVer](docs/VERSIONING.md).

## [1.0.0] - 2026-10-11

Primera versión estable.

### Añadido

- Inventario unificado de Pacman, AUR/yay, Flatpak, herramientas Cargo, herramientas uv, paquetes pip y gestores declarativos configurables.
- Integración opcional de dependencias directas en proyectos Cargo, npm y Bun.
- TUI Ratatui de tres paneles, navegación de teclado estilo Vim/Helix, búsqueda, ordenación, ayuda contextual y recarga en vivo de configuración.
- Temas, keybindings, fondos, bordes y proporciones de paneles personalizables mediante TOML.
- Clasificación opcional de dependencias, selección múltiple y detalles del paquete seleccionado.
- Desinstalación y actualización explícitas con confirmación, salida del proceso en Details y refresco del inventario.
- Prompt de contraseña sudo enmascarado; la contraseña se entrega solo por stdin, no se registra y se borra del buffer tras su uso.
- Consultas de actualizaciones globales y por proyecto, sin bloquear la carga inicial del inventario.
- Copia al portapapeles del nombre del paquete y del contenido visible de Details.
- CLI no interactiva para inventarios de texto/JSON y logs de diagnóstico persistentes con permisos privados.

### Seguridad y límites

- Omni se ejecuta como usuario normal; solo las operaciones que lo requieren invocan el helper de privilegios tras confirmación.
- Las plantillas `uninstall_cmd` de gestores personalizados ejecutan shell configurada por el usuario y deben tratarse como código confiable.
- Cobertura de tamaño, motivo de instalación, dependencias y actualizaciones depende de cada gestor; los límites conocidos están en [`docs/PROVIDER_SUPPORT.md`](docs/PROVIDER_SUPPORT.md).
