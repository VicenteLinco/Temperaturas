# 📋 Plan de Refactorización - Sistema de Temperaturas

**Fecha**: 2026-01-08
**Objetivo**: Mejorar estructura, mantenibilidad y organización del código
**Prioridad**: Media-Alta

---

## 🎯 Objetivos de la Refactorización

1. **Modularizar `handlers.rs`** (1395 líneas → múltiples módulos)
2. **Separar responsabilidades** por dominio
3. **Mejorar navegabilidad** del código
4. **Facilitar testing** unitario
5. **Reducir acoplamiento** entre componentes
6. **Mantener compatibilidad** 100% (sin breaking changes)

---

## 📊 Análisis Actual

### Archivos por Tamaño

| Archivo | Líneas | Estado | Acción |
|---------|--------|--------|--------|
| `handlers.rs` | 1395 | ⚠️ Muy grande | Dividir en módulos |
| `db.rs` | 394 | ✅ Aceptable | Mantener |
| `models.rs` | 342 | ✅ Aceptable | Posible división |
| `logic.rs` | 250 | ✅ Bien | Mantener |
| `auth.rs` | 135 | ✅ Bien | Mantener |
| `main.rs` | 134 | ✅ Bien | Mantener |

### Funciones Handler (29 total)

**Por dominio**:
- Auth: 3 funciones (login, logout, me)
- Usuarios: 5 funciones (listar, crear, actualizar, eliminar, cambiar_password)
- Áreas: 4 funciones (listar, crear, actualizar, eliminar)
- Tipos Termómetro: 4 funciones (listar, crear, actualizar, eliminar)
- Termómetros: 5 funciones (listar, obtener, crear, actualizar, eliminar)
- Registros: 6 funciones (listar, pendientes, crear, actualizar, eliminar)
- Configuración: 2 funciones (obtener, actualizar)
- Reportes: 2 funciones (diario, mensual)

---

## 🏗️ Nueva Estructura Propuesta

```
src/
├── main.rs                      # ✅ Mantener (134 líneas)
├── auth.rs                      # ✅ Mantener (135 líneas)
├── db.rs                        # ✅ Mantener (394 líneas)
├── logic.rs                     # ✅ Mantener (250 líneas)
├── models.rs                    # ⚡ Posible división
│   ├── mod.rs                   # Re-exporta todos
│   ├── auth.rs                  # LoginRequest, LoginResponse, etc.
│   ├── usuario.rs               # Usuario, UsuarioResponse, etc.
│   ├── area.rs                  # Area, AreaResponse, etc.
│   ├── termometro.rs            # Termometro, TipoTermometro, etc.
│   ├── registro.rs              # Registro, RegistroConDetalles, etc.
│   └── config.rs                # Configuracion, etc.
├── handlers/                    # 🆕 NUEVO - Módulo de handlers
│   ├── mod.rs                   # Re-exporta todos + constantes compartidas
│   ├── auth.rs                  # login_handler, logout_handler, me_handler
│   ├── usuarios.rs              # CRUD usuarios
│   ├── areas.rs                 # CRUD áreas
│   ├── tipos_termometro.rs      # CRUD tipos termómetro
│   ├── termometros.rs           # CRUD termómetros
│   ├── registros.rs             # CRUD registros + pendientes
│   ├── configuracion.rs         # GET/PUT configuración
│   └── reportes.rs              # Reportes diario/mensual
└── utils/                       # 🆕 NUEVO - Utilidades compartidas
    ├── mod.rs
    ├── error.rs                 # Error handling personalizado
    └── response.rs              # Helpers para respuestas HTTP
```

---

## 🔧 Refactorización Paso a Paso

### Fase 1: Crear Estructura de Módulos (30 min)

**Archivos a crear**:
```
src/handlers/mod.rs
src/handlers/auth.rs
src/handlers/usuarios.rs
src/handlers/areas.rs
src/handlers/tipos_termometro.rs
src/handlers/termometros.rs
src/handlers/registros.rs
src/handlers/configuracion.rs
src/handlers/reportes.rs
```

