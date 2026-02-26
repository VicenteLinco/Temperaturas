mod auth;
mod db;
mod handlers;
mod logic;
mod models;

use axum::{
    middleware,
    routing::{get, post, put, delete},
    Router,
};
use dotenv::dotenv;
use std::env;
use time;
use tower::ServiceBuilder;
use tower_http::{
    services::ServeDir,
    trace::TraceLayer,
};
use tower_sessions::{
    cookie::SameSite, Expiry, SessionManagerLayer,
};
use tower_sessions_sqlx_store::SqliteStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Cargar variables de entorno
    dotenv().ok();

    // Inicializar logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sistema_temperaturas=debug,tower_http=debug,axum::rejection=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Obtener configuración
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:datos.db".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

    // Inicializar base de datos
    tracing::info!("Conectando a base de datos: {}", database_url);
    let pool = db::init_db(&database_url).await?;

    // Configurar session store
    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await?;

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(cfg!(not(debug_assertions))) // ✅ Secure en producción, permite HTTP en desarrollo
        .with_same_site(SameSite::Strict) // ✅ Protección CSRF mejorada
        .with_http_only(true) // ✅ Previene acceso desde JavaScript
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(8)));

    // Configurar rutas

    // Rutas públicas (sin autenticación)
    let public_routes = Router::new()
        .route("/api/login", post(handlers::login_handler))
        .route("/api/me", get(handlers::me_handler));

    // Rutas que requieren autenticación básica
    let auth_routes = Router::new()
        .route("/api/logout", post(handlers::logout_handler))
        .route("/api/areas", get(handlers::listar_areas))
        .route("/api/tipos-termometro", get(handlers::listar_tipos_termometro))
        .route("/api/termometros", get(handlers::listar_termometros))
        .route("/api/termometros/:id", get(handlers::obtener_termometro))
        .route_layer(middleware::from_fn(auth::require_auth));

    // Rutas de Registrador
    let registrador_routes = Router::new()
        .route("/api/areas/:id/pendientes", get(handlers::obtener_pendientes_area))
        .route("/api/registros", post(handlers::crear_registro))
        .route("/api/registros/:id", put(handlers::actualizar_registro))
        .route("/api/termometros/reportar-fuera-servicio", post(handlers::reportar_fuera_de_servicio))
        .route("/api/termometros/:id/reparar", post(handlers::reparar_termometro))
        .route_layer(middleware::from_fn(auth::require_registrador));

    // Rutas de Administración
    let admin_routes = Router::new()
        .route("/api/admin/usuarios", get(handlers::listar_usuarios))
        .route("/api/admin/usuarios", post(handlers::crear_usuario))
        .route("/api/admin/usuarios/:id", put(handlers::actualizar_usuario))
        .route("/api/admin/usuarios/:id", delete(handlers::eliminar_usuario))
        .route("/api/admin/areas", post(handlers::crear_area))
        .route("/api/admin/areas/:id", put(handlers::actualizar_area))
        .route("/api/admin/areas/:id", delete(handlers::eliminar_area))
        .route("/api/admin/tipos-termometro", post(handlers::crear_tipo_termometro))
        .route("/api/admin/tipos-termometro/:id", put(handlers::actualizar_tipo_termometro))
        .route("/api/admin/tipos-termometro/:id", delete(handlers::eliminar_tipo_termometro))
        .route("/api/admin/termometros", post(handlers::crear_termometro))
        .route("/api/admin/termometros/:id", put(handlers::actualizar_termometro))
        .route("/api/admin/termometros/:id", delete(handlers::eliminar_termometro))
        .route("/api/admin/registros", get(handlers::listar_registros))
        .route("/api/admin/registros/:id", delete(handlers::eliminar_registro))
        .route("/api/admin/configuracion", get(handlers::obtener_configuracion))
        .route("/api/admin/configuracion", put(handlers::actualizar_configuracion))
        .route("/api/admin/reportes/diario", get(handlers::generar_reporte_diario))
        .route("/api/admin/reportes/mensual", get(handlers::generar_reporte_mensual))
        .route_layer(middleware::from_fn(auth::require_admin));

    // Combinar todas las rutas
    let app = Router::new()
        .merge(public_routes)
        .merge(auth_routes)
        .merge(registrador_routes)
        .merge(admin_routes)
        .fallback_service(ServeDir::new("public"))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(session_layer),
        )
        .with_state(pool);

    // Iniciar servidor
    let addr = format!("{}:{}", host, port);
    tracing::info!("Servidor iniciado en http://{}", addr);

    // Solo mostrar advertencia de seguridad en modo debug
    if cfg!(debug_assertions) {
        tracing::warn!("⚠️  Modo DEBUG activo: Asegúrese de configurar credenciales seguras en producción.");
    } else {
        tracing::info!("✅ Sistema iniciado correctamente");
    }

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}