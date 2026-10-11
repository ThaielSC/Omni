# Security review

Review date: 2026-10-11  
Reviewed revision: `feat/updates` after package-update fixes and compact TUI review.

## Command execution review

- Native manager operations use `std::process::Command` with separate arguments. Package names reject NUL bytes; project/update package names also reject leading `-`, whitespace, and control characters where they could be interpreted as options.
- Project-local Cargo, npm, and Bun commands run with the discovered project root as their working directory and pass package names as one argument. Project discovery reads manifests; it does not install or execute project code.
- Custom manager listing, metadata, update checks, and `update_cmd` use the configured executable and argv. They do not evaluate shell syntax.
- Custom `uninstall_cmd` is intentionally a shell template. `{name}` and `{version}` are POSIX single-quoted before substitution, but the rest of the configured template executes verbatim. Treat `managers.toml` as trusted code. `need_sudo = true` makes that trust boundary privileged and the confirmation UI displays the rendered command.
- Omni itself runs as the invoking user. When `sudo` is required, the user confirms first; the password goes only to sudo stdin, is not placed in arguments/environment/logs, and its buffer is zeroized. Alternate helpers such as `doas` require their own non-interactive authorization.
- Process output is shown in the operation console. Persistent logs record notices and outcomes, not the full subprocess stream or sudo password.

## Dependency audit

- `cargo audit` (cargo-audit 0.22.2) fetched the current RustSec advisory database and scanned `Cargo.lock` on 2026-10-11: **217 locked crate dependencies, 0 known vulnerabilities, 0 warnings** (exit status 0).
- An OSV batch query of the registry package versions also returned zero findings. Advisory databases change; rerun `cargo audit` before each release.
- `tokio` and `async-trait` were removed from direct dependencies because the current implementation uses standard-library threads and does not reference either crate. `cargo check` confirmed the reduced dependency graph builds.

## Remaining release verification

This review is a source/dependency audit, not proof that every manager behaves correctly on every host. Before a stable release, finish the remaining authentication check in [`VERSIONING.md`](VERSIONING.md): validate Omni's sudo prompt with real host credentials. The AUR build/update has been exercised against a private Pacman root under a user namespace. Never test destructive operations against the user's live package database.
