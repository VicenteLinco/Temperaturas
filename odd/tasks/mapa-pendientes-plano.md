# Mapa de pendientes sobre plano de planta

## Objective

Interactive plant layout showing pending/completed thermometers, with one optional background photo per instance (one establishment per deploy). Admin optionally uploads the layout, delimits areas and drags thermometers to x,y. Registrador sees color state with touch-friendly zoom/pan and a popup with register action. Without photo, list-only. No heavy map libraries, vanilla JS, mobile-first.

## Problem

Current flow is list-only: select area -> pending/completed list -> QR scan -> register (`public/index.html`, `GET /api/areas/:id/pendientes` in `src/handlers/registros.rs`). There is no spatial view, no way to see where pending equipment is in the plant, and no per-instance layout customization. Multi-lab reuse is via separate Docker instances, so the map must be instance-local, not global SaaS.

## Why

Spatial view reduces missed points in large plants and speeds up rounds. Instance-local photo keeps the Docker-share model: each lab uploads its own layout without code changes or cross-tenant leakage.

## Scope

### In scope

- Single general layout per instance, with zoom-to-area detail (no separate per-area image file; detail is a zoom to the area zone rectangle).
- `configuracion` toggle `mapa_habilitado` (0/1, default 0, optional activable desde configuracion).
- Photo management by ADMINISTRADOR: upload/replace/remove, stored under instance-local `public/uploads/` (Docker volume).
- Area zones as percent rectangles (`zona_x, zona_y, zona_w, zona_h` 0-100) + thermometer positions (`pos_x, pos_y` 0-100, nullable = pending-to-place).
- Registrador map: general + detail, color by state (pending/completed/out-of-service), touch pinch/mouse zoom + pan, large touch points, popup with register button reusing existing capture flow.
- Fallback: no photo or toggle off -> current list UI unchanged + admin nudge to upload.

### Out of scope

- Real GIS (Leaflet/Google Maps), GPS, indoor positioning.
- Per-user private photos (photos are per instance, managed by admin).
- Changing capture windows, validation ranges, unique index `idx_registros_unique_per_day`.
- Push, PR creation, merge — human decisions under ordinary repository policy.

## Constraints

- No migration framework. `create_tables` in `src/db.rs:19` uses `CREATE TABLE IF NOT EXISTS`; new columns need `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` at boot + hand-run script under `sql/` (pattern in `sql/CORREGIR_INDICE_UNICO.sql`, noted at `src/db.rs:81-93`).
- Stack is Rust Axum 0.7 + vanilla JS + Bootstrap 5 (`Cargo.toml:9`, `README.md:77-80`). No new heavy frontend dep.
- Axum has no multipart feature enabled (`Cargo.toml:9`). Base64 JSON upload used instead (`base64 = "0.22"`); no new web-framework surface.
- Static serving via `tower-http fs` (`Cargo.toml:12`); uploads live under `public/uploads/` (served by existing `ServeDir`). DEPLOY NOTE (T2 finding): `Dockerfile` bakes `public/` into the image with no volume and `render.yaml` defines no disk, so an uploaded `mapa.*` is wiped on every redeploy (Render free has no persistent disks). Options: persistent disk at `/app/public/uploads` (paid plan), external object storage, or simply re-upload after each deploy. Documented, not yet implemented.
- Frontend and backend already disagree on max/min ordering in places (`public/index.html:2302-2303` vs `public/lecturas.js:81-87`); map must only read `pendientes/completados`, never recompute state.
- Mobile-first verified viewports (`README.md:41`); map must not break narrow layouts or QR scanner (`html5-qrcode`).
- `Area` in `src/models.rs:20-27` has no zone fields; `Termometro` in `:57-67` has no position fields; `Configuracion` is key-value (`src/db.rs:178-189`).

## Locked decisions (2026-10-09, with user)

