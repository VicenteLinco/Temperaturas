use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::SqlitePool;
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
