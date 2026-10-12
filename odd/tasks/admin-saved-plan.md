# Repair saved floor plan rendering in admin

## Objective
Display an existing floor plan correctly and handle unavailable saved images without stale or collapsed editor state.

## Scope and constraints
Only public/admin.html and tests/admin-mapa.test.js; preserve existing Spanish UI copy conventions. No backend, data, credentials, remote operations, or unrelated user files. RDD is globally off. Delivery strategy: ask-on-risk; forecast below 250 authored changed lines.

## Tasks
- [x] T1: Fix inactive workflow banner visibility and saved-image loading/error state; add deterministic regression coverage.
  - Route: delegated direct; preparation for writing and two non-trivial files trigger delegation.
  - Acceptance: saved and cached images preserve natural aspect ratio; inactive banners stay hidden; broken/no image hides stale editor and preview; map navigation reload remains usable.
  - Test-first: observe failing new Node regression, implement, observe passing test.
  - Checks: node tests/admin-mapa.test.js; node tests/graficos-multi-series.test.js; node tests/operator-flow.test.js; node tests/scanner-integrity.test.js; node tests/index-orchestration.test.js; isolated browser if available; git diff --check.
  - Progress: confirmed Bootstrap d-flex overrides inline hiding; saved-image error has no handler. Historical nested section bug is absent.
  - Commit: 2378a87 (156 insertions + 16 deletions = 172 authored lines).
  - Verification: new regression observed RED then GREEN; all five Node suites and git diff --check passed. Writer and independent verifier passed isolated Chromium saved/cached/broken/absent/re-entry and banner transitions at 1280/768/360 widths. Parent reran targeted regression successfully.
  - Limitations: production authenticated/full-CDN integration not tested. Native risk assessment unassessable because unrelated untracked inventory; conservatively treated as high, independent verifier completed with no serious findings. No RDD lifecycle started (global OFF).
  - Review: disabled/unmanaged; assess risk after implementation for independent verification routing only.

## Next step
User can verify in their real admin session. No push or deployment performed; local work-unit commit complete.

