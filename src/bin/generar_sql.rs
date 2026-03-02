use sqlx::SqlitePool;
use std::fs::File;
use std::io::Write;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let sqlite_url = "sqlite:C:/Users/VALTEK/Documents/GitHub/Temperaturas/datos.db";
    let pool = SqlitePool::connect(sqlite_url).await?;

    println!("📖 Leyendo registros de SQLite...");
    let registros: Vec<(i32, i32, i32, String, Option<f32>, f32, f32, Option<f32>, i32, Option<String>, String)> = 
        sqlx::query_as("SELECT id, termometro_id, usuario_id, ventana_horaria, temp_actual, temp_maxima, temp_minima, humedad, fuera_rango_operativo, observaciones, fecha_registro FROM registros")
        .fetch_all(&pool).await?;

    let mut file = File::create("REGISTROS_MIGRACION.sql")?;
    writeln!(file, "-- MIGRACIÓN DE {} REGISTROS", registros.len())?;
    writeln!(file, "INSERT INTO registros (id, termometro_id, usuario_id, ventana_horaria, temp_actual, temp_maxima, temp_minima, humedad, fuera_rango_operativo, observaciones, fecha_registro) VALUES")?;

    for (i, r) in registros.iter().enumerate() {
        let temp_actual = r.4.map(|v| v.to_string()).unwrap_or_else(|| "NULL".to_string());
        let humedad = r.7.map(|v| v.to_string()).unwrap_or_else(|| "NULL".to_string());
        let fuera_rango = if r.8 == 1 { "true" } else { "false" };
        let obs = r.9.as_ref().map(|s| format!("'{}'", s.replace("'", "''"))).unwrap_or_else(|| "NULL".to_string());
        
        let comma = if i == registros.len() - 1 { ";" } else { "," };
        
        writeln!(file, "({}, {}, {}, '{}', {}, {}, {}, {}, {}, {}, '{}'::timestamp){}", 
            r.0, r.1, r.2, r.3, temp_actual, r.5, r.6, humedad, fuera_rango, obs, r.10, comma)?;
    }

    println!("✅ Archivo REGISTROS_MIGRACION.sql generado con éxito.");
    Ok(())
}
