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

// ===== ÁREAS CRUD =====

pub async fn listar_areas(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Area>>, StatusCode> {
    let areas: Vec<Area> = sqlx::query_as("SELECT * FROM areas ORDER BY nombre")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(areas))
}

pub async fn crear_area(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<CrearAreaRequest>,
) -> Result<Json<Area>, StatusCode> {
    let area: Area = sqlx::query_as(
        "INSERT INTO areas (nombre, descripcion) VALUES ($1, $2) RETURNING *"
    )
    .bind(&payload.nombre)
    .bind(&payload.descripcion)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Error creando área: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let area_id = area.id;

    // Log de auditoría
    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "CREATE",
        "areas",
        Some(area_id),
        None,
        Some(&serde_json::to_string(&area).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(Json(area))
}

pub async fn actualizar_area(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<ActualizarAreaRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut query = String::from("UPDATE areas SET updated_at = CURRENT_TIMESTAMP");
    let mut i = 1;
    
    if payload.nombre.is_some() { i += 1; query.push_str(&format!(", nombre = ${}", i)); }
    if payload.descripcion.is_some() { i += 1; query.push_str(&format!(", descripcion = ${}", i)); }
    if payload.activa.is_some() { i += 1; query.push_str(&format!(", activa = ${}", i)); }

    query.push_str(&format!(" WHERE id = $1"));

    let mut q = sqlx::query(&query).bind(id as i32);
    
    if let Some(nombre) = &payload.nombre { q = q.bind(nombre); }
    if let Some(descripcion) = &payload.descripcion { q = q.bind(descripcion); }
    if let Some(activa) = payload.activa { q = q.bind(activa); }

    q.execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "UPDATE",
        "areas",
        Some(id as i32),
        None,
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}

pub async fn eliminar_area(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("DELETE FROM areas WHERE id = $1")
        .bind(id as i32)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "DELETE",
        "areas",
        Some(id as i32),
        None,
        None,
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}
