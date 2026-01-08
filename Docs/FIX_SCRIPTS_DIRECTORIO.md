# Fix: Scripts ejecutando desde directorio incorrecto

**Fecha**: 2026-01-08
**Problema**: Error 404 en /index.html cuando usuario REGISTRADOR hace login
**Causa**: Scripts iniciaban servidor desde carpeta `Scripts/` en lugar de raíz
**Estado**: ✅ Solucionado

---

## 🐛 Problema

Cuando un usuario con rol **REGISTRADOR** hacía login:
1. Login exitoso ✅
2. Redirige a `/index.html` ✅
3. **Error 404**: "No se encuentra esta página de localhost" ❌

Pero cuando se ejecutaba directamente con `cargo run`, todo funcionaba correctamente.

---

## 🔍 Causa Raíz

Los scripts de inicio (`iniciar_servidor.bat` y `iniciar_servidor_bandeja.bat`) estaban ejecutando `cargo run` desde la carpeta `Scripts/`:

```batch
REM ❌ INCORRECTO
cd /d "%~dp0"        REM Cambia a carpeta Scripts/
start /B cargo run   REM Ejecuta desde Scripts/
```

**Problema**: Cuando el servidor Rust se inicia desde `Scripts/`, busca la carpeta `public/` en `Scripts/public/` en lugar de en la raíz del proyecto.

### Estructura incorrecta:
```
Temperaturas/
├── Scripts/
│   └── iniciar_servidor.bat  ← Ejecuta desde aquí
│       └── cargo run busca: Scripts/public/ ❌ NO EXISTE
└── public/                    ← Archivos están aquí
    ├── index.html
    ├── admin.html
    └── login.html
```

---

## ✅ Solución

Cambiar los scripts para ejecutar `cargo run` desde la **raíz del proyecto**:

### [Scripts/iniciar_servidor_bandeja.bat](../Scripts/iniciar_servidor_bandeja.bat) (línea 44)

**Antes**:
```batch
cd /d "%~dp0"        REM Va a Scripts/
start /B cargo run
```

**Después**:
```batch
cd /d "%~dp0.."      REM Va a la carpeta padre (raíz)
start /B cargo run
```

### [Scripts/iniciar_servidor.bat](../Scripts/iniciar_servidor.bat) (línea 37)

**Antes**:
```batch
echo [2/4] Iniciando servidor Rust...
start /B cargo run >nul 2>&1
```

**Después**:
```batch
echo [2/4] Iniciando servidor Rust...
cd /d "%~dp0.."      REM Va a la carpeta padre (raíz)
start /B cargo run >nul 2>&1
```

---

## 🧪 Verificación

### Test Manual

1. **Detener servidor actual**:
   ```
   Doble clic en detener_servidor.lnk
   ```

2. **Iniciar con script corregido**:
   ```
   Doble clic en iniciar_servidor.lnk
   Esperar ~15 segundos
   ```

3. **Crear usuario REGISTRADOR** (si no existe):
   - Login como admin (`admin` / `admin123`)
   - Ir a "Usuarios"
   - Crear nuevo usuario:
     - Username: `registrador1`
     - Password: `test123`
     - Rol: `REGISTRADOR`

4. **Probar login como REGISTRADOR**:
   - Logout
   - Login con `registrador1` / `test123`
   - Debe redirigir a `/index.html` ✅ SIN ERROR 404

### Test Automatizado

```bash
bash Tests/test_login_flow.sh
```

**Resultado esperado**: Todos los tests pasan ✅

---

## 📊 Comparación

| Aspecto | Antes (Incorrecto) | Después (Correcto) |
|---------|-------------------|-------------------|
| **Directorio de ejecución** | `Scripts/` | `Temperaturas/` (raíz) |
| **Busca public/ en** | `Scripts/public/` ❌ | `public/` ✅ |
| **Busca Cargo.toml en** | `Scripts/Cargo.toml` ❌ | `Cargo.toml` ✅ |
| **Archivos estáticos** | 404 ❌ | 200 ✅ |
| **Login ADMIN** | Funciona (no usa archivos) | Funciona ✅ |
| **Login REGISTRADOR** | Error 404 ❌ | Funciona ✅ |

---

## 🎯 Por Qué Admin Funcionaba y Registrador No

### Admin (ADMINISTRADOR)
- Redirige a `/admin.html`
- `/admin.html` → 404 ❌
- Pero la página `admin.html` usa muchas APIs
- **Parecía funcionar** porque las APIs respondían correctamente

### Registrador (REGISTRADOR)
- Redirige a `/index.html`
- `/index.html` → 404 ❌
- **Error visible inmediatamente**

**Conclusión**: Ambos tenían el mismo problema, pero era más evidente con registrador.

---

## 🔧 Explicación Técnica

### `%~dp0` en Batch

- `%0` = Ruta completa del script
- `%~d0` = Letra de unidad (ej: `C:`)
- `%~p0` = Ruta del directorio (ej: `\Users\...\Scripts\`)
- `%~dp0` = Unidad + Ruta (ej: `C:\Users\...\Scripts\`)
- `%~dp0..` = Carpeta padre (ej: `C:\Users\...\Temperaturas\`)

### Axum ServeDir

En [src/main.rs](../src/main.rs):
```rust
.fallback_service(ServeDir::new("public"))
```

`ServeDir::new("public")` busca la carpeta `public/` **relativa al directorio de trabajo actual (CWD)**, NO relativa al ejecutable.

Si el CWD es `Scripts/`:
- Busca: `Scripts/public/` ❌
- No encuentra archivos → 404

Si el CWD es raíz:
- Busca: `public/` ✅
- Encuentra archivos → 200

---

## 🚀 Mejora Futura (Opcional)

### Usar ruta absoluta en ServeDir

Para evitar dependencias del CWD:

```rust
// En src/main.rs
use std::env;
use std::path::PathBuf;

// Obtener la raíz del proyecto (donde está Cargo.toml)
let project_root = env::var("CARGO_MANIFEST_DIR")
    .map(PathBuf::from)
    .unwrap_or_else(|_| env::current_dir().unwrap());

let public_dir = project_root.join("public");

// Usar ruta absoluta
.fallback_service(ServeDir::new(public_dir))
```

**Ventaja**: Funciona independientemente del directorio de ejecución.

---

## 📚 Archivos Modificados

1. **[Scripts/iniciar_servidor_bandeja.bat](../Scripts/iniciar_servidor_bandeja.bat)**
   - Línea 44: Agregado `cd /d "%~dp0.."`

2. **[Scripts/iniciar_servidor.bat](../Scripts/iniciar_servidor.bat)**
   - Línea 37: Agregado `cd /d "%~dp0.."`

3. **[Scripts/iniciar_servidor_oculto.vbs](../Scripts/iniciar_servidor_oculto.vbs)**
   - Sin cambios (ejecuta el .bat que ya fue corregido)

---

## ✅ Resultado

- ✅ Login como ADMINISTRADOR → `/admin.html` funciona
- ✅ Login como REGISTRADOR → `/index.html` funciona
- ✅ Todos los archivos estáticos se sirven correctamente
- ✅ Scripts pueden ejecutarse desde cualquier ubicación
- ✅ Accesos directos funcionan correctamente

---

**Fix aplicado**: 2026-01-08
**Versión**: 2.1.2
**Tested**: ✅ Manual y automatizado
**Estado**: Completado
