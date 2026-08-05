# ⚡ Quick Wins - Mejoras Rápidas (< 2 horas cada una)

Estos son cambios que puedes implementar HOY para mejorar significativamente el proyecto con mínimo esfuerzo.

---

## 🔒 Seguridad (30 minutos)

### 1. Cookies Seguras
**Archivo**: `src/main.rs` línea 54

**Cambio**:
```rust
// ANTES:
let session_layer = SessionManagerLayer::new(session_store)
    .with_http_only(true)
    .with_expiry(Expiry::OnInactivity(Duration::from_secs(
        session_timeout_hours * 3600,
    )));

// DESPUÉS:
let session_layer = SessionManagerLayer::new(session_store)
    .with_secure(cfg!(not(debug_assertions)))  // ✅ Secure en producción
    .with_same_site(tower_sessions::cookie::SameSite::Strict)  // ✅ CSRF protection
    .with_http_only(true)
    .with_expiry(Expiry::OnInactivity(Duration::from_secs(
        session_timeout_hours * 3600,
    )));
```

**Beneficio**: 🔒 Protección contra CSRF y cookie hijacking

---

### 2. Ocultar Credenciales de Logs
**Archivo**: `src/main.rs` línea 121

**Cambio**:
```rust
// ANTES:
println!("Usuario por defecto: admin / admin123");

// DESPUÉS:
if cfg!(debug_assertions) {
    println!("⚠️  Usuario por defecto: admin / admin123 (CAMBIAR EN PRODUCCIÓN)");
} else {
    println!("✅ Sistema iniciado");
}
```

**Beneficio**: 🔒 No exponer credenciales en logs de producción

---

### 3. Validar Temp Máxima >= Temp Mínima
**Archivo**: `src/logic.rs` línea 153

**Cambio**:
```rust
// AGREGAR al inicio de validar_registro():
pub fn validar_registro(
    temp_maxima: f64,
    temp_minima: f64,
    humedad: Option<f64>,
    tipo: &TipoTermometro,
) -> Result<(bool, Vec<String>), String> {
    // ✅ NUEVA VALIDACIÓN
    if temp_maxima < temp_minima {
        return Err(format!(
            "Temperatura máxima ({:.1}°C) no puede ser menor que mínima ({:.1}°C)",
            temp_maxima, temp_minima
        ));
    }

    let mut advertencias = Vec::new();
    let mut fuera_rango = false;

    // ... resto del código
}
```

**Beneficio**: 🐛 Previene datos inválidos

---

## ⚡ Performance (45 minutos)

### 4. Agregar Índices a Base de Datos
**Archivo**: `src/db.rs` después de línea 145

**Cambio**:
```rust
// AGREGAR:
sqlx::query(
    "CREATE INDEX IF NOT EXISTS idx_registros_fecha
     ON registros(fecha_registro)"
).execute(pool).await?;

sqlx::query(
    "CREATE INDEX IF NOT EXISTS idx_registros_termometro
     ON registros(termometro_id)"
).execute(pool).await?;

sqlx::query(
    "CREATE INDEX IF NOT EXISTS idx_termometros_area
     ON termometros(area_id)"
).execute(pool).await?;
```

**Beneficio**: ⚡ Queries 5-10x más rápidas con muchos registros

---

### 5. Aumentar Pool de Conexiones
**Archivo**: `src/main.rs` línea 7

**Cambio**:
```rust
// ANTES:
.max_connections(5)

// DESPUÉS:
.max_connections(20)  // ✅ Mejor para producción
```

**Beneficio**: ⚡ Más requests concurrentes

---

## 🐛 Corrección de Bugs (30 minutos)

### 6. Validar Username Único al Crear
**Archivo**: `src/handlers.rs` línea 114

**Cambio**:
```rust
// AGREGAR ANTES de hash_password:
// Verificar si el usuario ya existe
let existe: Option<(i64,)> = sqlx::query_as(
    "SELECT id FROM usuarios WHERE username = ?"
)
.bind(&payload.username)
.fetch_optional(&pool)
.await
.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

if existe.is_some() {
    return Err(StatusCode::CONFLICT);  // 409 Conflict
}

// Hash de contraseña
let password_hash = hash_password(&payload.password)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
```

**Beneficio**: 🐛 Previene usuarios duplicados

---

### 7. Limpiar Valor de Humedad en Modal
**Archivo**: `public/index.html` línea 784

**Cambio**:
```javascript
// AGREGAR en modo nuevo:
document.getElementById('registroId').value = '';
document.getElementById('esEdicion').value = 'false';
document.getElementById('humedad').value = '';  // ✅ AGREGAR ESTA LÍNEA
```

**Beneficio**: 🐛 No reutilizar valor anterior

---

## 🎨 UX/UI (15 minutos)

### 8. Deshabilitar Botón Guardar Durante Request
**Archivo**: `public/index.html` línea 813

**Cambio**:
```javascript
async function guardarRegistro() {
    // ✅ AGREGAR AL INICIO:
    const btnGuardar = document.querySelector('#registroModal .btn-primary:last-child');
    btnGuardar.disabled = true;
    btnGuardar.textContent = 'Guardando...';

    try {
        // ... código existente ...
    } finally {
        // ✅ AGREGAR AL FINAL:
        btnGuardar.disabled = false;
        btnGuardar.textContent = 'Guardar';
    }
}
```

