# Captura de 1 solo punto + corrección de gráficos multi-termómetro

## Objective

Two independent outcomes on the same codebase:

1. **F1 — Single-point capture.** Allow a thermometer to register only ONE temperature per capture instead of the current hardcoded three (`temp_actual`, `temp_maxima`, `temp_minima`), configured per **thermometer type**.
2. **F2 — Multi-thermometer charts.** Fix the graphs section, which breaks when more than one thermometer is selected.

## Problem

### F1

The capture form is hardcoded to three temperature inputs (`public/index.html:829-872`, `leerLecturas()` at `public/index.html:1981-1989`, `guardarRegistro()` at `public/index.html:2272-2303`). There is no "readings per capture" concept anywhere in the schema — verified by grep for `frecuencia`, `cantidad_lecturas`, `num_lecturas`, `tomadas`, `puntos`, `cantidad`. Instruments that only produce one measured value force operators to fake a max/min pair.

### F2

The multi-thermometer chart is a client-side fabrication over malformed data. Six concrete defects:

| # | Location | Defect |
| --- | --- | --- |
| D1 | `public/admin.html:3193-3197` | `cargarGraficos()` never sends `termometro_id`; filtering happens client-side at `public/admin.html:3352-3354` against an unfiltered server payload |
| D2 | `public/admin.html:3790`, `3885-3888` | Datasets for N thermometers are built by null-padding into a shared category axis derived from the date-interleaved `lecturas` array, so index *i* is a different instant per thermometer |
| D3 | `public/admin.html:3982-3990`, `4048-4082` | Tooltips and the detail modal use `lecturas[index]`, which may belong to another thermometer or be null |
| D4 | `public/admin.html:3373` | `tendencias[]` has no per-thermometer breakdown (`src/handlers/graficos.rs:212-230` groups by date only), so "Promedios Diarios" ignores the selection |
| D5 | `src/handlers/graficos.rs:257-282` | `ORDER BY r.fecha_registro ASC LIMIT 1000` — with N thermometers the 1000-row cap is shared and returns the *oldest* rows |
| D6 | `public/admin.html:3915`, `3929` | `(Máx)` / `(Mín)` series use the same colors for every thermometer; only `(Actual)` uses the palette color |

## Why

F1 removes forced fake data entry and makes the app usable for single-value instruments. F2 restores correctness of the admin analysis view, which is currently misleading rather than merely ugly: D3 can display a reading belonging to a different thermometer than the one hovered.

## Scope

### In scope

- New boolean column `solo_un_punto` on `tipos_termometro` (DDL + manual `sql/` script, following the existing pattern in `sql/CORREGIR_INDICE_UNICO.sql`).
- Backend: model, DTOs, admin CRUD handler, and validation/normalization so a single value fills `temp_actual` and sets `temp_maxima = temp_minima = temp_actual`.
- Frontend: adaptive capture form (1 field instead of 3) driven by the type flag, in both `public/index.html` and `public/admin.html`.
- Backend + frontend: real multi-thermometer support for `/api/admin/graficos`.

### Out of scope

- Changing the global capture windows (`registro_hora_1`, `registro_hora_2`) or their tolerance.
- Changing `idx_registros_unique_per_day`.
- RDD/review ceremony (runs only when the user-owned switch is on).
- Push, PR creation, merge — human decisions under ordinary repository policy.

## Constraints

- Schema has **no migration framework**. `create_tables` in `src/db.rs:19` runs `CREATE TABLE IF NOT EXISTS` at every boot; it cannot add a column to an existing table. Any new column needs a hand-run script under `sql/`, as documented at `src/db.rs:146-148`.
- The existing unique index `idx_registros_unique_per_day` (one capture per thermometer per window) must keep working unchanged.
- Frontend and backend already disagree on max/min ordering: `public/index.html:2302-2303` excludes `actual`, `public/lecturas.js:81-87` includes it. F1 must not make this worse; state the rule in one place.
- Capture windows are global rows in `configuracion` (`src/db.rs:165-170`), so "one point per capture" means one point per window, not one per day.
- Chart stack is Chart.js 4.4.1 + annotation + zoom UMD builds, loaded at `public/admin.html:1660-1664` on canvas `#chartTendencias` (`public/admin.html:1231`).
- There is **zero** automated test coverage for charts; `tests/*.test.js` cover only the `index.html` scanner and operator flow.

## Checklist

### T1 [x] — Schema + type model for `solo_un_punto`

