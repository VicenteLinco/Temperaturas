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

// ===== TERMÓMETROS CRUD =====

pub async fn listar_termometros(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<TermometroConDetalles>>, StatusCode> {
    let termometros: Vec<TermometroConDetalles> = sqlx::query_as(
        r#"
        SELECT
            t.id, t.area_id, a.nombre as area_nombre,
            t.tipo_id, ti.nombre as tipo_nombre, ti.tiene_humedad,
            ti.temp_min_operativa, ti.temp_max_operativa,
            t.nombre, t.ubicacion, t.activo, t.fuera_de_servicio
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
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<TermometroConDetalles>, StatusCode> {
    let termometro: TermometroConDetalles = sqlx::query_as(
        r#"
        SELECT
            t.id, t.area_id, a.nombre as area_nombre,
            t.tipo_id, ti.nombre as tipo_nombre, ti.tiene_humedad,
            ti.temp_min_operativa, ti.temp_max_operativa,
            t.nombre, t.ubicacion, t.activo, t.fuera_de_servicio
        FROM termometros t
        JOIN areas a ON t.area_id = a.id
        JOIN tipos_termometro ti ON t.tipo_id = ti.id
        WHERE t.id = $1
        "#
    )
    .bind(id as i32)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(termometro))
}

pub async fn reportar_fuera_de_servicio(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<ReportarFueraServicioRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut tx = pool.begin().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Actualizar termómetro
    sqlx::query("UPDATE termometros SET fuera_de_servicio = TRUE, updated_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(payload.termometro_id as i32)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Crear registro de mantenimiento
    sqlx::query(
        r#"
        INSERT INTO mantenimiento_termometros (termometro_id, usuario_reporta_id, motivo, comentarios_reporte)
        VALUES ($1, $2, $3, $4)
        "#
    )
    .bind(payload.termometro_id as i32)
    .bind(current_user.0.id as i32)
    .bind(&payload.motivo)
    .bind(&payload.comentarios)
    .execute(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tx.commit().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "REPORT_OUT_OF_SERVICE",
        "termometros",
        Some(payload.termometro_id as i32),
        None,
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}

pub async fn reparar_termometro(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<RepararTermometroRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut tx = pool.begin().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Actualizar termómetro
    sqlx::query("UPDATE termometros SET fuera_de_servicio = FALSE, updated_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(id as i32)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Actualizar registro de mantenimiento pendiente
    sqlx::query(
        r#"
        UPDATE mantenimiento_termometros
        SET estado = 'REPARADO',
            fecha_reparacion = CURRENT_TIMESTAMP,
            usuario_repara_id = $1,
            detalle_reparacion = $2
        WHERE termometro_id = $3 AND estado = 'PENDIENTE'
        "#
    )
    .bind(current_user.0.id as i32)
    .bind(&payload.detalle_reparacion)
    .bind(id as i32)
    .execute(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tx.commit().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "REPAIR",
        "termometros",
        Some(id as i32),
        None,
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}


pub async fn crear_termometro(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<CrearTermometroRequest>,
) -> Result<Json<Termometro>, StatusCode> {
    let termometro: Termometro = sqlx::query_as(
        "INSERT INTO termometros (id, area_id, tipo_id, nombre, ubicacion) VALUES ($1, $2, $3, $4, $5) RETURNING *"
    )
    .bind(payload.id as i32)
    .bind(payload.area_id as i32)
    .bind(payload.tipo_id as i32)
    .bind(&payload.nombre)
    .bind(&payload.ubicacion)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "CREATE",
        "termometros",
        Some(payload.id as i32),
        None,
        Some(&serde_json::to_string(&termometro).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(Json(termometro))
}

pub async fn actualizar_termometro(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<ActualizarTermometroRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut query = String::from("UPDATE termometros SET updated_at = CURRENT_TIMESTAMP");
    let mut i = 1;

    if payload.area_id.is_some() { i += 1; query.push_str(&format!(", area_id = ${}", i)); }
    if payload.tipo_id.is_some() { i += 1; query.push_str(&format!(", tipo_id = ${}", i)); }
    if payload.nombre.is_some() { i += 1; query.push_str(&format!(", nombre = ${}", i)); }
    if payload.ubicacion.is_some() { i += 1; query.push_str(&format!(", ubicacion = ${}", i)); }
    if payload.activo.is_some() { i += 1; query.push_str(&format!(", activo = ${}", i)); }
    if payload.fuera_de_servicio.is_some() { i += 1; query.push_str(&format!(", fuera_de_servicio = ${}", i)); }

    query.push_str(" WHERE id = $1");

    let mut q = sqlx::query(&query).bind(id as i32);

    if let Some(v) = payload.area_id { q = q.bind(v as i32); }
    if let Some(v) = payload.tipo_id { q = q.bind(v as i32); }
    if let Some(v) = &payload.nombre { q = q.bind(v); }
    if let Some(v) = &payload.ubicacion { q = q.bind(v); }
    if let Some(v) = payload.activo { q = q.bind(v); }
    if let Some(v) = payload.fuera_de_servicio { q = q.bind(v); }

    q.execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "UPDATE",
        "termometros",
        Some(id as i32),
        None,
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}

pub async fn eliminar_termometro(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("DELETE FROM termometros WHERE id = $1")
        .bind(id as i32)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "DELETE",
        "termometros",
        Some(id as i32),
        None,
        None,
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}
