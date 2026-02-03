use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;
use sqlx::FromRow;

// ===== MODELOS DE BASE DE DATOS =====

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Usuario {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub rol: String,
    pub activo: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Area {
    pub id: i64,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub activa: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TipoTermometro {
    pub id: i64,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub tiene_humedad: bool,

    // Rangos de temperatura
    pub temp_min_operativa: f64,
    pub temp_max_operativa: f64,
    pub temp_min_fisica: f64,
    pub temp_max_fisica: f64,

    // Rangos de humedad
    pub hum_min_operativa: Option<f64>,
    pub hum_max_operativa: Option<f64>,
    pub hum_min_fisica: Option<f64>,
    pub hum_max_fisica: Option<f64>,

    pub activo: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Termometro {
    pub id: i64,
    pub area_id: i64,
    pub tipo_id: i64,
    pub nombre: Option<String>,
    pub ubicacion: Option<String>,
    pub activo: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Registro {
    pub id: i64,
    pub termometro_id: i64,
    pub usuario_id: i64,
    pub ventana_horaria: String,
    pub temp_actual: Option<f64>,
    pub temp_maxima: f64,
    pub temp_minima: f64,
    pub humedad: Option<f64>,
    pub fuera_rango_operativo: bool,
    pub observaciones: Option<String>,
    pub fecha_registro: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Configuracion {
    pub clave: String,
    pub valor: String,
    pub descripcion: Option<String>,
    pub updated_at: NaiveDateTime,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LogAuditoria {
    pub id: i64,
    pub usuario_id: i64,
    pub accion: String,
    pub tabla_afectada: String,
    pub registro_id: Option<i64>,
    pub datos_anteriores: Option<String>,
    pub datos_nuevos: Option<String>,
    pub timestamp: NaiveDateTime,
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
}

#[derive(Debug, Deserialize)]
pub struct CrearTipoTermometroRequest {
    pub nombre: String,
    pub descripcion: Option<String>,
    pub tiene_humedad: bool,
    pub temp_min_operativa: f64,
    pub temp_max_operativa: f64,
    pub temp_min_fisica: f64,
    pub temp_max_fisica: f64,
    pub hum_min_operativa: Option<f64>,
    pub hum_max_operativa: Option<f64>,
    pub hum_min_fisica: Option<f64>,
    pub hum_max_fisica: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActualizarTipoTermometroRequest {
    pub nombre: Option<String>,
    pub descripcion: Option<String>,
    pub tiene_humedad: Option<bool>,
    pub temp_min_operativa: Option<f64>,
    pub temp_max_operativa: Option<f64>,
    pub temp_min_fisica: Option<f64>,
    pub temp_max_fisica: Option<f64>,
    pub hum_min_operativa: Option<f64>,
    pub hum_max_operativa: Option<f64>,
    pub hum_min_fisica: Option<f64>,
    pub hum_max_fisica: Option<f64>,
    pub activo: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CrearTermometroRequest {
    pub id: i64,
    pub area_id: i64,
    pub tipo_id: i64,
    pub nombre: Option<String>,
    pub ubicacion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActualizarTermometroRequest {
    pub area_id: Option<i64>,
    pub tipo_id: Option<i64>,
    pub nombre: Option<String>,
    pub ubicacion: Option<String>,
    pub activo: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CrearRegistroRequest {
    pub termometro_id: i64,
    pub temp_actual: Option<f64>,
    pub temp_maxima: f64,
    pub temp_minima: f64,
    pub humedad: Option<f64>,
    pub observaciones: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActualizarRegistroRequest {
    pub temp_actual: Option<f64>,
    pub temp_maxima: Option<f64>,
    pub temp_minima: Option<f64>,
    pub humedad: Option<f64>,
    pub observaciones: Option<String>,
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

#[derive(Debug, Serialize, FromRow)]
pub struct TermometroConDetalles {
    pub id: i64,
    pub area_id: i64,
    pub area_nombre: String,
    pub tipo_id: i64,
    pub tipo_nombre: String,
    pub tiene_humedad: bool,
    pub temp_min_operativa: f64,
    pub temp_max_operativa: f64,
    pub nombre: Option<String>,
    pub ubicacion: Option<String>,
    pub activo: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct RegistroConDetalles {
    pub id: i64,
    pub termometro_id: i64,
    pub termometro_nombre: Option<String>,
    pub area_nombre: String,
    pub usuario_nombre: String,
    pub ventana_horaria: String,
    pub temp_actual: Option<f64>,
    pub temp_maxima: f64,
    pub temp_minima: f64,
    pub humedad: Option<f64>,
    pub fuera_rango_operativo: bool,
    pub observaciones: Option<String>,
    pub fecha_registro: NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct PendientesResponse {
    pub ventana_horaria: String,
    pub area_id: i64,
    pub area_nombre: String,
    pub pendientes: Vec<TermometroConDetalles>,
    pub completados: Vec<RegistroConDetalles>,
}

impl From<Usuario> for UsuarioResponse {
    fn from(u: Usuario) -> Self {
        UsuarioResponse {
            id: u.id,
            username: u.username,
            rol: u.rol,
            activo: u.activo,
        }
    }
}

// ===== ALERTAS =====

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Alerta {
    pub id: i64,
    pub registro_id: i64,
    pub tipo: String, // 'ADVERTENCIA', 'CRITICA'
    pub fecha_alerta: NaiveDateTime,
    pub temperatura_registrada: f64,
    pub humedad_registrada: Option<f64>,
    pub desviacion: f64,
    pub campo_afectado: String, // 'temp_maxima', 'temp_minima', 'humedad'

    // Notificación
    pub notificado: bool,
    pub fecha_notificacion: Option<NaiveDateTime>,
    pub destinatario: Option<String>,

    // Resolución
    pub estado: String, // 'PENDIENTE', 'RESUELTO', 'AUTO_RESUELTO'
    pub fecha_resolucion: Option<NaiveDateTime>,
    pub accion_correctiva: Option<String>,
    pub responsable_resolucion: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AlertaConDetalles {
    // Datos de alerta
    pub id: i64,
    pub tipo: String,
    pub fecha_alerta: NaiveDateTime,
    pub temperatura_registrada: f64,
    pub humedad_registrada: Option<f64>,
    pub desviacion: f64,
    pub campo_afectado: String,
    pub notificado: bool,
    pub fecha_notificacion: Option<NaiveDateTime>,
    pub destinatario: Option<String>,
    pub estado: String,
    pub fecha_resolucion: Option<NaiveDateTime>,
    pub accion_correctiva: Option<String>,
    pub responsable_resolucion: Option<String>,

    // Datos del registro asociado
    pub registro_id: i64,
    pub fecha_registro: NaiveDateTime,
    pub ventana_horaria: String,

    // Datos del termómetro
    pub termometro_id: i64,
    pub termometro_nombre: Option<String>,

    // Datos del área
    pub area_nombre: String,

    // Datos del usuario
    pub usuario_nombre: String,
}

#[derive(Debug, Deserialize)]
pub struct ResolverAlertaRequest {
    pub accion_correctiva: String,
    pub responsable: String,
}
