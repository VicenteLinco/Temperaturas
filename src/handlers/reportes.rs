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

#[derive(Deserialize)]
pub struct FiltrosReporteIncidencias {
    pub tipo: Option<String>,
    pub incluir_observaciones: Option<String>,
    pub formato: String,
}

const CONDICION_HUMEDAD_NO_DISPONIBLE: &str = "(r.observaciones LIKE '%[HUMEDAD:LOW]%' OR r.observaciones LIKE '%[HUMEDAD:ERROR]%')";

fn condicion_reporte_incidencias(tipo: Option<&str>) -> String {
    match tipo {
        // LOW/ERROR es una incidencia aunque las temperaturas estén dentro de rango.
        Some("fuera_rango") => format!(
            "(r.fuera_rango_operativo = true OR {})",
            CONDICION_HUMEDAD_NO_DISPONIBLE
        ),
        Some("fuera_servicio") => "t.fuera_de_servicio = true".to_string(),
        _ => format!(
            "(r.fuera_rango_operativo = true OR t.fuera_de_servicio = true OR {})",
            CONDICION_HUMEDAD_NO_DISPONIBLE
        ),
    }
}

pub async fn generar_reporte_incidencias(
    _current_user: CurrentUser,
    State(pool): State<PgPool>,
    Query(filtros): Query<FiltrosReporteIncidencias>,
) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    // El filtro de estado viene del selector de la vista de reportes (admin.html)
    let condicion = condicion_reporte_incidencias(filtros.tipo.as_deref());

    // "Solo Mediciones" exporta las lecturas sin la columna de acciones correctivas
    let columna_observaciones = match filtros.incluir_observaciones.as_deref() {
        Some("no") => "NULL::text as observaciones",
        _ => "r.observaciones",
    };

    let rows = sqlx::query(&format!(
        r#"
        SELECT
            r.id, r.fecha_registro, r.ventana_horaria,
            a.nombre as area_nombre, t.nombre as termometro_nombre, t.id as termometro_id,
            ti.nombre as tipo_nombre,
            r.temp_maxima, r.temp_minima, r.humedad,
            r.fuera_rango_operativo, {},
            u.username as usuario_nombre
        FROM registros r
        JOIN termometros t ON r.termometro_id = t.id
        JOIN areas a ON t.area_id = a.id
        JOIN tipos_termometro ti ON t.tipo_id = ti.id
        JOIN usuarios u ON r.usuario_id = u.id
        WHERE {}
        ORDER BY r.fecha_registro DESC
        "#,
        columna_observaciones, condicion
    ))
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let fecha_tag = chrono::Utc::now().with_timezone(&chrono_tz::America::Santiago).format("%Y-%m-%d").to_string();

    if filtros.formato == "csv" {
        generar_csv_diario(rows, &fecha_tag)
    } else {
        generar_pdf_diario(rows, &format!("Auditoría de Incidencias HACCP ({})", fecha_tag))
    }
}

#[cfg(test)]
mod tests_ajuste_ancho_pdf {
    use super::*;

    #[test]
    fn texto_corto_no_se_trunca() {
        assert_eq!(truncar_a_ancho("Cámara Fría", 40.0, 7.5, false), "Cámara Fría");
    }

    #[test]
    fn nombre_largo_de_termometro_cabe_en_columna_estrecha() {
        // Peor caso real: nombre larguísimo con letras anchas
        let nombre = "Termómetro MUMMÁX WAREHOUSE CÁMARA CONGELACIÓN N°4 PASILLO NORTE";
        // Columna "Termómetro" del informe de franja: 42 - 1.5 mm
        let ajustado = truncar_a_ancho(nombre, 40.5, 7.5, false);
        assert!(ancho_texto_mm(&ajustado, 7.5, false) <= 40.5);
        assert!(ajustado.ends_with("..."));
        assert!(ajustado.chars().count() < nombre.chars().count());
    }

    #[test]
    fn texto_vacio_y_ancho_minimo_no_panic() {
        assert_eq!(truncar_a_ancho("", 4.0, 7.5, false), "");
        let ajustado = truncar_a_ancho("MMMMM", 1.0, 7.5, true);
        assert!(ancho_texto_mm(&ajustado, 7.5, true) <= 1.0 + 0.001);
    }

