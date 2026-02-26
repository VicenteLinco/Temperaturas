use axum::{extract::State, http::StatusCode, Json};
use sqlx::SqlitePool;
use tower_sessions::Session;

use crate::{
    auth::{self, verify_password},
    models::{LoginRequest, LoginResponse, Usuario, UsuarioResponse},
};

/// Handler para login de usuarios
pub async fn login_handler(
    session: Session,
    State(pool): State<SqlitePool>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    // Buscar usuario
    let usuario: Option<Usuario> = sqlx::query_as(
        "SELECT * FROM usuarios WHERE username = ? AND activo = 1"
    )
    .bind(&payload.username)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(usuario) = usuario else {
        return Ok(Json(LoginResponse {
            success: false,
            user: None,
            message: Some("Usuario o contraseña incorrectos".to_string()),
        }));
    };

    // Verificar contraseña
    if !verify_password(&payload.password, &usuario.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Ok(Json(LoginResponse {
            success: false,
            user: None,
            message: Some("Usuario o contraseña incorrectos".to_string()),
        }));
    }

    // Guardar en sesión
    auth::save_user_to_session(&session, &usuario)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse {
        success: true,
        user: Some(usuario.into()),
        message: None,
    }))
}

/// Handler para logout de usuarios
pub async fn logout_handler(session: Session) -> Result<StatusCode, StatusCode> {
    auth::logout(&session)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

/// Handler para obtener información del usuario actual
pub async fn me_handler(
    session: Session,
) -> Result<Json<Option<UsuarioResponse>>, StatusCode> {
    let user = auth::get_current_user(&session).await;
    Ok(Json(user.map(|u| UsuarioResponse {
        id: u.id,
        username: u.username,
        rol: u.rol,
        activo: true,
    })))
}