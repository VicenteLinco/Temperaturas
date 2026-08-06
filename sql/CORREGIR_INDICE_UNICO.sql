-- ============================================================================
-- Corrección del índice único de `registros`
--
-- PROBLEMA
-- El índice vigente en producción agrupa por el día en UTC:
--   idx_registros_unique_per_day (termometro_id, (fecha_registro AT TIME ZONE 'UTC')::date, ventana_horaria)
-- Chile es UTC-3/-4, así que ese "día" cambia a las 20:00 o 21:00 hora local, en
-- plena ronda nocturna. Consecuencia: el mismo equipo admite un segundo registro
-- de la misma ronda (uno antes de esa hora y otro después) y ya hay casos reales.
--
-- El arreglo no es solo cambiar UTC por Chile: la ronda nocturna va de las 20:00 a
-- las 08:00 y cruza medianoche igualmente. Hay que agrupar por DÍA OPERATIVO, el
-- que empieza a las 08:00 de Chile:  (fecha - 8 horas)::date
--
-- src/db.rs define el índice con America/Santiago, pero usa CREATE ... IF NOT EXISTS,
-- que NO redefine un índice existente. Por eso el arreglo nunca llegó a producción.
--
-- ESTE SCRIPT NO SE EJECUTA SOLO. Ejecuta los pasos en orden y revisa el paso 1
-- antes de tocar nada: el paso 3 fallará si quedan duplicados sin resolver.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- PASO 1 (solo lectura). Duplicados que impedirán crear el índice corregido.
-- Revisa esta lista ANTES de continuar. Si sale vacía, salta al paso 3.
-- ----------------------------------------------------------------------------
SELECT
    r.termometro_id,
    t.nombre                                                        AS termometro,
    r.ventana_horaria,
    ((r.fecha_registro AT TIME ZONE 'America/Santiago') - INTERVAL '8 hours')::date AS dia_operativo,
    COUNT(*)                                                        AS registros,
    array_agg(r.id ORDER BY r.fecha_registro)                       AS ids,
    array_agg(to_char(r.fecha_registro AT TIME ZONE 'America/Santiago', 'HH24:MI')
              ORDER BY r.fecha_registro)                            AS horas
FROM registros r
LEFT JOIN termometros t ON t.id = r.termometro_id
GROUP BY 1, 2, 3, 4
HAVING COUNT(*) > 1
ORDER BY dia_operativo DESC, r.termometro_id;


-- ----------------------------------------------------------------------------
-- PASO 2 (escribe). Resolver los duplicados encontrados.
--
-- Decide tú qué registro conservar. La consulta de abajo NO borra: marca en
-- observaciones los que se conservarían con el criterio "el más reciente de cada
-- grupo". Ejecútala para revisar, y solo entonces ejecuta el DELETE comentado.
--
-- Criterio propuesto: conservar el ÚLTIMO registro de cada grupo, que es el que
-- corrigió al anterior. Cámbialo si en tu operación vale el primero.
-- ----------------------------------------------------------------------------
WITH ordenados AS (
    SELECT
        id,
        termometro_id,
        ventana_horaria,
        ((fecha_registro AT TIME ZONE 'America/Santiago') - INTERVAL '8 hours')::date AS dia_operativo,
        ROW_NUMBER() OVER (
            PARTITION BY termometro_id,
                         ventana_horaria,
                         ((fecha_registro AT TIME ZONE 'America/Santiago') - INTERVAL '8 hours')::date
            ORDER BY fecha_registro DESC
        ) AS posicion
    FROM registros
)
SELECT
    o.id,
    o.termometro_id,
    o.ventana_horaria,
    o.dia_operativo,
    to_char(r.fecha_registro AT TIME ZONE 'America/Santiago', 'HH24:MI') AS hora,
    r.temp_actual,
    CASE WHEN o.posicion = 1 THEN 'SE CONSERVA' ELSE 'SE ELIMINARÍA' END AS accion
FROM ordenados o
JOIN registros r ON r.id = o.id
WHERE o.termometro_id IN (
        SELECT termometro_id FROM ordenados WHERE posicion > 1
      )
ORDER BY o.dia_operativo DESC, o.termometro_id, o.posicion;

-- Cuando estés conforme con la lista anterior, descomenta y ejecuta:
--
-- BEGIN;
--   DELETE FROM registros WHERE id IN (
--       SELECT id FROM (
--           SELECT id, ROW_NUMBER() OVER (
--               PARTITION BY termometro_id, ventana_horaria,
--                            ((fecha_registro AT TIME ZONE 'America/Santiago') - INTERVAL '8 hours')::date
--               ORDER BY fecha_registro DESC) AS posicion
--           FROM registros
--       ) x WHERE posicion > 1
--   );
--   -- Revisa el número de filas borradas antes de confirmar:
-- COMMIT;   -- o ROLLBACK; si no cuadra


-- ----------------------------------------------------------------------------
-- PASO 3 (escribe). Recrear el índice agrupando por día operativo (08:00 → 08:00).
-- Falla si el paso 2 no dejó los duplicados resueltos; en ese caso no se pierde
-- nada: la transacción se revierte y el índice antiguo sigue en pie.
-- ----------------------------------------------------------------------------
BEGIN;
    DROP INDEX IF EXISTS idx_registros_unique_per_day;

    CREATE UNIQUE INDEX idx_registros_unique_per_day
        ON registros (
            termometro_id,
            (((fecha_registro AT TIME ZONE 'America/Santiago') - INTERVAL '8 hours')::date),
            ventana_horaria
        );
COMMIT;


-- ----------------------------------------------------------------------------
-- PASO 4 (verificación, solo lectura). La definición debe decir America/Santiago.
-- ----------------------------------------------------------------------------
SELECT indexname, indexdef
FROM pg_indexes
WHERE tablename = 'registros'
  AND indexname = 'idx_registros_unique_per_day';
