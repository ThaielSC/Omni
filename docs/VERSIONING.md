# Versionado y releases

Omni usa [Semantic Versioning 2.0.0](https://semver.org/), con tags de Git que comienzan con `v` y coinciden con la versión de `Cargo.toml`.

## Criterios

- **PATCH** (`1.0.1`): correcciones compatibles, ajustes de rendimiento y cambios internos que no alteran el uso ni la configuración existente.
- **MINOR** (`1.1.0`): funciones nuevas compatibles, como otro gestor de paquetes o nuevas opciones de configuración.
- **MAJOR** (`2.0.0`): cambios incompatibles en el CLI, la configuración o el comportamiento sobre el que dependen los usuarios.
- Las versiones previas a una estable usan sufijos SemVer, por ejemplo `1.1.0-alpha.1` o `1.1.0-rc.1`. No se mantienen hitos históricos aquí; el historial publicado está en [`CHANGELOG.md`](../CHANGELOG.md) y en los tags.

## Publicar una versión

1. Actualizar `version` en `Cargo.toml` y `Cargo.lock`, y añadir las notas de la versión a [`CHANGELOG.md`](../CHANGELOG.md).
2. Ejecutar `cargo fmt --all -- --check`, `cargo test --all-targets`, `cargo build --release` y `cargo audit`; comprobar que la CI de la revisión final pase.
3. Integrar los cambios a `main` mediante la revisión habitual del repositorio.
4. Crear y publicar el tag **anotado** `vX.Y.Z` sobre el commit aprobado de `main`, con un mensaje breve de release. El workflow de distribución compila los artefactos Linux y crea la GitHub Release adjuntándolos junto a las notas del changelog.
5. Verificar que el tag apunta al commit esperado y que los metadatos del paquete siguen indicando la misma versión.

Para la primera estable, el candidato es `1.0.0`; se considera publicada cuando `v1.0.0` y su release están disponibles. Hasta entonces, las notas usan el estado `Unreleased`.
