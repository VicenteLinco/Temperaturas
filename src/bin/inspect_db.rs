use sqlx::PgPool;
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;

    println!("🔍 Comprobando JOIN de termómetros con áreas y tipos_termometro:");
    
    let total_termometros: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM termometros").fetch_one(&pool).await?;
    println!("- Total en tabla 'termometros': {}", total_termometros.0);

    let join_termometros: Vec<(i32, Option<String>, i32, i32, Option<String>, Option<String>)> = sqlx::query_as(
        r#"
        SELECT t.id, t.nombre, t.area_id, t.tipo_id, a.nombre as area_nombre, ti.nombre as tipo_nombre
        FROM termometros t
        LEFT JOIN areas a ON t.area_id = a.id
        LEFT JOIN tipos_termometro ti ON t.tipo_id = ti.id
        "#
    ).fetch_all(&pool).await?;

    println!("- Total recuperados por JOIN: {}", join_termometros.len());

    let mut huérfanos = 0;
    for t in &join_termometros {
        if t.4.is_none() || t.5.is_none() {
            huérfanos += 1;
            println!("  ⚠️ Termómetro ID {} ({:?}): Area={:?} (ID {}), Tipo={:?} (ID {})", 
                t.0, t.1, t.4, t.2, t.5, t.3);
        }
    }

    if huérfanos == 0 {
        println!("✅ Todos los {} termómetros tienen áreas y tipos válidos.", join_termometros.len());
    } else {
        println!("⚠️ Se encontraron {} termómetros huérfanos.", huérfanos);
    }

    println!("\n📋 Lista de IDs de termómetros en la base de datos:");
    let ids: Vec<i32> = join_termometros.iter().map(|t| t.0).collect();
    println!("{:?}", ids);

    Ok(())
}
