use sqlx::{SqlitePool, PgPool};
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    
    let sqlite_url = "sqlite:C:/Users/VALTEK/Documents/GitHub/Temperaturas/datos.db";
    let pg_url = env::var("DATABASE_URL").expect("DATABASE_URL no configurada");
    let pg_url = pg_url.trim_matches('"').to_string();

    println!("🚀 Iniciando migración TOTAL de datos...");
    
    let sqlite_pool = SqlitePool::connect(sqlite_url).await?;
    // Usamos una conexión simple para evitar el error de UTF8 con el pooler de Neon si persiste
    let pg_pool = PgPool::connect(&pg_url).await?;

    // 1. ÁREAS
    println!("📍 Migrando Áreas...");
    let areas: Vec<(i32, String, Option<String>, bool)> = sqlx::query_as("SELECT id, nombre, descripcion, activa FROM areas").fetch_all(&sqlite_pool).await?;
    for a in areas {
        sqlx::query("INSERT INTO areas (id, nombre, descripcion, activa) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET nombre = EXCLUDED.nombre")
            .bind(a.0).bind(a.1).bind(a.2).bind(a.3).execute(&pg_pool).await?;
    }

    // 2. TIPOS
    println!("🌡️ Migrando Tipos...");
    let tipos: Vec<(i32, String, Option<String>, bool, f32, f32, f32, f32, Option<f32>, Option<f32>, Option<f32>, Option<f32>, bool)> = 
        sqlx::query_as("SELECT id, nombre, descripcion, tiene_humedad, temp_min_operativa, temp_max_operativa, temp_min_fisica, temp_max_fisica, hum_min_operativa, hum_max_operativa, hum_min_fisica, hum_max_fisica, activo FROM tipos_termometro")
        .fetch_all(&sqlite_pool).await?;
    for t in tipos {
        sqlx::query("INSERT INTO tipos_termometro (id, nombre, descripcion, tiene_humedad, temp_min_operativa, temp_max_operativa, temp_min_fisica, temp_max_fisica, hum_min_operativa, hum_max_operativa, hum_min_fisica, hum_max_fisica, activo) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) ON CONFLICT (id) DO NOTHING")
            .bind(t.0).bind(t.1).bind(t.2).bind(t.3).bind(t.4).bind(t.5).bind(t.6).bind(t.7).bind(t.8).bind(t.9).bind(t.10).bind(t.11).bind(t.12).execute(&pg_pool).await?;
    }

    // 3. TERMÓMETROS
    println!("📏 Migrando Termómetros...");
    let termos: Vec<(i32, i32, i32, Option<String>, Option<String>, bool, bool)> = 
        sqlx::query_as("SELECT id, area_id, tipo_id, nombre, ubicacion, activo, fuera_de_servicio FROM termometros").fetch_all(&sqlite_pool).await?;
    for t in termos {
        sqlx::query("INSERT INTO termometros (id, area_id, tipo_id, nombre, ubicacion, activo, fuera_de_servicio) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (id) DO NOTHING")
            .bind(t.0).bind(t.1).bind(t.2).bind(t.3).bind(t.4).bind(t.5).bind(t.6).execute(&pg_pool).await?;
    }

    // 4. REGISTROS (El plato fuerte)
    println!("📝 Migrando 1021 Registros (en bloques)...");
    let registros: Vec<(i32, i32, i32, String, Option<f32>, f32, f32, Option<f32>, bool, Option<String>, String)> = 
        sqlx::query_as("SELECT id, termometro_id, usuario_id, ventana_horaria, temp_actual, temp_maxima, temp_minima, humedad, fuera_rango_operativo, observaciones, fecha_registro FROM registros").fetch_all(&sqlite_pool).await?;
    
    let mut migrados = 0;
    for r in registros {
        // Convertir la fecha de String (SQLite) a TIMESTAMP (Postgres)
        let res = sqlx::query("INSERT INTO registros (id, termometro_id, usuario_id, ventana_horaria, temp_actual, temp_maxima, temp_minima, humedad, fuera_rango_operativo, observaciones, fecha_registro) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::timestamp) ON CONFLICT (id) DO NOTHING")
            .bind(r.0).bind(r.1).bind(r.2).bind(r.3).bind(r.4).bind(r.5).bind(r.6).bind(r.7).bind(r.8).bind(r.9).bind(r.10)
            .execute(&pg_pool).await;
        
        if res.is_ok() { migrados += 1; }
        if migrados % 100 == 0 { println!("   - {} registros procesados...", migrados); }
    }

    println!("
✅ PROCESO FINALIZADO");
    println!("📊 Registros migrados exitosamente: {}", migrados);
    Ok(())
}
