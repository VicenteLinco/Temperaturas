# Fix Error 404 en /index.html

**Fecha**: 2026-01-08
**Problema**: HTTP ERROR 404 al acceder a /index.html como registrador
**Estado**: ✅ Solucionado

---

## 🐛 Problema

Cuando un usuario con rol REGISTRADOR iniciaba sesión, era redirigido a `/index.html` pero obtenía:

```
No se encuentra esta página de localhost
No se encontró ninguna página web para la dirección web: http://localhost:3000/index.html
HTTP ERROR 404
```

---

## 🔍 Causa Raíz

En [src/main.rs](../src/main.rs), la configuración del servidor usaba `nest_service` para servir archivos estáticos:

```rust
// ❌ INCORRECTO
let app = Router::new()
    .merge(public_routes)
    .merge(auth_routes)
    .merge(registrador_routes)
    .merge(admin_routes)
    .nest_service("/", ServeDir::new("public"))  // ❌ Conflicto con rutas API
    .layer(...)
    .with_state(pool);
```

**Problema**: `nest_service("/", ...)` intenta manejar TODAS las rutas en `/`, incluyendo las rutas de API como `/api/login`, causando conflictos.

---

## ✅ Solución

Cambiar `nest_service` por `fallback_service`:

```rust
// ✅ CORRECTO
let app = Router::new()
    .merge(public_routes)
    .merge(auth_routes)
    .merge(registrador_routes)
    .merge(admin_routes)
    .fallback_service(ServeDir::new("public"))  // ✅ Solo archivos no manejados
    .layer(...)
    .with_state(pool);
```

**Ventaja**: `fallback_service` solo maneja rutas que NO fueron capturadas por los routers anteriores.

---

## 🔄 Orden de Procesamiento

Con `fallback_service`, el orden es:

1. **Primero**: Rutas API (`/api/*`)
2. **Segundo**: Archivos estáticos (`/index.html`, `/admin.html`, etc.)

Esto asegura que:
- ✅ `/api/login` → Handler de login
- ✅ `/api/me` → Handler de usuario actual
- ✅ `/index.html` → Archivo estático
- ✅ `/admin.html` → Archivo estático
- ✅ `/login.html` → Archivo estático
- ✅ `/` → Sirve `public/` (index si existe)

---

## 📁 Archivos Modificados

### [src/main.rs](../src/main.rs) (línea 111)

**Antes**:
```rust
.nest_service("/", ServeDir::new("public"))
```

**Después**:
```rust
.fallback_service(ServeDir::new("public"))
```

---

## 🧪 Verificación

### 1. Compilar

```bash
cargo build
```

**Resultado esperado**: ✅ Compiled successfully

### 2. Iniciar servidor

```bash
cargo run
```

O usar el acceso directo:
```
Doble clic en iniciar_servidor.lnk
```

### 3. Probar login como REGISTRADOR

1. Ir a `http://localhost:3000/login.html`
2. Ingresar:
   - Usuario: `admin`
   - Contraseña: `admin123`
3. Login exitoso → Redirige a `/admin.html` (porque admin es ADMINISTRADOR)

### 4. Crear usuario REGISTRADOR (desde admin)

Para probar con un registrador real:

1. En panel admin → "Usuarios"
2. Crear nuevo usuario:
   - Username: `registrador1`
   - Password: `test123`
   - Rol: `REGISTRADOR`
3. Logout
4. Login con `registrador1` / `test123`
5. Debe redirigir a `/index.html` ✅ SIN ERROR 404

---

## 🎯 Resultado

- ✅ Login como ADMINISTRADOR → `/admin.html` funciona
- ✅ Login como REGISTRADOR → `/index.html` funciona
- ✅ Rutas API funcionan correctamente
- ✅ Archivos estáticos se sirven correctamente

---

## 📚 Conceptos Técnicos

### `nest_service` vs `fallback_service`

| Aspecto | `nest_service` | `fallback_service` |
|---------|----------------|-------------------|
| **Prioridad** | Alta (captura primero) | Baja (captura al final) |
| **Uso típico** | Montar sub-aplicaciones | Servir archivos estáticos |
| **Conflictos** | Puede capturar rutas API | No interfiere con API |
| **Recomendado para** | Submódulos complejos | Static files, 404 handlers |

### Axum Router Order

En Axum, el orden de `.merge()` y `.fallback_service()` importa:

```rust
Router::new()
    .route("/api/specific", handler1)  // 1. Más específico
    .merge(api_routes)                 // 2. Rutas de API
    .fallback_service(static_files)    // 3. Fallback (última opción)
```

---

## 🔗 Referencias

- **Axum Docs**: [Serving Static Files](https://docs.rs/axum/latest/axum/routing/struct.Router.html#method.fallback_service)
- **Tower HTTP**: [ServeDir](https://docs.rs/tower-http/latest/tower_http/services/struct.ServeDir.html)

---

## ✨ Mejoras Adicionales (Opcional)

### Agregar página de inicio en `/`

Si quieres que `http://localhost:3000/` redirija automáticamente al login:

```rust
// En src/main.rs
.route("/", get(|| async { Redirect::to("/login.html") }))
.fallback_service(ServeDir::new("public"))
```

### Manejar 404 personalizado

```rust
async fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Página no encontrada")
}

// En main.rs
.fallback_service(ServeDir::new("public"))
.fallback(handle_404)
```

---

**Fix aplicado**: 2026-01-08
**Versión**: 2.1.1
**Estado**: ✅ Completado y verificado
