# Provider support and metadata limits

This matrix documents what Omni currently reads and what it can safely infer. A blank or `unknown` field is intentional: providers expose different metadata, and Omni does not estimate package size or invent dependency relationships.

## Global package managers

| Manager | Inventory source | Metadata currently exposed | Update discovery | Update / uninstall behavior and limits |
| --- | --- | --- | --- | --- |
| Pacman | `pacman -Qi` | Version, description, installed size, license, URL, install date, declared dependencies, and install reason (`explicit` / `dependency`) when recognized. | `checkupdates` for repository packages; an empty result with its normal status code means no updates. | Updates use `pacman -Syu --needed` with selected targets. Arch requires a full system upgrade; this is not a package-only partial upgrade. Uninstall uses `pacman -R` (no recursive dependency removal) and privilege escalation. |
| AUR / yay | `pacman -Qm` identifies foreign packages, then `pacman -Qi` supplies their installed metadata. | Same fields as Pacman where pacman reports them. The AUR origin is inferred from membership in `-Qm`; the package source is shown as AUR. | `yay -Qua`; an empty successful/no-result response is treated as no updates. | Updates use `yay -S --aur --needed` for selected targets. Removal uses `pacman -R`, so it removes the selected package without asking pacman to cascade through dependencies. |
| Flatpak | `flatpak list --app --json` | Application ID, version, description, installed size when returned in a recognized string format, origin, and required runtime reference. Flatpak does not supply an explicit-vs-dependency install reason here, so it stays `unknown`. | `flatpak remote-ls --updates --app --columns=application,version`. | Update uses `flatpak update`; uninstall uses `flatpak uninstall`. Size can remain unknown when the Flatpak CLI omits it or returns an unsupported representation. Runtime is the app's declared runtime requirement, not a complete resolved dependency graph. |
| Cargo global tools | `cargo install --list`, falling back to Cargo's `.crates2.json` registry if the command fails and that file is readable. | Installed tool name/version, binary names, and explicit install reason. Package size, license, description, and dependency graph are not collected. | Optional `cargo-update` helper via `cargo install-update --list`; without it the check is reported as unsupported. | Update uses `cargo install --force <name>`; uninstall uses `cargo uninstall <name>`. Reinstalling a tool from a non-default source may need source metadata Cargo's install registry does not retain. |
| uv tools | `uv tool list` | Tool name/version and explicit install reason. Size, license, description, and dependencies are not currently collected. | `uv tool list --outdated`; parsing recognizes current and latest tool versions in supported uv text formats. | Update uses `uv tool upgrade <name>`; uninstall uses `uv tool uninstall <name>`. |
| pip | `pip inspect`, falling back to `pip list --format=json` for older pip. | Name/version; `pip inspect` additionally supplies requested status and declared `Requires-Dist` names. Legacy `pip list` cannot establish install reason or dependencies, which stay unknown/empty. These are declarations, not an environment-resolved dependency graph. Size, license, and description are not currently collected. | `pip list --outdated --format=json`. | Update uses `pip install --upgrade <name>`; uninstall uses `pip uninstall -y <name>`. A system-managed Python installation can reject writes (PEP 668); use a user environment/virtual environment or the owning OS manager. Omni shows the manager's diagnostic and does not bypass that protection. |

## Custom global managers

Definitions in `managers.toml` may emit JSON, TSV, CSV, or line-regex records. The schema can provide a description, license, installed size, declared dependency names, URL, and other supported package fields. A `size_cmd` or `info_cmd` can enrich records; if it is absent, fails, or emits an unrecognized size, the corresponding value remains unknown. Install reason, resolved dependency graph, and update support are not inferred.

Update discovery is opt-in (`update_check_cmd`) and currently accepts JSON or line-regex output with configured name/version paths or named captures. The update action uses configured argv tokens and substitutions, without shell interpretation. Custom uninstall templates are a separate, intentionally powerful feature: they are shell commands and can run arbitrary configured actions. Omni displays the command for review, escapes interpolated values, honors `need_sudo`, and logs the operation outcome. Only configure templates you trust.

## Project-local managers

Cargo, npm, and Bun integrations read direct dependency declarations from the nearest supported project manifests (including supported workspace members). The displayed version is the manifest's requirement/range, not necessarily the resolved version in the lockfile. These rows describe declarations, not globally installed packages; size, install reason, and full resolved dependency graph are unavailable.