**Beneficio**: 🎨 Evita doble-click y da feedback

---

### 9. Mejorar Mensajes de Error
**Archivo**: `public/index.html` línea 924

**Cambio**:
```javascript
// ANTES:
alert('Error al guardar: ' + error);

// DESPUÉS:
const errorMsg = error.includes('UNIQUE constraint')
    ? 'Este termómetro ya tiene un registro en esta ventana'
    : 'Error al guardar: ' + error;
mostrarEstado(errorMsg, 'danger');
```

**Beneficio**: 🎨 Mensajes más claros

---

## 📝 Código Limpio (20 minutos)

### 10. Extraer Constantes Mágicas
**Archivo**: `src/handlers.rs`

**Cambio**:
```rust
// AGREGAR al inicio del archivo:
const MAX_REGISTROS_POR_PAGINA: usize = 500;
const ZONA_HORARIA: &str = "America/Santiago";  // O la que uses
const SESSION_TIMEOUT_DEFAULT_HOURS: u64 = 8;

// Usar en lugar de números hardcodeados:
query.push_str(&format!(" ORDER BY r.fecha_registro DESC, r.id DESC LIMIT {}",
    MAX_REGISTROS_POR_PAGINA));
```

**Beneficio**: 📝 Más legible y mantenible

---

### 11. Agregar Comentarios a Queries Complejas
**Archivo**: `src/handlers.rs` línea 700

**Cambio**:
```rust
let mut query = String::from(
    r#"
    -- Obtiene registros con detalles de termómetros, áreas y usuarios
    -- Filtra por fecha, área y ventana horaria según parámetros
    SELECT
        r.id, r.termometro_id, t.nombre as termometro_nombre,
        a.nombre as area_nombre, u.username as usuario_nombre,
        r.ventana_horaria, r.temp_actual, r.temp_maxima, r.temp_minima, r.humedad,
        r.fuera_rango_operativo, r.observaciones, r.fecha_registro
    FROM registros r
    JOIN termometros t ON r.termometro_id = t.id
    JOIN areas a ON t.area_id = a.id
    JOIN usuarios u ON r.usuario_id = u.id
    WHERE 1=1
    "#
);
```

**Beneficio**: 📝 Entender queries rápidamente

---

## 🧪 Preparación para Tests (30 minutos)

### 12. Separar Lógica de DB de Handlers
**Archivo**: Crear `src/repositories/registros.rs` (NUEVO)

**Cambio**:
```rust
// NUEVO ARCHIVO
use sqlx::SqlitePool;
use crate::models::*;

pub async fn crear_registro_db(
    pool: &SqlitePool,
    registro: &CrearRegistroRequest,
    usuario_id: i64,
    ventana: &str,
    fuera_rango: bool,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO registros (
            termometro_id, usuario_id, ventana_horaria,
            temp_actual, temp_maxima, temp_minima, humedad,
            fuera_rango_operativo, observaciones
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(registro.termometro_id)
    .bind(usuario_id)
    .bind(ventana)
    .bind(registro.temp_actual)
    .bind(registro.temp_maxima)
    .bind(registro.temp_minima)
    .bind(registro.humedad)
    .bind(fuera_rango)
    .bind(&registro.observaciones)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}
```

**Beneficio**: 🧪 Fácil de testear en aislamiento

---

## 📊 Monitoreo Básico (15 minutos)

### 13. Agregar Logging Estructurado
**Archivo**: `src/main.rs`

**Cambio**:
```rust
// Al inicio de main():
tracing_subscriber::fmt()
    .with_target(false)
    .with_level(true)
    .with_timer(tracing_subscriber::fmt::time::LocalTime::rfc_3339())
    .init();

// En lugar de println!, usar:
tracing::info!("Servidor iniciado en http://0.0.0.0:3000");
tracing::warn!("Usando credenciales por defecto");
tracing::error!("Error al conectar a BD: {:?}", e);
```

**Beneficio**: 📊 Logs más útiles en producción

---

## ✅ Checklist Rápido

Puedes implementar estos 13 cambios en ~4 horas total:

- [ ] Cookies seguras (5 min)
- [ ] Ocultar credenciales (2 min)
- [ ] Validar temp máx >= mín (10 min)
- [ ] Agregar índices BD (5 min)
- [ ] Aumentar pool conexiones (1 min)
- [ ] Username único (10 min)
- [ ] Limpiar campo humedad (2 min)
- [ ] Deshabilitar botón guardar (5 min)
- [ ] Mejorar mensajes error (5 min)
- [ ] Extraer constantes (10 min)
- [ ] Comentar queries (10 min)
- [ ] Preparar para tests (30 min)
- [ ] Logging estructurado (5 min)

**Total**: ~1h 40min de trabajo, **impacto ENORME** 🚀

---

## 🎯 Orden Sugerido

1. **Seguridad primero** (#1, #2)
2. **Performance** (#4, #5)
3. **Bugs críticos** (#3, #6, #7)
4. **UX** (#8, #9)
5. **Código limpio** (#10, #11, #13)
6. **Preparación futura** (#12)

---

## 💡 Pro Tips

- Haz un commit después de cada cambio
- Testa cada cambio antes de continuar
- Si algo rompe, simplemente revierte ese commit
- Documenta cualquier configuración nueva

---

**Tiempo total estimado**: 2-4 horas
**Mejora percibida**: 80% del valor con 20% del esfuerzo 📈
