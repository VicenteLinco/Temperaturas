use sqlx::SqlitePool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let sqlite_url = "sqlite:C:/Users/VALTEK/Documents/GitHub/Temperaturas/datos.db";
    let pool = SqlitePool::connect(sqlite_url).await?;

    let count_registros: (i64,) = sqlx::query_as("SELECT count(*) FROM registros").fetch_one(&pool).await?;
    let count_termometros: (i64,) = sqlx::query_as("SELECT count(*) FROM termometros").fetch_one(&pool).await?;
    let count_areas: (i64,) = sqlx::query_as("SELECT count(*) FROM areas").fetch_one(&pool).await?;

    println!("--- RESUMEN DE BASE DE DATOS LOCAL ---");
    println!("📍 Áreas: {}", count_areas.0);
    println!("📏 Termómetros: {}", count_termometros.0);
    println!("📝 Registros: {}", count_registros.0);
    
    // Ver los últimos 5 registros para estar seguros de que son recientes
    println!("
--- ÚLTIMOS 5 REGISTROS ---");
    let ultimos: Vec<(i64, String, f64, String)> = sqlx::query_as("SELECT id, ventana_horaria, temp_maxima, fecha_registro FROM registros ORDER BY id DESC LIMIT 5")
        .fetch_all(&pool).await?;
    
    for r in ultimos {
        println!("ID: {} | Ventana: {} | Temp Máx: {}°C | Fecha: {}", r.0, r.1, r.2, r.3);
    }

    Ok(())
}
