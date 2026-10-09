use axum::{extract::State, http::StatusCode, Json};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sqlx::PgPool;

use crate::{
    auth::CurrentUser,
    db::{get_config, log_auditoria, set_config},
    logic::estado_ventana_actual,
    models::*,
};

/// GET /api/mapa — vista espacial para cualquier rol autenticado.
///
/// Clasifica cada termómetro activo EXACTAMENTE igual que
/// `obtener_pendientes_area`: fuera_de_servicio primero, si no registrado en
/// la ventana actual -> completado, si no -> pendiente. Reusa los mismos
/// helpers (`get_config`, `estado_ventana_actual`); no hay matemática de
/// tiempo nueva aquí.
pub async fn obtener_mapa(
    _current_user: CurrentUser,
    State(pool): State<PgPool>,
) -> Result<Json<MapaResponse>, StatusCode> {
    let habilitado = get_config(&pool, "mapa_habilitado")
        .await
        .map(|v| v == "1")
        .unwrap_or(false);
    let imagen_url = match get_config(&pool, "mapa_imagen_path").await {
        Ok(p) if !p.trim().is_empty() => Some(p),
        _ => None,
    };

    // Ventana actual con los mismos helpers que obtener_pendientes_area.
    let hora_1 = get_config(&pool, "registro_hora_1")
        .await
        .unwrap_or_else(|_| "14:00".to_string());
    let hora_2 = get_config(&pool, "registro_hora_2")
        .await
        .unwrap_or_else(|_| "02:00".to_string());
    let tolerancia: i32 = get_config(&pool, "ventana_tolerancia_minutos")
        .await
        .unwrap_or_else(|_| "119".to_string())
        .parse()
        .unwrap_or(119);
    let restriccion_activa = get_config(&pool, "restriccion_ventana_activa")
        .await
        .unwrap_or_else(|_| "0".to_string())
        == "1";
    let estado = estado_ventana_actual(&hora_1, &hora_2, tolerancia, restriccion_activa)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let areas_db: Vec<Area> = sqlx::query_as("SELECT * FROM areas WHERE activa = TRUE ORDER BY id")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let areas = areas_db
        .into_iter()
        .map(|a| {
            let zona =
                match (a.zona_x, a.zona_y, a.zona_w, a.zona_h) {
                    (Some(x), Some(y), Some(w), Some(h)) => Some(ZonaRect { x, y, w, h }),
                    _ => None,
                };
            MapaArea {
                id: a.id,
                nombre: a.nombre,
                zona,
            }
        })
        .collect();

    // Misma lista de columnas que obtener_pendientes_area, sin filtro de área.
    let termometros: Vec<TermometroConDetalles> = sqlx::query_as(
        r#"
        SELECT
            t.id, t.area_id, a.nombre as area_nombre,
            t.tipo_id, ti.nombre as tipo_nombre, ti.tiene_humedad, ti.solo_un_punto,
            ti.temp_min_operativa, ti.temp_max_operativa,
            t.nombre, t.ubicacion, t.activo, t.fuera_de_servicio, t.pos_x, t.pos_y
        FROM termometros t
        JOIN areas a ON t.area_id = a.id
        JOIN tipos_termometro ti ON t.tipo_id = ti.id
        WHERE t.activo = TRUE
        ORDER BY t.id
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Mismo día OPERATIVO (08:00 → 08:00) que obtener_pendientes_area.
    let registros: Vec<RegistroConDetalles> = sqlx::query_as(
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
        WHERE r.ventana_horaria = $1
          AND ((r.fecha_registro AT TIME ZONE 'America/Santiago') - INTERVAL '8 hours')::date
            = ((CURRENT_TIMESTAMP AT TIME ZONE 'America/Santiago') - INTERVAL '8 hours')::date
        "#,
    )
    .bind(&estado.ventana.nombre)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let registrados_ids: Vec<i32> = registros.iter().map(|r| r.termometro_id).collect();
    let termometros = termometros
        .into_iter()
        .map(|t| {
            let estado_term = if t.fuera_de_servicio {
                "fuera_de_servicio"
            } else if registrados_ids.contains(&t.id) {
                "completado"
            } else {
                "pendiente"
            };
            MapaTermometro {
                id: t.id,
                area_id: t.area_id,
                nombre: t.nombre,
                pos_x: t.pos_x,
                pos_y: t.pos_y,
                estado: estado_term.to_string(),
            }
        })
        .collect();

    Ok(Json(MapaResponse {
        habilitado,
        imagen_url,
        areas,
        termometros,
    }))
}

/// PUT /api/admin/mapa/config — toggle `mapa_habilitado` (admin, vía ruta).
pub async fn actualizar_config_mapa(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<MapaConfigRequest>,
) -> Result<StatusCode, StatusCode> {
    let valor = if payload.mapa_habilitado { "1" } else { "0" };
    set_config(&pool, "mapa_habilitado", valor)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
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

const MAX_FOTO_BYTES: usize = 5 * 1024 * 1024;
const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
const JPG_MAGIC: [u8; 3] = [0xFF, 0xD8, 0xFF];

/// POST /api/admin/mapa/foto — reemplaza la foto del plano (admin, vía ruta).
///
/// Base64 JSON (sin multipart: Axum no trae esa feature). Valida PNG/JPG por
/// magic bytes y tamaño ya decodificado ≤5MB; escribe
/// `public/uploads/mapa.{png,jpg}` y actualiza `mapa_imagen_path`.
pub async fn subir_foto_mapa(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<FotoRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Tolerar prefijo data-URL ("data:image/png;base64,...") si el cliente lo manda.
    let b64 = match payload.content_base64.find(',') {
        Some(i) if payload.content_base64[..i].contains("base64") => &payload.content_base64[i + 1..],
        _ => payload.content_base64.as_str(),
    };
    let bytes = BASE64
        .decode(b64.trim())
        .map_err(|_| (StatusCode::BAD_REQUEST, "Base64 inválido".to_string()))?;
    if bytes.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Imagen vacía".to_string()));
    }
    if bytes.len() > MAX_FOTO_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "La imagen supera los 5MB".to_string(),
        ));
    }
    let ext = if bytes.starts_with(&PNG_MAGIC) {
        "png"
    } else if bytes.starts_with(&JPG_MAGIC) {
        "jpg"
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            "Solo se admite PNG o JPG".to_string(),
        ));
    };

    tokio::fs::create_dir_all("public/uploads")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error creando uploads: {}", e)))?;

    let otra_ext = if ext == "png" { "jpg" } else { "png" };
    // Si se cambia de formato, borrar el archivo anterior para no servir uno obsoleto.
    let _ = tokio::fs::remove_file(format!("public/uploads/mapa.{}", otra_ext)).await;
    tokio::fs::write(format!("public/uploads/mapa.{}", ext), &bytes)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error guardando imagen: {}", e)))?;

    let imagen_path = format!("/uploads/mapa.{}", ext);
    set_config(&pool, "mapa_imagen_path", &imagen_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error guardando config: {}", e)))?;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "UPDATE",
        "configuracion",
        None,
        None,
        Some(&serde_json::json!({ "mapa_imagen_path": imagen_path, "filename": payload.filename }).to_string()),
    )
    .await
    .ok();

    Ok(Json(serde_json::json!({ "imagen_url": imagen_path })))
}

