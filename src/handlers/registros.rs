use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Local;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    auth::CurrentUser,
    db::{get_config, log_auditoria},
    logic::{determinar_ventana_actual, validar_registro},
    models::*,
};

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
            r.ventana_horaria, r.temp_actual, r.temp_maxima, r.temp_minima, r.humedad,
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

    query.push_str(&format!(" ORDER BY r.fecha_registro DESC, r.id DESC LIMIT {}", super::MAX_REGISTROS_POR_PAGINA));

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
        .map_err(|e| {
            tracing::error!("Error al cargar registros: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

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
            r.ventana_horaria, r.temp_actual, r.temp_maxima, r.temp_minima, r.humedad,
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
            temp_actual, temp_maxima, temp_minima, humedad,
            fuera_rango_operativo, observaciones
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(payload.termometro_id)
    .bind(current_user.0.id)
    .bind(&ventana.nombre)
    .bind(payload.temp_actual)
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
    let temp_actual = payload.temp_actual.or(registro.temp_actual);
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
            temp_actual = ?, temp_maxima = ?, temp_minima = ?, humedad = ?,
            fuera_rango_operativo = ?, observaciones = ?
        WHERE id = ?
        "#
    )
    .bind(temp_actual)
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
