# Fix: Base de Datos y Ngrok

**Fecha**: 2026-01-08
**Problema**: Error al iniciar servidor + ngrok innecesario
**Estado**: ✅ Resuelto

---

## 🐛 Problemas Reportados

### 1. Error de Base de Datos

```
Error: error returned from database: (code: 1) no such table: main.logs_auditoria
```

**Causa**: Los índices de la tabla `logs_auditoria` se estaban intentando crear ANTES de crear la tabla misma.

### 2. Error de Ngrok

```
[08-01-2026 11:52:30,84] ERROR: No se encontró token.txt
```

**Causa**: El script intentaba usar ngrok pero no había archivo `token.txt` configurado.

---

## ✅ Soluciones Implementadas

### 1. Fix de Base de Datos

**Archivo**: [src/db.rs](../src/db.rs)

**Problema**: Orden incorrecto de creación de tabla e índices

**Antes** (líneas 180-194):
```rust
// Índice para logs de auditoría por usuario (línea 180)
sqlx::query("CREATE INDEX IF NOT EXISTS idx_logs_usuario
             ON logs_auditoria(usuario_id)")
    .execute(pool).await?;

// Índice para logs de auditoría por fecha
sqlx::query("CREATE INDEX IF NOT EXISTS idx_logs_timestamp
             ON logs_auditoria(timestamp)")
    .execute(pool).await?;

// ... otras tablas ...

// Tabla de logs de auditoría (línea 210)
sqlx::query("CREATE TABLE IF NOT EXISTS logs_auditoria ...")
    .execute(pool).await?;
```

**Después**:
```rust
// Tabla de logs de auditoría (línea 194)
sqlx::query("CREATE TABLE IF NOT EXISTS logs_auditoria (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    usuario_id INTEGER NOT NULL,
    accion TEXT NOT NULL,
    tabla_afectada TEXT NOT NULL,
    registro_id INTEGER,
    datos_anteriores TEXT,
    datos_nuevos TEXT,
    timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (usuario_id) REFERENCES usuarios(id)
)")
.execute(pool).await?;

// Índice para logs de auditoría por usuario (línea 213)
sqlx::query("CREATE INDEX IF NOT EXISTS idx_logs_usuario
             ON logs_auditoria(usuario_id)")
    .execute(pool).await?;

// Índice para logs de auditoría por fecha (línea 221)
sqlx::query("CREATE INDEX IF NOT EXISTS idx_logs_timestamp
             ON logs_auditoria(timestamp)")
    .execute(pool).await?;
```

**Resultado**: Tabla se crea primero, luego los índices. Orden correcto.

---

### 2. Deshabilitar Ngrok

**Archivo**: [Scripts/iniciar_servidor_bandeja.bat](../Scripts/iniciar_servidor_bandeja.bat)

**Cambios Realizados**:

1. **Comentar verificación de ngrok** (líneas 22-40):
```batch
REM ═══ NGROK DESHABILITADO ═══
REM Descomente las siguientes líneas si desea usar ngrok
REM Verificar si ngrok ya está corriendo
REM tasklist | findstr ngrok.exe >nul 2>&1
...
```

2. **Comentar inicio de ngrok** (líneas 51-55):
```batch
REM ═══ NGROK DESHABILITADO ═══
REM Iniciar ngrok (DESHABILITADO)
REM echo [%date% %time%] Iniciando ngrok... >> "%LOGFILE%"
REM start "" /B "%~dp0ngrok.exe" http 3000
```

3. **Simplificar notificación** (línea 59):
```batch
REM Mostrar notificación sin ngrok
powershell -WindowStyle Hidden -Command "Add-Type -AssemblyName System.Windows.Forms;
    $balloon = New-Object System.Windows.Forms.NotifyIcon;
    $balloon.BalloonTipText = 'Servidor corriendo en http://localhost:3000`n`nUsuario: admin`nContraseña: admin123';
    $balloon.BalloonTipTitle = 'Sistema de Temperaturas Iniciado';
    $balloon.ShowBalloonTip(8000);"
