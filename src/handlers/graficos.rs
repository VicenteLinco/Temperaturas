use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, FromRow, PgPool, Row};

use crate::auth::CurrentUser;

// ===== GRÁFICOS =====

#[derive(Deserialize, Debug)]
pub struct FiltrosGraficos {
    pub dias: Option<i64>,
    pub fecha_desde: Option<String>,
    pub fecha_hasta: Option<String>,
    pub area_id: Option<i32>,
    // Acepta un id suelto (?termometro_id=5), repetido
    // (?termometro_id=1&termometro_id=2) o separado por comas
    // (?termometro_id=1,2). El nombre del parámetro se conserva
    // para no romper a los clientes actuales.
    #[serde(default, deserialize_with = "deserializar_ids_termometros")]
    pub termometro_id: Option<Vec<i32>>,
}

// Presupuesto total histórico de lecturas (antes: LIMIT 1000 compartido).
const LIMITE_TOTAL_LECTURAS: i64 = 1000;
// Piso por termómetro para que una selección grande no deje series casi vacías.
const MIN_LECTURAS_POR_TERMOMETRO: i64 = 50;

/// Parte un fragmento "1, 2,3" en ids. Los trozos vacíos se ignoran para
/// tolerar "?termometro_id=" o comas colgadas ("7,"). Lo inválido da error,
/// igual que antes devolvía 422 un escalar no numérico.
fn parsear_fragmento_id(texto: &str) -> Result<Vec<i32>, String> {
    let mut ids = Vec::new();
    for parte in texto.split(',') {
        let recorte = parte.trim();
        if recorte.is_empty() {
            continue;
        }
        match recorte.parse::<i32>() {
            Ok(n) => ids.push(n),
            Err(_) => return Err(format!("termometro_id inválido: {:?}", recorte)),
        }
    }
    Ok(ids)
}

/// Quita duplicados manteniendo el orden de llegada.
/// No filtra valores inexistentes (<= 0): antes un id sin filas devolvía
/// vacío y convertirlo en "sin filtro" expondría todos los termómetros.
fn normalizar_ids_termometros(ids: Vec<i32>) -> Vec<i32> {
    let mut vistos = std::collections::HashSet::new();
    ids.into_iter().filter(|id| vistos.insert(*id)).collect()
}

/// Reparte el presupuesto total entre los termómetros pedidos para que uno
/// no hambree a los otros. Con un solo id devuelve el total histórico (1000).
fn limite_por_termometro(cantidad: usize) -> i32 {
    if cantidad == 0 {
        return LIMITE_TOTAL_LECTURAS as i32;
    }
    (LIMITE_TOTAL_LECTURAS / cantidad as i64)
        .clamp(MIN_LECTURAS_POR_TERMOMETRO, LIMITE_TOTAL_LECTURAS) as i32
}

/// Un valor del query para termometro_id: llega como cadena (urlencoded) y
/// puede traer varios ids separados por comas.
struct FragmentoId(Vec<i32>);

impl<'de> Deserialize<'de> for FragmentoId {
    fn deserialize<D>(deserializador: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Este filtro solo se usa en Query (urlencoded): todo llega como cadena.
        let texto = String::deserialize(deserializador)?;
        parsear_fragmento_id(&texto)
            .map(FragmentoId)
            .map_err(|e| <D::Error as serde::de::Error>::custom(e))
    }
}

fn deserializar_ids_termometros<'de, D>(
    deserializador: D,
) -> Result<Option<Vec<i32>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let fragmentos: Option<Vec<FragmentoId>> = Option::deserialize(deserializador)?;
    Ok(fragmentos
        .map(|fs| {
            normalizar_ids_termometros(fs.into_iter().flat_map(|f| f.0).collect())
        })
        .filter(|v| !v.is_empty()))
}

#[derive(Serialize, FromRow)]
pub struct ResumenGraficos {
    pub total_registros: i64,
    pub total_alertas: i64,
    pub promedio_max: Option<f32>,
    pub promedio_min: Option<f32>,
    pub promedio_actual: Option<f32>,
}

#[derive(Serialize)]
pub struct LimitesInfo {
    pub termometro_id: Option<i32>,
    pub termometro_nombre: Option<String>,
    pub area_nombre: Option<String>,
    pub temp_min_operativa: f32,
    pub temp_max_operativa: f32,
    pub hum_min_operativa: Option<f32>,
    pub hum_max_operativa: Option<f32>,
}

