use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::SqlitePool;

use crate::{
    auth::CurrentUser,
    db::log_auditoria,
    models::*,
};

// ===== TIPOS DE TERMÓMETRO CRUD =====

pub async fn listar_tipos_termometro(
    State(pool): State<SqlitePool>,
) -> Result<Json<Vec<TipoTermometro>>, StatusCode> {
    let tipos: Vec<TipoTermometro> = sqlx::query_as("SELECT * FROM tipos_termometro ORDER BY nombre")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(tipos))
}

pub async fn crear_tipo_termometro(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Json(payload): Json<CrearTipoTermometroRequest>,
) -> Result<Json<TipoTermometro>, StatusCode> {
    let result = sqlx::query(
        r#"
        INSERT INTO tipos_termometro (
            nombre, descripcion, tiene_humedad,
            temp_min_operativa, temp_max_operativa,
            temp_min_fisica, temp_max_fisica,
            hum_min_operativa, hum_max_operativa,
            hum_min_fisica, hum_max_fisica
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tipo_id = result.last_insert_rowid();

    let tipo: TipoTermometro = sqlx::query_as("SELECT * FROM tipos_termometro WHERE id = ?")
        .bind(tipo_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id,
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
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(payload): Json<ActualizarTipoTermometroRequest>,
) -> Result<StatusCode, StatusCode> {
    // Construir query dinámico
    let mut sets = vec!["updated_at = CURRENT_TIMESTAMP"];

    if payload.nombre.is_some() { sets.push("nombre = ?"); }
    if payload.descripcion.is_some() { sets.push("descripcion = ?"); }
    if payload.tiene_humedad.is_some() { sets.push("tiene_humedad = ?"); }
    if payload.temp_min_operativa.is_some() { sets.push("temp_min_operativa = ?"); }
    if payload.temp_max_operativa.is_some() { sets.push("temp_max_operativa = ?"); }
    if payload.temp_min_fisica.is_some() { sets.push("temp_min_fisica = ?"); }
    if payload.temp_max_fisica.is_some() { sets.push("temp_max_fisica = ?"); }
    if payload.hum_min_operativa.is_some() { sets.push("hum_min_operativa = ?"); }
    if payload.hum_max_operativa.is_some() { sets.push("hum_max_operativa = ?"); }
    if payload.hum_min_fisica.is_some() { sets.push("hum_min_fisica = ?"); }
    if payload.hum_max_fisica.is_some() { sets.push("hum_max_fisica = ?"); }
    if payload.activo.is_some() { sets.push("activo = ?"); }

    let query = format!("UPDATE tipos_termometro SET {} WHERE id = ?", sets.join(", "));

    let mut q = sqlx::query(&query);

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

    q = q.bind(id);

    q.execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id,
        "UPDATE",
        "tipos_termometro",
        Some(id),
        None,
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}

pub async fn eliminar_tipo_termometro(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("DELETE FROM tipos_termometro WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id,
        "DELETE",
        "tipos_termometro",
        Some(id),
        None,
        None,
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}