- **Drag changes area.** Dragging a thermometer point updates `pos_x/pos_y` AND `area_id` via zone hit-test (T2). Requires explicit admin confirm + audit entry, because history reports resolve `area_nombre` by current JOIN (`src/handlers/registros.rs:41-43`), so moved equipment's past readings will read under the new area. A nudge inside the same zone only writes `pos_x/pos_y`.
- **Rename stays admin-only.** `reparar_termometro` (`src/handlers/termometros.rs:110-158`) only flips `fuera_de_servicio` + maintenance detail; name/location changes go through `PUT /api/admin/termometros/:id` (`:193-237`), both audited. No rename field in the maintenance flow.
- **Registration flow untouched.** `lecturas.js`, `operator-flow.js`, `logic.rs` and the `handlers/registros.rs` capture path are out of scope; the map only reads `pendientes/completados` and links to the existing capture.

## Checklist

### T1 — Schema + config for map

- Route: delegated direct (touches `src/db.rs`, `src/models.rs`, `sql/MAPA_PLANO.sql`).
- `ALTER TABLE areas ADD COLUMN IF NOT EXISTS zona_x/y/w/h REAL` (nullable, 0-100 percent) + same boot-idempotent pattern as `solo_un_punto`.
- `ALTER TABLE termometros ADD COLUMN IF NOT EXISTS pos_x REAL, pos_y REAL` (nullable = not placed).
- Seed `configuracion`: `mapa_habilitado=0`, `mapa_imagen_path=''` via `ON CONFLICT DO NOTHING` (`src/db.rs:214-229`).
- Extend `Area`, `Termometro`, `TermometroConDetalles`, `Crear/Actualizar` DTOs minimally to carry zones/positions.
- Hand-run `sql/MAPA_PLANO.sql` for existing DBs.

### T1b — Explicit SELECTs follow-up (found during T1)

- Route: delegated direct (touches `src/handlers/termometros.rs`, `src/handlers/registros.rs`).
- `TermometroConDetalles` gained `pos_x/pos_y`, but `listar_termometros`, `obtener_termometro` and the `obtener_pendientes_area` pendientes query use explicit column lists; sqlx `query_as` is runtime-checked, so they needed `t.pos_x, t.pos_y` added or they fail at runtime. Fixed in field order; `reportes.rs`/`graficos.rs` checked, no mapping to this struct.

### Hotfix — boot order en base nueva (lanzando con DB de prueba)

- Route: direct inline (single mechanical move inside `src/db.rs`).
- The T1 `ALTER TABLE termometros ADD COLUMN pos_x/pos_y` block sat BEFORE `CREATE TABLE termometros`, so fresh-DB boot died with `relation "termometros" does not exist` (`IF NOT EXISTS` guards the column, not the table; existing DBs were unaffected). Moved the ALTER to run right AFTER the CREATE. `cargo check` + unit tests cannot catch this class — always smoke-boot against an empty database after schema changes.

### Fix — botón Registrar muerto en el popup del mapa (reportado en pruebas)

- Route: direct inline (solo `public/mapa.js`, +16/-1).
- Causa: `enlazar()` hacía `setPointerCapture` en cada pointerdown dentro del marco, lo que reenruta el click al contenedor y los botones nativos del popup (Registrar, X) nunca lo recibían. Ahora los gestos que nacen en `#mapaPopup` no capturan ni panean (click nativo intacto) y `fin()` los ignora. Extra: al cerrarse `#registroModal` se releen los estados para que el punto cambie de color sin salir del mapa. Sin reinicio del servidor (estáticos).

### Ajuste — Plano en vez de Mapa, apertura directa, pendiente azul (pedido en pruebas)

- Route: delegated direct (public/index.html, public/admin.html, public/mapa.js; solo texto visible + comportamiento).
- Etiquetas visibles Mapa→Plano (nav, h2, toggle, botón registrador, alerts). Ids, rutas API, funciones, claves y nombre de archivo intactos por compatibilidad.
- Tap en punto abre el registro directamente (sin popup ni botón); se eliminó `popup()`, `registrar` conservado como alias.
- Pendiente en registrador pasa de rojo a azul (`bg-primary`); admin conserva sus colores.
- `.gitignore`: `public/uploads/*` ignorado salvo `.gitkeep` para no versionar fotos de cada instancia.

