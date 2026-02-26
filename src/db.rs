use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use anyhow::Result;

/// Inicializa la conexión a la base de datos y crea las tablas si no existen
pub async fn init_db(database_url: &str) -> Result<SqlitePool> {
    // Crear pool de conexiones con opción de crear el archivo si no existe
    let pool = SqlitePoolOptions::new()
        .max_connections(20)  // ✅ Aumentado de 5 a 20 para mejor concurrencia
        .min_connections(2)   // ✅ Mantener al menos 2 conexiones activas
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
            fuera_de_servicio BOOLEAN NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (area_id) REFERENCES areas(id),
            FOREIGN KEY (tipo_id) REFERENCES tipos_termometro(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Tabla de mantenimiento de termómetros
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS mantenimiento_termometros (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            termometro_id INTEGER NOT NULL,
            usuario_reporta_id INTEGER NOT NULL,
            fecha_reporte DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            motivo TEXT NOT NULL,
            comentarios_reporte TEXT,
            fecha_reparacion DATETIME,
            usuario_repara_id INTEGER,
            detalle_reparacion TEXT,
            estado TEXT NOT NULL DEFAULT 'PENDIENTE' CHECK(estado IN ('PENDIENTE', 'REPARADO')),
            FOREIGN KEY (termometro_id) REFERENCES termometros(id),
            FOREIGN KEY (usuario_reporta_id) REFERENCES usuarios(id),
            FOREIGN KEY (usuario_repara_id) REFERENCES usuarios(id)
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

            temp_actual REAL,
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

    // ✅ NUEVOS ÍNDICES PARA MEJORAR PERFORMANCE
    // Índice para búsquedas por fecha
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_registros_fecha
         ON registros(fecha_registro)"
    )
    .execute(pool)
    .await?;

    // Índice para joins con termómetros
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_registros_termometro
         ON registros(termometro_id)"
    )
    .execute(pool)
    .await?;

    // Índice para filtros por usuario
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_registros_usuario
         ON registros(usuario_id)"
    )
    .execute(pool)
    .await?;

    // Índice para joins de termómetros con áreas
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_termometros_area
         ON termometros(area_id)"
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

    // Índice para logs de auditoría por usuario
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_logs_usuario
         ON logs_auditoria(usuario_id)"
    )
    .execute(pool)
    .await?;

    // Índice para logs de auditoría por fecha
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_logs_timestamp
         ON logs_auditoria(timestamp)"
    )
    .execute(pool)
    .await?;

    // Tabla de alertas
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS alertas (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            registro_id INTEGER NOT NULL,
            tipo TEXT NOT NULL CHECK(tipo IN ('ADVERTENCIA', 'CRITICA')),
            fecha_alerta DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            temperatura_registrada REAL NOT NULL,
            humedad_registrada REAL,
            desviacion REAL NOT NULL,
            campo_afectado TEXT NOT NULL, -- 'temp_maxima', 'temp_minima', 'humedad'

            -- Notificación
            notificado BOOLEAN NOT NULL DEFAULT 0,
            fecha_notificacion DATETIME,
            destinatario TEXT,

            -- Resolución
            estado TEXT NOT NULL DEFAULT 'PENDIENTE' CHECK(estado IN ('PENDIENTE', 'RESUELTO', 'AUTO_RESUELTO')),
            fecha_resolucion DATETIME,
            accion_correctiva TEXT,
            responsable_resolucion TEXT,

            FOREIGN KEY (registro_id) REFERENCES registros(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // ✅ MIGRACIÓN: Agregar campo temp_actual si no existe
    sqlx::query("ALTER TABLE registros ADD COLUMN temp_actual REAL").execute(pool).await.ok();

    // ✅ MIGRACIÓN: Agregar campo fuera_de_servicio a termometros
    sqlx::query("ALTER TABLE termometros ADD COLUMN fuera_de_servicio BOOLEAN NOT NULL DEFAULT 0").execute(pool).await.ok();

    // Agregar campos a tabla areas si no existen
    sqlx::query("ALTER TABLE areas ADD COLUMN descripcion TEXT").execute(pool).await.ok();
    sqlx::query("ALTER TABLE areas ADD COLUMN responsable TEXT").execute(pool).await.ok();
    sqlx::query("ALTER TABLE areas ADD COLUMN email_notificacion TEXT").execute(pool).await.ok();

    // Obtener valores de configuración de variables de entorno o usar defaults
    let reg_hora_1 = std::env::var("REGISTRO_HORA_1").unwrap_or_else(|_| "14:00".to_string());
    let reg_hora_2 = std::env::var("REGISTRO_HORA_2").unwrap_or_else(|_| "02:00".to_string());
    let tol_minutos = std::env::var("VENTANA_TOLERANCIA_MINUTOS").unwrap_or_else(|_| "119".to_string());
    let sess_timeout = std::env::var("SESSION_TIMEOUT_HORAS").unwrap_or_else(|_| "8".to_string());
    let smtp_host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "".to_string());
    let smtp_port = std::env::var("SMTP_PORT").unwrap_or_else(|_| "587".to_string());
    let smtp_user = std::env::var("SMTP_USERNAME").unwrap_or_else(|_| "".to_string());
    let smtp_pass = std::env::var("SMTP_PASSWORD").unwrap_or_else(|_| "".to_string());
    let smtp_from_email = std::env::var("SMTP_FROM_EMAIL").unwrap_or_else(|_| "".to_string());
    let smtp_from_name = std::env::var("SMTP_FROM_NAME").unwrap_or_else(|_| "Sistema de Temperaturas".to_string());
    let notif_activas = std::env::var("NOTIFICACIONES_ACTIVAS").unwrap_or_else(|_| "0".to_string());
    let empresa_nombre = std::env::var("EMPRESA_NOMBRE").unwrap_or_else(|_| "".to_string());

    // Insertar configuración por defecto si no existe
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO configuracion (clave, valor, descripcion)
        VALUES
            ('registro_hora_1', ?, 'Primera ventana horaria de registro'),
            ('registro_hora_2', ?, 'Segunda ventana horaria de registro'),
            ('ventana_tolerancia_minutos', ?, 'Minutos de tolerancia antes y después del horario'),
            ('session_timeout_horas', ?, 'Horas de inactividad antes de cerrar sesión'),
            ('smtp_host', ?, 'Servidor SMTP para envío de emails'),
            ('smtp_port', ?, 'Puerto SMTP (587 para TLS, 465 para SSL)'),
            ('smtp_username', ?, 'Usuario SMTP'),
            ('smtp_password', ?, 'Contraseña SMTP'),
            ('smtp_from_email', ?, 'Email remitente'),
            ('smtp_from_name', ?, 'Nombre del remitente'),
            ('notificaciones_activas', ?, 'Activar/desactivar notificaciones automáticas'),
            ('empresa_nombre', ?, 'Nombre de la empresa para reportes')
        "#,
    )
    .bind(reg_hora_1)
    .bind(reg_hora_2)
    .bind(tol_minutos)
    .bind(sess_timeout)
    .bind(smtp_host)
    .bind(smtp_port)
    .bind(smtp_user)
    .bind(smtp_pass)
    .bind(smtp_from_email)
    .bind(smtp_from_name)
    .bind(notif_activas)
    .bind(empresa_nombre)
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

    // Migrar datos antiguos: actualizar ventana_horaria de "noche"/"mañana" a horas
    sqlx::query(
        r#"
        UPDATE registros
        SET ventana_horaria = '02:00'
        WHERE ventana_horaria IN ('noche', 'Noche', 'NOCHE', '02am', '2am')
        "#,
    )
    .execute(pool)
    .await
    .ok(); // Ignorar errores si no hay registros antiguos

    sqlx::query(
        r#"
        UPDATE registros
        SET ventana_horaria = '14:00'
        WHERE ventana_horaria IN ('mañana', 'Mañana', 'MAÑANA', 'dia', 'día', '14pm', '2pm')
        "#,
    )
    .execute(pool)
    .await
    .ok(); // Ignorar errores si no hay registros antiguos

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