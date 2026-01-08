use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::SqlitePool;

use crate::{
    auth::{hash_password, CurrentUser},
    db::log_auditoria,
    models::*,
};

// ===== USUARIOS CRUD =====

pub async fn listar_usuarios(
    _current_user: CurrentUser,
    State(pool): State<SqlitePool>,
) -> Result<Json<Vec<UsuarioResponse>>, StatusCode> {
    let usuarios: Vec<Usuario> = sqlx::query_as("SELECT * FROM usuarios ORDER BY username")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(usuarios.into_iter().map(|u| u.into()).collect()))
}

pub async fn crear_usuario(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Json(payload): Json<CrearUsuarioRequest>,
) -> Result<Json<UsuarioResponse>, StatusCode> {
    // Validar rol
    if payload.rol != "ADMINISTRADOR" && payload.rol != "REGISTRADOR" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // ✅ NUEVA VALIDACIÓN: Verificar que el username no exista
    let existe: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM usuarios WHERE username = ?"
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

    // Insertar usuario
    let result = sqlx::query(
        "INSERT INTO usuarios (username, password_hash, rol) VALUES (?, ?, ?)"
    )
    .bind(&payload.username)
    .bind(&password_hash)
    .bind(&payload.rol)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let usuario_id = result.last_insert_rowid();

    // Log de auditoría
    log_auditoria(
        &pool,
        current_user.0.id,
        "CREATE",
        "usuarios",
        Some(usuario_id),
        None,
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(Json(UsuarioResponse {
        id: usuario_id,
        username: payload.username,
        rol: payload.rol,
        activo: true,
    }))
}

pub async fn actualizar_usuario(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(payload): Json<ActualizarUsuarioRequest>,
) -> Result<StatusCode, StatusCode> {
    // Obtener datos anteriores
    let anterior: Option<Usuario> = sqlx::query_as("SELECT * FROM usuarios WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut query = String::from("UPDATE usuarios SET updated_at = CURRENT_TIMESTAMP");
    let mut params: Vec<String> = Vec::new();

    if let Some(username) = &payload.username {
        query.push_str(", username = ?");
        params.push(username.clone());
    }

    if let Some(password) = &payload.password {
        let hash = hash_password(password)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        query.push_str(", password_hash = ?");
        params.push(hash);
    }

    if let Some(rol) = &payload.rol {
        query.push_str(", rol = ?");
        params.push(rol.clone());
    }

    if let Some(activo) = payload.activo {
        query.push_str(", activo = ?");
        params.push(activo.to_string());
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

    // Log de auditoría
    log_auditoria(
        &pool,
        current_user.0.id,
        "UPDATE",
        "usuarios",
        Some(id),
        anterior.as_ref().and_then(|a| serde_json::to_string(a).ok()).as_deref(),
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}

pub async fn eliminar_usuario(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    // No permitir eliminar al propio usuario
    if id == current_user.0.id {
        return Err(StatusCode::BAD_REQUEST);
    }

    sqlx::query("DELETE FROM usuarios WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log de auditoría
    log_auditoria(
        &pool,
        current_user.0.id,
        "DELETE",
        "usuarios",
        Some(id),
        None,
        None,
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}