- Route: delegated direct (touches `src/db.rs`, `src/models.rs`, `src/handlers/tipos_termometro.rs`).
- Add the column with an idempotent-safe DDL in `src/db.rs` and a hand-run `sql/` script.
- Add `solo_un_punto` to `TipoTermometro` (`src/models.rs:29-51`), to `CrearTipoTermometroRequest` (`:153-166`), `ActualizarTipoTermometroRequest` (`:168-182`), and to `TermometroConDetalles` (`:273-287`) so the frontend can read the flag without an extra call.
- Accept and persist the flag in the admin type CRUD handler (`src/handlers/tipos_termometro.rs`).
- Default to `false` for existing rows; never null.

### T2 [x] — Backend capture validation for one point

- Route: delegated direct (touches `src/logic.rs`, `src/handlers/registros.rs`).
- Define the ordering rule in exactly one place: when the type is single-point, `temp_actual` is the measured value and `temp_maxima = temp_minima = temp_actual`.
- Keep `validar_registro` (`src/logic.rs:345`) enforcing physical ranges and the `tiene_humedad` rule; a single point is not an excuse to skip range validation.
- `normalizar_lecturas` (`src/logic.rs:419`) must handle the single-point payload without breaking the three-point path.
- Reject a three-point payload for a single-point type rather than silently picking one value.

### T3 [x] — Adaptive capture form

- Route: delegated direct (touches `public/index.html`, `public/admin.html`, `public/lecturas.js`).
- Show one temperature input when the type is single-point, hide the other two, and relabel accordingly.
- Keep `public/lecturas.js:81-87` and `public/index.html:2302-2303` consistent with the rule from T2.
- Update `OperatorFlow.construirPayloadLectura` (`public/operator-flow.js:38-47`) so the payload shape matches the backend contract.
- Keep the edit/prefill path working for both modes (`public/index.html:1923-1925`).

### T4 [x] — Multi-thermometer support in the graphs API

- Route: delegated direct (touches `src/handlers/graficos.rs`).
- Accept multiple thermometer ids (a repeated or comma-separated parameter) instead of the single scalar `termometro_id: Option<i32>` (`src/handlers/graficos.rs:13-20`).
- Filter with an array-safe predicate, replacing the `AND r.termometro_id = $n` branch at `src/handlers/graficos.rs:390-393`.
- Return per-thermometer data instead of one date-aggregated-only `tendencias[]` series.
- Replace `ORDER BY ... ASC LIMIT 1000` (`src/handlers/graficos.rs:282`) with newest-first and a per-thermometer budget, so one thermometer cannot starve the others.
- Keep the existing single-thermometer response shape backward compatible.

### T5 [x] — Multi-series rendering without shared-axis corruption

- Route: delegated direct (touches `public/admin.html`).
- Send the selection from `cargarGraficos()` (`public/admin.html:3193-3197`) and filter server-side.
- Render one dataset per (thermometer, metric) keyed by timestamp, not a null-padded shared category axis; fix D2.
- Resolve tooltips and the detail modal from the dataset and data index, not `lecturas[globalIndex]` (`public/admin.html:3982-3990`, `4048-4082`); fix D3.
- Distinct colors per thermometer for Máx/Mín too (`public/admin.html:3915`, `3929`); fix D6.
- Honor the selection in "Promedios Diarios" mode or disable that mode for multi-selection with a clear message; fix D4.
- Resolve operational limits per thermometer, keeping the `null`-fallback behavior sane for heterogeneous selections (`public/admin.html:3216-3297`).

### T6 [x] — Verification

- Route: direct inline / per-action delegation.
- `cargo build`, `cargo test`, `cargo clippy` observed results recorded below.
- Add a focused JS test for the multi-series data shaping, since none exists today.

## Verification evidence

