use sqlx::{PgPool, postgres::PgPoolOptions};
use anyhow::Result;

/// Inicializa la conexión a la base de datos y crea las tablas si no existen
pub async fn init_db(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .connect(database_url)
        .await?;

    // Crear tablas
    create_tables(&pool).await?;

    Ok(pool)
}

/// Crea todas las tablas necesarias en la base de datos (Sintaxis PostgreSQL)
async fn create_tables(pool: &PgPool) -> Result<()> {
    // Tabla de usuarios
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS usuarios (
            id SERIAL PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            rol TEXT NOT NULL CHECK(rol IN ('ADMINISTRADOR', 'REGISTRADOR')),
            activo BOOLEAN NOT NULL DEFAULT TRUE,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Tabla de áreas técnicas
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS areas (
            id SERIAL PRIMARY KEY,
            nombre TEXT NOT NULL UNIQUE,
            descripcion TEXT,
            responsable TEXT,
            email_notificacion TEXT,
            activa BOOLEAN NOT NULL DEFAULT TRUE,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Tabla de tipos de termómetros
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tipos_termometro (
            id SERIAL PRIMARY KEY,
            nombre TEXT NOT NULL UNIQUE,
            descripcion TEXT,
            tiene_humedad BOOLEAN NOT NULL DEFAULT FALSE,
            temp_min_operativa REAL NOT NULL,
            temp_max_operativa REAL NOT NULL,
            temp_min_fisica REAL NOT NULL,
            temp_max_fisica REAL NOT NULL,
            hum_min_operativa REAL,
            hum_max_operativa REAL,
            hum_min_fisica REAL,
            hum_max_fisica REAL,
            activo BOOLEAN NOT NULL DEFAULT TRUE,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Tabla de termómetros
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS termometros (
            id SERIAL PRIMARY KEY,
            area_id INTEGER NOT NULL REFERENCES areas(id),
            tipo_id INTEGER NOT NULL REFERENCES tipos_termometro(id),
            nombre TEXT,
            ubicacion TEXT,
            activo BOOLEAN NOT NULL DEFAULT TRUE,
            fuera_de_servicio BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Tabla de mantenimiento de termómetros
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS mantenimiento_termometros (
            id SERIAL PRIMARY KEY,
            termometro_id INTEGER NOT NULL REFERENCES termometros(id),
            usuario_reporta_id INTEGER NOT NULL REFERENCES usuarios(id),
            fecha_reporte TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
            motivo TEXT NOT NULL,
            comentarios_reporte TEXT,
            fecha_reparacion TIMESTAMP WITH TIME ZONE,
            usuario_repara_id INTEGER REFERENCES usuarios(id),
            detalle_reparacion TEXT,
            estado TEXT NOT NULL DEFAULT 'PENDIENTE' CHECK (estado IN ('PENDIENTE', 'REPARADO'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Tabla de registros de temperatura
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS registros (
            id SERIAL PRIMARY KEY,
            termometro_id INTEGER NOT NULL REFERENCES termometros(id),
            usuario_id INTEGER NOT NULL REFERENCES usuarios(id),
            ventana_horaria TEXT NOT NULL,
            temp_actual REAL,
            temp_maxima REAL NOT NULL,
            temp_minima REAL NOT NULL,
            humedad REAL,
            fuera_rango_operativo BOOLEAN NOT NULL DEFAULT FALSE,
            observaciones TEXT,
            fecha_registro TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Índice único para evitar duplicados
    sqlx::query(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS idx_registros_unique_per_day
        ON registros(termometro_id, (CAST(fecha_registro AT TIME ZONE 'UTC' AS DATE)), ventana_horaria)
        "#,
    )
    .execute(pool)
    .await?;

    // Tabla de configuración global
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS configuracion (
            clave TEXT PRIMARY KEY,
            valor TEXT NOT NULL,
            descripcion TEXT,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Tabla de logs de auditoría
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS logs_auditoria (
            id SERIAL PRIMARY KEY,
            usuario_id INTEGER NOT NULL REFERENCES usuarios(id),
            accion TEXT NOT NULL,
            tabla_afectada TEXT NOT NULL,
            registro_id INTEGER,
            datos_anteriores TEXT,
            datos_nuevos TEXT,
            timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Insertar configuración por defecto si no existe
    let reg_hora_1 = std::env::var("REGISTRO_HORA_1").unwrap_or_else(|_| "14:00".to_string());
    let reg_hora_2 = std::env::var("REGISTRO_HORA_2").unwrap_or_else(|_| "02:00".to_string());
    let ventana_tolerancia = std::env::var("VENTANA_TOLERANCIA_MINUTOS").unwrap_or_else(|_| "119".to_string());
    
    sqlx::query(
        r#"
        INSERT INTO configuracion (clave, valor, descripcion)
        VALUES
            ('registro_hora_1', $1, 'Primera ventana horaria'),
            ('registro_hora_2', $2, 'Segunda ventana horaria'),
            ('ventana_tolerancia_minutos', $3, 'Minutos de tolerancia para las ventanas horarias'),
            ('restriccion_ventana_activa', '0', 'Indica si se restringe el registro a las ventanas horarias (0=No, 1=Sí)')
        ON CONFLICT (clave) DO NOTHING
        "#,
    )
    .bind(reg_hora_1)
    .bind(reg_hora_2)
    .bind(ventana_tolerancia)
    .execute(pool)
    .await?;

    // Crear usuario administrador por defecto (contraseña: admin123)
    let admin_hash = "$2b$12$W.kQ0r3Xt8FSW6.0yvjyXOqqvbNomk9nUvCthqn9jLKwOibTRa.z.";
    sqlx::query(
        r#"
        INSERT INTO usuarios (username, password_hash, rol)
        VALUES ('admin', $1, 'ADMINISTRADOR')
        ON CONFLICT (username) DO NOTHING
        "#,
    )
    .bind(admin_hash)
    .execute(pool)
    .await?;

    Ok(())
}

/// Obtiene el valor de una configuración
pub async fn get_config(pool: &PgPool, clave: &str) -> Result<String> {
    let row: (String,) = sqlx::query_as("SELECT valor FROM configuracion WHERE clave = $1")
        .bind(clave)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// Actualiza el valor de una configuración (o la crea si no existe)
pub async fn set_config(pool: &PgPool, clave: &str, valor: &str) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO configuracion (clave, valor)
        VALUES ($1, $2)
        ON CONFLICT (clave) DO UPDATE SET 
            valor = EXCLUDED.valor,
            updated_at = CURRENT_TIMESTAMP
        "#
    )
    .bind(clave)
    .bind(valor)
    .execute(pool)
    .await?;
    Ok(())
}

/// Registra una acción en el log de auditoría
pub async fn log_auditoria(
    pool: &PgPool,
    usuario_id: i32,
    accion: &str,
    tabla_afectada: &str,
    registro_id: Option<i32>,
    datos_anteriores: Option<&str>,
    datos_nuevos: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO logs_auditoria (usuario_id, accion, tabla_afectada, registro_id, datos_anteriores, datos_nuevos)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(usuario_id)
    .bind(accion)
    .bind(tabla_afectada)
    .bind(registro_id)
    .bind(datos_anteriores)
    .bind(datos_nuevos)
    .execute(pool)
    .await?;
    Ok(())
}
