use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;

use crate::{
    auth::{hash_password, CurrentUser},
    db::log_auditoria,
    models::*,
};

// ===== USUARIOS CRUD =====

pub async fn listar_usuarios(
    _current_user: CurrentUser,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<UsuarioResponse>>, StatusCode> {
    let usuarios: Vec<Usuario> = sqlx::query_as("SELECT * FROM usuarios ORDER BY username")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(usuarios.into_iter().map(|u| u.into()).collect()))
}

pub async fn crear_usuario(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<CrearUsuarioRequest>,
) -> Result<Json<UsuarioResponse>, StatusCode> {
    // Validar rol
    if payload.rol != "ADMINISTRADOR" && payload.rol != "REGISTRADOR" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // ✅ NUEVA VALIDACIÓN: Verificar que el username no exista
    let existe: Option<(i32,)> = sqlx::query_as(
        "SELECT id FROM usuarios WHERE username = $1"
    )
    .bind(&payload.username)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if existe.is_some() {
        return Err(StatusCode::CONFLICT);  // 409 Conflict - Username ya existe
    }

    // Hash de contraseña
    let password_hash = hash_password(&payload.password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Insertar usuario y obtener ID
    let row: (i32,) = sqlx::query_as(
        "INSERT INTO usuarios (username, password_hash, rol) VALUES ($1, $2, $3) RETURNING id"
    )
    .bind(&payload.username)
    .bind(&password_hash)
    .bind(&payload.rol)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let usuario_id = row.0;

    // Log de auditoría
    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "CREATE",
        "usuarios",
        Some(usuario_id),
        None,
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(Json(UsuarioResponse {
        id: usuario_id as i64,
        username: payload.username,
        rol: payload.rol,
        activo: true,
    }))
}

pub async fn actualizar_usuario(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<ActualizarUsuarioRequest>,
) -> Result<StatusCode, StatusCode> {
    // Obtener datos anteriores
    let anterior: Option<Usuario> = sqlx::query_as("SELECT * FROM usuarios WHERE id = $1")
        .bind(id as i32)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut query = String::from("UPDATE usuarios SET updated_at = CURRENT_TIMESTAMP");
    let mut i = 1;

    // An empty/whitespace password means "no change": keep the existing hash
    // instead of storing a hash of the empty string.
    let nueva_password = payload
        .password
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string);

    if payload.username.is_some() { i += 1; query.push_str(&format!(", username = ${}", i)); }
    if nueva_password.is_some() { i += 1; query.push_str(&format!(", password_hash = ${}", i)); }
    if payload.rol.is_some() { i += 1; query.push_str(&format!(", rol = ${}", i)); }
    if payload.activo.is_some() { i += 1; query.push_str(&format!(", activo = ${}", i)); }

    query.push_str(&format!(" WHERE id = $1"));

    let mut q = sqlx::query(&query).bind(id as i32);
    
    if let Some(username) = &payload.username { q = q.bind(username); }
    if let Some(password) = &nueva_password {
        let hash = hash_password(password).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        q = q.bind(hash);
    }
    if let Some(rol) = &payload.rol { q = q.bind(rol); }
    if let Some(activo) = payload.activo { q = q.bind(activo); }

    q.execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log de auditoría
    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "UPDATE",
        "usuarios",
        Some(id as i32),
        anterior.as_ref().and_then(|a| serde_json::to_string(a).ok()).as_deref(),
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}

pub async fn eliminar_usuario(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    // No permitir eliminar al propio usuario
    if id == current_user.0.id {
        return Err(StatusCode::BAD_REQUEST);
    }

    sqlx::query("DELETE FROM usuarios WHERE id = $1")
        .bind(id as i32)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log de auditoría
    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "DELETE",
        "usuarios",
        Some(id as i32),
        None,
        None,
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}