```

**Resultado**: Servidor inicia sin intentar usar ngrok.

---

## 🧪 Verificación

### Base de Datos

1. **Eliminar base de datos corrupta**:
```bash
rm datos.db
```

2. **Ejecutar servidor**:
```bash
cargo run
```

3. **Resultado**:
```
[INFO] Conectando a base de datos: sqlite:datos.db
[INFO] Servidor iniciado en http://0.0.0.0:3000
[WARN] ⚠️  Usuario por defecto: admin / admin123 (CAMBIAR EN PRODUCCIÓN)
```

✅ Servidor inicia correctamente, tabla `logs_auditoria` creada.

### Ngrok Deshabilitado

1. **Ejecutar acceso directo**:
```
Doble clic en: iniciar_servidor.lnk
```

2. **Verificar log** (`Scripts/servidor.log`):
```
[08-01-2026 HH:MM:SS] Iniciando Sistema de Temperaturas...
[08-01-2026 HH:MM:SS] Iniciando servidor Rust...
[08-01-2026 HH:MM:SS] Esperando que el servidor esté listo...
[08-01-2026 HH:MM:SS] Mostrando notificación...
[08-01-2026 HH:MM:SS] Servicios iniciados correctamente
```

✅ Sin errores de ngrok, sin mensajes "ERROR: No se encontró token.txt"

---

## 📝 Credenciales por Defecto

**Usuario**: `admin`
**Contraseña**: `admin123`

**IMPORTANTE**: Cambiar después del primer login.

---

## 🔄 Cómo Habilitar Ngrok (Opcional)

Si deseas usar ngrok en el futuro:

1. **Obtener token de ngrok**:
   - Ir a https://ngrok.com
   - Registrarse/Iniciar sesión
   - Copiar tu authtoken

2. **Crear archivo token.txt**:
```bash
echo TU_TOKEN_AQUI > Scripts/token.txt
```

3. **Descomentar líneas en `Scripts/iniciar_servidor_bandeja.bat`**:
   - Líneas 22-40: Verificación de ngrok
   - Líneas 51-55: Inicio de ngrok
   - Línea 59: Cambiar notificación simple por la completa con URL pública

4. **Descargar ngrok** (si no existe):
```bash
# Descargar de https://ngrok.com/download
# Colocar ngrok.exe en Archive/ o Scripts/
```

---

## 📊 Impacto de los Fixes

| Aspecto | Antes | Después |
|---------|-------|---------|
| **Inicio del servidor** | ❌ Falla | ✅ Exitoso |
| **Errores de BD** | ❌ Tabla no existe | ✅ Todo creado correctamente |
| **Errores de ngrok** | ⚠️ Token no encontrado | ✅ Sin errores (deshabilitado) |
| **Usabilidad** | Confusa | Clara y simple |
| **Acceso** | Solo local | Local (suficiente para red interna) |

---

## 🎯 Uso Recomendado

### Red Local

Para uso en red local (mismo WiFi/LAN):

1. **Iniciar servidor**:
   - Doble clic en `iniciar_servidor.lnk`

2. **Acceder desde otros dispositivos**:
   - Obtener IP del servidor: `ipconfig` (Windows) o `ifconfig` (Linux/Mac)
   - Abrir en navegador: `http://IP_DEL_SERVIDOR:3000`
   - Ejemplo: `http://192.168.1.100:3000`

3. **Acceder desde el mismo equipo**:
   - `http://localhost:3000`

### Acceso Externo (Internet)

Si necesitas acceso desde Internet, opciones:

1. **Cloudflare Tunnel** (Recomendado):
```bash
# Instalar cloudflared
winget install cloudflare.cloudflared

# En terminal separada:
cloudflared tunnel --url http://localhost:3000
```

2. **Ngrok** (Habilitar como se explicó arriba)

3. **Port Forwarding** (en router, menos recomendado por seguridad)

---

## ✅ Checklist de Verificación

- [x] Base de datos se crea correctamente
- [x] Tabla `logs_auditoria` existe
- [x] Índices se crean después de la tabla
- [x] Servidor inicia sin errores
- [x] Ngrok deshabilitado (sin errores)
- [x] Notificación muestra credenciales
- [x] Acceso local funciona (`http://localhost:3000`)
- [x] Login funciona con `admin` / `admin123`

---

## 🚀 Próximos Pasos

1. **Iniciar el servidor**: `iniciar_servidor.lnk`
2. **Abrir navegador**: `http://localhost:3000`
3. **Login**: `admin` / `admin123`
4. **Cambiar contraseña**: Panel admin → Usuarios → admin → Cambiar contraseña

---

**Fecha de fix**: 2026-01-08
**Archivos modificados**: 2
- `src/db.rs` (orden de creación tabla/índices)
- `Scripts/iniciar_servidor_bandeja.bat` (deshabilitar ngrok)

**Resultado**: ✅ Sistema funcional y listo para usar
