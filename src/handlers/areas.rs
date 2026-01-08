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

// ===== ÁREAS CRUD =====

pub async fn listar_areas(
    State(pool): State<SqlitePool>,
) -> Result<Json<Vec<Area>>, StatusCode> {
    let areas: Vec<Area> = sqlx::query_as("SELECT * FROM areas ORDER BY nombre")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(areas))
}

pub async fn crear_area(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Json(payload): Json<CrearAreaRequest>,
) -> Result<Json<Area>, StatusCode> {
    let result = sqlx::query(
        "INSERT INTO areas (nombre, descripcion) VALUES (?, ?)"
    )
    .bind(&payload.nombre)
    .bind(&payload.descripcion)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let area_id = result.last_insert_rowid();

    let area: Area = sqlx::query_as("SELECT * FROM areas WHERE id = ?")
        .bind(area_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log de auditoría
    log_auditoria(
        &pool,
        current_user.0.id,
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
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(payload): Json<ActualizarAreaRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut query = String::from("UPDATE areas SET updated_at = CURRENT_TIMESTAMP");
    let mut params: Vec<String> = Vec::new();

    if let Some(nombre) = &payload.nombre {
        query.push_str(", nombre = ?");
        params.push(nombre.clone());
    }

    if let Some(descripcion) = &payload.descripcion {
        query.push_str(", descripcion = ?");
        params.push(descripcion.clone());
    }

    if let Some(activa) = payload.activa {
        query.push_str(", activa = ?");
        params.push(activa.to_string());
    }

    query.push_str(" WHERE id = ?");

    let mut q = sqlx::query(&query);
    for param in params {
        q = q.bind(param);
    }
    q = q.bind(id);

    q.execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id,
        "UPDATE",
        "areas",
        Some(id),
        None,
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}

pub async fn eliminar_area(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("DELETE FROM areas WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id,
        "DELETE",
        "areas",
        Some(id),
        None,
        None,
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}