    #[test]
    fn anchos_helvetica_son_coherentes() {
        // 'M' es más ancha que 'i'; el punto es ancho fijo conocido
        assert!(ancho_texto_mm("M", 7.5, false) > ancho_texto_mm("i", 7.5, false));
        let punto = ancho_texto_mm(".", 7.5, false);
        let esperado = 278.0 / 1000.0 * 7.5 * 0.352778;
        assert!((punto - esperado).abs() < 1e-6);
    }
}

#[cfg(test)]
mod tests_reporte_incidencias {
    use super::*;

    #[test]
    fn reporte_incidencias_incluye_low_y_error_con_temperatura_normal() {
        let todas = condicion_reporte_incidencias(None);
        assert!(todas.contains("[HUMEDAD:LOW]"));
        assert!(todas.contains("[HUMEDAD:ERROR]"));

        let mediciones = condicion_reporte_incidencias(Some("fuera_rango"));
        assert!(mediciones.contains("r.fuera_rango_operativo = true"));
        assert!(mediciones.contains("[HUMEDAD:LOW]"));
        assert!(mediciones.contains("[HUMEDAD:ERROR]"));

        let servicio = condicion_reporte_incidencias(Some("fuera_servicio"));
        assert_eq!(servicio, "t.fuera_de_servicio = true");
    }
}

#[derive(Deserialize)]
pub struct FiltrosReporteEstabilidad {
    pub dias: Option<i64>,
    pub agrupar_por: Option<String>,
    pub formato: String,
}

