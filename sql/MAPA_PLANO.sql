-- ============================================================================
-- Columnas del mapa del plano: zonas de área y posiciones de termómetros
--
-- QUÉ
-- Zonas de área como rectángulos en porcentaje (`zona_x, zona_y, zona_w, zona_h`
-- 0-100, NULL = sin delimitar) + posición del punto de cada termómetro
-- (`pos_x, pos_y` 0-100, NULL = pendiente de ubicar) + interruptor y ruta de la
-- foto en `configuracion` (`mapa_habilitado=0`, `mapa_imagen_path=''`).
--
-- POR QUÉ UN SCRIPT A MANO
-- src/db.rs crea las tablas con `CREATE TABLE IF NOT EXISTS`, que NO añade
-- columnas a una tabla ya existente. El arranque de la aplicación aplica además
-- los `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` idempotentes, así que la app no
-- depende de este script: queda para adelantar la migración sin desplegar, o
-- para revisarla a mano antes (mismo patrón que sql/AGREGAR_SOLO_UN_PUNTO.sql).
--
-- El script es idempotente de principio a fin: se puede ejecutar las veces que
-- haga falta sin romper nada. Solo añade columnas NULL, sin comportamiento: el
-- arrastre que mueve el punto Y cambia el área vía hit-test de zonas llega en T2.
-- ============================================================================


-- ----------------------------------------------------------------------------
-- PASO 1 (escribe, idempotente). Añadir las columnas de zona a `areas`.
--
-- `ADD COLUMN IF NOT EXISTS` existe desde PostgreSQL 9.6. Al ser REAL nullable
-- sin DEFAULT, PostgreSQL 11+ no reescribe la tabla: es un cambio solo de
-- catálogo, aunque `areas` tenga muchas filas.
--
-- Las filas que ya existan quedan en NULL: ninguna área se dibuja en el mapa
-- hasta que un administrador delimite su zona.
-- ----------------------------------------------------------------------------
ALTER TABLE areas
    ADD COLUMN IF NOT EXISTS zona_x REAL,
    ADD COLUMN IF NOT EXISTS zona_y REAL,
    ADD COLUMN IF NOT EXISTS zona_w REAL,
    ADD COLUMN IF NOT EXISTS zona_h REAL;


-- ----------------------------------------------------------------------------
-- PASO 2 (escribe, idempotente). Añadir la posición a `termometros`.
--
-- Las filas que ya existan quedan en NULL: todos los puntos salen en la lista
-- de "Pendientes de ubicar" hasta que un administrador los arrastre al plano.
-- ----------------------------------------------------------------------------
ALTER TABLE termometros
    ADD COLUMN IF NOT EXISTS pos_x REAL,
    ADD COLUMN IF NOT EXISTS pos_y REAL;


-- ----------------------------------------------------------------------------
-- PASO 3 (escribe, idempotente). Sembrar la configuración del mapa.
--
-- `mapa_habilitado` en '0': sin foto o con el interruptor apagado la interfaz
-- sigue en modo lista, igual que hoy. `ON CONFLICT DO NOTHING` respeta el valor
-- que el administrador ya haya guardado.
-- ----------------------------------------------------------------------------
INSERT INTO configuracion (clave, valor, descripcion)
VALUES
    ('mapa_habilitado', '0', 'Muestra la vista de mapa sobre el plano (0=No, 1=Sí)'),
    ('mapa_imagen_path', '', 'Ruta de la foto del plano dentro de public/uploads/')
ON CONFLICT (clave) DO NOTHING;


-- ----------------------------------------------------------------------------
-- PASO 4 (verificación, solo lectura).
--
-- Debe listar las seis columnas (cuatro en `areas`, dos en `termometros`) y las
-- dos claves de configuración con sus valores vigentes.
-- ----------------------------------------------------------------------------
SELECT column_name, data_type, is_nullable, column_default
FROM information_schema.columns
WHERE (table_name = 'areas' AND column_name IN ('zona_x', 'zona_y', 'zona_w', 'zona_h'))
   OR (table_name = 'termometros' AND column_name IN ('pos_x', 'pos_y'))
ORDER BY table_name, column_name;

SELECT clave, valor
FROM configuracion
WHERE clave IN ('mapa_habilitado', 'mapa_imagen_path')
ORDER BY clave;