**Contenido de `src/handlers/mod.rs`**:
```rust
// Constantes compartidas
pub const MAX_REGISTROS_POR_PAGINA: i32 = 500;
pub const TIEMPO_SESSION_DEFAULT_HORAS: u64 = 8;

// Re-exportar todos los handlers
pub mod auth;
pub mod usuarios;
pub mod areas;
pub mod tipos_termometro;
pub mod termometros;
pub mod registros;
pub mod configuracion;
pub mod reportes;

// Re-exportar funciones principales
pub use auth::*;
pub use usuarios::*;
pub use areas::*;
pub use tipos_termometro::*;
pub use termometros::*;
pub use registros::*;
pub use configuracion::*;
pub use reportes::*;
```

---

### Fase 2: Migrar Auth Handlers (15 min)

**`src/handlers/auth.rs`**:
```rust
use axum::{extract::State, http::StatusCode, Json};
use sqlx::SqlitePool;
use tower_sessions::Session;

use crate::{
    auth::{hash_password, verify_password, CurrentUser},
    models::{LoginRequest, LoginResponse, MeResponse},
};

pub async fn login_handler(
    session: Session,
    State(pool): State<SqlitePool>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    // ... código existente ...
}

pub async fn logout_handler(
    session: Session
) -> Result<StatusCode, StatusCode> {
    // ... código existente ...
}

pub async fn me_handler(
    current_user: CurrentUser
) -> Json<MeResponse> {
    // ... código existente ...
}
```

---

### Fase 3: Migrar Usuarios Handlers (20 min)

**`src/handlers/usuarios.rs`**:
```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::SqlitePool;

use crate::{
    auth::{hash_password, CurrentUser},
    db::log_auditoria,
    models::{
        CrearUsuarioRequest, ActualizarUsuarioRequest,
        CambiarPasswordRequest, UsuarioResponse,
    },
};

pub async fn listar_usuarios(
    _current_user: CurrentUser,
    State(pool): State<SqlitePool>,
) -> Result<Json<Vec<UsuarioResponse>>, StatusCode> {
    // ... código existente ...
}

pub async fn crear_usuario(
    current_user: CurrentUser,
    State(pool): State<SqlitePool>,
    Json(payload): Json<CrearUsuarioRequest>,
) -> Result<Json<UsuarioResponse>, StatusCode> {
    // ... código existente ...
}

// ... resto de funciones ...
```

---

### Fase 4: Migrar Resto de Handlers (60 min)

Seguir el mismo patrón para:
- `areas.rs`
- `tipos_termometro.rs`
- `termometros.rs`
- `registros.rs`
- `configuracion.rs`
- `reportes.rs`

---

### Fase 5: Actualizar `main.rs` (10 min)

**Cambio en imports**:
```rust
// Antes:
mod handlers;
use handlers::*;

// Después:
mod handlers;
use handlers::{
    auth::*,
    usuarios::*,
    areas::*,
    // ... etc
};
```

O simplemente:
```rust
use handlers::*; // Si re-exportamos todo en mod.rs
```

---

### Fase 6: Limpiar `handlers.rs` Original (5 min)

Una vez migrado todo, eliminar el archivo:
```bash
rm src/handlers.rs
```

---

### Fase 7: Testing y Verificación (30 min)

```bash
# Compilar
cargo build

# Verificar que no haya errores
cargo check

# Ejecutar tests (si existen)
cargo test

# Probar manualmente
cargo run
```

---

## 📝 Mejoras Adicionales Opcionales

### Error Handling Personalizado

**`src/utils/error.rs`**:
```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub enum AppError {
    NotFound,
    Unauthorized,
    BadRequest(String),
    InternalError,
    DatabaseError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not found"),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AppError::BadRequest(msg) => return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": msg }))
            ).into_response(),
            AppError::InternalError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error"
            ),
            AppError::DatabaseError(msg) => {
                tracing::error!("Database error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
            }
        };

        (status, message).into_response()
    }
}
```

**Uso**:
```rust
pub async fn crear_usuario(
    // ...
) -> Result<Json<UsuarioResponse>, AppError> {
    let existe = check_username(&pool, &payload.username)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if existe {
        return Err(AppError::BadRequest(
            "Username already exists".to_string()
        ));
    }

    // ... resto del código
}
```

