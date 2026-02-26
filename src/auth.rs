use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use async_trait::async_trait;
use bcrypt::{hash, verify, DEFAULT_COST};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use anyhow::Result;

use crate::models::Usuario;

const SESSION_USER_KEY: &str = "user_id";
const SESSION_ROLE_KEY: &str = "user_role";

/// Datos del usuario en sesión
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUser {
    pub id: i64,
    pub username: String,
    pub rol: String,
}

/// Hash una contraseña usando bcrypt
pub fn hash_password(password: &str) -> Result<String> {
    Ok(hash(password, DEFAULT_COST)?)
}

/// Verifica una contraseña contra su hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    Ok(verify(password, hash)?)
}

/// Obtiene el usuario actual de la sesión
pub async fn get_current_user(session: &Session) -> Option<SessionUser> {
    let user_id: i64 = session.get(SESSION_USER_KEY).await.ok()??;
    let username: String = session.get("username").await.ok()??;
    let rol: String = session.get(SESSION_ROLE_KEY).await.ok()??;

    Some(SessionUser {
        id: user_id,
        username,
        rol,
    })
}

/// Guarda el usuario en la sesión
pub async fn save_user_to_session(session: &Session, usuario: &Usuario) -> Result<()> {
    session.insert(SESSION_USER_KEY, usuario.id).await?;
    session.insert("username", &usuario.username).await?;
    session.insert(SESSION_ROLE_KEY, &usuario.rol).await?;
    Ok(())
}

/// Cierra la sesión del usuario
pub async fn logout(session: &Session) -> Result<()> {
    session.flush().await?;
    Ok(())
}

/// Middleware que requiere autenticación
pub async fn require_auth(
    session: Session,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let user = get_current_user(&session).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Agregar usuario al request para que esté disponible en los handlers
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

/// Middleware que requiere rol de administrador
pub async fn require_admin(
    session: Session,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let user = get_current_user(&session).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if user.rol != "ADMINISTRADOR" {
        return Err(StatusCode::FORBIDDEN);
    }

    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

/// Middleware que requiere rol de registrador o administrador
pub async fn require_registrador(
    session: Session,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let user = get_current_user(&session).await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if user.rol != "REGISTRADOR" && user.rol != "ADMINISTRADOR" {
        return Err(StatusCode::FORBIDDEN);
    }

    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

/// Extractor para obtener el usuario actual de los extensions
pub struct CurrentUser(pub SessionUser);

#[async_trait]
impl<S> axum::extract::FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<SessionUser>()
            .cloned()
            .map(CurrentUser)
            .ok_or((StatusCode::UNAUTHORIZED, "Usuario no autenticado"))
    }
}