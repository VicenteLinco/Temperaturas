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

// ===== TERMÓMETROS CRUD =====

pub async fn listar_termometros(
    State(pool): State<SqlitePool>,
) -> Result<Json<Vec<TermometroConDetalles>>, StatusCode> {
    let termometros: Vec<TermometroConDetalles> = sqlx::query_as(
        r#"
        SELECT
            t.id, t.area_id, a.nombre as area_nombre,
            t.tipo_id, ti.nombre as tipo_nombre, ti.tiene_humedad,
            t.nombre, t.ubicacion, t.activo
        FROM termometros t
        JOIN areas a ON t.area_id = a.id
        JOIN tipos_termometro ti ON t.tipo_id = ti.id
        ORDER BY a.nombre, t.id
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(termometros))
}

pub async fn obtener_termometro(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<Json<TermometroConDetalles>, StatusCode> {
    let termometro: TermometroConDetalles = sqlx::query_as(
        r#"
        SELECT
            t.id, t.area_id, a.nombre as area_nombre,
            t.tipo_id, ti.nombre as tipo_nombre, ti.tiene_humedad,
            t.nombre, t.ubicacion, t.activo
        FROM termometros t
        JOIN areas a ON t.area_id = a.id
        JOIN tipos_termometro ti ON t.tipo_id = ti.id
        WHERE t.id = ?
        "#
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(termometro))
}

pub async fn crear_termometro(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Json(payload): Json<CrearTermometroRequest>,
) -> Result<Json<Termometro>, StatusCode> {
    sqlx::query(
        "INSERT INTO termometros (id, area_id, tipo_id, nombre, ubicacion) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(payload.id)
    .bind(payload.area_id)
    .bind(payload.tipo_id)
    .bind(&payload.nombre)
    .bind(&payload.ubicacion)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let termometro: Termometro = sqlx::query_as("SELECT * FROM termometros WHERE id = ?")
        .bind(payload.id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id,
        "CREATE",
        "termometros",
        Some(payload.id),
        None,
        Some(&serde_json::to_string(&termometro).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(Json(termometro))
}

pub async fn actualizar_termometro(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(payload): Json<ActualizarTermometroRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut sets = vec!["updated_at = CURRENT_TIMESTAMP"];

    if payload.area_id.is_some() { sets.push("area_id = ?"); }
    if payload.tipo_id.is_some() { sets.push("tipo_id = ?"); }
    if payload.nombre.is_some() { sets.push("nombre = ?"); }
    if payload.ubicacion.is_some() { sets.push("ubicacion = ?"); }
    if payload.activo.is_some() { sets.push("activo = ?"); }

    let query = format!("UPDATE termometros SET {} WHERE id = ?", sets.join(", "));

    let mut q = sqlx::query(&query);

    if let Some(v) = payload.area_id { q = q.bind(v); }
    if let Some(v) = payload.tipo_id { q = q.bind(v); }
    if let Some(v) = &payload.nombre { q = q.bind(v); }
    if let Some(v) = &payload.ubicacion { q = q.bind(v); }
    if let Some(v) = payload.activo { q = q.bind(v); }

    q = q.bind(id);

    q.execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id,
        "UPDATE",
        "termometros",
        Some(id),
        None,
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}

pub async fn eliminar_termometro(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("DELETE FROM termometros WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id,
        "DELETE",
        "termometros",
        Some(id),
        None,
        None,
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}
