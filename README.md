# Sistema de Registro de Temperaturas

Sistema completo de gestión y registro de temperaturas para termómetros en áreas técnicas, desarrollado en Rust con Axum y SQLite.

## Características Principales

- **Autenticación y Roles**: Sistema de login con sesiones seguras (bcrypt) y dos roles (ADMINISTRADOR y REGISTRADOR)
- **Gestión de Termómetros**: CRUD completo de áreas, tipos de termómetros y termómetros individuales
- **Registro por Ventanas Horarias**: Configuración de horarios específicos con ventanas de tolerancia
- **Validación de Rangos**: Rangos operativos (advertencia) y físicos (rechazo) para temperaturas y humedad
- **Auditoría Completa**: Logs de todos los cambios en el sistema
- **Interfaz Web**: Frontend responsive con Bootstrap 5
- **Escaneo QR**: Soporte para escaneo de códigos QR de termómetros
- **Reportes**: Generación de reportes diarios y mensuales con exportación CSV/PDF

## Stack Tecnológico

### Backend
- **Rust** (Edition 2021)
- **Axum** - Framework web asíncrono
- **SQLx** - Base de datos SQLite asíncrona
- **Tower Sessions** - Manejo de sesiones
- **BCrypt** - Hash de contraseñas
- **Chrono** - Manejo de fechas y horas

### Frontend
- **HTML5 + JavaScript Vanilla**
- **Bootstrap 5** - UI Framework
- **html5-qrcode** - Escaneo de códigos QR

## Instalación

### Requisitos Previos
- **Rust** 1.70 o superior
- **Cargo** (incluido con Rust)

### Pasos de Instalación

1. Clonar el repositorio:
```bash
git clone <url-del-repositorio>
cd Temperaturas
```

2. Copiar el archivo de configuración:
```bash
cp .env.example .env
```

3. (Opcional) Editar `.env` con tu configuración:
```env
DATABASE_URL=sqlite:datos.db
PORT=3000
HOST=0.0.0.0
REGISTRO_HORA_1=14:00
REGISTRO_HORA_2=02:00
VENTANA_TOLERANCIA_MINUTOS=119
SESSION_TIMEOUT_HORAS=8
```

4. Compilar y ejecutar:
```bash
cargo run
```

El servidor iniciará en `http://localhost:3000`

## Usuario Por Defecto

Al iniciar por primera vez, se crea automáticamente un usuario administrador:

- **Usuario**: `admin`
- **Contraseña**: `admin123`

**IMPORTANTE**: Cambia esta contraseña después del primer login.

## Estructura del Proyecto

```
Temperaturas/
├── src/
│   ├── main.rs           # Punto de entrada y configuración del servidor
│   ├── auth.rs           # Autenticación y middleware
│   ├── db.rs             # Esquema de base de datos e inicialización
│   ├── handlers.rs       # Endpoints de la API
│   ├── logic.rs          # Lógica de ventanas horarias y validaciones
│   └── models.rs         # Modelos y DTOs
├── public/
│   ├── login.html        # Página de inicio de sesión
│   ├── index.html        # Interfaz del registrador
│   └── admin.html        # Panel de administración
├── Cargo.toml            # Dependencias del proyecto
├── .env.example          # Ejemplo de configuración
└── README.md             # Este archivo
```

## Modelo de Datos

### Tablas Principales

1. **usuarios**: Gestión de usuarios con roles (ADMINISTRADOR/REGISTRADOR)
2. **areas**: Áreas técnicas donde se ubican los termómetros
3. **tipos_termometro**: Tipos con configuración de rangos
4. **termometros**: Termómetros individuales con ID numérico para QR
5. **registros**: Mediciones de temperatura/humedad por ventana horaria
6. **configuracion**: Parámetros globales del sistema
7. **logs_auditoria**: Auditoría de cambios

## API Endpoints

### Autenticación
- `POST /api/login` - Iniciar sesión
- `POST /api/logout` - Cerrar sesión
- `GET /api/me` - Obtener usuario actual

### Áreas
- `GET /api/areas` - Listar áreas
- `POST /api/admin/areas` - Crear área (admin)
- `PUT /api/admin/areas/:id` - Actualizar área (admin)
- `DELETE /api/admin/areas/:id` - Eliminar área (admin)

### Tipos de Termómetros
- `GET /api/tipos-termometro` - Listar tipos
- `POST /api/admin/tipos-termometro` - Crear tipo (admin)
- `PUT /api/admin/tipos-termometro/:id` - Actualizar tipo (admin)
- `DELETE /api/admin/tipos-termometro/:id` - Eliminar tipo (admin)

### Termómetros
- `GET /api/termometros` - Listar termómetros
- `GET /api/termometros/:id` - Obtener termómetro
- `POST /api/admin/termometros` - Crear termómetro (admin)
- `PUT /api/admin/termometros/:id` - Actualizar termómetro (admin)
- `DELETE /api/admin/termometros/:id` - Eliminar termómetro (admin)