Update discovery is read-only: Cargo runs `cargo update --dry-run`, Bun runs `bun update --dry-run --no-save`, and both filter results to direct declarations; npm uses `npm outdated --json` and reads its `wanted` version, the newest version admitted by the manifest range. Bun's `outdated` table can report the locked version in its `Update` column even when a compatible version exists, so Omni uses Bun's dry-run update output instead. An explicit update, add, or remove action can modify the project's manifest and/or lockfile according to the package manager. These operations use argv and run in the project directory, without sudo.

## Local read-only inventory check

On 2026-10-10, the release workspace successfully enumerated all six installed global sources through the release binary:

| Source | Packages found | Metadata limitations observed |
| --- | ---: | --- |
| Pacman | 1,372 | No missing installed sizes or install reasons in this machine's result. |
| AUR / yay | 31 | No missing installed sizes in this machine's result. |
| Flatpak | 7 | All 7 sizes and install reasons unknown, as allowed by the source contract. |
| Cargo | 1 | Size unknown. |
| uv | 2 | Sizes unknown. |
| pip | 108 | Sizes unknown; 14 install reasons unknown. |

This is a read-only smoke check of the managers installed on one Arch Linux host, not proof of every manager version or an uninstall/update acceptance test. The repeatable parser, command-construction, and CLI tests are in `cargo test --all-targets`. Real mutations still require a disposable VM/container or disposable user environment; do not validate destructive operations against a daily-use installation.

The same host's read-only update queries were exercised on 2026-10-10: `yay -Qua` exited 1 with empty output and is intentionally interpreted as “no AUR updates,” Flatpak returned no updates, uv reported two outdated tools, pip returned valid JSON candidates, and `cargo update --dry-run` reported two project dependency candidates without writing the lockfile. Pacman's `checkupdates` binary and Cargo's optional `cargo install-update` helper are not installed; Omni reports the Pacman helper as a notification and treats global Cargo update discovery as unsupported. These observations verify current command/output handling only; they do not authorize or perform an update.

A real TUI acceptance check used a temporary Python virtual environment: Omni found `idna 3.10`, confirmed its uninstall command, removed it through the Details console, then automatically refreshed the Python list from four entries to three while retaining the successful command output. The environment was removed afterward.

On 2026-10-10/11, disposable TUI checks also completed uninstall and inventory refresh for every built-in global provider:

- Pacman and AUR: installed synthetic packages into a private pacman database beneath `/tmp`, inside an unprivileged user namespace. Pacman package removal and foreign/AUR classification both worked. The sudo prompt was tested with a fixture helper: Escape cancelled without invoking it, and a sentinel password was delivered on stdin for the successful removal. No host package database was modified; this does not validate real sudo/doas authentication.
- Flatpak: built and installed a tiny app from a local OSTree repository into a temporary user installation, then removed it from the TUI and confirmed the inventory refreshed. Only the temporary app/repository were changed; the system runtime was used read-only.
- Cargo: installed a minimal local crate into a temporary `CARGO_HOME`, then uninstalled it through the TUI and confirmed the refreshed list was empty.
- uv: installed a minimal local tool into a temporary `UV_TOOL_DIR`, then uninstalled it through the TUI and confirmed the refreshed list was empty.
- pip: the temporary venv check above covered uninstall and refresh.

Project-local Cargo add/remove and a real `cargo update -p serde_json` were exercised in throwaway projects. The update was discovered read-only and changed only the temporary lockfile. npm and Bun add/remove were exercised with `is-number` in throwaway projects, and their manifests returned to the initial state after removal. npm update was exercised with a temporary `lodash` dependency; Bun update was exercised with a temporary `semver` dependency, including discovery through `bun update --dry-run --no-save`, confirmation, lockfile/manifest update, success output in Details, and inventory refresh. These project operations changed only their disposable directories. Global package update mutations remain unverified.

These checks exposed and fixed two edge cases: an empty Pacman database (`pacman -Qi` status 1 with no output) now produces an empty inventory instead of a scan error, and update operations now parse `project:<manager>:<path>` correctly before setting the process working directory. Native Flatpak uninstall also no longer requires a version field that Flatpak may leave blank. Unit tests cover the parser and project-root cases.
