use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;

use crate::{
    auth::CurrentUser,
    db::log_auditoria,
    models::*,
};

// ===== TIPOS DE TERMÓMETRO CRUD =====

pub async fn listar_tipos_termometro(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<TipoTermometro>>, StatusCode> {
    let tipos: Vec<TipoTermometro> = sqlx::query_as("SELECT * FROM tipos_termometro ORDER BY nombre")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(tipos))
}

pub async fn crear_tipo_termometro(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<CrearTipoTermometroRequest>,
) -> Result<Json<TipoTermometro>, StatusCode> {
    let tipo: TipoTermometro = sqlx::query_as(
        r#"
        INSERT INTO tipos_termometro (
            nombre, descripcion, tiene_humedad,
            temp_min_operativa, temp_max_operativa,
            temp_min_fisica, temp_max_fisica,
            hum_min_operativa, hum_max_operativa,
            hum_min_fisica, hum_max_fisica
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING *
        "#
    )
    .bind(&payload.nombre)
    .bind(&payload.descripcion)
    .bind(payload.tiene_humedad)
    .bind(payload.temp_min_operativa)
    .bind(payload.temp_max_operativa)
    .bind(payload.temp_min_fisica)
    .bind(payload.temp_max_fisica)
    .bind(payload.hum_min_operativa)
    .bind(payload.hum_max_operativa)
    .bind(payload.hum_min_fisica)
    .bind(payload.hum_max_fisica)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tipo_id = tipo.id as i32;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "CREATE",
        "tipos_termometro",
        Some(tipo_id),
        None,
        Some(&serde_json::to_string(&tipo).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(Json(tipo))
}

pub async fn actualizar_tipo_termometro(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<ActualizarTipoTermometroRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut query = String::from("UPDATE tipos_termometro SET updated_at = CURRENT_TIMESTAMP");
    let mut i = 1;

    if payload.nombre.is_some() { i += 1; query.push_str(&format!(", nombre = ${}", i)); }
    if payload.descripcion.is_some() { i += 1; query.push_str(&format!(", descripcion = ${}", i)); }
    if payload.tiene_humedad.is_some() { i += 1; query.push_str(&format!(", tiene_humedad = ${}", i)); }
    if payload.temp_min_operativa.is_some() { i += 1; query.push_str(&format!(", temp_min_operativa = ${}", i)); }
    if payload.temp_max_operativa.is_some() { i += 1; query.push_str(&format!(", temp_max_operativa = ${}", i)); }
    if payload.temp_min_fisica.is_some() { i += 1; query.push_str(&format!(", temp_min_fisica = ${}", i)); }
    if payload.temp_max_fisica.is_some() { i += 1; query.push_str(&format!(", temp_max_fisica = ${}", i)); }
    if payload.hum_min_operativa.is_some() { i += 1; query.push_str(&format!(", hum_min_operativa = ${}", i)); }
    if payload.hum_max_operativa.is_some() { i += 1; query.push_str(&format!(", hum_max_operativa = ${}", i)); }
    if payload.hum_min_fisica.is_some() { i += 1; query.push_str(&format!(", hum_min_fisica = ${}", i)); }
    if payload.hum_max_fisica.is_some() { i += 1; query.push_str(&format!(", hum_max_fisica = ${}", i)); }
    if payload.activo.is_some() { i += 1; query.push_str(&format!(", activo = ${}", i)); }

    query.push_str(" WHERE id = $1");

    let mut q = sqlx::query(&query).bind(id as i32);

    if let Some(v) = &payload.nombre { q = q.bind(v); }
    if let Some(v) = &payload.descripcion { q = q.bind(v); }
    if let Some(v) = payload.tiene_humedad { q = q.bind(v); }
    if let Some(v) = payload.temp_min_operativa { q = q.bind(v); }
    if let Some(v) = payload.temp_max_operativa { q = q.bind(v); }
    if let Some(v) = payload.temp_min_fisica { q = q.bind(v); }
    if let Some(v) = payload.temp_max_fisica { q = q.bind(v); }
    if let Some(v) = payload.hum_min_operativa { q = q.bind(v); }
    if let Some(v) = payload.hum_max_operativa { q = q.bind(v); }
    if let Some(v) = payload.hum_min_fisica { q = q.bind(v); }
    if let Some(v) = payload.hum_max_fisica { q = q.bind(v); }
    if let Some(v) = payload.activo { q = q.bind(v); }

    q.execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "UPDATE",
        "tipos_termometro",
        Some(id as i32),
        None,
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}

pub async fn eliminar_tipo_termometro(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("DELETE FROM tipos_termometro WHERE id = $1")
        .bind(id as i32)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "DELETE",
        "tipos_termometro",
        Some(id as i32),
        None,
        None,
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}