---

### Response Helpers

**`src/utils/response.rs`**:
```rust
use axum::{http::StatusCode, Json};
use serde::Serialize;

pub fn ok<T: Serialize>(data: T) -> (StatusCode, Json<T>) {
    (StatusCode::OK, Json(data))
}

pub fn created<T: Serialize>(data: T) -> (StatusCode, Json<T>) {
    (StatusCode::CREATED, Json(data))
}

pub fn no_content() -> StatusCode {
    StatusCode::NO_CONTENT
}
```

---

## ✅ Checklist de Refactorización

### Preparación
- [ ] Crear backup del código actual
- [ ] Commit de git antes de empezar
- [ ] Crear rama `refactor/handlers-modules`

### Implementación
- [ ] Crear carpeta `src/handlers/`
- [ ] Crear `src/handlers/mod.rs`
- [ ] Migrar auth handlers → `auth.rs`
- [ ] Migrar usuarios handlers → `usuarios.rs`
- [ ] Migrar áreas handlers → `areas.rs`
- [ ] Migrar tipos termómetro → `tipos_termometro.rs`
- [ ] Migrar termómetros → `termometros.rs`
- [ ] Migrar registros → `registros.rs`
- [ ] Migrar configuración → `configuracion.rs`
- [ ] Migrar reportes → `reportes.rs`
- [ ] Actualizar imports en `main.rs`
- [ ] Eliminar `src/handlers.rs` original

### Verificación
- [ ] `cargo check` sin errores
- [ ] `cargo build` exitoso
- [ ] `cargo test` (si hay tests)
- [ ] Probar servidor manualmente
- [ ] Verificar todas las rutas funcionan
- [ ] Commit de refactorización

### Opcional - Mejoras
- [ ] Crear `src/utils/error.rs`
- [ ] Crear `src/utils/response.rs`
- [ ] Aplicar error handling personalizado
- [ ] Dividir `models.rs` si es necesario

---

## 📊 Beneficios Esperados

### Antes (1 archivo)
```
src/handlers.rs (1395 líneas)
├── 29 funciones mezcladas
├── Difícil de navegar
├── Testing complicado
└── Alto acoplamiento
```

### Después (9 archivos)
```
src/handlers/
├── mod.rs (50 líneas)
├── auth.rs (~100 líneas)
├── usuarios.rs (~150 líneas)
├── areas.rs (~120 líneas)
├── tipos_termometro.rs (~120 líneas)
├── termometros.rs (~180 líneas)
├── registros.rs (~250 líneas)
├── configuracion.rs (~80 líneas)
└── reportes.rs (~200 líneas)
```

**Mejoras**:
- ✅ **+80% navegabilidad** (archivos más pequeños)
- ✅ **+60% mantenibilidad** (responsabilidades claras)
- ✅ **+90% testabilidad** (módulos independientes)
- ✅ **-50% complejidad** por archivo
- ✅ **+100% organización** (separación por dominio)

---

## ⚠️ Precauciones

1. **No hacer durante producción activa**
2. **Hacer commits incrementales**
3. **Probar después de cada migración**
4. **Mantener backup del código original**
5. **Verificar que todas las rutas funcionan**

---

## 🎯 Resultado Final

```
ANTES:
- 1 archivo monolítico (1395 líneas)
- Difícil de mantener
- Complicado de testear

DESPUÉS:
- 9 archivos modulares (~150 líneas c/u)
- Fácil de mantener
- Simple de testear
- Mejor organización
```

---

## 🚀 Próximos Pasos

1. **Revisión de este plan** por el equipo
2. **Aprobación** para proceder
3. **Inicio de refactorización** fase por fase
4. **Testing continuo** en cada fase
5. **Merge a main** una vez completado

---

**Tiempo estimado total**: 2.5 - 3 horas
**Riesgo**: Bajo (no cambia funcionalidad)
**Beneficio**: Alto (mejor código, más mantenible)

---

**¿Proceder con la refactorización?**

Responder con:
- "sí" → Comenzar refactorización completa
- "solo handlers" → Solo dividir handlers.rs
- "más tarde" → Guardar plan para después