#[derive(Serialize)]
pub struct LecturaGraficoItem {
    pub id: i32,
    pub fecha_registro: String,
    pub termometro_id: i32,
    pub termometro_nombre: String,
    pub area_nombre: String,
    pub ventana_horaria: String,
    pub temp_actual: Option<f32>,
    pub temp_maxima: f32,
    pub temp_minima: f32,
    pub humedad: Option<f32>,
    pub fuera_rango_operativo: bool,
    pub temp_min_operativa: f32,
    pub temp_max_operativa: f32,
    pub observaciones: Option<String>,
}

#[derive(Serialize)]
pub struct TendenciasItem {
    pub fecha: String,
    pub promedio_max: Option<f32>,
    pub promedio_min: Option<f32>,
    pub promedio_actual: Option<f32>,
    pub alertas: i64,
    pub total_registros: i64,
    pub temp_min_operativa: Option<f32>,
    pub temp_max_operativa: Option<f32>,
}

#[derive(Serialize)]
pub struct TendenciaTermometroItem {
    pub termometro_id: i32,
    pub termometro_nombre: String,
    pub fecha: String,
    pub promedio_max: Option<f32>,
    pub promedio_min: Option<f32>,
    pub promedio_actual: Option<f32>,
    pub alertas: i64,
    pub total_registros: i64,
    pub temp_min_operativa: Option<f32>,
    pub temp_max_operativa: Option<f32>,
}

#[derive(Serialize)]
pub struct AreaResumen {
    pub area: String,
    pub total: i64,
    pub alertas: i64,
}

#[derive(Serialize)]
pub struct GraficosResponse {
    pub dias: i64,
    pub fecha_desde: Option<String>,
    pub fecha_hasta: Option<String>,
    pub area_id: Option<i32>,
    // Eco del filtro cuando se pide un solo termómetro (compat con clientes viejos).
    pub termometro_id: Option<i32>,
    // Eco normalizado del filtro multi (None = sin filtro por termómetro).
    pub termometro_ids: Option<Vec<i32>>,
    pub limites: Option<LimitesInfo>,
    pub resumen: ResumenGraficos,
    pub tendencias: Vec<TendenciasItem>,
    // Desglose por termómetro y día (vacío cuando no se filtra por termómetro).
    pub tendencias_por_termometro: Vec<TendenciaTermometroItem>,
    pub lecturas: Vec<LecturaGraficoItem>,
    pub areas: Vec<AreaResumen>,
}

// Columnas compartidas por ambas ramas de lecturas (ventana por termómetro
// o últimas globales) para no duplicar el SELECT.
const COLUMNAS_LECTURAS_GRAFICO: &str = r#"
    r.id,
    to_char(r.fecha_registro AT TIME ZONE 'America/Santiago', 'YYYY-MM-DD HH24:MI') AS fecha_registro,
    r.termometro_id,
    COALESCE(t.nombre, 'Termómetro #' || t.id) AS termometro_nombre,
    a.nombre AS area_nombre,
    r.ventana_horaria,
    r.temp_actual,
    r.temp_maxima,
    r.temp_minima,
    r.humedad,
    r.fuera_rango_operativo,
    tt.temp_min_operativa,
    tt.temp_max_operativa,
    r.observaciones
"#;

fn fila_a_lectura(row: &PgRow) -> LecturaGraficoItem {
    LecturaGraficoItem {
        id: row.get("id"),
        fecha_registro: row.get("fecha_registro"),
        termometro_id: row.get("termometro_id"),
        termometro_nombre: row.get("termometro_nombre"),
        area_nombre: row.get("area_nombre"),
        ventana_horaria: row.get("ventana_horaria"),
        temp_actual: row.get("temp_actual"),
        temp_maxima: row.get("temp_maxima"),
        temp_minima: row.get("temp_minima"),
        humedad: row.get("humedad"),
        fuera_rango_operativo: row.get("fuera_rango_operativo"),
        temp_min_operativa: row.get("temp_min_operativa"),
        temp_max_operativa: row.get("temp_max_operativa"),
        observaciones: row.get("observaciones"),
    }
}

