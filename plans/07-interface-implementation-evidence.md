# Interface implementation and verification evidence

Implemented October 13–14, 2026 on `implementation/trustworthy-scraping`, following [the interface plan](07-interface-simplification.md). Existing migration-repair behavior and the user's database were preserved; all persistence tests used temporary databases.

## Delivered behavior

- One global interface font, with locally bundled licensed assets for all five downloadable families. Settings, headings, descriptions, controls, dialogs and navigation use the selected family. YAML/code retain an explicit fixed-width exception. Failed preference saves roll back and show an error. Appearance uses one family picker and preview.
- Shared control sizing, quiet dialog scrims, sticky workflow actions and responsive navigation/search. All top-level tabs remain visible at the tested narrow width. Reduced-motion transitions are immediate.
- One continuous workflow form: Source, optional Matching, and Schedule. Automatic detection suggests the name. Preview re-detects a source if editing interrupted detection, retaining the latest matching rules. Advanced fields/YAML survive round trips. Configuration edits invalidate previews. Successful zero-match previews can be saved; saving does not run a scrape.
- One responsive job inspector across discovered and tracked views. The header groups favorite, trash and close SVG controls. The main actions, description and notes are consistent; metadata, evidence and history use disclosures. Notes report save state, retry failures, and flush against the correct job when switching. Removal retains confirmation, blocking, Undo and restore semantics.
- Six default tracked columns, optional persisted columns with reset, and repository sorting by stage-change time. Timestamp sorting handles both legacy SQLite and RFC3339 representations. List/board selection and scroll positions are retained across view changes.
- Solid Kanban surfaces, readable two-line titles and metadata, compact empty states, a narrow-window stage selector, pointer drag/drop and keyboard-accessible stage selection. Counts explicitly describe loaded page records, rather than pretending to be complete stage totals.
- Normal browser examples use captured SEP postings extracted by the actual Rust worker. They reject scrape execution. Synthetic data is restricted to test mode or explicit development-only fixture harnesses; production ignores the fixture flag. Raw SEP/Linear responses have SHA-256 provenance manifests. Verification replays HTTP through the packaged worker and compares provider IDs, titles and destinations. Rust integration continues through the production ingestion mapper into SQLite.

## Commits

| Commit | Change |
| --- | --- |
| `b96013b` | Global bundled typography and shared controls |
| `898bf5a` | Continuous workflow creation and preview |
| `5cc11e9` | Isolated native smoke and WebView font assertions |
| `ec2e12b` | Recorded examples and real-worker provider verification |
| `d965d59` | Shared inspector, simpler List/board, persistence regressions |
| `8088d73` | Complete journey, contrast helpers and reviewed screenshots |
| `ed8f64e` | Narrow navigation, interrupted detection and numeric timestamp ordering |
| `016a4f9` | Isolated browser ports for clean-checkout verification |
| `c3646f4` | Stable screenshot states, immediate reduced motion, cold WebView readiness |

## Checks actually run

The initial frontend baseline had 360 passing tests. Final frontend coverage has 365 tests across 52 files. New/adapted tests cover font loading, the new workflow interaction path, interrupted detection, advanced/YAML preservation, note failures/retry/job switching, distinct action targets, optional columns, actual worker persistence, typed stage sorting and browser/runtime mode boundaries. Obsolete markup assertions were adapted alongside observable interaction tests.

| Check | Result |
| --- | --- |
| `npm ci` in fresh checkout | Passed; dependencies installed from the committed lockfile |
| `npm run build:sidecar` in fresh checkout | Passed; the required packaged worker was built before dependent tests |
| `npm test` | 365 passed; frontend development entrypoint smoke passed |
| `npx tsc --noEmit` | Passed |
| `npm run build` | Passed; existing bundle-size advisory remains |
| `node scripts/check-style.mjs` | Passed |
| `cargo fmt --all --check` | Passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | 256 passed, including actual worker → repository and generated-binding tests |
| Binding drift check after export tests | No diff in `src/bindings` |
| `npm run test:providers` | Recorded HTTP replay: 8 SEP and 31 Linear postings; all associations preserved, no extraction errors |
| Complete Playwright suite | 16 passed from the fresh checkout on its own server port, including actual 200% zoom |
| `node scripts/native-smoke.mjs` | Passed from the fresh checkout: actual WebView fonts/selection, preview/save IPC, packaged worker, persistence, blocking/restore, cancellation, malformed output/nonzero exit/timeout, partial recovery and external destination reached |
| `node scripts/test-native-startup.mjs` | Actual Tauri development startup passed for fresh, recognized historical and current databases; unknown state was rejected and preserved |

The clean checkout was `/tmp/scrape-ui-clean-verification`, with independent npm installation and an isolated browser server on port 1439. It reused the Rust compiler cache through a target-directory symlink; it did not copy the application's ignored database or an existing packaged worker into the checkout. The sidecar build generated that required artifact there. Browser engine binaries were reused as a tool cache. Native fixtures each owned a validated temporary directory.

Visual review covered Paper and Obsidian, workflow forms, inspectors, application List and Kanban at 480/1024/1440px, plus existing 1280px settings/removal baselines and a bundled monospace workflow specimen. Rendered contrast is checked in both themes; dedicated tests cover essential icon/focus indicators. Browser tests exercise keyboard controls, notes, removal/Undo, persisted native hourly cents, view transitions and pointer dragging. A separate extension verifies real browser zoom at 200%.

## Live-provider evidence

At **2026-10-14 03:45:46 UTC**, the actual Rust worker fetched public provider data and preserved every tested provider ID/title/destination association:

| Adapter/source | Posting associations verified | Extraction errors |
| --- | --- | --- |
| Lever / SEP | 8 | 0 |
| Ashby / Linear | 31 | 0 |
| Greenhouse / Bitwarden | 41 | 0 |

The source endpoints and captured-response checksums are recorded by `scripts/provider-verification.mjs`; committed raw captures and their metadata are in `fixtures/providers`. This was live worker verification, separate from browser fixtures. It establishes the tested sources at that timestamp, not future availability or all postings from every supported publisher. Generic JSON-LD/selector behavior uses deterministic local HTTP and native fixtures; no live check is claimed for those generic adapters.

## Problems discovered and resolved during verification

- Several tests expected the removed text deletion trigger and boxed URL layout. They now preserve identity/opener/removal assertions through actual controls and the new inspector.
- A running development instance could make native smoke exit before testing because of the single-instance plugin. Validated debug-only smoke runs bypass that plugin; production keeps it. A persisted completion report is required before success.
- A cold WebView could be inspected before its interface/font CSS had loaded. The harness now waits for the actual font picker before asserting loaded faces and global selection.
- Pointer position could trigger a delayed tooltip in a screenshot; very short reduced-motion transitions could yield transient contrast readings. Screenshots now capture an explicit resting state, and reduced-motion transitions use zero duration. Assertions were retained; screenshot differences were reviewed before baseline updates.
- Interrupted detection could leave a new workflow using its initial adapter. A dedicated interaction regression proves re-detection and latest-rule preservation before preview.

## Practical limits

No independent participant usability study was conducted. Native checks are automated macOS WebView/IPC checks, complemented by browser visual and keyboard review; no manual QA on Windows or Linux is claimed. Kanban deliberately reports page-local counts rather than implementing a new per-stage aggregation API. Live-provider checks remain optional and timestamped; required CI stays deterministic. No schema migration or repair of the user's live database was needed for these interface changes.
