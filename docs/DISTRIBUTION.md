# Downloads and distribution formats

Omni is a terminal application for Linux. The initial release provides two x86-64 downloads, both built from the same optimized Rust binary.

## AppImage

A single executable file that is convenient to download and try without installing a system package. Download it from GitHub Releases, mark it executable, and run it from a terminal:

```sh
chmod +x Omni-1.0.0-x86_64.AppImage
./Omni-1.0.0-x86_64.AppImage
```

AppImage Type 2 normally uses FUSE to mount its contents. If FUSE is unavailable, the runtime provides `--appimage-extract-and-run` as an alternative. The AppImage does not include or install the package managers Omni manages.

## Portable tarball

The `.tar.gz` archive contains the `omni` binary, `LICENSE`, and `README.md`. It does not install files on the system:

```sh
tar -xzf omni-1.0.0-x86_64-unknown-linux-gnu.tar.gz
./omni
```

The GNU/Linux binary is built on Ubuntu 22.04 to support systems with the same or newer glibc version. Package managers you want to inspect must be installed separately and available on `PATH`.

## Install from source with Cargo

If you have a local repository checkout and Rust/Cargo installed, install the executable directly from its `Cargo.toml`:

```sh
cd Omni
cargo install --path . --locked
```

Cargo builds the local package and places `omni` in its configured install directory, usually `~/.cargo/bin`. To update from another local revision, repeat the command with `--force`. This does not publish the project to crates.io or install system dependencies.

## Other possible formats

The 1.0.0 release does not build native distribution packages:

- **Arch Linux:** a `PKGBUILD` could be added for the AUR or to build a `.pkg.tar.zst` package.
- **Debian/Ubuntu:** a `.deb` package could be added.
- **Fedora/RHEL/openSUSE:** an `.rpm` package could be added.
- **crates.io:** publishing there is not required for `cargo install --path`; it would be a separate option for installing by name from the registry.

These formats need their own metadata, install/uninstall checks, and publication maintenance. The AppImage and tarball provide simple Linux downloads without implying integration with those distribution repositories.
