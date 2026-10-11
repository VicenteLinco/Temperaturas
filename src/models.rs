use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::FromRow;

// ===== MODELOS DE BASE DE DATOS =====

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Usuario {
    pub id: i32,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub rol: String,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Area {
    pub id: i32,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub activa: bool,
    /// Zona del área sobre el plano, en porcentaje 0-100.
    /// NULL (None) = sin delimitar: el área no se dibuja en el mapa.
    pub zona_x: Option<f32>,
    pub zona_y: Option<f32>,
    pub zona_w: Option<f32>,
    pub zona_h: Option<f32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TipoTermometro {
    pub id: i32,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub tiene_humedad: bool,
    /// true = el equipo entrega UNA sola temperatura por captura
    /// (temp_actual); temp_maxima y temp_minima se derivan de ella.
    pub solo_un_punto: bool,

    // Rangos de temperatura
    pub temp_min_operativa: f32,
    pub temp_max_operativa: f32,
    pub temp_min_fisica: f32,
    pub temp_max_fisica: f32,

    // Rangos de humedad
    pub hum_min_operativa: Option<f32>,
    pub hum_max_operativa: Option<f32>,
    pub hum_min_fisica: Option<f32>,
    pub hum_max_fisica: Option<f32>,

    pub activo: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Termometro {
    pub id: i32,
    pub area_id: i32,
    pub tipo_id: i32,
    pub nombre: Option<String>,
    pub ubicacion: Option<String>,
    pub activo: bool,
    pub fuera_de_servicio: bool,
    /// Posición del punto sobre el plano, en porcentaje 0-100.
    /// NULL (None) = pendiente de ubicar (sale en la lista de T3).
    pub pos_x: Option<f32>,
    pub pos_y: Option<f32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MantenimientoTermometro {
    pub id: i32,
    pub termometro_id: i32,
    pub usuario_reporta_id: i32,
    pub fecha_reporte: DateTime<Utc>,
    pub motivo: String,
    pub comentarios_reporte: Option<String>,
    pub fecha_reparacion: Option<DateTime<Utc>>,
    pub usuario_repara_id: Option<i32>,
    pub detalle_reparacion: Option<String>,
    pub estado: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Registro {
    pub id: i32,
    pub termometro_id: i32,
    pub usuario_id: i32,
    pub ventana_horaria: String,
    pub temp_actual: Option<f32>,
    pub temp_maxima: f32,
    pub temp_minima: f32,
    pub humedad: Option<f32>,
    pub fuera_rango_operativo: bool,
    pub observaciones: Option<String>,
    pub fecha_registro: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Configuracion {
    pub clave: String,
    pub valor: String,
    pub descripcion: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LogAuditoria {
    pub id: i32,
    pub usuario_id: i32,
    pub accion: String,
    pub tabla_afectada: String,
    pub registro_id: Option<i32>,
    pub datos_anteriores: Option<String>,
    pub datos_nuevos: Option<String>,
    pub timestamp: DateTime<Utc>,
}

// ===== DTOs PARA REQUESTS =====

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrearUsuarioRequest {
    pub username: String,
    pub password: String,
    pub rol: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActualizarUsuarioRequest {
    pub username: Option<String>,
    pub password: Option<String>,
    pub rol: Option<String>,
    pub activo: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CrearAreaRequest {
    pub nombre: String,
    pub descripcion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActualizarAreaRequest {
    pub nombre: Option<String>,
    pub descripcion: Option<String>,
    pub activa: Option<bool>,
    // Zona en porcentaje 0-100; opcionales para no romper clientes actuales.
    pub zona_x: Option<f32>,
    pub zona_y: Option<f32>,
    pub zona_w: Option<f32>,
    pub zona_h: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct CrearTipoTermometroRequest {
    pub nombre: String,
    pub descripcion: Option<String>,
    pub tiene_humedad: bool,
    // Opcional a propósito: el cliente que no lo manda obtiene false (tres lecturas por
    // captura), igual que las filas anteriores a esta columna.
    pub solo_un_punto: Option<bool>,
    pub temp_min_operativa: f32,
    pub temp_max_operativa: f32,
    pub temp_min_fisica: f32,
    pub temp_max_fisica: f32,
    pub hum_min_operativa: Option<f32>,
    pub hum_max_operativa: Option<f32>,
    pub hum_min_fisica: Option<f32>,
    pub hum_max_fisica: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActualizarTipoTermometroRequest {
    pub nombre: Option<String>,
    pub descripcion: Option<String>,
    pub tiene_humedad: Option<bool>,
    pub solo_un_punto: Option<bool>,
    pub temp_min_operativa: Option<f32>,
    pub temp_max_operativa: Option<f32>,
    pub temp_min_fisica: Option<f32>,
    pub temp_max_fisica: Option<f32>,
    pub hum_min_operativa: Option<f32>,
    pub hum_max_operativa: Option<f32>,
    pub hum_min_fisica: Option<f32>,
    pub hum_max_fisica: Option<f32>,
    pub activo: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CrearTermometroRequest {
    pub id: i32,
    pub area_id: i32,
    pub tipo_id: i32,
    pub nombre: Option<String>,
    pub ubicacion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActualizarTermometroRequest {
    pub area_id: Option<i32>,
    pub tipo_id: Option<i32>,
    pub nombre: Option<String>,
    pub ubicacion: Option<String>,
    pub activo: Option<bool>,
    pub fuera_de_servicio: Option<bool>,
    // Posición en porcentaje 0-100; opcionales para no romper clientes actuales.
    pub pos_x: Option<f32>,
    pub pos_y: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportarFueraServicioRequest {
    pub termometro_id: i32,
    pub motivo: String,
    pub comentarios: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepararTermometroRequest {
    pub detalle_reparacion: String,
}

#[derive(Debug, Deserialize)]
pub struct CrearRegistroRequest {
    pub termometro_id: i32,
    pub temp_actual: Option<f32>,
    // Opcionales para admitir tipos de un solo punto (solo temp_actual);
    // el camino de tres puntos sigue exigiendo las tres lecturas y la regla
    // de un solo lugar (`normalizar_lecturas_por_tipo`) lo valida.
    #[serde(default)]
    pub temp_maxima: Option<f32>,
    #[serde(default)]
    pub temp_minima: Option<f32>,
    pub humedad: Option<f32>,
    pub observaciones: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActualizarRegistroRequest {
    pub temp_actual: Option<f32>,
    pub temp_maxima: Option<f32>,
    pub temp_minima: Option<f32>,
    // Outer Option = field omitted; inner Option = explicit null. This lets the
    // operator replace a numeric humidity with LOW/ERROR and clear old notes.
    #[serde(default, deserialize_with = "deserializar_campo_opcional_presente")]
    pub humedad: Option<Option<f32>>,
    #[serde(default, deserialize_with = "deserializar_campo_opcional_presente")]
    pub observaciones: Option<Option<String>>,
}

fn deserializar_campo_opcional_presente<'de, D, T>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActualizarConfiguracionRequest {
    pub registro_hora_1: Option<String>,
    pub registro_hora_2: Option<String>,
    pub ventana_tolerancia_minutos: Option<i32>,
    pub restriccion_ventana_activa: Option<bool>,
}

// ===== DTOs PARA RESPONSES =====

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub user: Option<UsuarioResponse>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UsuarioResponse {
    pub id: i64,
    pub username: String,
    pub rol: String,
    pub activo: bool,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TermometroConDetalles {
    pub id: i32,
    pub area_id: i32,
    pub area_nombre: String,
    pub tipo_id: i32,
    pub tipo_nombre: String,
    pub tiene_humedad: bool,
    /// true = la captura pide UNA sola temperatura (ver `TipoTermometro::solo_un_punto`)
    pub solo_un_punto: bool,
    pub temp_min_operativa: f32,
    pub temp_max_operativa: f32,
    pub nombre: Option<String>,
    pub ubicacion: Option<String>,
    pub activo: bool,
    pub fuera_de_servicio: bool,
    /// Posición del punto sobre el plano, en porcentaje 0-100 (NULL = pendiente).
    pub pos_x: Option<f32>,
    pub pos_y: Option<f32>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RegistroConDetalles {
    pub id: i32,
    pub termometro_id: i32,
    pub termometro_nombre: Option<String>,
    pub area_nombre: Option<String>,
    pub usuario_nombre: Option<String>,
    pub ventana_horaria: String,
    pub temp_actual: Option<f32>,
    pub temp_maxima: f32,
    pub temp_minima: f32,
    pub humedad: Option<f32>,
    pub fuera_rango_operativo: bool,
    pub observaciones: Option<String>,
    pub fecha_registro: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PendientesResponse {
    pub ventana_horaria: String,
    pub area_id: i64,
    pub area_nombre: String,
    pub pendientes: Vec<TermometroConDetalles>,
    pub completados: Vec<RegistroConDetalles>,
    /// Falso cuando la restricción está activa y la hora actual cae fuera de la ventana:
    /// el registrador puede consultar, pero el guardado será rechazado.
    pub ventana_activa: bool,
    /// Hora "HH:MM" a la que abre la próxima ventana. Solo se envía si está cerrada.
    pub proxima_apertura: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegistrosPaginados {
    pub registros: Vec<RegistroConDetalles>,
    pub total: i64,
    pub pagina: u32,
    pub page_size: u32,
    pub total_paginas: u32,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct FueraDeRangoItem {
    pub id: i32,
    pub termometro_id: i32,
    pub termometro_nombre: Option<String>,
    pub area_nombre: String,
    pub temp_maxima: f32,
    pub temp_minima: f32,
    pub humedad: Option<f32>,
    pub observaciones: Option<String>,
    pub usuario_nombre: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct FueraDeServicioItem {
    pub id: i32,
    pub termometro_id: i32,
    pub termometro_nombre: Option<String>,
    pub area_nombre: String,
    pub tipo_nombre: String,
    pub ubicacion: Option<String>,
    pub motivo: String,
    pub comentarios_reporte: Option<String>,
    pub fecha_reporte: DateTime<Utc>,
}

/// Informe de la franja horaria: mediciones fuera de rango y equipos sin funcionamiento
#[derive(Debug, Serialize)]
pub struct InformeFranjaResponse {
    pub fecha: String,
    pub ventana_horaria: String,
    pub total_mediciones: i64,
    pub fuera_de_rango: Vec<FueraDeRangoItem>,
    pub fuera_de_servicio: Vec<FueraDeServicioItem>,
}

impl From<Usuario> for UsuarioResponse {
    fn from(u: Usuario) -> Self {
        UsuarioResponse {
            id: u.id as i64,
            username: u.username,
            rol: u.rol,
            activo: u.activo,
        }
    }
}

// ===== ALERTAS =====
// Los tipos de alertas quedan definidos para la futura feature de notificaciones;
// aún no están conectados a endpoints.

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Alerta {
    pub id: i32,
    pub registro_id: i32,
    pub tipo: String, // 'ADVERTENCIA', 'CRITICA'
    pub fecha_alerta: DateTime<Utc>,
    pub temperatura_registrada: f32,
    pub humedad_registrada: Option<f32>,
    pub desviacion: f32,
    pub campo_afectado: String, // 'temp_maxima', 'temp_minima', 'humedad'

    // Notificación
    pub notificado: bool,
    pub fecha_notificacion: Option<DateTime<Utc>>,
    pub destinatario: Option<String>,

    // Resolución
    pub estado: String, // 'PENDIENTE', 'RESUELTO', 'AUTO_RESUELTO'
    pub fecha_resolucion: Option<DateTime<Utc>>,
    pub accion_correctiva: Option<String>,
    pub responsable_resolucion: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, FromRow)]
pub struct AlertaConDetalles {
    // Datos de alerta
    pub id: i32,
    pub tipo: String,
    pub fecha_alerta: DateTime<Utc>,
    pub temperatura_registrada: f32,
    pub humedad_registrada: Option<f32>,
    pub desviacion: f32,
    pub campo_afectado: String,
    pub notificado: bool,
    pub fecha_notificacion: Option<DateTime<Utc>>,
    pub destinatario: Option<String>,
    pub estado: String,
    pub fecha_resolucion: Option<DateTime<Utc>>,
    pub accion_correctiva: Option<String>,
    pub responsable_resolucion: Option<String>,

    // Datos del registro asociado
    pub registro_id: i32,
    pub fecha_registro: DateTime<Utc>,
    pub ventana_horaria: String,

    // Datos del termómetro
    pub termometro_id: i32,
    pub termometro_nombre: Option<String>,

    // Datos del área
    pub area_nombre: String,

    // Datos del usuario
    pub usuario_nombre: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ResolverAlertaRequest {
    pub accion_correctiva: String,
    pub responsable: String,
}

// ===== MAPA DE PENDIENTES SOBRE PLANO (T2) =====
// Coordenadas en porcentaje 0-100 sobre la imagen del plano.
// snake_case en JSON, igual que el resto de DTOs.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZonaRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MapaArea {
    pub id: i32,
    pub nombre: String,
    pub zona: Option<ZonaRect>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MapaTermometro {
    pub id: i32,
    pub area_id: i32,
    pub nombre: Option<String>,
    pub ubicacion: Option<String>,
    pub pos_x: Option<f32>,
    pub pos_y: Option<f32>,
    /// pendiente | completado | fuera_de_servicio
    pub estado: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MapaResponse {
    pub habilitado: bool,
    pub imagen_url: Option<String>,
    pub areas: Vec<MapaArea>,
    pub termometros: Vec<MapaTermometro>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermometroPosRequest {
    pub id: i32,
    pub pos_x: f32,
    pub pos_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaZonaRequest {
    pub id: i32,
    pub zona: Option<ZonaRect>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GuardarPosicionesRequest {
    #[serde(default)]
    pub termometros: Vec<TermometroPosRequest>,
    #[serde(default)]
    pub areas: Vec<AreaZonaRequest>,
    #[serde(default)]
    pub confirmar_cambio_area: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FotoRequest {
    pub filename: String,
    pub content_base64: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MapaConfigRequest {
    pub mapa_habilitado: bool,
}

// ===== REPORTE Y ANÁLISIS DE SALIDAS DE RANGO TÉRMICO =====

#[derive(Debug, Deserialize)]
pub struct FiltrosSalidasRango {
    pub modo: Option<String>,
    pub fecha: Option<String>,
    pub fecha_desde: Option<String>,
    pub fecha_hasta: Option<String>,
    pub hora: Option<i32>,
    pub hora_desde: Option<i32>,
    pub hora_hasta: Option<i32>,
    pub ventana_horaria: Option<String>,
    pub area_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ResumenSalidasRango {
    pub total_registros: i64,
    pub total_salidas_rango: i64,
    pub porcentaje_salidas: f32,
    pub termometros_con_desvio: i64,
    pub zonas_con_desvio: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SalidaRangoTermometroItem {
    pub termometro_id: i32,
    pub termometro_nombre: String,
    pub ubicacion: Option<String>,
    pub area_id: i32,
    pub area_nombre: String,
    pub tipo_nombre: String,
    pub temp_min_operativa: f32,
    pub temp_max_operativa: f32,
    pub total_registros: i64,
    pub salidas_rango: i64,
    pub porcentaje_salidas: f32,
    pub temp_min_registrada: Option<f32>,
    pub temp_max_registrada: Option<f32>,
    pub temp_promedio: Option<f32>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SalidaRangoZonaItem {
    pub area_id: i32,
    pub area_nombre: String,
    pub total_termometros: i64,
    pub termometros_con_desvio: i64,
    pub total_registros: i64,
    pub salidas_rango: i64,
    pub porcentaje_salidas: f32,
    pub temp_min_registrada: Option<f32>,
    pub temp_max_registrada: Option<f32>,
    #[sqlx(default)]
    pub termometro_mas_critico: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SalidasRangoResponse {
    pub resumen: ResumenSalidasRango,
    pub por_termometro: Vec<SalidaRangoTermometroItem>,
    pub por_zona: Vec<SalidaRangoZonaItem>,
}

