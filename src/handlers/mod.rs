// Módulo de handlers - Organiza todos los endpoints de la API por dominio

// Constantes compartidas
pub const MAX_REGISTROS_POR_PAGINA: i32 = 500;
#[allow(dead_code)]
pub const TIEMPO_SESSION_DEFAULT_HORAS: u64 = 8;

// Submódulos
pub mod auth;
pub mod usuarios;
pub mod areas;
pub mod tipos_termometro;
pub mod termometros;
pub mod registros;
pub mod configuracion;
pub mod reportes;
pub mod graficos;

// Re-exportar todas las funciones para mantener compatibilidad
pub use auth::*;
pub use usuarios::*;
pub use areas::*;
pub use tipos_termometro::*;
pub use termometros::*;
pub use registros::*;
pub use configuracion::*;
pub use reportes::*;
pub use graficos::*;
