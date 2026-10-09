# Phase 5 — CI/CD Pipeline

**Goal:** Automated checks on every PR and cross-platform release builds on every version tag.

← [Phase 4](./04-metrics-dashboard.md) · [Master Plan](./00-master-plan.md)

---

## 5.1 `ci.yml` — PR / push checks (bonus, recommended)

Triggers: `pull_request` → `main`/`develop`, `push` → `main`/`develop` (see [branching model](./00-master-plan.md#7-git-branching--github-actions)). Created in Phase 1 bootstrap and extended here. Single `ubuntu-22.04` job named `ci` (the required status check in branch rulesets):
- [ ] Install Linux Tauri deps (below), Rust stable, Node LTS.
- [ ] `cargo fmt --all --check` · `cargo clippy --workspace -- -D warnings` · `cargo test --workspace`
- [ ] `npm ci` · `npx tsc --noEmit` · `npx vitest run`
- [ ] Style guard: `! grep -rnE "rounded-|shadow-|border-radius:\s*[1-9]" src/` (enforces brutalist rules).
- [ ] Bindings drift: regenerate `src/bindings` and `git diff --exit-code`.

## 5.2 `release.yml` — tag-triggered builds

```yaml
name: release
on:
  push:
    tags: ['v*.*.*']

permissions:
  contents: write

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: macos-latest      # Apple Silicon
            target: aarch64-apple-darwin
          - platform: macos-latest      # Intel (cross-compiled)
            target: x86_64-apple-darwin
          - platform: ubuntu-22.04
            target: x86_64-unknown-linux-gnu
          - platform: windows-latest
            target: x86_64-pc-windows-msvc
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4

      - name: Linux deps
        if: startsWith(matrix.platform, 'ubuntu')
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf

      - uses: actions/setup-node@v4
        with: { node-version: lts/*, cache: npm }

      - uses: dtolnay/rust-toolchain@stable
        with: { targets: '${{ matrix.target }}' }

      - uses: swatinem/rust-cache@v2
        with: { workspaces: '. -> target', key: '${{ matrix.target }}' }

      - run: npm ci

      - name: Verify tag matches app version
        shell: bash
        run: node scripts/check-version.mjs "${GITHUB_REF_NAME#v}"

      - name: Verify tag is on main
        shell: bash
        run: |
          git fetch origin main --depth=200
          git merge-base --is-ancestor "$GITHUB_SHA" origin/main

      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'Scrape ${{ github.ref_name }}'
          releaseBody: 'See CHANGELOG.md. Builds are unsigned — see README for first-launch instructions.'
          releaseDraft: true
          prerelease: false
          args: --target ${{ matrix.target }}
```

Notes:
- [ ] The sidecar is built automatically: `tauri build` runs `beforeBuildCommand` → `scripts/build-sidecar.mjs`, which reads `TAURI_ENV_TARGET_TRIPLE` (set by the Tauri CLI from `--target`) and emits `src-tauri/binaries/scrape-worker-<triple>[.exe]`. No extra workflow step needed.
- [ ] All 4 matrix jobs upload into the **same draft release** (tauri-action creates it once, others attach).
- [ ] Artifacts produced: macOS `.dmg` + `.app.tar.gz` (×2 arch) · Windows `.msi` + NSIS `.exe` · Linux `.deb` + `.AppImage` (+ `.rpm`).
- [ ] `bundle.targets: "all"` in `tauri.conf.json`.

## 5.3 Release process

1. On `develop`: `node scripts/bump-version.mjs 0.2.0` → updates `package.json`, `src-tauri/Cargo.toml`, `tauri.conf.json`, `crates/*/Cargo.toml`; update `CHANGELOG.md`.
2. Open release PR `develop → main`; merge (merge commit) once `ci` is green.
3. On `main`: `git tag v0.2.0 && git push origin v0.2.0`.
4. Review the draft release on GitHub → **Publish**.

## 5.4 Signing & future hooks (optional, not blocking v0.x)

| Platform | Secrets to add later |
|---|---|
| macOS sign + notarize | `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` |
| Windows | Azure Trusted Signing / code-signing cert |
| Auto-updater | `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` + `tauri-plugin-updater` (`includeUpdaterJson: true`) |

The workflow passes these env vars only if present, so adding them later requires no YAML changes.

## Acceptance Criteria
- [ ] PR with a `rounded-md` class or a clippy warning fails `ci.yml`.
- [ ] Pushing `v0.1.0` produces a draft release with installers for all 4 targets.
- [ ] Each installer launches to the tray and the bundled `scrape-worker` runs (verified via "Run now" on one source per OS).
- [ ] Tag/version mismatch fails fast before compilation.
