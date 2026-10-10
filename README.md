# bd-neon

Fast, native Kanban GUI for [Beads](https://github.com/gastownhall/beads) issue tracking — built in Rust (`eframe 0.36`).
For devs and agents using the `bd` CLI: instant board, keyboard-first navigation, and a glass OLED-friendly UI. Shells out to `bd` for all issue data — no daemon, no lock-in.

## Prerequisites

- **OS matching the release artifact you download**
  (currently `x86_64-pc-windows-msvc` for Windows 10/11 64-bit).
- **GPU with OpenGL support** — the UI renders via `eframe`/`glow`.
- **`bd` (Beads CLI) on `PATH` at runtime** — bd-neon shells out to `bd`
  for all issue data. Verify with `bd --version` before launching.
  Without `bd` on `PATH`, the issue list stays empty / shows stale state.

## Install (portable artifacts)

No installer. Each release ships portable artifact(s):

```text
bd-neon-<ver>-x86_64-pc-windows-msvc.zip   (contains bd-neon.exe + README.md)
bd-neon-<ver>-x86_64-pc-windows-msvc.zip.sha256
```

1. Download the zip and its `.sha256` from the GitHub release page.
2. (Optional) Verify the hash, e.g. `certutil -hashfile <zip> SHA256`
   and compare with the `.sha256` file.
3. Extract anywhere (e.g. `C:\tools\bd-neon`).
4. Run with an optional repo path argument:

```powershell
.\bd-neon.exe                    # opens current directory as repo
.\bd-neon.exe C:\path\to\repo   # opens the given repo
```

The exe is fully portable — no registry writes, no admin rights needed.

## SmartScreen note (unsigned exe)

The release exe is **unsigned**, so Windows SmartScreen / Defender may show
an "Unknown publisher" / "Windows protected your PC" warning on first run.
This is expected for unsigned builds, not a sign of malware.

To run anyway:

1. Right-click `bd-neon.exe` → **Properties** → check **Unblock** → OK
   (only needed if the file is still marked as downloaded from the web).
2. On the SmartScreen dialog, click **More info** → **Run anyway**.
3. Alternatively, verify the SHA256 hash (see above) before bypassing.

## Release flow (maintainers)

Versions live in `Cargo.toml` (`[package] version`). `bump.config.ts`
drives the bump:

```powershell
npx bumpp        # bumps Cargo.toml, then runs `cargo check`
```

This updates `Cargo.toml` and runs `cargo check` as a sanity gate.
Then commit, tag, and push:

```powershell
git commit -am "chore: release vX.Y.Z"
git tag vX.Y.Z
git push origin main vX.Y.Z
```

Pushing a `v*` tag triggers `.github/workflows/release.yml`, which:

1. Reads the version from `Cargo.toml` and from the tag name.
2. **Fails if they differ** — the tag must be exactly
   `v<Cargo.toml version>` (e.g. tag `v0.1.0` for `version = "0.1.0"`).
3. Runs `cargo build --release --locked` on `windows-latest`.
4. Stages `bd-neon.exe` (+ `README.md`) into
   `bd-neon-<ver>-x86_64-pc-windows-msvc.zip` plus a `.sha256` file.
5. Publishes a GitHub release with the zip attached.
   (Manual `workflow_dispatch` builds use the Cargo.toml version or the
   `version` input override instead of a tag.)

## Footer version stamp

The footer (bottom-right) shows `v<version>` baked in at compile time via
`concat!("v", env!("CARGO_PKG_VERSION"))` in `src/ui/footer.rs`.
To verify a release build matches its tag, compare the footer stamp with
the `Cargo.toml` version and the release tag — all three must agree.