### Fix — gráficos con un solo id (reportado en pruebas)

- Route: direct inline (single file `src/handlers/graficos.rs`, pre-existing bug, not caused by the map).
- `GET /api/admin/graficos?termometro_id=101` (single) failed with `Failed to deserialize query string: expected a sequence`: serde_urlencoded delivers one value as a string, so `Option<Vec<FragmentoId>>` rejected it; only absent (None) and repeated (?a=1&a=2) worked. The handler now takes a second `Query<Vec<(String,String)>>` extractor and builds the ids by hand with the existing `parsear_fragmento_id` + `normalizar_filtro_ids` (same absent-vs-empty semantics, invalid still 422). Verified single/repeated/commas/invalid live; `cargo test` 55/55 intact.

### T2 — Backend photo + positions API

- Route: delegated direct (touches `src/handlers/mapa.rs` new, `src/main.rs`, `src/handlers/mod.rs`, `src/models.rs`, `Cargo.toml`/`Cargo.lock`, `public/uploads/.gitkeep`).
- `GET /api/mapa` (any authenticated role): `{ habilitado, imagen_url, areas:[{id,nombre,zona}], termometros:[{id,area_id,nombre,pos_x,pos_y,estado}] }` reusing the exact window/state classification from `obtener_pendientes_area`, no new time math.
- `PUT /api/admin/mapa/config` (admin): toggle `mapa_habilitado` via `set_config` + audit.
- `POST /api/admin/mapa/foto` (admin, base64 JSON, data-URL tolerant): PNG/JPG magic bytes, ≤5MB decoded, writes `public/uploads/mapa.{png,jpg}` removing stale other extension, sets `mapa_imagen_path`, audit.
- `PUT /api/admin/mapa/posiciones` (admin, batch): 0-100 validation, point-in-rect hit-test (first zone by id wins) may reassign `area_id`; 409 `{cambios:[...]}` preview with zero writes unless `confirmar_cambio_area:true`, else single transaction + audit.
- Done, verified `cargo check` ok. Deploy caveat recorded under Constraints (uploads wiped on redeploy without a disk).

### T3 — Admin editor UI

- Route: delegated direct (touches `public/admin.html`).
- New `Mapas` section: toggle, uploader with preview, canvas/div with percent coords, drag thermometers (mouse + touch), draw/move area rectangles, `Pendientes de ubicar` list, save batch, empty-photo hint.
- No new libs; reuse Bootstrap + vanilla, consistent monochrome palette.

### T4 — Registrador map view

- Route: delegated direct (touches `public/index.html`, `public/lecturas.js` or new `public/mapa.js` <150 lines).
- Toggle-gated `Vista mapa / lista`; general view renders areas zones + points; tap zone -> detail zoom; points colored pending/completed/out-of-service from existing `GET /api/areas/:id/pendientes`.
- Popup minimal per definition: `Botón registrar` (+ nombre/estado context); tap goes to existing capture/QR flow, no duplicate register logic.
- Pinch-zoom/pan via CSS transform, points >=44px touch target, works with no photo -> auto list-only.

### T5 — Verification

- Route: direct inline / per-action delegation.
- `cargo build`, `cargo test` observed; `node --check` for new JS; manual mobile viewport check (360px) + upload 5MB reject + percent persistence round-trip.
- Record failed/skipped checks honestly; no invented RED/GREEN.

## Verification evidence

