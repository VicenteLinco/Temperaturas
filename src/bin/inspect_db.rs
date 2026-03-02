use sqlx::PgPool;
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;

    let count_usuarios: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM usuarios").fetch_one(&pool).await?;
    let count_areas: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM areas").fetch_one(&pool).await?;
    let count_termometros: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM termometros").fetch_one(&pool).await?;
    let count_registros: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM registros").fetch_one(&pool).await?;

    println!("📊 Estadísticas de la Base de Datos:");
    println!("- Usuarios: {}", count_usuarios.0);
    println!("- Áreas: {}", count_areas.0);
    println!("- Termómetros: {}", count_termometros.0);
    println!("- Registros: {}", count_registros.0);

    if count_registros.0 > 0 {
        println!("
📝 Últimos 5 registros:");
        let registros: Vec<(i32, i32, i32, String, f32, f32, String)> = 
            sqlx::query_as("SELECT id, termometro_id, usuario_id, ventana_horaria, temp_maxima, temp_minima, fecha_registro::text FROM registros ORDER BY id DESC LIMIT 5")
            .fetch_all(&pool).await?;
        
        for r in registros {
            println!("  ID: {}, Termo: {}, User: {}, Ventana: {}, Max: {}, Min: {}, Fecha: {}", r.0, r.1, r.2, r.3, r.4, r.5, r.6);
        }
    }

    Ok(())
}
