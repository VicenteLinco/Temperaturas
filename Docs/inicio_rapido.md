# 🚀 Inicio Rápido - Sistema de Temperaturas

## ⚡ Opción 1: Ejecutar Directamente (Más Rápido)

### 📌 Con Icono en Bandeja (Recomendado)

Doble clic en:
```
iniciar_servidor_oculto.vbs
```

**Características**:
- ✅ Sin ventanas visibles
- ✅ Icono en bandeja del sistema
- ✅ Notificación emergente con URL
- ✅ Menú contextual completo

**Luego**:
- Espera 10 segundos
- Aparecerá notificación con la URL
- Doble clic en el icono de la bandeja para abrir

---

### 🪟 Con Ventana Tradicional

Doble clic en:
```
iniciar_servidor.bat
```

**Características**:
- ✅ Muestra progreso visual
- ✅ Se minimiza automáticamente
- ⚠️ Ventana visible en barra de tareas

---

## ⚡ Opción 2: Crear Acceso Directo en Escritorio

### Paso 1: Crear el acceso directo

Doble clic en:
```
CREAR_ACCESO_DIRECTO.bat
```

### Paso 2: Usar el acceso directo

Doble clic en el nuevo icono en tu escritorio:
```
Sistema de Temperaturas
```

---

## 🎛️ Usar el Sistema

### Desde Icono de Bandeja

**Clic derecho en icono**:
- 🌐 Abrir en navegador → Abre URL pública
- 🏠 Abrir local → Abre localhost:3000
- ℹ️ Ver estado → Muestra información completa
- 📄 Ver log → Abre archivo de log
- 🛑 Detener servidor → Con confirmación

**Doble clic en icono**:
- Abre automáticamente en navegador

---

## 🔐 Credenciales

**Usuario**: `admin`
**Contraseña**: `admin123`

> ⚠️ Cambiar en producción desde panel de administración

---

## 🛑 Detener el Servidor

### Opción 1: Desde Icono de Bandeja (Recomendado)
1. Clic derecho en icono
2. "Detener servidor"
3. Confirmar

### Opción 2: Script de Detención
Doble clic en:
```
detener_servidor_bandeja.vbs
```

### Opción 3: Script Tradicional
Doble clic en:
```
detener_servidor.bat
```

---

## 📱 Acceder al Sistema

### Desde la Misma Computadora
```
http://localhost:3000
```

### Desde Internet (URL Pública)
- Ver notificación emergente al iniciar
- O clic derecho en icono → "Ver estado"
- O revisar la ventana del script (si usaste .bat)

---

## 🔧 Archivos Importantes

| Archivo | Descripción |
|---------|-------------|
| `iniciar_servidor_oculto.vbs` | ⭐ Inicia en bandeja (recomendado) |
| `iniciar_servidor.bat` | Inicia con ventana tradicional |
| `detener_servidor_bandeja.vbs` | Detiene servidor de bandeja |
| `detener_servidor.bat` | Detiene servidor tradicional |
| `CREAR_ACCESO_DIRECTO.bat` | Crea icono en escritorio |
| `servidor.log` | Log de eventos (auto-generado) |

---

## ❓ ¿Qué Método Usar?

### 🏆 Método Recomendado: VBS en Bandeja

**Usar cuando**:
- Quieres que quede en segundo plano
- No quieres ventanas visibles
- Prefieres control desde bandeja
- Uso diario/producción

**Ventajas**:
- Sin ventanas molestas
- Control completo desde bandeja
- Notificaciones automáticas
- Más profesional

---

### 🪟 Método Tradicional: BAT con Ventana

**Usar cuando**:
- Quieres ver el progreso visual
- Prefieres tener ventana abierta
- Debugging/desarrollo
- Primera vez usando el sistema

**Ventajas**:
- Progreso visible
- Fácil de entender
- Log en tiempo real en consola

---

## 📝 Notas Importantes

1. **Puerto 3000**: El servidor usa este puerto. Si está ocupado, el script lo liberará automáticamente.

2. **Token ngrok**: Debe existir el archivo `token.txt` con tu token de ngrok para acceso público.

3. **Tiempo de inicio**: Espera 10-15 segundos para que todo inicie correctamente.

4. **Antivirus**: Algunos antivirus pueden bloquear scripts VBS. Agrega excepción si es necesario.

5. **Primera ejecución**: La primera vez puede tardar más mientras compila el servidor Rust.

---

## 🚨 Solución Rápida de Problemas

### El icono no aparece en la bandeja
- Buscar en iconos ocultos (flecha `^`)
- Esperar 15 segundos más

### "Puerto 3000 ocupado"
- El script lo libera automáticamente
- O ejecutar `detener_servidor_bandeja.vbs`

### No puedo abrir la URL
- Verificar que `token.txt` existe
- O usar URL local: `http://localhost:3000`

### El servidor no inicia
- Revisar `servidor.log`
- Verificar que Rust está instalado: `cargo --version`

---

## ✅ Checklist Primera Vez

- [ ] Ejecutar `iniciar_servidor_oculto.vbs`
- [ ] Esperar notificación emergente (10-15 seg)
- [ ] Verificar icono en bandeja del sistema
- [ ] Hacer doble clic en icono
- [ ] Se abre el navegador con el sistema
- [ ] Iniciar sesión con admin/admin123
- [ ] ✅ Todo funciona

---

**Última actualización**: 2026-01-08
**Versión**: 2.0
**Estado**: ✅ Listo para uso
