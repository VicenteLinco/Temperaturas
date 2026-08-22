use sqlx::PgPool;
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;

    println!("============================================================");
    println!("🔍 INSPECCIÓN DE BASE DE DATOS");
    println!("============================================================");
    
    // 1. Áreas
    let areas: Vec<(i32, String, bool)> = sqlx::query_as(
        "SELECT id, nombre, activa FROM areas ORDER BY id"
    ).fetch_all(&pool).await?;
    println!("\n📍 ÁREAS (Total: {}):", areas.len());
    for a in &areas {
        println!("  - [{}] {} (Activa: {})", a.0, a.1, if a.2 { "✅" } else { "❌" });
    }

    // 2. Termómetros y Estado
    let termometros: Vec<(i32, Option<String>, String, String, bool, bool)> = sqlx::query_as(
        r#"
        SELECT t.id, t.nombre, a.nombre as area, ti.nombre as tipo, t.activo, t.fuera_de_servicio
        FROM termometros t
        JOIN areas a ON t.area_id = a.id
        JOIN tipos_termometro ti ON t.tipo_id = ti.id
        ORDER BY t.id
        "#
    ).fetch_all(&pool).await?;

    let mut fs_count = 0;
    let mut inactivos_count = 0;
    let mut operativos_count = 0;

    println!("\n🌡️ TERMÓMETROS (Total: {}):", termometros.len());
    for t in &termometros {
        let estado = if !t.4 {
            inactivos_count += 1;
            "🔴 INACTIVO"
        } else if t.5 {
            fs_count += 1;
            "⚠️ FUERA DE SERVICIO (MODO REPARACIÓN)"
        } else {
            operativos_count += 1;
            "🟢 OPERATIVO"
        };

        if t.5 || !t.4 {
            println!("  - ID {}: {:?} | Área: {} | Tipo: {} | Estado: {}", 
                t.0, t.1.as_deref().unwrap_or("Sin nombre"), t.2, t.3, estado);
        }
    }

    println!("\n📊 RESUMEN TERMÓMETROS:");
    println!("  - Operativos: {}", operativos_count);
    println!("  - Fuera de Servicio / Reparación: {}", fs_count);
    println!("  - Inactivos: {}", inactivos_count);

    // 3. Mantenimientos Pendientes
    let mantenimientos: Vec<(i32, i32, String, Option<String>, String, String)> = sqlx::query_as(
        r#"
        SELECT mt.id, mt.termometro_id, mt.motivo, mt.comentarios_reporte, mt.estado, u.username
        FROM mantenimiento_termometros mt
        JOIN usuarios u ON mt.usuario_reporta_id = u.id
        ORDER BY mt.id DESC
        "#
    ).fetch_all(&pool).await?;

    println!("\n🛠️ REGISTROS DE MANTENIMIENTO (Total: {}):", mantenimientos.len());
    let mut pendientes = 0;
    for m in &mantenimientos {
        if m.4 == "PENDIENTE" {
            pendientes += 1;
            println!("  - [PENDIENTE] ID: {}, Termómetro ID: {} | Motivo: '{}' | Detalle: {:?} | Reportado por: {}", 
                m.0, m.1, m.2, m.3, m.5);
        }
    }
    if pendientes == 0 {
        println!("  - No hay mantenimientos marcados como PENDIENTE.");
    }

    // 5. Verificación de Integridad de Datos (Nombres y Rangos)
    println!("\n🔍 VERIFICACIÓN DE INTEGRIDAD DE DATOS:");
    let tipos: Vec<(i32, String, f32, f32, Option<f32>, Option<f32>)> = sqlx::query_as(
        "SELECT id, nombre, temp_min_operativa, temp_max_operativa, hum_min_operativa, hum_max_operativa FROM tipos_termometro"
    ).fetch_all(&pool).await?;

    let mut rangos_invalidos = 0;
    for tip in &tipos {
        if tip.2 >= tip.3 {
            rangos_invalidos += 1;
            println!("  ⚠️ Rango inválido en Tipo ID {}: {} (Min {}°C >= Max {}°C)", tip.0, tip.1, tip.2, tip.3);
        }
    }
    if rangos_invalidos == 0 {
        println!("  ✅ Todos los tipos de termómetro tienen rangos operativos válidos (Min < Max).");
    }

    let mut nombres_sucios = 0;
    for t in &termometros {
        if let Some(nom) = &t.1 {
            if nom.contains('\n') || nom.contains('\r') || nom.contains("  ") {
                nombres_sucios += 1;
                println!("  ⚠️ Nombre con saltos de línea/espacios extra en Termómetro ID {}: {:?}", t.0, nom);
            }
        }
    }
    if nombres_sucios == 0 {
        println!("  ✅ Todos los nombres de termómetros están limpios.");
    }

    println!("============================================================");

    Ok(())
}

