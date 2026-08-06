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
    pub termometro_id: Option<i32>,
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
    pub termometro_id: Option<i32>,
    pub limites: Option<LimitesInfo>,
    pub resumen: ResumenGraficos,
    pub tendencias: Vec<TendenciasItem>,
    pub lecturas: Vec<LecturaGraficoItem>,
    pub areas: Vec<AreaResumen>,
}

pub async fn obtener_graficos(
    _current_user: CurrentUser,
    State(pool): State<PgPool>,
    Query(filtros): Query<FiltrosGraficos>,
) -> Result<Json<GraficosResponse>, StatusCode> {
    let dias = filtros.dias.unwrap_or(30).clamp(1, 365);
    let fecha_desde = filtros.fecha_desde.filter(|s| !s.trim().is_empty());
    let fecha_hasta = filtros.fecha_hasta.filter(|s| !s.trim().is_empty());

    // 1. Obtener límites operativos si hay termómetro o área seleccionada
    let mut limites: Option<LimitesInfo> = None;
    if let Some(t_id) = filtros.termometro_id {
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

    aplicar_filtros_where(&mut builder_resumen, &fecha_desde, &fecha_hasta, dias, filtros.area_id, filtros.termometro_id);

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
            MIN(tt.temp_min_operativa) AS temp_min_operativa,
            MAX(tt.temp_max_operativa) AS temp_max_operativa
        FROM registros r
        JOIN termometros t ON r.termometro_id = t.id
        JOIN tipos_termometro tt ON t.tipo_id = tt.id
        WHERE 1=1
        "#
    );

    aplicar_filtros_where(&mut builder_tendencias, &fecha_desde, &fecha_hasta, dias, filtros.area_id, filtros.termometro_id);
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
            temp_min_operativa: row.get("temp_min_operativa"),
            temp_max_operativa: row.get("temp_max_operativa"),
        })
        .collect();

    // 4. Lecturas punto por punto (individuales)
    let mut builder_lecturas = sqlx::QueryBuilder::new(
        r#"
        SELECT
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
        FROM registros r
        JOIN termometros t ON r.termometro_id = t.id
        JOIN tipos_termometro tt ON t.tipo_id = tt.id
        JOIN areas a ON t.area_id = a.id
        WHERE 1=1
        "#
    );

    aplicar_filtros_where(&mut builder_lecturas, &fecha_desde, &fecha_hasta, dias, filtros.area_id, filtros.termometro_id);
    builder_lecturas.push(" ORDER BY r.fecha_registro ASC LIMIT 1000 ");

    let lecturas_rows: Vec<PgRow> = builder_lecturas
        .build()
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Error graficos lecturas: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let lecturas: Vec<LecturaGraficoItem> = lecturas_rows
        .iter()
        .map(|row| LecturaGraficoItem {
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
        })
        .collect();

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

    aplicar_filtros_where(&mut builder_areas, &fecha_desde, &fecha_hasta, dias, filtros.area_id, filtros.termometro_id);
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
        termometro_id: filtros.termometro_id,
        limites,
        resumen,
        tendencias,
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
    termometro_id: Option<i32>,
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
    if let Some(tid) = termometro_id {
        builder.push(" AND r.termometro_id = ");
        builder.push_bind(tid);
    }
}

