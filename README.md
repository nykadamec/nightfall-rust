# Nightfall (Rust + Bevy + gpui-kit)

Isometric 2D survivor-like game and native desktop studio editor built in Rust, with **remote macOS Apple Silicon builds on GitHub Actions** so no local `target/` directory is needed.

## Workspace Structure

- `crates/nightfall-data` — shared JSON schemas (`enemies.json`, `weapons.json`, `waves.json`) and 2:1 isometric projection helpers.
- `crates/nightfall-game` — 2D isometric survivor-like runtime built with **Bevy**.
- `crates/nightfall-editor` — native desktop editor built with **`gpui-kit`**, including direct integration with Modal FLUX `AssetGenerator`.
- `.github/workflows/macos-build.yml` — manual (`workflow_dispatch`) macOS arm64 release build workflow (saves private GitHub Actions minutes).
- `scripts/remote-build.sh` — push, trigger remote build, wait, and download binaries into `./dist/`.

## Remote Build & Run (0 GB local `target/`)

```bash
./scripts/remote-build.sh        # builds both nightfall-game & nightfall-editor
./scripts/remote-build.sh game   # builds only nightfall-game
./scripts/remote-build.sh editor # builds only nightfall-editor

./dist/nightfall-editor
./dist/nightfall-game
```
