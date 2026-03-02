use sqlx::{SqlitePool, PgPool};
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    
    let sqlite_url = "sqlite:datos.db";
    let pg_url = env::var("DATABASE_URL").expect("DATABASE_URL no configurada");
    
    // Limpiar comillas si existen
    let pg_url = pg_url.trim_matches('"').to_string();

    println!("🚀 Iniciando migración de datos...");
    println!("📦 Origen: {}", sqlite_url);
    println!("☁️  Destino: [URL Protegida]");

    let sqlite_pool = SqlitePool::connect(sqlite_url).await?;
    let pg_pool = PgPool::connect(&pg_url).await?;

    // 1. Migrar Áreas
    println!("📍 Migrando Áreas...");
    let areas: Vec<(i32, String, Option<String>, bool)> = sqlx::query_as("SELECT id, nombre, descripcion, activa FROM areas")
        .fetch_all(&sqlite_pool).await?;

    for area in areas {
        println!("   - Área: {}", area.1);
        sqlx::query("INSERT INTO areas (id, nombre, descripcion, activa) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO NOTHING")
            .bind(area.0).bind(area.1).bind(area.2).bind(area.3)
            .execute(&pg_pool).await?;
    }

    // 2. Migrar Tipos de Termómetro
    println!("🌡️ Migrando Tipos de Termómetro...");
    let tipos: Vec<(i32, String, Option<String>, bool, f32, f32, f32, f32, Option<f32>, Option<f32>, Option<f32>, Option<f32>, bool)> = 
        sqlx::query_as("SELECT id, nombre, descripcion, tiene_humedad, temp_min_operativa, temp_max_operativa, temp_min_fisica, temp_max_fisica, hum_min_operativa, hum_max_operativa, hum_min_fisica, hum_max_fisica, activo FROM tipos_termometro")
        .fetch_all(&sqlite_pool).await?;

    for t in tipos {
        println!("   - Tipo: {}", t.1);
        sqlx::query("INSERT INTO tipos_termometro (id, nombre, descripcion, tiene_humedad, temp_min_operativa, temp_max_operativa, temp_min_fisica, temp_max_fisica, hum_min_operativa, hum_max_operativa, hum_min_fisica, hum_max_fisica, activo) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) ON CONFLICT (id) DO NOTHING")
            .bind(t.0).bind(t.1).bind(t.2).bind(t.3).bind(t.4).bind(t.5).bind(t.6).bind(t.7).bind(t.8).bind(t.9).bind(t.10).bind(t.11).bind(t.12)
            .execute(&pg_pool).await?;
    }

    // 3. Migrar Termómetros
    println!("📏 Migrando Termómetros...");
    let termos: Vec<(i32, i32, i32, Option<String>, Option<String>, bool, bool)> = 
        sqlx::query_as("SELECT id, area_id, tipo_id, nombre, ubicacion, activo, fuera_de_servicio FROM termometros")
        .fetch_all(&sqlite_pool).await?;

    for t in termos {
        println!("   - Termómetro: {}", t.3.as_deref().unwrap_or("Sin nombre"));
        sqlx::query("INSERT INTO termometros (id, area_id, tipo_id, nombre, ubicacion, activo, fuera_de_servicio) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (id) DO NOTHING")
            .bind(t.0).bind(t.1).bind(t.2).bind(t.3).bind(t.4).bind(t.5).bind(t.6)
            .execute(&pg_pool).await?;
    }

    println!("\n✅ MIGRACIÓN EXITOSA: Tus datos locales ya están en la nube.");
    Ok(())
}
