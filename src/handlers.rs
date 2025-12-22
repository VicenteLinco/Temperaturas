use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Local;
use serde::Deserialize;
use sqlx::SqlitePool;
use tower_sessions::Session;
use printpdf::*;

use crate::{
    auth::{self, hash_password, verify_password, CurrentUser},
    db::{get_config, log_auditoria, set_config},
    logic::{determinar_ventana_actual, validar_registro},
    models::*,
};

// ===== AUTH HANDLERS =====

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

pub async fn logout_handler(session: Session) -> Result<StatusCode, StatusCode> {
    auth::logout(&session)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

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

// ===== REGISTROS CRUD Y LÓGICA DE PENDIENTES =====

#[derive(Deserialize)]
pub struct FiltrosRegistros {
    fecha_desde: Option<String>,
    fecha_hasta: Option<String>,
    area_id: Option<i64>,
    ventana_horaria: Option<String>,
}

pub async fn listar_registros(
    _current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Query(filtros): Query<FiltrosRegistros>,
) -> Result<Json<Vec<RegistroConDetalles>>, StatusCode> {
    let mut query = String::from(
        r#"
        SELECT
            r.id, r.termometro_id, t.nombre as termometro_nombre,
            a.nombre as area_nombre, u.username as usuario_nombre,
            r.ventana_horaria, r.temp_maxima, r.temp_minima, r.humedad,
            r.fuera_rango_operativo, r.observaciones, r.fecha_registro
        FROM registros r
        JOIN termometros t ON r.termometro_id = t.id
        JOIN areas a ON t.area_id = a.id
        JOIN usuarios u ON r.usuario_id = u.id
        WHERE 1=1
        "#
    );

    let mut conditions = Vec::new();

    if filtros.fecha_desde.is_some() {
        conditions.push("DATE(r.fecha_registro) >= ?");
    }
    if filtros.fecha_hasta.is_some() {
        conditions.push("DATE(r.fecha_registro) <= ?");
    }
    if filtros.area_id.is_some() {
        conditions.push("t.area_id = ?");
    }
    if filtros.ventana_horaria.is_some() {
        conditions.push("r.ventana_horaria = ?");
    }

    if !conditions.is_empty() {
        query.push_str(" AND ");
        query.push_str(&conditions.join(" AND "));
    }

    query.push_str(" ORDER BY r.fecha_registro DESC, r.id DESC LIMIT 500");

    let mut q = sqlx::query_as(&query);

    if let Some(fecha) = &filtros.fecha_desde {
        q = q.bind(fecha);
    }
    if let Some(fecha) = &filtros.fecha_hasta {
        q = q.bind(fecha);
    }
    if let Some(area) = filtros.area_id {
        q = q.bind(area);
    }
    if let Some(ventana) = &filtros.ventana_horaria {
        q = q.bind(ventana);
    }

    let registros = q
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(registros))
}