### Registros
- `GET /api/areas/:id/pendientes` - Obtener pendientes de un área
- `POST /api/registros` - Crear registro (registrador)
- `PUT /api/registros/:id` - Actualizar registro (registrador)
- `DELETE /api/admin/registros/:id` - Eliminar registro (admin)

### Usuarios
- `GET /api/admin/usuarios` - Listar usuarios (admin)
- `POST /api/admin/usuarios` - Crear usuario (admin)
- `PUT /api/admin/usuarios/:id` - Actualizar usuario (admin)
- `DELETE /api/admin/usuarios/:id` - Eliminar usuario (admin)

### Configuración
- `GET /api/admin/configuracion` - Obtener configuración (admin)
- `PUT /api/admin/configuracion` - Actualizar configuración (admin)

## Lógica de Ventanas Horarias

El sistema permite configurar dos horarios de registro diarios con ventanas de tolerancia:

- **Horario 1** (por defecto 14:00 ± 119 minutos): 12:01 - 15:59
- **Horario 2** (por defecto 02:00 ± 119 minutos): 00:01 - 03:59

Los registros solo pueden crearse dentro de estas ventanas. Fuera de ellas, el sistema rechaza el registro.

## Validación de Temperaturas

Cada tipo de termómetro define:

1. **Rangos Operativos**: Valores esperados normales
   - Fuera de rango → Advertencia amarilla, permite registrar, marca `fuera_rango_operativo = true`

2. **Rangos Físicos**: Límites absolutos
   - Fuera de rango → Rechaza el registro (error 400)

## Flujo de Trabajo del Registrador

1. Login con credenciales
2. Seleccionar área
3. Ver lista de termómetros pendientes y completados
4. Escanear código QR del termómetro
5. Ingresar temperatura máxima, mínima y humedad (si aplica)
6. Sistema valida rangos y muestra advertencias
7. Guardar registro
8. Continuar con siguiente termómetro
9. Al finalizar, botón "Cerrar Mediciones" muestra resumen de pendientes

## Flujo de Trabajo del Administrador

1. Login como administrador
2. Acceso a panel con 7 secciones:
   - Tipos de Termómetros
   - Áreas Técnicas
   - Termómetros (con generación de QR)
   - Usuarios
   - Configuración Global
   - Reportes (diario/mensual con exportación)
   - Gestión de Registros (CRUD completo)

## Exposición a Internet con Cloudflare Tunnel

### Instalación de Cloudflared

#### Windows
```powershell
winget install cloudflare.cloudflared
```

#### macOS
```bash
brew install cloudflared
```

#### Linux
```bash
wget https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64
sudo mv cloudflared-linux-amd64 /usr/local/bin/cloudflared
sudo chmod +x /usr/local/bin/cloudflared
```

### Uso

1. Iniciar el sistema normalmente:
```bash
cargo run
```

2. En otra terminal, ejecutar:
```bash
cloudflared tunnel --url http://localhost:3000
```

3. Cloudflare te proporcionará una URL pública (ej: `https://random-words-1234.trycloudflare.com`)

4. Compartir esa URL para acceso desde cualquier lugar

## Desarrollo

### Compilar en modo debug
```bash
cargo build
```

### Compilar en modo release (optimizado)
```bash
cargo build --release
./target/release/sistema-temperaturas
```

### Ejecutar tests
```bash
cargo test
```

## Seguridad

- ✅ Contraseñas hasheadas con BCrypt (cost 12)
- ✅ Sesiones con timeout de 8 horas de inactividad
- ✅ Middleware de autenticación y autorización por roles
- ✅ Validación de inputs
- ✅ Auditoría completa de cambios

**Recomendaciones**:
- Cambiar el SECRET_KEY en producción
- Usar HTTPS en producción (Cloudflare Tunnel lo hace automáticamente)
- Cambiar contraseña del admin por defecto
- Revisar logs de auditoría regularmente

## Solución de Problemas

### Error: "database is locked"
- SQLite solo permite un escritor a la vez
- El sistema está configurado con pool de 5 conexiones
- Si persiste, aumentar `max_connections` en `src/db.rs`

### Error: "Fuera de ventana horaria"
- Verificar configuración de horarios en panel admin
- Verificar que la ventana de tolerancia sea suficiente
- Revisar zona horaria del servidor

### No se pueden escanear códigos QR
- Verificar que el navegador tenga permisos de cámara
- Probar con HTTPS (Cloudflare Tunnel)
- Navegadores móviles funcionan mejor para QR

## Roadmap

- [ ] Notificaciones push para alertas
- [ ] Dashboard con gráficos de tendencias
- [ ] API REST completa con Swagger/OpenAPI
- [ ] Integración con sensores IoT
- [ ] App móvil nativa (Flutter/React Native)
- [ ] Backup automático de base de datos

## Licencia

[Especificar licencia]

## Contacto

[Información de contacto]