fn en_rango_0_100(v: f32) -> bool {
    v.is_finite() && (0.0..=100.0).contains(&v)
}

fn punto_en_zona(x: f32, y: f32, z: &ZonaRect) -> bool {
    x >= z.x && x <= z.x + z.w && y >= z.y && y <= z.y + z.h
}

/// PUT /api/admin/mapa/posiciones — guardado batch de zonas y puntos (admin, vía ruta).
///
/// Si arrastrar un punto lo deja dentro de la zona de OTRA área, eso reasigna
/// su `area_id` (los reportes históricos resuelven `area_nombre` por JOIN
/// actual). Sin `confirmar_cambio_area` responde 409 con el detalle y NO
/// escribe nada; con confirmación aplica todo en una sola transacción.
pub async fn guardar_posiciones(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<GuardarPosicionesRequest>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let err = |status: StatusCode, msg: &str| {
        (
            status,
            Json(serde_json::json!({ "error": msg })),
        )
    };

    for t in &payload.termometros {
        if !en_rango_0_100(t.pos_x) || !en_rango_0_100(t.pos_y) {
            return Err(err(
                StatusCode::BAD_REQUEST,
                &format!("Posición del termómetro {} fuera de 0-100", t.id),
            ));
        }
    }
    for a in &payload.areas {
        if let Some(z) = &a.zona {
            if !en_rango_0_100(z.x) || !en_rango_0_100(z.y) || !en_rango_0_100(z.w) || !en_rango_0_100(z.h) {
                return Err(err(
                    StatusCode::BAD_REQUEST,
                    &format!("Zona del área {} fuera de 0-100", a.id),
                ));
            }
        }
    }

    // Zonas vigentes tras aplicar el batch: las pedidas pisan a las de la DB.
    let zonas_db: Vec<(i32, Option<f32>, Option<f32>, Option<f32>, Option<f32>)> =
        sqlx::query_as("SELECT id, zona_x, zona_y, zona_w, zona_h FROM areas ORDER BY id")
            .fetch_all(&pool)
            .await
            .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Error leyendo áreas"))?;
    let mut zonas: std::collections::HashMap<i32, Option<ZonaRect>> = zonas_db
        .into_iter()
        .map(|(id, x, y, w, h)| {
            let z = match (x, y, w, h) {
                (Some(x), Some(y), Some(w), Some(h)) => Some(ZonaRect { x, y, w, h }),
                _ => None,
            };
            (id, z)
        })
        .collect();
    for a in &payload.areas {
        if !zonas.contains_key(&a.id) {
            return Err(err(StatusCode::NOT_FOUND, &format!("Área {} no existe", a.id)));
        }
        zonas.insert(a.id, a.zona.clone());
    }
    // Hit-test determinista: primera zona (por id) que contenga el punto.
    let mut zona_ids: Vec<i32> = zonas.keys().copied().collect();
    zona_ids.sort_unstable();
    let area_implicada = |x: f32, y: f32| -> Option<i32> {
        zona_ids.iter().copied().find(|id| match &zonas[id] {
            Some(z) => punto_en_zona(x, y, z),
            None => false,
        })
    };

    // Área actual de cada termómetro pedido.
    let ids: Vec<i32> = payload.termometros.iter().map(|t| t.id).collect();
    let actuales: Vec<(i32, i32)> = if ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as("SELECT id, area_id FROM termometros WHERE id = ANY($1)")
            .bind(&ids)
            .fetch_all(&pool)
            .await
            .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Error leyendo termómetros"))?
    };
    if actuales.len() != ids.len() {
        return Err(err(StatusCode::NOT_FOUND, "Algún termómetro no existe"));
    }
    let area_actual: std::collections::HashMap<i32, i32> = actuales.into_iter().collect();

    let mut cambios = Vec::new();
    for t in &payload.termometros {
        let de = area_actual[&t.id];
        let a = area_implicada(t.pos_x, t.pos_y).unwrap_or(de);
        if a != de {
            cambios.push(serde_json::json!({
                "termometro_id": t.id,
                "de_area_id": de,
                "a_area_id": a,
            }));
        }
    }
    if !cambios.is_empty() && !payload.confirmar_cambio_area {
        // 409 SIN escribir nada: el admin debe confirmar el cambio de área.
        return Err((StatusCode::CONFLICT, Json(serde_json::json!({ "cambios": cambios }))));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Error iniciando transacción"))?;
    for a in &payload.areas {
        let (x, y, w, h) = match &a.zona {
            Some(z) => (Some(z.x), Some(z.y), Some(z.w), Some(z.h)),
            None => (None, None, None, None),
        };
        sqlx::query("UPDATE areas SET zona_x = $1, zona_y = $2, zona_w = $3, zona_h = $4 WHERE id = $5")
            .bind(x)
            .bind(y)
            .bind(w)
            .bind(h)
            .bind(a.id)
            .execute(&mut *tx)
            .await
            .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Error guardando zonas"))?;
    }
    for t in &payload.termometros {
        let de = area_actual[&t.id];
        let a = area_implicada(t.pos_x, t.pos_y).unwrap_or(de);
        sqlx::query("UPDATE termometros SET pos_x = $1, pos_y = $2, area_id = $3 WHERE id = $4")
            .bind(t.pos_x)
            .bind(t.pos_y)
            .bind(a)
            .bind(t.id)
            .execute(&mut *tx)
            .await
            .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Error guardando posiciones"))?;
    }
    tx.commit()
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "Error confirmando transacción"))?;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "UPDATE",
        "mapa_posiciones",
        None,
        None,
        Some(&serde_json::to_string(&payload).unwrap_or_default()),
    )
    .await
    .ok();

    Ok(StatusCode::OK)
}