pub async fn obtener_pendientes_area(
    _current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Path(area_id): Path<i64>,
) -> Result<Json<PendientesResponse>, StatusCode> {
    // Obtener configuración
    let hora_1 = get_config(&pool, "registro_hora_1").await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let hora_2 = get_config(&pool, "registro_hora_2").await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tolerancia: i32 = get_config(&pool, "ventana_tolerancia_minutos").await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .parse()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Determinar ventana actual
    let ventana = determinar_ventana_actual(&hora_1, &hora_2, tolerancia)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::BAD_REQUEST)?; // Fuera de ventana permitida

    let fecha_hoy = Local::now().format("%Y-%m-%d").to_string();

    // Obtener área
    let area: Area = sqlx::query_as("SELECT * FROM areas WHERE id = ?")
        .bind(area_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Obtener todos los termómetros activos del área
    let termometros: Vec<TermometroConDetalles> = sqlx::query_as(
        r#"
        SELECT
            t.id, t.area_id, a.nombre as area_nombre,
            t.tipo_id, ti.nombre as tipo_nombre, ti.tiene_humedad,
            t.nombre, t.ubicacion, t.activo
        FROM termometros t
        JOIN areas a ON t.area_id = a.id
        JOIN tipos_termometro ti ON t.tipo_id = ti.id
        WHERE t.area_id = ? AND t.activo = 1
        ORDER BY t.id
        "#
    )
    .bind(area_id)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Obtener registros del día actual para esta ventana
    let registros: Vec<RegistroConDetalles> = sqlx::query_as(
        r#"
        SELECT
            r.id, r.termometro_id, t.nombre as termometro_nombre,
            a.nombre as area_nombre, u.username as usuario_nombre,
            r.ventana_horaria, r.temp_maxima, r.temp_minima, r.humedad,
            r.fuera_rango_operativo, r.observaciones, r.fecha_registro
        FROM registros r
        JOIN termometros t ON r.termometro_id = t.id
        JOIN areas a ON t.area_id = a.id
        JOIN usuarios u ON r.usuario_id = u.id
        WHERE t.area_id = ? AND DATE(r.fecha_registro) = ? AND r.ventana_horaria = ?
        "#
    )
    .bind(area_id)
    .bind(&fecha_hoy)
    .bind(&ventana.nombre)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Separar en pendientes y completados
    let registrados_ids: Vec<i64> = registros.iter().map(|r| r.termometro_id).collect();
    let pendientes: Vec<TermometroConDetalles> = termometros
        .into_iter()
        .filter(|t| !registrados_ids.contains(&t.id))
        .collect();

    Ok(Json(PendientesResponse {
        ventana_horaria: ventana.nombre,
        area_id,
        area_nombre: area.nombre,
        pendientes,
        completados: registros,
    }))
}

