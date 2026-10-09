-- ============================================================================
-- Columna `solo_un_punto` en `tipos_termometro`
--
-- QUÉ
-- Nuevo booleano por tipo de termómetro: cuando está en TRUE la captura pide UNA
-- sola temperatura (`temp_actual`) en lugar de tres (`temp_actual` + `temp_maxima`
-- + `temp_minima`). En ese modo temp_maxima y temp_minima se derivan del valor
-- único. Sirve para equipos que solo entregan una medición y para los que hoy el
-- operador se ve obligado a inventarse el par máximo/mínimo.
--
-- POR QUÉ UN SCRIPT A MANO
-- src/db.rs crea las tablas con `CREATE TABLE IF NOT EXISTS`, que NO añade
-- columnas a una tabla ya existente. El arranque de la aplicación aplica además
-- un `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` idempotente, así que la app no
-- depende de este script: queda para adelantar la migración sin desplegar, o
-- para revisarla a mano antes.
--
-- El script es idempotente de principio a fin: se puede ejecutar las veces que
-- haga falta sin romper nada.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- PASO 1 (escribe, idempotente). Añadir la columna.
--
-- `ADD COLUMN IF NOT EXISTS` existe desde PostgreSQL 9.6. Con el DEFAULT FALSE
-- (constante no volátil) PostgreSQL 11+ no reescribe la tabla: es un cambio solo
-- de catálogo, aunque `tipos_termometro` tenga muchas filas.
--
-- Las filas que ya existan quedan en FALSE: ningún termómetro cambia de
-- comportamiento hasta que un administrador active el modo en el tipo.
-- ----------------------------------------------------------------------------
ALTER TABLE tipos_termometro
    ADD COLUMN IF NOT EXISTS solo_un_punto BOOLEAN NOT NULL DEFAULT FALSE;


-- ----------------------------------------------------------------------------
-- PASO 2 (verificación, solo lectura).
--
-- Debe listar la columna `solo_un_punto` con todas las filas en false. Si alguna
-- sale en true, es que alguien activó el modo en algún tipo.
-- ----------------------------------------------------------------------------
SELECT
    tt.id,
    tt.nombre,
    tt.solo_un_punto,
    COUNT(t.id) AS termometros_que_usan_el_tipo
FROM tipos_termometro tt
LEFT JOIN termometros t ON t.tipo_id = tt.id
GROUP BY tt.id, tt.nombre, tt.solo_un_punto
ORDER BY tt.nombre;


-- ----------------------------------------------------------------------------
-- PASO 3 (solo informativo, opcional). Ver la definición vigente de la columna.
-- ----------------------------------------------------------------------------
SELECT column_name, data_type, is_nullable, column_default
FROM information_schema.columns
WHERE table_name = 'tipos_termometro'
  AND column_name = 'solo_un_punto';
