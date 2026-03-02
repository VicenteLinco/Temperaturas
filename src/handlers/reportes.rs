use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::{PgPool, postgres::PgRow, Row};
use printpdf::*;

use crate::auth::CurrentUser;

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
    State(pool): State<PgPool>,
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
        WHERE (r.fecha_registro::date) = CAST($1 AS DATE)
        "#
    );

    if filtros.area_id.is_some() {
        query.push_str(" AND t.area_id = $2");
    }

    query.push_str(" ORDER BY a.nombre, t.id, r.ventana_horaria");

    let mut q = sqlx::query(&query).bind(&filtros.fecha);

    if let Some(area_id) = filtros.area_id {
        q = q.bind(area_id as i32);
    }

    let rows = q
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Error reporte diario: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

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

    query.push_str(" ORDER BY r.fecha_registro, a.nombre, t.id, r.ventana_horaria");

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
    generar_csv_diario(rows, &format!("{}-{:02}", anio, mes))
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
    current_layer.use_text("Fecha", 9.0, Mm(15.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Ventana", 9.0, Mm(50.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Área", 9.0, Mm(70.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Termómetro", 9.0, Mm(105.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Tipo", 9.0, Mm(145.0), Mm(y_pos), &font_bold);
    current_layer.use_text("T.Máx", 9.0, Mm(170.0), Mm(y_pos), &font_bold);
    current_layer.use_text("T.Mín", 9.0, Mm(185.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Hum.", 9.0, Mm(200.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Estado", 9.0, Mm(215.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Usuario", 9.0, Mm(235.0), Mm(y_pos), &font_bold);
    current_layer.use_text("Obs.", 9.0, Mm(260.0), Mm(y_pos), &font_bold);

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

        // Truncar si es muy largo (aprox 25 caracteres para la columna de 40mm)
        if termo.len() > 25 {
            if termometro_nombre.len() > 15 {
                termo = format!("{}({}...)", termometro_id, &termometro_nombre[..12]);
            }
            if termo.len() > 25 {
                termo = format!("{}...", &termo[..22]);
            }
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
            .map(|o| if o.len() > 15 { format!("{}...", &o[..12]) } else { o.clone() })
            .unwrap_or_else(|| "-".to_string());

        let estado = if fuera_rango { "⚠ Alert" } else { "✓ OK" };

        current_layer.use_text(&id.to_string(), 8.0, Mm(5.0), Mm(y_pos), &font_regular);
        current_layer.use_text(&fecha_str, 8.0, Mm(15.0), Mm(y_pos), &font_regular);
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
        if y_pos < 15.0 { break; }
    }

    let pdf_bytes = doc.save_to_bytes().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::OK, pdf_bytes))
}

fn generar_pdf_mensual(rows: Vec<PgRow>, mes: u32, anio: i32) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    generar_pdf_diario(rows, &format!("{}-{:02}", anio, mes))
}
