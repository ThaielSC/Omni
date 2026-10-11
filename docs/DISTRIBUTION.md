# Descargas y formatos de distribución

Omni es una aplicación de terminal para Linux. La release inicial publica dos descargas x86-64; ambas se generan a partir del mismo binario Rust optimizado.

## AppImage

Un archivo autocontenido y ejecutable, cómodo para descargar y probar sin instalar un paquete del sistema. Descárgalo desde GitHub Releases, marca el archivo como ejecutable y lánzalo desde una terminal:

```sh
chmod +x Omni-1.0.0-x86_64.AppImage
./Omni-1.0.0-x86_64.AppImage
```

AppImage Type 2 normalmente usa FUSE para montar su contenido. Si el sistema no lo tiene disponible, el runtime ofrece `--appimage-extract-and-run` como alternativa. El AppImage no incluye ni instala los gestores de paquetes que Omni administra.

## Tarball portable

El archivo `.tar.gz` incluye el binario `omni`, `LICENSE` y `README.md`. No instala archivos en el sistema:

```sh
tar -xzf omni-1.0.0-x86_64-unknown-linux-gnu.tar.gz
./omni
```

El binario GNU/Linux se construye en Ubuntu 22.04 para mantener compatibilidad con sistemas que ofrecen una versión de glibc igual o posterior. Los gestores de paquetes que quieras consultar deben estar instalados por separado y disponibles en `PATH`.

## Instalar desde el código con Cargo

Si ya tienes una copia local del repositorio y Rust/Cargo instalados, puedes instalar el ejecutable directamente desde su `Cargo.toml`:

```sh
cd Omni
cargo install --path . --locked
```

Cargo compila el paquete local y coloca `omni` en el directorio de instalación configurado, normalmente `~/.cargo/bin`. Para actualizarlo desde otra revisión local, repite el comando con `--force`. Esto no publica el proyecto en crates.io ni instala dependencias del sistema.

## Otros formatos posibles

La release 1.0.0 no genera paquetes nativos adicionales:

- **Arch Linux:** se puede añadir un `PKGBUILD` para publicar en AUR o construir un paquete `.pkg.tar.zst`.
- **Debian/Ubuntu:** se puede añadir un paquete `.deb`.
- **Fedora/RHEL/openSUSE:** se puede añadir un paquete `.rpm`.
- **Crates.io:** no es necesario para la instalación con `cargo install --path`; publicar allí sería una opción aparte para quien quiera instalar por nombre desde el registro.

Estos formatos requieren sus propios metadatos, pruebas de instalación/desinstalación y mantenimiento de publicación. El AppImage y el tarball ofrecen una primera descarga Linux sencilla sin fingir que ya existe integración con esos repositorios.
