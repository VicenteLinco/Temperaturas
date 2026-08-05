use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, FromRow, PgPool, Row};

use crate::auth::CurrentUser;

// ===== GRÁFICOS =====

#[derive(Deserialize)]
pub struct FiltrosGraficos {
    dias: Option<i64>,
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
pub struct TendenciasItem {
    pub fecha: String,
    pub promedio_max: Option<f32>,
    pub promedio_min: Option<f32>,
    pub promedio_actual: Option<f32>,
    pub alertas: i64,
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
    pub resumen: ResumenGraficos,
    pub tendencias: Vec<TendenciasItem>,
    pub areas: Vec<AreaResumen>,
}

pub async fn obtener_graficos(
    _current_user: CurrentUser,
    State(pool): State<PgPool>,
    Query(filtros): Query<FiltrosGraficos>,
) -> Result<Json<GraficosResponse>, StatusCode> {
    let dias = filtros.dias.unwrap_or(30).clamp(1, 365);

    // Resumen del período
    let resumen: ResumenGraficos = sqlx::query_as(
        r#"
        SELECT
            COUNT(*)::bigint AS total_registros,
            COUNT(*) FILTER (WHERE r.fuera_rango_operativo)::bigint AS total_alertas,
            ROUND(AVG(r.temp_maxima)::numeric, 1)::real AS promedio_max,
            ROUND(AVG(r.temp_minima)::numeric, 1)::real AS promedio_min,
            ROUND(AVG(r.temp_actual)::numeric, 1)::real AS promedio_actual
        FROM registros r
        WHERE (r.fecha_registro AT TIME ZONE 'America/Santiago')::date >= ((CURRENT_TIMESTAMP AT TIME ZONE 'America/Santiago')::date - $1::int)
        "#
    )
    .bind(dias)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Error graficos resumen: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Tendencias diarias
    let rows: Vec<PgRow> = sqlx::query(
        r#"
        SELECT
            (r.fecha_registro AT TIME ZONE 'America/Santiago')::date AS fecha,
            ROUND(AVG(r.temp_maxima)::numeric, 1)::real AS promedio_max,
            ROUND(AVG(r.temp_minima)::numeric, 1)::real AS promedio_min,
            ROUND(AVG(r.temp_actual)::numeric, 1)::real AS promedio_actual,
            COUNT(*) FILTER (WHERE r.fuera_rango_operativo)::bigint AS alertas
        FROM registros r
        WHERE (r.fecha_registro AT TIME ZONE 'America/Santiago')::date >= ((CURRENT_TIMESTAMP AT TIME ZONE 'America/Santiago')::date - $1::int)
        GROUP BY fecha
        ORDER BY fecha
        "#
    )
    .bind(dias)
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Error graficos tendencias: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let tendencias: Vec<TendenciasItem> = rows
        .iter()
        .map(|row| TendenciasItem {
            fecha: row.get::<chrono::NaiveDate, _>("fecha").to_string(),
            promedio_max: row.get("promedio_max"),
            promedio_min: row.get("promedio_min"),
            promedio_actual: row.get("promedio_actual"),
            alertas: row.get("alertas"),
        })
        .collect();

    // Resumen por área
    let area_rows: Vec<PgRow> = sqlx::query(
        r#"
        SELECT
            a.nombre AS area,
            COUNT(*)::bigint AS total,
            COUNT(*) FILTER (WHERE r.fuera_rango_operativo)::bigint AS alertas
        FROM registros r
        JOIN termometros t ON r.termometro_id = t.id
        JOIN areas a ON t.area_id = a.id
        WHERE (r.fecha_registro AT TIME ZONE 'America/Santiago')::date >= ((CURRENT_TIMESTAMP AT TIME ZONE 'America/Santiago')::date - $1::int)
        GROUP BY a.nombre
        ORDER BY a.nombre
        "#
    )
    .bind(dias)
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
        resumen,
        tendencias,
        areas,
    }))
}
