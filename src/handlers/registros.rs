use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use sqlx::{PgPool, Row};

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
    page: Option<u32>,
    page_size: Option<u32>,
}

pub async fn listar_registros(
    _current_user: CurrentUser,
    State(pool): State<PgPool>,
    Query(filtros): Query<FiltrosRegistros>,
) -> Result<Json<RegistrosPaginados>, StatusCode> {
    const SELECT_FIELDS: &str = r#"
        SELECT
            r.id, r.termometro_id, t.nombre as termometro_nombre,
            a.nombre as area_nombre, u.username as usuario_nombre,
            r.ventana_horaria, r.temp_actual, r.temp_maxima, r.temp_minima, r.humedad,
            r.fuera_rango_operativo, r.observaciones, r.fecha_registro
        FROM registros r
        LEFT JOIN termometros t ON r.termometro_id = t.id
        LEFT JOIN areas a ON t.area_id = a.id
        LEFT JOIN usuarios u ON r.usuario_id = u.id
    "#;

    let mut i = 0;
    let mut conditions = Vec::new();

    if let Some(f) = &filtros.fecha_desde {
        if !f.is_empty() {
            i += 1;
            conditions.push(format!("(CAST(r.fecha_registro AT TIME ZONE 'America/Santiago' AS DATE)) >= ${}::date", i));
        }
    }
    if let Some(f) = &filtros.fecha_hasta {
        if !f.is_empty() {
            i += 1;
            conditions.push(format!("(CAST(r.fecha_registro AT TIME ZONE 'America/Santiago' AS DATE)) <= ${}::date", i));
        }
    }
    if filtros.area_id.is_some() {
        i += 1;
        conditions.push(format!("t.area_id = ${}", i));
    }
    if let Some(ventana) = &filtros.ventana_horaria {
        if !ventana.is_empty() {
            i += 1;
            conditions.push(format!("r.ventana_horaria = ${}", i));
        }
    }

    let where_clause = if conditions.is_empty() {
        String::from("")
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    // Total de registros que cumplen los filtros
    let count_query = format!(
        "SELECT COUNT(*)::bigint AS total FROM registros r \
         LEFT JOIN termometros t ON r.termometro_id = t.id \
         LEFT JOIN areas a ON t.area_id = a.id \
         LEFT JOIN usuarios u ON r.usuario_id = u.id{}",
        where_clause
    );

    let mut count_q = sqlx::query(&count_query);
    if let Some(f) = &filtros.fecha_desde { if !f.is_empty() { count_q = count_q.bind(f); } }
    if let Some(f) = &filtros.fecha_hasta { if !f.is_empty() { count_q = count_q.bind(f); } }
    if let Some(area) = filtros.area_id { count_q = count_q.bind(area as i32); }
    if let Some(ventana) = &filtros.ventana_horaria { if !ventana.is_empty() { count_q = count_q.bind(ventana); } }

    let total: i64 = count_q
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Error SQL count listar_registros: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .get("total");

    // Paginación
    let page_size = filtros.page_size.unwrap_or(50).clamp(1, crate::handlers::MAX_REGISTROS_POR_PAGINA as u32);
    let page = filtros.page.unwrap_or(1).max(1);
    let offset = (page - 1) * page_size;
    let total_paginas = ((total as f64) / (page_size as f64)).ceil().max(1.0) as u32;

    let select_query = format!(
        "{}{} ORDER BY r.fecha_registro DESC, r.id DESC LIMIT ${} OFFSET ${}",
        SELECT_FIELDS,
        where_clause,
        i + 1,
        i + 2
    );

    let mut q = sqlx::query_as(&select_query);
    if let Some(f) = &filtros.fecha_desde { if !f.is_empty() { q = q.bind(f); } }
    if let Some(f) = &filtros.fecha_hasta { if !f.is_empty() { q = q.bind(f); } }
    if let Some(area) = filtros.area_id { q = q.bind(area as i32); }
    if let Some(ventana) = &filtros.ventana_horaria { if !ventana.is_empty() { q = q.bind(ventana); } }
    q = q.bind(page_size as i32).bind(offset as i32);

    let registros: Vec<RegistroConDetalles> = q
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Error SQL listar_registros: {:?}", e);
            tracing::error!("Query: {}", select_query);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::debug!("listar_registros página {} con {} registros (total {})", page, registros.len(), total);

    Ok(Json(RegistrosPaginados {
        registros,
        total,
        pagina: page,
        page_size,
        total_paginas,
    }))
}

pub async fn obtener_pendientes_area(
    _current_user: CurrentUser,
    State(pool): State<PgPool>,
    Path(area_id): Path<i64>,
) -> Result<Json<PendientesResponse>, StatusCode> {
    // 1. Obtener área (Simple)
    let area: Area = sqlx::query_as("SELECT * FROM areas WHERE id = $1")
        .bind(area_id as i32)
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Error buscando área {}: {:?}", area_id, e);
            StatusCode::NOT_FOUND
        })?;

    // 2. Obtener configuración
    let hora_1 = get_config(&pool, "registro_hora_1").await.unwrap_or_else(|_| "14:00".to_string());
    let hora_2 = get_config(&pool, "registro_hora_2").await.unwrap_or_else(|_| "02:00".to_string());
    let tolerancia: i32 = get_config(&pool, "ventana_tolerancia_minutos").await.unwrap_or_else(|_| "119".to_string()).parse().unwrap_or(119);
    
    // Determinar ventana (Usa lógica de Rust, no de DB)
    let ventana = determinar_ventana_actual(&hora_1, &hora_2, tolerancia, false)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::BAD_REQUEST)?;

    // 3. Obtener termómetros
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
        WHERE t.area_id = $1 AND t.activo = TRUE
        ORDER BY t.id
        "#
    )
    .bind(area_id as i32)
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Error SQL Termómetros: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // 4. Obtener registros de hoy (Comparación de fecha ultra simple)
    let registros_result: Result<Vec<RegistroConDetalles>, _> = sqlx::query_as(
        r#"
        SELECT
            r.id, r.termometro_id, t.nombre as termometro_nombre,
            a.nombre as area_nombre, u.username as usuario_nombre,
            r.ventana_horaria, r.temp_actual, r.temp_maxima, r.temp_minima, r.humedad,
            r.fuera_rango_operativo, r.observaciones, r.fecha_registro
        FROM registros r
        LEFT JOIN termometros t ON r.termometro_id = t.id
        LEFT JOIN areas a ON t.area_id = a.id
        LEFT JOIN usuarios u ON r.usuario_id = u.id
        WHERE t.area_id = $1 
          AND r.ventana_horaria = $2
          AND (r.fecha_registro AT TIME ZONE 'America/Santiago')::date = (CURRENT_TIMESTAMP AT TIME ZONE 'America/Santiago')::date
        "#
    )
    .bind(area_id as i32)
    .bind(&ventana.nombre)
    .fetch_all(&pool)
    .await;

    let registros = registros_result.unwrap_or_default();

    let registrados_ids: Vec<i32> = registros.iter().map(|r| r.termometro_id).collect();
    let pendientes: Vec<TermometroConDetalles> = termometros
        .clone()
        .into_iter()
        .filter(|t| !t.fuera_de_servicio && !registrados_ids.contains(&t.id))
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
    State(pool): State<PgPool>,
    Json(payload): Json<CrearRegistroRequest>,
) -> Result<Json<Registro>, (StatusCode, String)> {
    // Obtener configuración
    let hora_1 = get_config(&pool, "registro_hora_1").await.unwrap_or_else(|_| "14:00".to_string());
    let hora_2 = get_config(&pool, "registro_hora_2").await.unwrap_or_else(|_| "02:00".to_string());
    let tolerancia: i32 = get_config(&pool, "ventana_tolerancia_minutos").await
        .unwrap_or_else(|_| "119".to_string())
        .parse()
        .unwrap_or(119);
    let restriccion_activa = get_config(&pool, "restriccion_ventana_activa").await.unwrap_or_else(|_| "0".to_string()) == "1";

    // Verificar ventana
    let ventana = determinar_ventana_actual(&hora_1, &hora_2, tolerancia, restriccion_activa)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error ventana: {}", e)))?
        .ok_or((StatusCode::BAD_REQUEST, "No hay ventana horaria activa actualmente.".to_string()))?;

    // Obtener tipo de termómetro para validación
    let termometro: Termometro = sqlx::query_as("SELECT * FROM termometros WHERE id = $1")
        .bind(payload.termometro_id as i32)
        .fetch_one(&pool)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Termómetro no encontrado".to_string()))?;

    let tipo: TipoTermometro = sqlx::query_as("SELECT * FROM tipos_termometro WHERE id = $1")
        .bind(termometro.tipo_id as i32)
        .fetch_one(&pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error obteniendo tipo termómetro".to_string()))?;

    // Validar
    let (fuera_rango_operativo, _advertencias) = validar_registro(
        payload.temp_maxima,
        payload.temp_minima,
        payload.humedad,
        &tipo,
    )
    .map_err(|e| (StatusCode::BAD_REQUEST, format!("Error de validación: {}", e)))?;

    // Insertar y obtener registro completo
    let registro: Registro = sqlx::query_as(
        r#"
        INSERT INTO registros (
            termometro_id, usuario_id, ventana_horaria,
            temp_actual, temp_maxima, temp_minima, humedad,
            fuera_rango_operativo, observaciones
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#
    )
    .bind(payload.termometro_id as i32)
    .bind(current_user.0.id as i32)
    .bind(&ventana.nombre)
    .bind(payload.temp_actual)
    .bind(payload.temp_maxima)
    .bind(payload.temp_minima)
    .bind(payload.humedad)
    .bind(fuera_rango_operativo)
    .bind(&payload.observaciones)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unique_per_day") {
            (StatusCode::CONFLICT, "Ya existe un registro para este termómetro en el día de hoy.".to_string())
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error al guardar: {}", e))
        }
    })?;

    let registro_id = registro.id as i32;

    // Log de auditoría
    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
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
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
    Json(payload): Json<ActualizarRegistroRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Obtener registro existente
    let registro: Registro = sqlx::query_as("SELECT * FROM registros WHERE id = $1")
        .bind(id as i32)
        .fetch_one(&pool)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Registro no encontrado".to_string()))?;

    // Si no es admin, solo puede editar sus propios registros
    if current_user.0.rol != "ADMINISTRADOR" && registro.usuario_id != current_user.0.id as i32 {
        return Err((StatusCode::FORBIDDEN, "No tienes permiso para editar este registro".to_string()));
    }

    // Obtener tipo para validación
    let termometro: Termometro = sqlx::query_as("SELECT * FROM termometros WHERE id = $1")
        .bind(registro.termometro_id as i32)
        .fetch_one(&pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error obteniendo termómetro".to_string()))?;

    let tipo: TipoTermometro = sqlx::query_as("SELECT * FROM tipos_termometro WHERE id = $1")
        .bind(termometro.tipo_id as i32)
        .fetch_one(&pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error obteniendo tipo".to_string()))?;

    // Usar valores actuales si no se proporcionan nuevos
    let temp_actual = payload.temp_actual.or(registro.temp_actual);
    let temp_max = payload.temp_maxima.unwrap_or(registro.temp_maxima);
    let temp_min = payload.temp_minima.unwrap_or(registro.temp_minima);
    let humedad = payload.humedad.or(registro.humedad);

    // Validar
    let (fuera_rango_operativo, _) = validar_registro(temp_max, temp_min, humedad, &tipo)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Error de validación: {}", e)))?;

    // Actualizar
    sqlx::query(
        r#"
        UPDATE registros SET
            temp_actual = $1, temp_maxima = $2, temp_minima = $3, humedad = $4,
            fuera_rango_operativo = $5, observaciones = $6
        WHERE id = $7
        "#
    )
    .bind(temp_actual)
    .bind(temp_max)
    .bind(temp_min)
    .bind(humedad)
    .bind(fuera_rango_operativo)
    .bind(&payload.observaciones)
    .bind(id as i32)
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error al actualizar: {}", e)))?;

    // Log de auditoría
    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "UPDATE",
        "registros",
        Some(id as i32),
        Some(&serde_json::to_string(&registro).unwrap_or_default()),
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}

pub async fn eliminar_registro(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    // Solo admin puede eliminar registros
    if current_user.0.rol != "ADMINISTRADOR" {
        return Err(StatusCode::FORBIDDEN);
    }

    sqlx::query("DELETE FROM registros WHERE id = $1")
        .bind(id as i32)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "DELETE",
        "registros",
        Some(id as i32),
        None,
        None,
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}
