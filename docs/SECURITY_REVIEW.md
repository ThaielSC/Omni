# Security review

Review date: 2026-10-11  
Reviewed revision: merged `main` candidate `1.0.0` (commit `996a1d1` plus release metadata).

## Command execution review

- Native manager operations use `std::process::Command` with separate arguments. Package names reject NUL bytes; project/update package names also reject leading `-`, whitespace, and control characters where they could be interpreted as options.
- Project-local Cargo, npm, and Bun commands run with the discovered project root as their working directory and pass package names as one argument. Project discovery reads manifests; it does not install or execute project code.
- Custom manager listing, metadata, update checks, and `update_cmd` use the configured executable and argv. They do not evaluate shell syntax.
- Custom `uninstall_cmd` is intentionally a shell template. `{name}` and `{version}` are POSIX single-quoted before substitution, but the rest of the configured template executes verbatim. Treat `managers.toml` as trusted code. `need_sudo = true` makes that trust boundary privileged and the confirmation UI displays the rendered command.
- Omni itself runs as the invoking user. When `sudo` is required, the user confirms first; the password goes only to sudo stdin, is not placed in arguments/environment/logs, and its buffer is zeroized. Alternate helpers such as `doas` require their own non-interactive authorization.
- Process output is shown in the operation console. Persistent logs record notices and outcomes, not the full subprocess stream or sudo password.

## Dependency audit

- `cargo audit` (cargo-audit 0.22.2) fetched the current RustSec advisory database and scanned `Cargo.lock` on 2026-10-11: **217 locked crate dependencies, 0 known vulnerabilities, 0 warnings** (exit status 0). JSON summary: `vulnerabilities.found=false`, `vulnerabilities.count=0`, `warnings={}`.
- An OSV batch query of the registry package versions also returned zero findings. Advisory databases change; rerun `cargo audit` before each release.
- `tokio` and `async-trait` were removed from direct dependencies because the current implementation uses standard-library threads and does not reference either crate. `cargo check` confirmed the reduced dependency graph builds.

## Manual authentication verification

On 2026-10-11, the actual Omni TUI prompt was exercised with host sudo credentials through an isolated temporary custom manager. The selected fake package ran `sudo -n sh -c true`; Omni displayed the operation as successful in Details. The command is harmless and changed no packages. This verifies the real sudo authentication path and host policy for this command, but does not claim a real privileged Pacman removal. Disposable package-operation tests and the private-root AUR build/update are recorded in [`PROVIDER_SUPPORT.md`](PROVIDER_SUPPORT.md).

This review is still a source/dependency audit, not proof that every manager behaves identically on every host. Repeat CI, dependency advisories, and a final limitation review against the release candidate. Never test destructive operations against the user's live package database.