pub async fn generar_reporte_estabilidad(
    _current_user: CurrentUser,
    State(pool): State<PgPool>,
    Query(filtros): Query<FiltrosReporteEstabilidad>,
) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    let num_dias = filtros.dias.unwrap_or(30);

    // Determina el orden de las filas del análisis (selector "Agrupar Por" en admin.html)
    let orden = match filtros.agrupar_por.as_deref() {
        Some("termometro") => "t.nombre, a.nombre, r.fecha_registro DESC",
        _ => "a.nombre, t.nombre, r.fecha_registro DESC",
    };

    let rows = sqlx::query(&format!(
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
        WHERE r.fecha_registro >= NOW() - (INTERVAL '1 day' * $1)
        ORDER BY {}
        "#,
        orden
    ))
    .bind(num_dias as f64)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let etiqueta = format!("Análisis Estabilidad (Últimos {} Días)", num_dias);

    if filtros.formato == "csv" {
        generar_csv_diario(rows, &etiqueta)
    } else {
        generar_pdf_diario(rows, &etiqueta)
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
    let mut pdf = PdfEscritor::nuevo("Reporte de Control de Temperaturas", true)?;

    pdf.escribir_linea("REPORTE DE CONTROL DE TEMPERATURAS", 14.0, 6.0, 10.0, true);
    pdf.escribir_linea(&format!("Período: {}", fecha), 10.0, 6.0, 8.0, false);
    pdf.y -= 4.0;

    let columnas = vec![
        ("ID".to_string(), 6.0),
        ("Fecha".to_string(), 16.0),
        ("Ventana".to_string(), 37.0),
        ("Área".to_string(), 52.0),
        ("Termómetro".to_string(), 84.0),
        ("Tipo".to_string(), 132.0),
        ("T.Máx".to_string(), 160.0),
        ("T.Mín".to_string(), 176.0),
        ("Hum.".to_string(), 192.0),
        ("Estado".to_string(), 207.0),
        ("Usuario".to_string(), 226.0),
        ("Obs.".to_string(), 250.0),
    ];

    escribir_encabezados(&mut pdf, &columnas);

    let filas: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            let id: i32 = row.get("id");
            let fecha_registro: chrono::DateTime<chrono::Utc> = row.get("fecha_registro");
            let ventana: String = row.get("ventana_horaria");
            let area: String = row.get("area_nombre");
            let termometro_id: i32 = row.get("termometro_id");
            let termometro_nombre: String = row.try_get::<String, _>("termometro_nombre").unwrap_or_default();
            let termo_full = if !termometro_nombre.is_empty() {
                format!("{}({})", termometro_id, termometro_nombre)
            } else {
                termometro_id.to_string()
            };
            let tipo: String = row.get("tipo_nombre");
            let temp_max: f32 = row.get("temp_maxima");
            let temp_min: f32 = row.get("temp_minima");
            let humedad: Option<f64> = row.try_get::<Option<f64>, _>("humedad").unwrap_or(None);
            let fuera_rango: bool = row.get("fuera_rango_operativo");
            let usuario: String = row.get("usuario_nombre");
            let observaciones: Option<String> = row.try_get::<Option<String>, _>("observaciones").unwrap_or(None);

            vec![
                id.to_string(),
                fecha_registro.format("%Y-%m-%d").to_string(),
                ventana,
                area,
                termo_full,
                tipo,
                format!("{:.1}°C", temp_max),
                format!("{:.1}°C", temp_min),
                humedad.map(|h| format!("{:.1}%", h)).unwrap_or_else(|| "-".to_string()),
                if fuera_rango { "⚠ Alerta".to_string() } else { "✓ OK".to_string() },
                usuario,
                observaciones.unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect();

    escribir_filas(&mut pdf, &columnas, &filas);

    let bytes = pdf.guardar()?;
    Ok((StatusCode::OK, bytes))
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

/// Día operativo actual: el que empieza a las 08:00 de Chile y termina a las 07:59
/// del día siguiente. La ronda nocturna cruza medianoche, así que usar el día natural
/// partía el informe en dos y dejaba fuera lo registrado antes de las 00:00.
fn fecha_hoy_santiago() -> String {
    (chrono::Utc::now().with_timezone(&chrono_tz::America::Santiago)
        - chrono::Duration::hours(8))
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
         WHERE ((r.fecha_registro AT TIME ZONE 'America/Santiago') - INTERVAL '8 hours')::date = $1::date
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
          AND ((r.fecha_registro AT TIME ZONE 'America/Santiago') - INTERVAL '8 hours')::date = $1::date
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

/// Recorta un texto para que quepa en una columna del PDF sin solapamiento
/// Anchos estándar de Helvetica (AFM, unidades/1000) para los caracteres ASCII 32..=126.
/// Cualquier carácter fuera de ese rango usa el ancho por defecto (556).
const HELV_WIDTHS: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];
const HELV_BOLD_WIDTHS: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611, 975, 722, 722, 722, 722, 667,
    611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 333, 278, 333, 584, 556, 333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556,
    278, 889, 611, 611, 611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
];

const PT_A_MM: f32 = 0.352778;

/// Ancho estimado del texto en mm para Helvetica a `size` pt
fn ancho_texto_mm(texto: &str, size: f32, bold: bool) -> f32 {
    let tabla = if bold { &HELV_BOLD_WIDTHS } else { &HELV_WIDTHS };
    let unidades: u32 = texto
        .chars()
        .map(|c| {
            let u = c as u32;
            if (32..=126).contains(&u) {
                tabla[(u - 32) as usize] as u32
            } else {
                556
            }
        })
        .sum();
    unidades as f32 / 1000.0 * size * PT_A_MM
}

/// Trunca el texto con "..." para que quepa dentro de `max_mm` según la fuente.
/// A diferencia de `truncar` (por cantidad de caracteres), garantiza que el texto
/// no se solape con la columna siguiente sin importar qué caracteres contenga.
fn truncar_a_ancho(texto: &str, max_mm: f32, size: f32, bold: bool) -> String {
    let clean = texto.trim();
    if ancho_texto_mm(clean, size, bold) <= max_mm {
        return clean.to_string();
    }
    const ELLIPSIS: &str = "...";
    let presupuesto = max_mm - ancho_texto_mm(ELLIPSIS, size, bold);
    if presupuesto <= 0.0 {
        // La columna es más angosta que los propios "...": mejor vacío
        return String::new();
    }
    let mut out = String::new();
    for ch in clean.chars() {
        let candidato = format!("{}{}", out, ch);
        if ancho_texto_mm(&candidato, size, bold) > presupuesto {
            break;
        }
        out = candidato;
    }
    format!("{}{}", out.trim_end(), ELLIPSIS)
}

fn sanitize_pdf_str(input: &str) -> String {
    input
        .replace('°', " ")
        .replace('⚠', "[!]")
        .replace('✓', "[OK]")
        .replace('á', "a").replace('é', "e").replace('í', "i").replace('ó', "o").replace('ú', "u")
        .replace('Á', "A").replace('É', "E").replace('Í', "I").replace('Ó', "O").replace('Ú', "U")
        .replace('ñ', "n").replace('Ñ', "N")
        .replace('ü', "u").replace('Ü', "U")
}

/// Escritor de PDF simple con soporte para orientación Portrait/Landscape y paginación
struct PdfEscritor {
    doc: PdfDocumentReference,
    layer: PdfLayerReference,
    width: f32,
    height: f32,
    /// Margen horizontal usado por las columnas de las tablas
    margen_x: f32,
    y: f32,
    font_bold: IndirectFontRef,
    font_regular: IndirectFontRef,
}

impl PdfEscritor {
    fn nuevo(titulo: &str, landscape: bool) -> Result<Self, StatusCode> {
        let (w, h) = if landscape { (297.0, 210.0) } else { (210.0, 297.0) };
        let margen_x = if landscape { 6.0 } else { 8.0 };
        let (doc, page, layer) = PdfDocument::new(titulo, Mm(w), Mm(h), "Capa 1");
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
            width: w,
            height: h,
            margen_x,
            y: h - 15.0,
            font_bold,
            font_regular,
        })
    }

    fn nueva_pagina(&mut self) {
        let (page, layer) = self.doc.add_page(Mm(self.width), Mm(self.height), "Capa 1");
        self.layer = self.doc.get_page(page).get_layer(layer);
        self.y = self.height - 15.0;
    }

    /// Ancho disponible (mm) de cada columna a partir de sus posiciones X:
    /// cada columna llega hasta el inicio de la siguiente (con 1.5 mm de
    /// separación); la última llega hasta el margen derecho de la página.
    fn anchos_columnas(&self, columnas: &[(String, f32)]) -> Vec<f32> {
        let x_ultima = self.width - self.margen_x;
        (0..columnas.len())
            .map(|i| {
                let x = columnas[i].1;
                let x_sig = columnas.get(i + 1).map(|c| c.1).unwrap_or(x_ultima);
                (x_sig - x - 1.5).max(4.0)
            })
            .collect()
    }

    fn asegurar_espacio(&mut self, alto: f32) -> bool {
        if self.y - alto < 12.0 {
            self.nueva_pagina();
            true
        } else {
            false
        }
    }

    fn texto(&self, texto: &str, size: f32, x: f32, y: f32, bold: bool) {
        let clean_text = sanitize_pdf_str(texto);
        let font = if bold { &self.font_bold } else { &self.font_regular };
        self.layer.use_text(&clean_text, size, Mm(x), Mm(y), font);
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
    let anchos = pdf.anchos_columnas(columnas);
    for (i, (texto, x)) in columnas.iter().enumerate() {
        let ajustado = truncar_a_ancho(texto, anchos[i], 8.5, true);
        pdf.texto(&ajustado, 8.5, *x, pdf.y, true);
    }
    pdf.y -= 7.0;
}

