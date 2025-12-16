use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use anyhow::Result;

/// Inicializa la conexión a la base de datos y crea las tablas si no existen
pub async fn init_db(database_url: &str) -> Result<SqlitePool> {
    // Crear pool de conexiones con opción de crear el archivo si no existe
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            database_url
                .parse::<sqlx::sqlite::SqliteConnectOptions>()?
                .create_if_missing(true)
        )
        .await?;

    // Crear tablas
    create_tables(&pool).await?;

    Ok(pool)
}

/// Crea todas las tablas necesarias en la base de datos
async fn create_tables(pool: &SqlitePool) -> Result<()> {
    // Tabla de usuarios
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS usuarios (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            rol TEXT NOT NULL CHECK(rol IN ('ADMINISTRADOR', 'REGISTRADOR')),
            activo BOOLEAN NOT NULL DEFAULT 1,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Tabla de áreas técnicas
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS areas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL UNIQUE,
            descripcion TEXT,
            activa BOOLEAN NOT NULL DEFAULT 1,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Tabla de tipos de termómetros
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tipos_termometro (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            nombre TEXT NOT NULL UNIQUE,
            descripcion TEXT,
            tiene_humedad BOOLEAN NOT NULL DEFAULT 0,

            -- Rangos de temperatura
            temp_min_operativa REAL NOT NULL,
            temp_max_operativa REAL NOT NULL,
            temp_min_fisica REAL NOT NULL,
            temp_max_fisica REAL NOT NULL,

            -- Rangos de humedad (solo si tiene_humedad = true)
            hum_min_operativa REAL,
            hum_max_operativa REAL,
            hum_min_fisica REAL,
            hum_max_fisica REAL,

            activo BOOLEAN NOT NULL DEFAULT 1,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Tabla de termómetros
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS termometros (
            id INTEGER PRIMARY KEY,
            area_id INTEGER NOT NULL,
            tipo_id INTEGER NOT NULL,
            nombre TEXT,
            ubicacion TEXT,
            activo BOOLEAN NOT NULL DEFAULT 1,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (area_id) REFERENCES areas(id),
            FOREIGN KEY (tipo_id) REFERENCES tipos_termometro(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Tabla de registros de temperatura
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS registros (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            termometro_id INTEGER NOT NULL,
            usuario_id INTEGER NOT NULL,
            ventana_horaria TEXT NOT NULL,

            temp_maxima REAL NOT NULL,
            temp_minima REAL NOT NULL,
            humedad REAL,

            -- Flags de validación
            fuera_rango_operativo BOOLEAN NOT NULL DEFAULT 0,

            observaciones TEXT,
            fecha_registro DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

            FOREIGN KEY (termometro_id) REFERENCES termometros(id),
            FOREIGN KEY (usuario_id) REFERENCES usuarios(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Crear índice único para evitar registros duplicados por día
    // (Un termómetro solo puede tener un registro por ventana horaria por día)
    sqlx::query(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS idx_registros_unique_per_day
        ON registros(termometro_id, DATE(fecha_registro), ventana_horaria)
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
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Tabla de logs de auditoría
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS logs_auditoria (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            usuario_id INTEGER NOT NULL,
            accion TEXT NOT NULL,
            tabla_afectada TEXT NOT NULL,
            registro_id INTEGER,
            datos_anteriores TEXT,
            datos_nuevos TEXT,
            timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (usuario_id) REFERENCES usuarios(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Insertar configuración por defecto si no existe
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO configuracion (clave, valor, descripcion)
        VALUES
            ('registro_hora_1', '14:00', 'Primera ventana horaria de registro'),
            ('registro_hora_2', '02:00', 'Segunda ventana horaria de registro'),
            ('ventana_tolerancia_minutos', '119', 'Minutos de tolerancia antes y después del horario'),
            ('session_timeout_horas', '8', 'Horas de inactividad antes de cerrar sesión')
        "#,
    )
    .execute(pool)
    .await?;

    // Crear usuario administrador por defecto (contraseña: admin123)
    // Hash de bcrypt para "admin123"
    let admin_hash = "$2b$12$W.kQ0r3Xt8FSW6.0yvjyXOqqvbNomk9nUvCthqn9jLKwOibTRa.z.";
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO usuarios (id, username, password_hash, rol)
        VALUES (1, 'admin', ?, 'ADMINISTRADOR')
        "#,
    )
    .bind(admin_hash)
    .execute(pool)
    .await?;

    Ok(())
}

/// Obtiene el valor de una configuración
pub async fn get_config(pool: &SqlitePool, clave: &str) -> Result<String> {
    let row: (String,) = sqlx::query_as("SELECT valor FROM configuracion WHERE clave = ?")
        .bind(clave)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// Actualiza el valor de una configuración
pub async fn set_config(pool: &SqlitePool, clave: &str, valor: &str) -> Result<()> {
    sqlx::query("UPDATE configuracion SET valor = ?, updated_at = CURRENT_TIMESTAMP WHERE clave = ?")
        .bind(valor)
        .bind(clave)
        .execute(pool)
        .await?;
    Ok(())
}

/// Registra una acción en el log de auditoría
pub async fn log_auditoria(
    pool: &SqlitePool,
    usuario_id: i64,
    accion: &str,
    tabla_afectada: &str,
    registro_id: Option<i64>,
    datos_anteriores: Option<&str>,
    datos_nuevos: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO logs_auditoria (usuario_id, accion, tabla_afectada, registro_id, datos_anteriores, datos_nuevos)
        VALUES (?, ?, ?, ?, ?, ?)
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