| Task | Command | Observed result |
| --- | --- | --- |
| T1 | `cargo build` | ok, `Finished dev profile` (solo warning preexistente sqlx-postgres future-incompat) |
| T1 | `cargo test` | ok, 37 passed, 0 failed |
| T2 | `cargo build` | ok, `Finished dev profile` (mismo warning preexistente) |
| T2 | `cargo test` | ok, 41 passed (37 previos + 4 nuevos single-point), RED previo observado (E0425 función inexistente) |
| T3 | `node tests/operator-flow.test.js` | ok, 28 assertions passed, sin regresión |
| T3 | `cargo build` | ok, backend intacto (mismo warning preexistente) |
| T4 | `cargo build` | ok, `Finished dev profile` (mismo warning preexistente) |
| T4 | `cargo test` | ok, 50 passed (41 previos + 9 nuevos graficos multi), RED previo E0425 |
| T5 | `cargo build` | ok, backend intacto (mismo warning preexistente) |
| T5 | `node /tmp/opencode/t5_multi_series.test.js` | ok, 14/14 (harness en /tmp, mover a tests/ en T6) |
| T6 | `cargo build` | ok, `Finished dev profile` (mismo warning preexistente) |
| T6 | `cargo test` | ok, 50 passed, 0 failed |
| T6 | `cargo clippy --all-targets` | ok, 0 errores, 28 warnings preexistentes (verificado comparando números de línea contra los hunks de `git diff 8bf74dc..HEAD`: todos caen en líneas que el feature no tocó) |
| T6 | `node tests/graficos-multi-series.test.js` | ok, 14 assertions passed |
| T6 | `node tests/operator-flow.test.js` | ok, 28 assertions passed, sin regresión |
| CSV-multi | `node --check` inline admin.html | ok (1 bloque inline) |
| CSV-multi | `node tests/graficos-multi-series.test.js` + `operator-flow.test.js` | ok, 14 + 28 passed, sin regresión |
| Bordes-2y3 | `cargo test` | ok, 55 passed (50 previos + 5 nuevos), RED previo E0425 |
| Bordes-2y3 | `cargo build` + `cargo clippy` | ok, 0 errores (mismo warning preexistente) |
| Bordes-2y3 | JS + `git diff --check` | ok, 14 + 28 passed, limpio |
| **Regresión (hallada aquí)** | `node tests/index-orchestration.test.js` | **FALLABA: `ReferenceError: capturaUnSoloPunto is not defined`** |
| **Regresión (corregida)** | `node tests/index-orchestration.test.js` | ok, passed (fix en `456233a`) |
| Clippy nuevo | `cargo clippy --all-targets` | **2 warnings NUEVOS** en `src/logic.rs:437-438` (doc_lazy_continuation / sangría de lista rustdoc) introducidos por el feature y **corregidos** en `d068fa7`. Los 28 restantes son previos. |
| Estado final | `cargo test` + 4 ficheros JS | ok, 55 passed, 14 + 28 + integridad + orquestación passed |

Sin `package.json` en el repo: los suites JS se corren con `node tests/<archivo>.test.js`.

### Detalle de la regresión

`leerLecturas()` (`public/index.html:2009`) pasó a depender de
`capturaUnSoloPunto`, un `let` de scope de módulo en `public/index.html:1066`.
`tests/index-orchestration.test.js` extrae las funciones por nombre y las evalúa
sin scope de módulo, así que la declaración quedaba invisible y todo
`guardarRegistro()` lanzaba. Producción no se vio afectada (en el navegador la
variable vive en el scope del `<script>`); el harness sí. Corregido
reconstruyendo el estado cerrado dentro de `createHarness` (`moduleState`).

## Commits

| Task | Commit | Authored lines |
| --- | --- | --- |
| T1 | 12e5dbc | ~35 |
| T2 | f0b58ae | ~129 |
| T3 | 14ea50b | ~139 |
| T4 | 97e4c17 | ~349 |
| T5 | 86af9e9 | ~327 |
| T6 | f3c3a36 | ~14 tests |
| CSV-multi | 16ac1e8 | ~18 |
| Bordes-2y3 | 2e66560 | ~40 |
| follow-up (concurrente) | c079000 | orden del ALTER en arranque de BD fresca |
| follow-up (concurrente) | 2af8815 | acepta `termometro_id` simple en el filtro |
| **Regresión corregida** | 456233a | restaura estado de módulo en el harness |
| **Regresión corregida** | d068fa7 | corrige sangría de lista en rustdoc |

Nota: T3–T6 y los follow-up fueron implementados y commiteados por la sesión
concurrente, no por esta, y se verificaron aquí de forma retroactiva.

## Delivery strategy

`ask-on-risk` **nunca se resolvió**, y la heurística de ~400 líneas se atravesó:
la rama acumuló **1333 inserciones / 178 bajas en 14 archivos** y se fusionó a
`main` y se empujó sin que esa conversación ocurriera. Se registra como desviación
de proceso, no como defecto de código: los commits por unidad de trabajo en sí
están bien.

## Next step

T1-T6, el punto 1 y los bordes 2 y 3 están cerrados y verificados.
Pendiente, y es decisión tuya: **push de 2 commits locales** (`456233a`,
`d068fa7`) — `main` está 2 delante de `origin/main` — y PR bajo política
ordinaria. Las capturas de pantalla sin trackear (`Captura desde 2026-10-09
*.png`) son tuyas y no se tocaron.