pub async fn obtener_graficos(
    _current_user: CurrentUser,
    State(pool): State<PgPool>,
    Query(filtros): Query<FiltrosGraficos>,
) -> Result<Json<GraficosResponse>, StatusCode> {
    let dias = filtros.dias.unwrap_or(30).clamp(1, 365);
    let fecha_desde = filtros.fecha_desde.filter(|s| !s.trim().is_empty());
    let fecha_hasta = filtros.fecha_hasta.filter(|s| !s.trim().is_empty());
    // Filtro multi ya normalizado por el deserializador (repetido o comas).
    let ids: Option<Vec<i32>> = filtros.termometro_id.clone();
    // Eco single para clientes viejos: solo cuando se pide exactamente uno.
    let termometro_unico: Option<i32> = match &ids {
        Some(v) if v.len() == 1 => Some(v[0]),
        _ => None,
    };

    // 1. Obtener límites operativos si hay termómetro o área seleccionada.
    // Con varios termómetros no hay un único rango, así que limites queda en
    // None; cada lectura ya trae el suyo para el detalle por termómetro.
    let mut limites: Option<LimitesInfo> = None;
    if let Some(v) = ids.as_ref() {
        if v.len() == 1 {
            let t_id = v[0];
        let lim_row: Option<PgRow> = sqlx::query(
            r#"
            SELECT 
                t.id AS termometro_id,
                COALESCE(t.nombre, 'Termómetro #' || t.id) AS termometro_nombre,
                a.nombre AS area_nombre,
                tt.temp_min_operativa,
                tt.temp_max_operativa,
                tt.hum_min_operativa,
                tt.hum_max_operativa
            FROM termometros t
            JOIN tipos_termometro tt ON t.tipo_id = tt.id
            JOIN areas a ON t.area_id = a.id
            WHERE t.id = $1
            "#
        )
        .bind(t_id)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        if let Some(row) = lim_row {
            limites = Some(LimitesInfo {
                termometro_id: Some(row.get("termometro_id")),
                termometro_nombre: Some(row.get("termometro_nombre")),
                area_nombre: Some(row.get("area_nombre")),
                temp_min_operativa: row.get("temp_min_operativa"),
                temp_max_operativa: row.get("temp_max_operativa"),
                hum_min_operativa: row.get("hum_min_operativa"),
                hum_max_operativa: row.get("hum_max_operativa"),
            });
        }
        }
    } else if let Some(a_id) = filtros.area_id {
        let lim_row: Option<PgRow> = sqlx::query(
            r#"
            SELECT 
                a.nombre AS area_nombre,
                MIN(tt.temp_min_operativa) AS min_temp_min,
                MAX(tt.temp_min_operativa) AS max_temp_min,
                MIN(tt.temp_max_operativa) AS min_temp_max,
                MAX(tt.temp_max_operativa) AS max_temp_max,
                MIN(tt.hum_min_operativa) AS hum_min_operativa,
                MAX(tt.hum_max_operativa) AS hum_max_operativa
            FROM termometros t
            JOIN tipos_termometro tt ON t.tipo_id = tt.id
            JOIN areas a ON t.area_id = a.id
            WHERE a.id = $1
            GROUP BY a.nombre
            "#
        )
        .bind(a_id)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        if let Some(row) = lim_row {
            let min_tmin: Option<f32> = row.get("min_temp_min");
            let max_tmin: Option<f32> = row.get("max_temp_min");
            let min_tmax: Option<f32> = row.get("min_temp_max");
            let max_tmax: Option<f32> = row.get("max_temp_max");

            // Solo fijar límites a nivel de área si TODOS los termómetros del área tienen EXACTAMENTE los mismos límites operativos
            if let (Some(tn1), Some(tn2), Some(tx1), Some(tx2)) = (min_tmin, max_tmin, min_tmax, max_tmax) {
                if (tn1 - tn2).abs() < 0.01 && (tx1 - tx2).abs() < 0.01 {
                    limites = Some(LimitesInfo {
                        termometro_id: None,
                        termometro_nombre: None,
                        area_nombre: Some(row.get("area_nombre")),
                        temp_min_operativa: tn1,
                        temp_max_operativa: tx1,
                        hum_min_operativa: row.get("hum_min_operativa"),
                        hum_max_operativa: row.get("hum_max_operativa"),
                    });
                }
            }
        }
    }

    // 2. Construir cláusulas WHERE dinámicas
    // Usamos QueryBuilder para consultas de Sqlx
    let mut builder_resumen = sqlx::QueryBuilder::new(
        r#"
        SELECT
            COUNT(*)::bigint AS total_registros,
            COUNT(*) FILTER (WHERE r.fuera_rango_operativo)::bigint AS total_alertas,
            ROUND(AVG(r.temp_maxima)::numeric, 1)::real AS promedio_max,
            ROUND(AVG(r.temp_minima)::numeric, 1)::real AS promedio_min,
            ROUND(AVG(r.temp_actual)::numeric, 1)::real AS promedio_actual
        FROM registros r
        JOIN termometros t ON r.termometro_id = t.id
        WHERE 1=1
        "#
    );

    aplicar_filtros_where(&mut builder_resumen, &fecha_desde, &fecha_hasta, dias, filtros.area_id, ids.as_deref());

    let resumen: ResumenGraficos = builder_resumen
        .build_query_as()
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Error graficos resumen: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // 3. Tendencias diarias
    let mut builder_tendencias = sqlx::QueryBuilder::new(
        r#"
        SELECT
            (r.fecha_registro AT TIME ZONE 'America/Santiago')::date AS fecha,
            ROUND(AVG(r.temp_maxima)::numeric, 1)::real AS promedio_max,
            ROUND(AVG(r.temp_minima)::numeric, 1)::real AS promedio_min,
            ROUND(AVG(r.temp_actual)::numeric, 1)::real AS promedio_actual,
            COUNT(*) FILTER (WHERE r.fuera_rango_operativo)::bigint AS alertas,
            COUNT(*)::bigint AS total_registros,
            MIN(tt.temp_min_operativa) AS temp_min_operativa,
            MAX(tt.temp_max_operativa) AS temp_max_operativa
        FROM registros r
        JOIN termometros t ON r.termometro_id = t.id
        JOIN tipos_termometro tt ON t.tipo_id = tt.id
        WHERE 1=1
        "#
    );

    aplicar_filtros_where(&mut builder_tendencias, &fecha_desde, &fecha_hasta, dias, filtros.area_id, ids.as_deref());
    builder_tendencias.push(" GROUP BY fecha ORDER BY fecha ");

    let tend_rows: Vec<PgRow> = builder_tendencias
        .build()
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Error graficos tendencias: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let tendencias: Vec<TendenciasItem> = tend_rows
        .iter()
        .map(|row| TendenciasItem {
            fecha: row.get::<chrono::NaiveDate, _>("fecha").to_string(),
            promedio_max: row.get("promedio_max"),
            promedio_min: row.get("promedio_min"),
            promedio_actual: row.get("promedio_actual"),
            alertas: row.get("alertas"),
            total_registros: row.get("total_registros"),
            temp_min_operativa: row.get("temp_min_operativa"),
            temp_max_operativa: row.get("temp_max_operativa"),
        })
        .collect();

    // 3b. Tendencias por termómetro y día (desglose para multi; D4).
    // Se calcula solo cuando hay filtro por termómetro para acotar el
    // payload; `tendencias` sigue siendo el agregado global por compat.
    let tendencias_por_termometro: Vec<TendenciaTermometroItem> = if ids.is_some() {
        let mut builder_detalle = sqlx::QueryBuilder::new(
            r#"
            SELECT
                r.termometro_id,
                COALESCE(t.nombre, 'Termómetro #' || t.id) AS termometro_nombre,
                (r.fecha_registro AT TIME ZONE 'America/Santiago')::date AS fecha,
                ROUND(AVG(r.temp_maxima)::numeric, 1)::real AS promedio_max,
                ROUND(AVG(r.temp_minima)::numeric, 1)::real AS promedio_min,
                ROUND(AVG(r.temp_actual)::numeric, 1)::real AS promedio_actual,
                COUNT(*) FILTER (WHERE r.fuera_rango_operativo)::bigint AS alertas,
                COUNT(*)::bigint AS total_registros,
                MIN(tt.temp_min_operativa) AS temp_min_operativa,
                MAX(tt.temp_max_operativa) AS temp_max_operativa
            FROM registros r
            JOIN termometros t ON r.termometro_id = t.id
            JOIN tipos_termometro tt ON t.tipo_id = tt.id
            WHERE 1=1
            "#
        );

        aplicar_filtros_where(&mut builder_detalle, &fecha_desde, &fecha_hasta, dias, filtros.area_id, ids.as_deref());
        builder_detalle.push(" GROUP BY r.termometro_id, termometro_nombre, fecha ORDER BY r.termometro_id, fecha ");

        let detalle_rows: Vec<PgRow> = builder_detalle
            .build()
            .fetch_all(&pool)
            .await
            .map_err(|e| {
                tracing::error!("Error graficos tendencias por termómetro: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        detalle_rows
            .iter()
            .map(|row| TendenciaTermometroItem {
                termometro_id: row.get("termometro_id"),
                termometro_nombre: row.get("termometro_nombre"),
                fecha: row.get::<chrono::NaiveDate, _>("fecha").to_string(),
                promedio_max: row.get("promedio_max"),
                promedio_min: row.get("promedio_min"),
                promedio_actual: row.get("promedio_actual"),
                alertas: row.get("alertas"),
                total_registros: row.get("total_registros"),
                temp_min_operativa: row.get("temp_min_operativa"),
                temp_max_operativa: row.get("temp_max_operativa"),
            })
            .collect()
    } else {
        Vec::new()
    };

    // 4. Lecturas punto por punto (individuales).
    // Con filtro por termómetro: ventana por termómetro (ROW_NUMBER) con lo
    // más nuevo primero y presupuesto repartido, para que un termómetro no
    // hambree a los otros (D5). Sin filtro: las 1000 más nuevas.
    // En ambos casos la respuesta sale en orden cronológico ascendente.
    let lecturas: Vec<LecturaGraficoItem> = if let Some(pedidos) = ids.as_ref() {
        let por_termometro = limite_por_termometro(pedidos.len());
        let mut builder_lecturas = sqlx::QueryBuilder::new(
            r#"
            SELECT * FROM (
                SELECT
                "#
        );
        builder_lecturas.push(COLUMNAS_LECTURAS_GRAFICO);
        builder_lecturas.push(
            r#",
                    ROW_NUMBER() OVER (PARTITION BY r.termometro_id ORDER BY r.fecha_registro DESC) AS rn,
                    r.fecha_registro AS momento_orden
                FROM registros r
                JOIN termometros t ON r.termometro_id = t.id
                JOIN tipos_termometro tt ON t.tipo_id = tt.id
                JOIN areas a ON t.area_id = a.id
                WHERE 1=1
            "#
        );

        aplicar_filtros_where(&mut builder_lecturas, &fecha_desde, &fecha_hasta, dias, filtros.area_id, ids.as_deref());
        builder_lecturas.push(" ) s WHERE rn <= ");
        builder_lecturas.push_bind(por_termometro);
        builder_lecturas.push(" ORDER BY momento_orden ASC ");

        let lecturas_rows: Vec<PgRow> = builder_lecturas
            .build()
            .fetch_all(&pool)
            .await
            .map_err(|e| {
                tracing::error!("Error graficos lecturas: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        lecturas_rows.iter().map(fila_a_lectura).collect()
    } else {
        let mut builder_lecturas = sqlx::QueryBuilder::new(" SELECT ");
        builder_lecturas.push(COLUMNAS_LECTURAS_GRAFICO);
        builder_lecturas.push(
            r#"
            FROM registros r
            JOIN termometros t ON r.termometro_id = t.id
            JOIN tipos_termometro tt ON t.tipo_id = tt.id
            JOIN areas a ON t.area_id = a.id
            WHERE 1=1
            "#
        );

        aplicar_filtros_where(&mut builder_lecturas, &fecha_desde, &fecha_hasta, dias, filtros.area_id, ids.as_deref());
        builder_lecturas.push(" ORDER BY r.fecha_registro DESC LIMIT 1000 ");

        let lecturas_rows: Vec<PgRow> = builder_lecturas
            .build()
            .fetch_all(&pool)
            .await
            .map_err(|e| {
                tracing::error!("Error graficos lecturas: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let mut lecturas: Vec<LecturaGraficoItem> =
            lecturas_rows.iter().map(fila_a_lectura).collect();
        lecturas.reverse();
        lecturas
    };

    // 5. Resumen por área
    let mut builder_areas = sqlx::QueryBuilder::new(
        r#"
        SELECT
            a.nombre AS area,
            COUNT(*)::bigint AS total,
            COUNT(*) FILTER (WHERE r.fuera_rango_operativo)::bigint AS alertas
        FROM registros r
        JOIN termometros t ON r.termometro_id = t.id
        JOIN areas a ON t.area_id = a.id
        WHERE 1=1
        "#
    );

    aplicar_filtros_where(&mut builder_areas, &fecha_desde, &fecha_hasta, dias, filtros.area_id, ids.as_deref());
    builder_areas.push(" GROUP BY a.nombre ORDER BY a.nombre ");

    let area_rows: Vec<PgRow> = builder_areas
        .build()
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Error graficos áreas: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let areas: Vec<AreaResumen> = area_rows
        .iter()
        .map(|row| AreaResumen {
            area: row.get("area"),
            total: row.get("total"),
            alertas: row.get("alertas"),
        })
        .collect();

    Ok(Json(GraficosResponse {
        dias,
        fecha_desde,
        fecha_hasta,
        area_id: filtros.area_id,
        termometro_id: termometro_unico,
        termometro_ids: ids,
        limites,
        resumen,
        tendencias,
        tendencias_por_termometro,
        lecturas,
        areas,
    }))
}

fn aplicar_filtros_where<'a>(
    builder: &mut sqlx::QueryBuilder<'a, sqlx::Postgres>,
    fecha_desde: &'a Option<String>,
    fecha_hasta: &'a Option<String>,
    dias: i64,
    area_id: Option<i32>,
    termometro_ids: Option<&[i32]>,
) {
    if let Some(fd) = fecha_desde {
        builder.push(" AND (r.fecha_registro AT TIME ZONE 'America/Santiago')::date >= ");
        builder.push_bind(fd.clone());
        builder.push("::date ");
    }
    if let Some(fh) = fecha_hasta {
        builder.push(" AND (r.fecha_registro AT TIME ZONE 'America/Santiago')::date <= ");
        builder.push_bind(fh.clone());
        builder.push("::date ");
    }
    if fecha_desde.is_none() && fecha_hasta.is_none() {
        builder.push(" AND (r.fecha_registro AT TIME ZONE 'America/Santiago')::date >= ((CURRENT_TIMESTAMP AT TIME ZONE 'America/Santiago')::date - ");
        builder.push_bind(dias as i32);
        builder.push("::int) ");
    }

    if let Some(aid) = area_id {
        builder.push(" AND t.area_id = ");
        builder.push_bind(aid);
    }
    // Predicado array-safe: sin concatenar SQL crudo (mismo patrón ANY
    // que ya usa el repo en registros.rs).
    if let Some(ids) = termometro_ids {
        if !ids.is_empty() {
            builder.push(" AND r.termometro_id = ANY(");
            builder.push_bind(ids.to_vec());
            builder.push(") ");
        }
    }
}

#[cfg(test)]
mod pruebas_graficos_multi {
    use super::*;

    #[test]
    fn un_solo_id_en_texto_se_partea() {
        assert_eq!(parsear_fragmento_id("5").unwrap(), vec![5]);
    }

    #[test]
    fn comas_con_espacios_se_partean() {
        assert_eq!(parsear_fragmento_id("1, 2,3").unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn texto_vacio_da_lista_vacia() {
        assert!(parsear_fragmento_id("").unwrap().is_empty());
        assert!(parsear_fragmento_id(" , ").unwrap().is_empty());
    }

    #[test]
    fn coma_final_se_tolera() {
        assert_eq!(parsear_fragmento_id("7,").unwrap(), vec![7]);
    }

    #[test]
    fn texto_invalido_da_error() {
        assert!(parsear_fragmento_id("abc").is_err());
        assert!(parsear_fragmento_id("1,x").is_err());
    }

    #[test]
    fn normalizar_quita_duplicados_manteniendo_orden() {
        assert_eq!(
            normalizar_ids_termometros(vec![3, 1, 3, 2, 1]),
            vec![3, 1, 2]
        );
    }

    #[test]
    fn presupuesto_individual_mantiene_total_historico() {
        assert_eq!(limite_por_termometro(1), 1000);
        assert_eq!(limite_por_termometro(0), 1000);
    }

    #[test]
    fn presupuesto_reparte_sin_hambrear() {
        assert_eq!(limite_por_termometro(2), 500);
        assert_eq!(limite_por_termometro(4), 250);
    }

    #[test]
    fn presupuesto_tiene_piso_por_termometro() {
        assert_eq!(limite_por_termometro(20), 50);
        assert_eq!(limite_por_termometro(100), 50);
    }
}

