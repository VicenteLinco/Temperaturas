use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use sqlx::SqlitePool;

use crate::{
    auth::CurrentUser,
    db::{log_auditoria, set_config},
    models::*,
};

// ===== CONFIGURACIÓN =====

pub async fn obtener_configuracion(
    _current_user: CurrentUser,
    State(pool): State<SqlitePool>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let configs: Vec<Configuracion> = sqlx::query_as("SELECT * FROM configuracion")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut map = serde_json::Map::new();
    for config in configs {
        map.insert(config.clave, serde_json::Value::String(config.valor));
    }

    Ok(Json(serde_json::Value::Object(map)))
}

pub async fn actualizar_configuracion(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Json(payload): Json<ActualizarConfiguracionRequest>,
) -> Result<StatusCode, StatusCode> {
    if let Some(hora) = &payload.registro_hora_1 {
        set_config(&pool, "registro_hora_1", hora).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if let Some(hora) = &payload.registro_hora_2 {
        set_config(&pool, "registro_hora_2", hora).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if let Some(min) = payload.ventana_tolerancia_minutos {
        set_config(&pool, "ventana_tolerancia_minutos", &min.to_string()).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if let Some(activa) = payload.restriccion_ventana_activa {
        let valor = if activa { "1" } else { "0" };
        set_config(&pool, "restriccion_ventana_activa", valor).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    log_auditoria(
        &pool,
        current_user.0.id,
        "UPDATE",
        "configuracion",
        None,
        None,
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}
