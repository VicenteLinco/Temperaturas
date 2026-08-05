use axum::{
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use sqlx::{PgPool, postgres::PgRow, Row};
use printpdf::*;

use crate::{
    auth::CurrentUser,
    db::log_auditoria,
    models::*,
};

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

#[derive(Deserialize)]
pub struct FiltrosInformeDia {
    formato: String,
    fecha: Option<String>,
    area_id: Option<i64>,
}

/// Construye las filas del reporte diario (compartido entre admin y registrador)
async fn construir_registros_diario(
    pool: &PgPool,
    fecha: &str,
    area_id: Option<i64>,
) -> Result<Vec<PgRow>, StatusCode> {
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
        WHERE (r.fecha_registro AT TIME ZONE 'America/Santiago')::date = CAST($1 AS DATE)
        "#
    );

    if area_id.is_some() {
        query.push_str(" AND t.area_id = $2");
    }

    query.push_str(" ORDER BY a.nombre, t.nombre, r.ventana_horaria");

    let mut q = sqlx::query(&query).bind(fecha);

    if let Some(area_id) = area_id {
        q = q.bind(area_id as i32);
    }

    q.fetch_all(pool)
        .await
        .map_err(|e| {
            tracing::error!("Error reporte diario: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn generar_reporte_diario(
    _current_user: CurrentUser,
    State(pool): State<PgPool>,
    Query(filtros): Query<FiltrosReporteDiario>,
) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    let rows = construir_registros_diario(&pool, &filtros.fecha, filtros.area_id).await?;

    if filtros.formato == "csv" {
        generar_csv_diario(rows, &filtros.fecha)
    } else if filtros.formato == "pdf" {
        generar_pdf_diario(rows, &filtros.fecha)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// Informe del día para registradores (generado al finalizar el registro diario)
pub async fn generar_informe_dia(
    _current_user: CurrentUser,
    State(pool): State<PgPool>,
    Query(filtros): Query<FiltrosInformeDia>,
) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    let fecha = filtros.fecha.unwrap_or_else(|| {
        chrono::Utc::now()
            .with_timezone(&chrono_tz::America::Santiago)
            .format("%Y-%m-%d")
            .to_string()
    });

    let rows = construir_registros_diario(&pool, &fecha, filtros.area_id).await?;

    if filtros.formato == "csv" {
        generar_csv_diario(rows, &fecha)
    } else if filtros.formato == "pdf" {
        generar_pdf_diario(rows, &fecha)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

pub async fn generar_reporte_mensual(
    _current_user: CurrentUser,
    State(pool): State<PgPool>,
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
        WHERE EXTRACT(YEAR FROM r.fecha_registro) = $1 AND EXTRACT(MONTH FROM r.fecha_registro) = $2
        "#
    );

    if filtros.area_id.is_some() {
        query.push_str(" AND t.area_id = $3");
    }

    query.push_str(" ORDER BY r.fecha_registro, 
        CASE 
            WHEN r.ventana_horaria = '02:00' THEN 1 
            WHEN r.ventana_horaria = '14:00' THEN 2 
            ELSE 3 
        END, 
        a.nombre, t.nombre");

    let mut q = sqlx::query(&query)
        .bind(filtros.anio as f64)
        .bind(filtros.mes as f64);

    if let Some(area_id) = filtros.area_id {
        q = q.bind(area_id as i32);
    }

    let rows = q
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Error reporte mensual: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if filtros.formato == "csv" {
        generar_csv_mensual(rows, filtros.mes, filtros.anio)
    } else if filtros.formato == "pdf" {
        generar_pdf_mensual(rows, filtros.mes, filtros.anio)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

fn generar_csv_diario(rows: Vec<PgRow>, _fecha: &str) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    let mut wtr = csv::Writer::from_writer(vec![]);

    // Encabezados
    wtr.write_record(&[
        "ID", "Fecha", "Ventana", "Área", "Termómetro", "Tipo",
        "Temp. Máx", "Temp. Mín", "Humedad", "Fuera Rango", "Observaciones", "Usuario"
    ]).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Datos
    for row in rows {
        let humedad: Option<f64> = row.try_get::<Option<f64>, _>("humedad").unwrap_or(None);
        let observaciones: Option<String> = row.try_get::<Option<String>, _>("observaciones").unwrap_or(None);
        let fecha_registro: chrono::DateTime<chrono::Utc> = row.get("fecha_registro");
        
        let termometro_id: i32 = row.get("termometro_id");
        let termometro_nombre: String = row.try_get::<String, _>("termometro_nombre").unwrap_or_default();
        let termo_display = if !termometro_nombre.is_empty() {
            format!("{}({})", termometro_id, termometro_nombre)
        } else {
            termometro_id.to_string()
        };

        wtr.write_record(&[
            row.get::<i32, _>("id").to_string(),
            fecha_registro.to_rfc3339(),
            row.get::<String, _>("ventana_horaria"),
            row.get::<String, _>("area_nombre"),
            termo_display,
            row.get::<String, _>("tipo_nombre"),
            row.get::<f32, _>("temp_maxima").to_string(),
            row.get::<f32, _>("temp_minima").to_string(),
            humedad.map(|h| h.to_string()).unwrap_or_else(|| "-".to_string()),
            if row.get::<bool, _>("fuera_rango_operativo") { "Sí" } else { "No" }.to_string(),
            observaciones.unwrap_or_else(|| "-".to_string()),
            row.get::<String, _>("usuario_nombre"),
        ]).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let mut data = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
    let csv_bytes = wtr.into_inner().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    data.extend_from_slice(&csv_bytes);
    
    Ok((StatusCode::OK, data))
}

fn generar_csv_mensual(rows: Vec<PgRow>, mes: u32, anio: i32) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    generar_csv_diario(rows, &periodo_mensual(mes, anio))
}

const MESES_ES: [&str; 12] = [
    "Enero", "Febrero", "Marzo", "Abril", "Mayo", "Junio",
    "Julio", "Agosto", "Septiembre", "Octubre", "Noviembre", "Diciembre",
];

/// Etiqueta legible del período mensual, ej: "Julio 2026"
fn periodo_mensual(mes: u32, anio: i32) -> String {
    let idx = (mes.saturating_sub(1)) as usize;
    let nombre = if idx < MESES_ES.len() { MESES_ES[idx] } else { "Mes" };
    format!("{} {}", nombre, anio)
}

fn generar_pdf_diario(rows: Vec<PgRow>, fecha: &str) -> Result<(StatusCode, Vec<u8>), StatusCode> {
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

    // Encabezados de tabla
    let mut y_pos = 175.0;
    current_layer.use_text("ID", 9.0, Mm(5.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Fecha", 9.0, Mm(12.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Ventana", 9.0, Mm(32.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Área", 9.0, Mm(47.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Termómetro", 9.0, Mm(80.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Tipo", 9.0, Mm(140.0), Mm(y_pos), &font_bold);
    current_layer.use_text("T.Máx", 9.0, Mm(165.0), Mm(y_pos), &font_bold);
    current_layer.use_text("T.Mín", 9.0, Mm(180.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Hum.", 9.0, Mm(195.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Estado", 9.0, Mm(210.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Usuario", 9.0, Mm(230.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Obs.", 9.0, Mm(255.0), Mm(y_pos), &font_bold);

    // Datos
    y_pos -= 10.0;
    for row in rows.iter().take(300) {
        let id: i32 = row.get("id");
        let fecha_registro: chrono::DateTime<chrono::Utc> = row.get("fecha_registro");
        let ventana: String = row.get("ventana_horaria");
        let area: String = row.get("area_nombre");
        
        let termometro_id: i32 = row.get("termometro_id");
        let termometro_nombre: String = row.try_get::<String, _>("termometro_nombre").unwrap_or_default();
        let mut termo = if !termometro_nombre.is_empty() {
            format!("{}({})", termometro_id, termometro_nombre)
        } else {
            termometro_id.to_string()
        };

        // Truncar solo si es extremadamente largo (para una columna de ~60mm)
        if termo.len() > 45 {
            termo = format!("{}...", &termo[..42]);
        }

        let tipo: String = row.get("tipo_nombre");
        let temp_max: f32 = row.get("temp_maxima");
        let temp_min: f32 = row.get("temp_minima");
        let humedad: Option<f64> = row.try_get::<Option<f64>, _>("humedad").unwrap_or(None);
        let fuera_rango: bool = row.get("fuera_rango_operativo");
        let usuario: String = row.get("usuario_nombre");
        let observaciones: Option<String> = row.try_get::<Option<String>, _>("observaciones").unwrap_or(None);

        let fecha_str = fecha_registro.format("%Y-%m-%d").to_string();
        let obs_corta = observaciones
            .map(|o| if o.len() > 30 { format!("{}...", &o[..27]) } else { o.clone() })
            .unwrap_or_else(|| "-".to_string());

        let estado = if fuera_rango { "⚠ Alert" } else { "✓ OK" };

        current_layer.use_text(&id.to_string(), 8.0, Mm(5.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&fecha_str, 8.0, Mm(12.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&ventana, 8.0, Mm(32.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&area, 8.0, Mm(47.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&termo, 8.0, Mm(80.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&tipo, 7.0, Mm(140.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&format!("{:.1}°C", temp_max), 8.0, Mm(165.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&format!("{:.1}°C", temp_min), 8.0, Mm(180.0), Mm(y_pos), &font_regular);
        current_layer.use_text(
            &humedad.map(|h| format!("{:.1}%", h)).unwrap_or_else(|| "-".to_string()),
            8.0, Mm(195.0), Mm(y_pos), &font_regular
        );
        current_layer.use_text(estado, 8.0, Mm(210.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&usuario, 8.0, Mm(230.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&obs_corta, 7.0, Mm(255.0), Mm(y_pos), &font_regular);

        y_pos -= 5.5;
        if y_pos < 15.0 { break; }
    }

    let pdf_bytes = doc.save_to_bytes().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::OK, pdf_bytes))
}

fn generar_pdf_mensual(rows: Vec<PgRow>, mes: u32, anio: i32) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    generar_pdf_diario(rows, &periodo_mensual(mes, anio))
}

// ===== INFORME DE FRANJA HORARIA =====

#[derive(Deserialize)]
pub struct FiltrosInformeFranja {
    formato: Option<String>,
    fecha: Option<String>,
    ventana_horaria: Option<String>,
}

/// Fecha de hoy en zona horaria de Chile (America/Santiago)
fn fecha_hoy_santiago() -> String {
    chrono::Utc::now()
        .with_timezone(&chrono_tz::America::Santiago)
        .format("%Y-%m-%d")
        .to_string()
}

/// Consulta los datos del informe de franja horaria:
/// mediciones fuera de rango operativo y termómetros sin funcionamiento.
async fn consultar_informe_franja(
    pool: &PgPool,
    fecha: &str,
    ventana: Option<&str>,
) -> Result<InformeFranjaResponse, StatusCode> {
    // Determinar la ventana horaria si no viene indicada
    let ventana_horaria = match ventana {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => {
            let hora_1 = crate::db::get_config(pool, "registro_hora_1")
                .await
                .unwrap_or_else(|_| "14:00".to_string());
            let hora_2 = crate::db::get_config(pool, "registro_hora_2")
                .await
                .unwrap_or_else(|_| "02:00".to_string());
            let tolerancia: i32 = crate::db::get_config(pool, "ventana_tolerancia_minutos")
                .await
                .unwrap_or_else(|_| "119".to_string())
                .parse()
                .unwrap_or(119);
            crate::logic::determinar_ventana_actual(&hora_1, &hora_2, tolerancia, false)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .ok_or(StatusCode::BAD_REQUEST)?
                .nombre
        }
    };

    // Total de mediciones registradas en la franja
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM registros r
         JOIN termometros t ON r.termometro_id = t.id
         WHERE (r.fecha_registro AT TIME ZONE 'America/Santiago')::date = $1::date
           AND r.ventana_horaria = $2",
    )
    .bind(fecha)
    .bind(&ventana_horaria)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("Error total informe franja: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Registros fuera de rango operativo
    let fuera_de_rango: Vec<FueraDeRangoItem> = sqlx::query_as(
        r#"
        SELECT
            t.id as termometro_id, t.nombre as termometro_nombre,
            a.nombre as area_nombre,
            r.temp_maxima, r.temp_minima, r.humedad,
            r.observaciones, u.username as usuario_nombre
        FROM registros r
        JOIN termometros t ON r.termometro_id = t.id
        JOIN areas a ON t.area_id = a.id
        JOIN usuarios u ON r.usuario_id = u.id
        WHERE r.fuera_rango_operativo = TRUE
          AND (r.fecha_registro AT TIME ZONE 'America/Santiago')::date = $1::date
          AND r.ventana_horaria = $2
        ORDER BY a.nombre, t.nombre
        "#,
    )
    .bind(fecha)
    .bind(&ventana_horaria)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Error fuera de rango informe franja: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Termómetros sin funcionamiento (reporte de mantenimiento pendiente)
    let fuera_de_servicio: Vec<FueraDeServicioItem> = sqlx::query_as(
        r#"
        SELECT
            t.id as termometro_id, t.nombre as termometro_nombre,
            a.nombre as area_nombre,
            ti.nombre as tipo_nombre, t.ubicacion,
            m.motivo, m.comentarios_reporte, m.fecha_reporte
        FROM termometros t
        JOIN areas a ON t.area_id = a.id
        JOIN tipos_termometro ti ON t.tipo_id = ti.id
        JOIN LATERAL (
            SELECT mt.motivo, mt.comentarios_reporte, mt.fecha_reporte
            FROM mantenimiento_termometros mt
            WHERE mt.termometro_id = t.id AND mt.estado = 'PENDIENTE'
            ORDER BY mt.fecha_reporte DESC
            LIMIT 1
        ) m ON TRUE
        WHERE t.activo = TRUE AND t.fuera_de_servicio = TRUE
        ORDER BY a.nombre, t.nombre
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Error fuera de servicio informe franja: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(InformeFranjaResponse {
        fecha: fecha.to_string(),
        ventana_horaria,
        total_mediciones: total,
        fuera_de_rango,
        fuera_de_servicio,
    })
}

/// GET /api/registros/informe-franja
/// Devuelve el informe de la franja horaria en JSON (formato=pdf lo entrega como PDF).
pub async fn generar_informe_franja(
    _current_user: CurrentUser,
    State(pool): State<PgPool>,
    Query(filtros): Query<FiltrosInformeFranja>,
) -> Result<Response, StatusCode> {
    let fecha = filtros.fecha.unwrap_or_else(fecha_hoy_santiago);
    let informe = consultar_informe_franja(&pool, &fecha, filtros.ventana_horaria.as_deref()).await?;

    if filtros.formato.as_deref() == Some("pdf") {
        let (_, pdf) = generar_pdf_informe_franja(&informe)?;
        Response::builder()
            .header("Content-Type", "application/pdf")
            .header(
                "Content-Disposition",
                format!("attachment; filename=\"informe_franja_{}.pdf\"", fecha),
            )
            .body(Body::from(pdf))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    } else {
        Ok(Json(informe).into_response())
    }
}

/// POST /api/registros/enviar-informe-franja
/// Genera el informe PDF y lo envía por correo como adjunto.
pub async fn enviar_informe_franja(
    current_user: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<EnviarInformeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let email = payload.email.trim().to_string();
    if email.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Debe indicar un correo destinatario.".to_string()));
    }

    let fecha = fecha_hoy_santiago();
    let informe = consultar_informe_franja(&pool, &fecha, payload.ventana_horaria.as_deref())
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al generar el informe".to_string()))?;

    let (_, pdf) = generar_pdf_informe_franja(&informe)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al generar el PDF".to_string()))?;

    let asunto = format!(
        "Informe de franja horaria {} - Ventana {}",
        fecha, informe.ventana_horaria
    );
    let cuerpo = format!(
        "Se adjunta el informe de la franja horaria {fecha} (ventana {ventana}).\n\n\
         Total de mediciones: {total}\n\
         Registros fuera de rango operativo: {fr}\n\
         Termómetros sin funcionamiento: {fs}\n\n\
         Este mensaje fue generado automáticamente por el Sistema de Control de Temperaturas.",
        fecha = fecha,
        ventana = informe.ventana_horaria,
        total = informe.total_mediciones,
        fr = informe.fuera_de_rango.len(),
        fs = informe.fuera_de_servicio.len(),
    );
    let nombre_pdf = format!("informe_franja_{}.pdf", fecha);

    crate::mail::enviar_correo_con_pdf(&email, &asunto, &cuerpo, pdf, &nombre_pdf)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("No se pudo enviar el correo: {}", e)))?;

    log_auditoria(
        &pool,
        current_user.0.id.try_into().unwrap_or(0),
        "EMAIL",
        "informes",
        None,
        None,
        Some(&format!("Informe franja {} enviado a {}", fecha, email)),
    )
    .await
    .ok();

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Informe enviado a {}", email)
    })))
}

// ===== GENERACIÓN DE PDF DEL INFORME DE FRANJA =====

/// Recorta un texto para que quepa en una columna del PDF
fn truncar(texto: &str, max: usize) -> String {
    if texto.chars().count() > max {
        let recortado: String = texto.chars().take(max.saturating_sub(3)).collect();
        format!("{}...", recortado)
    } else {
        texto.to_string()
    }
}

/// Escritor de PDF simple: gestiona paginación automática y dibujo de texto
struct PdfEscritor {
    doc: PdfDocumentReference,
    layer: PdfLayerReference,
    y: f32,
    font_bold: IndirectFontRef,
    font_regular: IndirectFontRef,
}

impl PdfEscritor {
    fn nuevo(titulo: &str) -> Result<Self, StatusCode> {
        // Orientación vertical (portrait): 210mm x 297mm
        let (doc, page, layer) = PdfDocument::new(titulo, Mm(210.0), Mm(297.0), "Capa 1");
        let layer_ref = doc.get_page(page).get_layer(layer);
        let font_bold = doc
            .add_builtin_font(BuiltinFont::HelveticaBold)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let font_regular = doc
            .add_builtin_font(BuiltinFont::Helvetica)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Self {
            doc,
            layer: layer_ref,
            y: 285.0,
            font_bold,
            font_regular,
        })
    }

    fn nueva_pagina(&mut self) {
        let (page, layer) = self.doc.add_page(Mm(210.0), Mm(297.0), "Capa 1");
        self.layer = self.doc.get_page(page).get_layer(layer);
        self.y = 285.0;
    }

    fn asegurar_espacio(&mut self, alto: f32) {
        if self.y - alto < 15.0 {
            self.nueva_pagina();
        }
    }

    fn texto(&self, texto: &str, size: f32, x: f32, y: f32, bold: bool) {
        let font = if bold { &self.font_bold } else { &self.font_regular };
        self.layer.use_text(texto, size, Mm(x), Mm(y), font);
    }

    /// Escribe una línea en la posición actual y baja la coordenada vertical
    fn escribir_linea(&mut self, texto: &str, size: f32, x: f32, alto: f32, bold: bool) {
        self.asegurar_espacio(alto);
        self.texto(texto, size, x, self.y, bold);
        self.y -= alto;
    }

    fn guardar(self) -> Result<Vec<u8>, StatusCode> {
        self.doc.save_to_bytes().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}

fn escribir_encabezados(pdf: &mut PdfEscritor, columnas: &[(String, f32)]) {
    pdf.asegurar_espacio(10.0);
    for (texto, x) in columnas {
        pdf.texto(texto, 9.0, *x, pdf.y, true);
    }
    pdf.y -= 8.0;
}

fn escribir_filas(pdf: &mut PdfEscritor, columnas: &[(String, f32)], filas: &[Vec<String>]) {
    for fila in filas {
        pdf.asegurar_espacio(7.0);
        for (i, celda) in fila.iter().enumerate() {
            let x = columnas.get(i).map(|c| c.1).unwrap_or(10.0);
            pdf.texto(celda, 8.0, x, pdf.y, false);
        }
        pdf.y -= 5.5;
    }
    pdf.y -= 4.0;
}

/// Construye el PDF del informe de franja horaria
fn generar_pdf_informe_franja(
    informe: &InformeFranjaResponse,
) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    let mut pdf = PdfEscritor::nuevo("Informe de Franja Horaria")?;

    // Encabezado
    pdf.escribir_linea("INFORME DE FRANJA HORARIA", 16.0, 10.0, 12.0, true);
    pdf.escribir_linea(&format!("Fecha: {}", informe.fecha), 11.0, 10.0, 8.0, false);
    pdf.escribir_linea(&format!("Ventana horaria: {}", informe.ventana_horaria), 11.0, 10.0, 8.0, false);
    pdf.escribir_linea(
        &format!("Total de mediciones de la franja: {}", informe.total_mediciones),
        11.0,
        10.0,
        10.0,
        false,
    );

    // Sección: Registros fuera de rango operativo
    let fr = &informe.fuera_de_rango;
    pdf.escribir_linea(
        &format!("REGISTROS FUERA DE RANGO OPERATIVO ({})", fr.len()),
        12.0,
        10.0,
        10.0,
        true,
    );

    if fr.is_empty() {
        pdf.escribir_linea("No hay registros fuera de rango operativo.", 9.0, 10.0, 7.0, false);
    } else {
        let columnas = vec![
            ("Área".to_string(), 10.0),
            ("Termómetro".to_string(), 45.0),
            ("T.Máx".to_string(), 88.0),
            ("T.Mín".to_string(), 103.0),
            ("Hum.".to_string(), 118.0),
            ("Observaciones".to_string(), 135.0),
            ("Usuario".to_string(), 180.0),
        ];
        escribir_encabezados(&mut pdf, &columnas);
        let filas: Vec<Vec<String>> = fr
            .iter()
            .map(|r| {
                vec![
                    truncar(&r.area_nombre, 22),
                    truncar(
                        &r.termometro_nombre
                            .clone()
                            .unwrap_or_else(|| format!("ID {}", r.termometro_id)),
                        26,
                    ),
                    format!("{:.1}°C", r.temp_maxima),
                    format!("{:.1}°C", r.temp_minima),
                    r.humedad.map(|h| format!("{:.1}%", h)).unwrap_or_else(|| "-".to_string()),
                    truncar(
                        &r.observaciones.clone().unwrap_or_else(|| "-".to_string()),
                        28,
                    ),
                    truncar(&r.usuario_nombre, 12),
                ]
            })
            .collect();
        escribir_filas(&mut pdf, &columnas, &filas);
    }

    pdf.y -= 6.0;

    // Sección: Termómetros sin funcionamiento
    let fs = &informe.fuera_de_servicio;
    pdf.escribir_linea(
        &format!("TERMÓMETROS SIN FUNCIONAMIENTO ({})", fs.len()),
        12.0,
        10.0,
        10.0,
        true,
    );

    if fs.is_empty() {
        pdf.escribir_linea("No hay termómetros sin funcionamiento.", 9.0, 10.0, 7.0, false);
    } else {
        let columnas = vec![
            ("Área".to_string(), 10.0),
            ("Termómetro".to_string(), 45.0),
            ("Tipo".to_string(), 88.0),
            ("Motivo".to_string(), 120.0),
            ("Comentarios".to_string(), 150.0),
            ("Fecha".to_string(), 185.0),
        ];
        escribir_encabezados(&mut pdf, &columnas);
        let filas: Vec<Vec<String>> = fs
            .iter()
            .map(|r| {
                vec![
                    truncar(&r.area_nombre, 22),
                    truncar(
                        &r.termometro_nombre
                            .clone()
                            .unwrap_or_else(|| format!("ID {}", r.termometro_id)),
                        26,
                    ),
                    truncar(&r.tipo_nombre, 20),
                    truncar(&r.motivo, 18),
                    truncar(
                        &r.comentarios_reporte.clone().unwrap_or_else(|| "-".to_string()),
                        22,
                    ),
                    r.fecha_reporte.format("%Y-%m-%d").to_string(),
                ]
            })
            .collect();
        escribir_filas(&mut pdf, &columnas, &filas);
    }

    let bytes = pdf.guardar()?;
    Ok((StatusCode::OK, bytes))
}