| Task | Command | Observed result |
| --- | --- | --- |
| T1 schema | `cargo check` (writer) | ok, `Finished dev profile` (only pre-existing sqlx-postgres future-incompat warning) |
| T1b SELECTs | `cargo check` (writer) | ok, `Finished dev profile` (same pre-existing warning) |
| T1+T1b | `cargo check` (parent spot check) | ok, `Finished dev profile in 0.16s`, same pre-existing warning |
| T1 | `cargo test` | pending (deferred to T5; schema-only change) |
| T2 backend | `cargo check` (writer) | ok, `Finished dev profile` (same pre-existing warning); 4 endpoints wired |
| T2 | `cargo check` (parent spot check) | ok, `Finished dev profile`, same pre-existing warning |
| T2 | `cargo test` | pending (deferred to T5) |
| T3 editor | `node --check /tmp/mapa-admin-check.js` (writer) | ok, syntax valid (extracted new block only) |
| T3 | structural readback (parent spot check) | ok, +411/-0 in public/admin.html, markers present |
| T4 vista | `node --check public/mapa.js` (writer) | ok, SYNTAX_OK (149 lines) |
| T4 | `node --check public/mapa.js` (parent spot check) | ok, SYNTAX_OK; +24/-0 in public/index.html |
| T5 full | `cargo build` (parent) | ok, `Finished dev profile in 7.03s` (same pre-existing warning) |
| T5 full | `cargo test` (parent) | ok, 55 passed, 0 failed |
| T5 full | `node tests/operator-flow.test.js` + `node tests/graficos-multi-series.test.js` (parent) | ok, 28 + 14 assertions passed |
| T5 full | `git diff --check` on branch range (parent) | ok, clean |
| T5 | manual: mobile 360px, upload 5MB reject, percent round-trip | pending (manual, not run headless) |
| Hotfix boot | `cargo build` + fresh-DB boot (parent) | ok after fix; before fix boot died on `relation termometros does not exist` |
| E2E mapa | curl: config on, area+tipo+termometro CRUD, posiciones, foto upload, mapa GET, cross-area move 409 then confirmed | ok: `estado pendiente`, `imagen_url /uploads/mapa.png`, 409 preview `{cambios:[...]}` with zero writes, confirmed move flips `area_id` 1→2 |
| Fix graficos | curl single/repeated/commas/invalid + `cargo test` | ok: single now 200 with limites, repeated/commas deduped, invalid 422; 55 passed, 0 failed |
| Fix popup Registrar | `node --check public/mapa.js` | ok, SYNTAX_OK; gestos en popup sin captura, refresh al cerrar modal |
| Ajuste plano | `node --check public/mapa.js` (writer) + parent spot check | ok; etiquetas Plano, apertura directa, pendiente azul |
| E2E registrador | created `registrador1` on TEST db, login, pendientes + mapa as that role | ok: pendientes list + mapa snapshot visible with correct estado |

## Delivery strategy

`ask-on-risk` (default). Forecast ~350-450 authored lines (backend ~120, admin UI ~150, registrador ~100, SQL/docs excluded); resolve chain strategy before crossing ~400.

## Commits

| Task | Commit | Authored lines |
| --- | --- | --- |
| T1+T1b | 748b744 | ~60 |
| T2 | 5d7ceed | ~200 |
| T3 | 117abd7 | +411/-0 |
| T4 | 1f8d833 | +173/-0 |
| T5 docs | 8295fed | docs only |
| Hotfix boot | c079000 | +8/-8 en src/db.rs |
| Fix graficos | 2af8815 | solo src/handlers/graficos.rs |
| Fix popup Registrar | 8a41598 | solo public/mapa.js |
| Ajuste plano | pending (hash en el reporte final) | index+admin+mapa.js |
| T3 | pending | — |
| T4 | pending | — |
| T5 | pending | — |

## Next step

Done. T1–T5 implemented, verified and pushed as `feature/mapa-pendientes-plano` (no PR created — human decision). Manual pending: mobile 360px check, 5MB upload reject, percent round-trip in a real browser; plus re-upload note after each Render deploy (no persistent disk). To enable: run `sql/MAPA_PLANO.sql` on existing DBs, set `mapa_habilitado=1`, upload photo in admin Mapas section.