pub async fn crear_registro(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Json(payload): Json<CrearRegistroRequest>,
) -> Result<Json<Registro>, StatusCode> {
    // Obtener configuración
    let hora_1 = get_config(&pool, "registro_hora_1").await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let hora_2 = get_config(&pool, "registro_hora_2").await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tolerancia: i32 = get_config(&pool, "ventana_tolerancia_minutos").await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .parse()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Verificar que estamos en ventana horaria permitida
    let ventana = determinar_ventana_actual(&hora_1, &hora_2, tolerancia)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Obtener tipo de termómetro para validación
    let termometro: Termometro = sqlx::query_as("SELECT * FROM termometros WHERE id = ?")
        .bind(payload.termometro_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let tipo: TipoTermometro = sqlx::query_as("SELECT * FROM tipos_termometro WHERE id = ?")
        .bind(termometro.tipo_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Validar registro
    let (fuera_rango_operativo, _advertencias) = validar_registro(
        payload.temp_maxima,
        payload.temp_minima,
        payload.humedad,
        &tipo,
    )
    .map_err(|e| {
        eprintln!("Error de validación: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    // Insertar registro
    let result = sqlx::query(
        r#"
        INSERT INTO registros (
            termometro_id, usuario_id, ventana_horaria,
            temp_maxima, temp_minima, humedad,
            fuera_rango_operativo, observaciones
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(payload.termometro_id)
    .bind(current_user.0.id)
    .bind(&ventana.nombre)
    .bind(payload.temp_maxima)
    .bind(payload.temp_minima)
    .bind(payload.humedad)
    .bind(fuera_rango_operativo)
    .bind(&payload.observaciones)
    .execute(&pool)
    .await
    .map_err(|e| {
        eprintln!("Error al insertar registro: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let registro_id = result.last_insert_rowid();

    let registro: Registro = sqlx::query_as("SELECT * FROM registros WHERE id = ?")
        .bind(registro_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log de auditoría
    log_auditoria(
        &pool,
        current_user.0.id,
        "CREATE",
        "registros",
        Some(registro_id),
        None,
        Some(&serde_json::to_string(&registro).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(Json(registro))
}

pub async fn actualizar_registro(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
    Json(payload): Json<ActualizarRegistroRequest>,
) -> Result<StatusCode, StatusCode> {
    // Obtener registro existente
    let registro: Registro = sqlx::query_as("SELECT * FROM registros WHERE id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Si no es admin, solo puede editar sus propios registros
    if current_user.0.rol != "ADMINISTRADOR" && registro.usuario_id != current_user.0.id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Obtener tipo para validación
    let termometro: Termometro = sqlx::query_as("SELECT * FROM termometros WHERE id = ?")
        .bind(registro.termometro_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tipo: TipoTermometro = sqlx::query_as("SELECT * FROM tipos_termometro WHERE id = ?")
        .bind(termometro.tipo_id)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Usar valores actuales si no se proporcionan nuevos
    let temp_max = payload.temp_maxima.unwrap_or(registro.temp_maxima);
    let temp_min = payload.temp_minima.unwrap_or(registro.temp_minima);
    let humedad = payload.humedad.or(registro.humedad);

    // Validar
    let (fuera_rango_operativo, _) = validar_registro(temp_max, temp_min, humedad, &tipo)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Actualizar
    sqlx::query(
        r#"
        UPDATE registros SET
            temp_maxima = ?, temp_minima = ?, humedad = ?,
            fuera_rango_operativo = ?, observaciones = ?
        WHERE id = ?
        "#
    )
    .bind(temp_max)
    .bind(temp_min)
    .bind(humedad)
    .bind(fuera_rango_operativo)
    .bind(&payload.observaciones)
    .bind(id)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log de auditoría
    log_auditoria(
        &pool,
        current_user.0.id,
        "UPDATE",
        "registros",
        Some(id),
        Some(&serde_json::to_string(&registro).unwrap_or_default()),
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}

pub async fn eliminar_registro(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    // Solo admin puede eliminar registros
    if current_user.0.rol != "ADMINISTRADOR" {
        return Err(StatusCode::FORBIDDEN);
    }

    sqlx::query("DELETE FROM registros WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id,
        "DELETE",
        "registros",
        Some(id),
        None,
        None,
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}

// ===== REPORTES =====

#[derive(Deserialize)]
pub struct FiltrosReporteDiario {
    fecha: String,
    formato: String,
    area_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct FiltrosReporteMensual {
    mes: u32,
    anio: i32,
    formato: String,
    area_id: Option<i64>,
}

pub async fn generar_reporte_diario(
    _current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Query(filtros): Query<FiltrosReporteDiario>,
) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    // Construir query
    let mut query = String::from(
        r#"
        SELECT
            r.id, r.fecha_registro, r.ventana_horaria,
            a.nombre as area_nombre, t.nombre as termometro_nombre, t.id as termometro_id,
            ti.nombre as tipo_nombre,
            r.temp_maxima, r.temp_minima, r.humedad,
            r.fuera_rango_operativo, r.observaciones,
            u.username as usuario_nombre
        FROM registros r
        JOIN termometros t ON r.termometro_id = t.id
        JOIN areas a ON t.area_id = a.id
        JOIN tipos_termometro ti ON t.tipo_id = ti.id
        JOIN usuarios u ON r.usuario_id = u.id
        WHERE DATE(r.fecha_registro) = ?
        "#
    );

    if filtros.area_id.is_some() {
        query.push_str(" AND t.area_id = ?");
    }

    query.push_str(" ORDER BY a.nombre, t.id, r.ventana_horaria");

    let mut q = sqlx::query(&query).bind(&filtros.fecha);

    if let Some(area_id) = filtros.area_id {
        q = q.bind(area_id);
    }

    let rows = q
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if filtros.formato == "csv" {
        generar_csv_diario(rows, &filtros.fecha)
    } else if filtros.formato == "pdf" {
        generar_pdf_diario(rows, &filtros.fecha)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

pub async fn generar_reporte_mensual(
    _current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Query(filtros): Query<FiltrosReporteMensual>,
) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    // Construir query
    let mut query = String::from(
        r#"
        SELECT
            r.id, r.fecha_registro, r.ventana_horaria,
            a.nombre as area_nombre, t.nombre as termometro_nombre, t.id as termometro_id,
            ti.nombre as tipo_nombre,
            r.temp_maxima, r.temp_minima, r.humedad,
            r.fuera_rango_operativo, r.observaciones,
            u.username as usuario_nombre
        FROM registros r
        JOIN termometros t ON r.termometro_id = t.id
        JOIN areas a ON t.area_id = a.id
        JOIN tipos_termometro ti ON t.tipo_id = ti.id
        JOIN usuarios u ON r.usuario_id = u.id
        WHERE strftime('%Y', r.fecha_registro) = ? AND strftime('%m', r.fecha_registro) = ?
        "#
    );

    if filtros.area_id.is_some() {
        query.push_str(" AND t.area_id = ?");
    }

    query.push_str(" ORDER BY r.fecha_registro, a.nombre, t.id, r.ventana_horaria");

    let mes_str = format!("{:02}", filtros.mes);
    let mut q = sqlx::query(&query)
        .bind(filtros.anio.to_string())
        .bind(&mes_str);

    if let Some(area_id) = filtros.area_id {
        q = q.bind(area_id);
    }

    let rows = q
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if filtros.formato == "csv" {
        generar_csv_mensual(rows, filtros.mes, filtros.anio)
    } else if filtros.formato == "pdf" {
        generar_pdf_mensual(rows, filtros.mes, filtros.anio)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

fn generar_csv_diario(rows: Vec<sqlx::sqlite::SqliteRow>, _fecha: &str) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    use sqlx::Row;

    let mut wtr = csv::Writer::from_writer(vec![]);

    // Encabezados
    wtr.write_record(&[
        "ID", "Fecha", "Ventana", "Área", "Termómetro", "Tipo",
        "Temp. Máx", "Temp. Mín", "Humedad", "Fuera Rango", "Observaciones", "Usuario"
    ]).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Datos
    for row in rows {
        let humedad: Option<f64> = row.try_get("humedad").ok();
        let observaciones: Option<String> = row.try_get("observaciones").ok();

        wtr.write_record(&[
            row.get::<i64, _>("id").to_string(),
            row.get::<String, _>("fecha_registro"),
            row.get::<String, _>("ventana_horaria"),
            row.get::<String, _>("area_nombre"),
            row.try_get::<String, _>("termometro_nombre")
                .unwrap_or_else(|_| format!("ID: {}", row.get::<i64, _>("termometro_id"))),
            row.get::<String, _>("tipo_nombre"),
            row.get::<f64, _>("temp_maxima").to_string(),
            row.get::<f64, _>("temp_minima").to_string(),
            humedad.map(|h| h.to_string()).unwrap_or_else(|| "-".to_string()),
            if row.get::<bool, _>("fuera_rango_operativo") { "Sí" } else { "No" }.to_string(),
            observaciones.unwrap_or_else(|| "-".to_string()),
            row.get::<String, _>("usuario_nombre"),
        ]).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let data = wtr.into_inner().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::OK, data))
}

fn generar_csv_mensual(rows: Vec<sqlx::sqlite::SqliteRow>, mes: u32, anio: i32) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    generar_csv_diario(rows, &format!("{}-{:02}", anio, mes))
}

fn generar_pdf_diario(rows: Vec<sqlx::sqlite::SqliteRow>, fecha: &str) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    use sqlx::Row;

    // Crear documento PDF en orientación HORIZONTAL (Landscape)
    let (doc, page1, layer1) = PdfDocument::new("Reporte de Temperaturas", Mm(297.0), Mm(210.0), "Capa 1");
    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Fuentes
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let font_regular = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Título y fecha
    current_layer.use_text("REPORTE DE CONTROL DE TEMPERATURAS", 14.0, Mm(10.0), Mm(195.0), &font_bold);
    current_layer.use_text(&format!("Período: {}", fecha), 11.0, Mm(10.0), Mm(188.0), &font_regular);

    // Encabezados de tabla (más columnas aprovechando el ancho)
    let mut y_pos = 175.0;
    current_layer.use_text("ID", 9.0, Mm(5.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Fecha/Hora", 9.0, Mm(15.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Ventana", 9.0, Mm(50.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Área", 9.0, Mm(70.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Termómetro", 9.0, Mm(105.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Tipo", 9.0, Mm(145.0), Mm(y_pos), &font_bold);
    current_layer.use_text("T.Máx", 9.0, Mm(170.0), Mm(y_pos), &font_bold);
    current_layer.use_text("T.Mín", 9.0, Mm(185.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Hum.", 9.0, Mm(200.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Estado", 9.0, Mm(215.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Usuario", 9.0, Mm(235.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Observaciones", 9.0, Mm(260.0), Mm(y_pos), &font_bold);

    // Línea separadora
    y_pos -= 2.0;

    // Datos
    y_pos -= 5.0;
    for row in rows.iter().take(40) { // Más registros por página en landscape
        let id: i64 = row.get("id");
        let fecha_registro: String = row.get("fecha_registro");
        let ventana: String = row.get("ventana_horaria");
        let area: String = row.get("area_nombre");
        let termo: String = row.try_get::<String, _>("termometro_nombre")
            .unwrap_or_else(|_| format!("ID: {}", row.get::<i64, _>("termometro_id")));
        let tipo: String = row.get("tipo_nombre");
        let temp_max: f64 = row.get("temp_maxima");
        let temp_min: f64 = row.get("temp_minima");
        let humedad: Option<f64> = row.try_get("humedad").ok();
        let fuera_rango: bool = row.get("fuera_rango_operativo");
        let usuario: String = row.get("usuario_nombre");
        let observaciones: Option<String> = row.try_get("observaciones").ok();

        // Fecha corta (solo fecha, sin hora)
        let fecha_corta = if fecha_registro.len() > 10 {
            &fecha_registro[..10]
        } else {
            &fecha_registro
        };

        // Observaciones cortas
        let obs_corta = observaciones
            .as_ref()
            .map(|o| if o.len() > 20 { format!("{}...", &o[..17]) } else { o.clone() })
            .unwrap_or_else(|| "-".to_string());

        // Estado visual
        let estado = if fuera_rango { "⚠ Alert" } else { "✓ OK" };

        current_layer.use_text(&id.to_string(), 8.0, Mm(5.0), Mm(y_pos), &font_regular);
        current_layer.use_text(fecha_corta, 8.0, Mm(15.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&ventana, 8.0, Mm(50.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&area, 8.0, Mm(70.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&termo, 8.0, Mm(105.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&tipo, 7.0, Mm(145.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&format!("{:.1}°C", temp_max), 8.0, Mm(170.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&format!("{:.1}°C", temp_min), 8.0, Mm(185.0), Mm(y_pos), &font_regular);
        current_layer.use_text(
            &humedad.map(|h| format!("{:.1}%", h)).unwrap_or_else(|| "-".to_string()),
            8.0, Mm(200.0), Mm(y_pos), &font_regular
        );
        current_layer.use_text(estado, 8.0, Mm(215.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&usuario, 8.0, Mm(235.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&obs_corta, 7.0, Mm(260.0), Mm(y_pos), &font_regular);

        y_pos -= 5.5;

        if y_pos < 15.0 {
            break; // Evitar salirse de la página
        }
    }

    // Pie de página
    current_layer.use_text(
        &format!("Total de registros: {} | Generado: {}",
            rows.len(),
            chrono::Local::now().format("%Y-%m-%d %H:%M")
        ),
        9.0, Mm(10.0), Mm(8.0), &font_regular
    );
    current_layer.use_text(
        "Sistema de Control de Temperaturas",
        9.0, Mm(220.0), Mm(8.0), &font_regular
    );

    // Guardar PDF en memoria
    let pdf_bytes = doc.save_to_bytes()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::OK, pdf_bytes))
}

fn generar_pdf_mensual(rows: Vec<sqlx::sqlite::SqliteRow>, mes: u32, anio: i32) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    generar_pdf_diario(rows, &format!("{}-{:02}", anio, mes))
}

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