fn escribir_filas(pdf: &mut PdfEscritor, columnas: &[(String, f32)], filas: &[Vec<String>]) {
    let anchos = pdf.anchos_columnas(columnas);
    for fila in filas {
        let creo_nueva_pagina = pdf.asegurar_espacio(6.0);
        if creo_nueva_pagina {
            escribir_encabezados(pdf, columnas);
        }
        for (i, celda) in fila.iter().enumerate() {
            let x = columnas.get(i).map(|c| c.1).unwrap_or(8.0);
            let ancho = anchos.get(i).copied().unwrap_or(40.0);
            let ajustado = truncar_a_ancho(celda, ancho, 7.5, false);
            pdf.texto(&ajustado, 7.5, x, pdf.y, false);
        }
        pdf.y -= 5.0;
    }
    pdf.y -= 3.0;
}

/// Construye el PDF del informe de franja horaria
fn generar_pdf_informe_franja(
    informe: &InformeFranjaResponse,
) -> Result<(StatusCode, Vec<u8>), StatusCode> {
    let mut pdf = PdfEscritor::nuevo("Informe de Franja Horaria", false)?;

    // Encabezado
    pdf.escribir_linea("INFORME DE FRANJA HORARIA", 15.0, 8.0, 10.0, true);
    pdf.escribir_linea(&format!("Fecha: {}", informe.fecha), 10.0, 8.0, 7.0, false);
    pdf.escribir_linea(&format!("Ventana horaria: {}", informe.ventana_horaria), 10.0, 8.0, 7.0, false);
    pdf.escribir_linea(
        &format!("Total de mediciones de la franja: {}", informe.total_mediciones),
        10.0,
        8.0,
        9.0,
        false,
    );

    // Sección: Registros fuera de rango operativo
    let fr = &informe.fuera_de_rango;
    pdf.escribir_linea(
        &format!("REGISTROS FUERA DE RANGO OPERATIVO ({})", fr.len()),
        11.0,
        8.0,
        9.0,
        true,
    );

    if fr.is_empty() {
        pdf.escribir_linea("No hay registros fuera de rango operativo.", 9.0, 8.0, 6.0, false);
    } else {
        let columnas = vec![
            ("Área".to_string(), 8.0),
            ("Termómetro".to_string(), 40.0),
            ("T.Máx".to_string(), 82.0),
            ("T.Mín".to_string(), 98.0),
            ("Hum.".to_string(), 114.0),
            ("Observaciones".to_string(), 129.0),
            ("Usuario".to_string(), 178.0),
        ];
        escribir_encabezados(&mut pdf, &columnas);
        let filas: Vec<Vec<String>> = fr
            .iter()
            .map(|r| {
                vec![
                    r.area_nombre.clone(),
                    r.termometro_nombre
                        .clone()
                        .unwrap_or_else(|| format!("ID {}", r.termometro_id)),
                    format!("{:.1}°C", r.temp_maxima),
                    format!("{:.1}°C", r.temp_minima),
                    r.humedad.map(|h| format!("{:.1}%", h)).unwrap_or_else(|| "-".to_string()),
                    r.observaciones.clone().unwrap_or_else(|| "-".to_string()),
                    r.usuario_nombre.clone(),
                ]
            })
            .collect();
        escribir_filas(&mut pdf, &columnas, &filas);
    }

    pdf.y -= 4.0;

    // Sección: Termómetros sin funcionamiento
    let fs = &informe.fuera_de_servicio;
    pdf.escribir_linea(
        &format!("TERMÓMETROS SIN FUNCIONAMIENTO ({})", fs.len()),
        11.0,
        8.0,
        9.0,
        true,
    );

    if fs.is_empty() {
        pdf.escribir_linea("No hay termómetros sin funcionamiento.", 9.0, 8.0, 6.0, false);
    } else {
        let columnas = vec![
            ("Área".to_string(), 8.0),
            ("Termómetro".to_string(), 40.0),
            ("Tipo".to_string(), 82.0),
            ("Motivo".to_string(), 110.0),
            ("Comentarios".to_string(), 135.0),
            ("Fecha".to_string(), 182.0),
        ];
        escribir_encabezados(&mut pdf, &columnas);
        let filas: Vec<Vec<String>> = fs
            .iter()
            .map(|r| {
                vec![
                    r.area_nombre.clone(),
                    r.termometro_nombre
                        .clone()
                        .unwrap_or_else(|| format!("ID {}", r.termometro_id)),
                    r.tipo_nombre.clone(),
                    r.motivo.clone(),
                    r.comentarios_reporte.clone().unwrap_or_else(|| "-".to_string()),
                    r.fecha_reporte.format("%Y-%m-%d").to_string(),
                ]
            })
            .collect();
        escribir_filas(&mut pdf, &columnas, &filas);
    }

    let bytes = pdf.guardar()?;
    Ok((StatusCode::OK, bytes))
}
