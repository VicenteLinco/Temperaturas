# 🎯 Servidor en Bandeja del Sistema

## 📋 Descripción

El sistema ahora puede ejecutarse completamente en segundo plano sin ventanas visibles, con un icono en la bandeja del sistema (system tray) que permite controlarlo.

---

## 🚀 Cómo Iniciar el Servidor

### Opción 1: Doble clic en el archivo VBS (Recomendado)

```
iniciar_servidor_oculto.vbs
```

**Características**:
- ✅ Sin ventanas visibles
- ✅ Icono en bandeja del sistema
- ✅ Notificación emergente con URL
- ✅ Menú contextual completo

---

## 🎛️ Funciones del Icono en Bandeja

Al hacer **clic derecho** en el icono de la bandeja verás:

### 📌 Menú Contextual

1. **Abrir en navegador**
   - Abre automáticamente la URL pública de ngrok
   - Si ngrok no está disponible, abre localhost:3000

2. **Abrir local (localhost:3000)**
   - Abre directamente la URL local
   - Útil para acceso desde la misma computadora

3. **Ver estado**
   - Muestra ventana con información completa:
     - URL pública (si ngrok está activo)
     - URL local
     - Credenciales de acceso

4. **Ver log**
   - Abre el archivo `servidor.log` en el Bloc de notas
   - Útil para diagnóstico de problemas

5. **Detener servidor**
   - Pregunta confirmación
   - Detiene todos los servicios (Rust + ngrok)
   - Cierra el icono de la bandeja

### 🖱️ Doble Clic en el Icono

Abre automáticamente el sistema en el navegador (URL pública o local)

---

## 🛑 Cómo Detener el Servidor

### Opción 1: Desde el icono de bandeja (Recomendado)
1. Clic derecho en el icono
2. "Detener servidor"
3. Confirmar

### Opción 2: Ejecutar script de detención
```
detener_servidor_bandeja.vbs
```

### Opción 3: Administrador de Tareas
- Buscar procesos `ngrok.exe` y `cargo.exe`
- Finalizar tareas

---

## 📊 Archivos Creados

| Archivo | Descripción |
|---------|-------------|
| `iniciar_servidor_oculto.vbs` | **Archivo principal** - Inicia todo sin ventanas |
| `iniciar_servidor_bandeja.bat` | Script BAT que ejecuta el VBS (llamado automáticamente) |
| `detener_servidor_bandeja.vbs` | Detiene el servidor manualmente |
| `servidor.log` | Log de eventos (se crea automáticamente) |

---

## ✨ Características

### ✅ Ventajas del Nuevo Sistema

1. **Sin ventanas molestas**
   - Todo corre en segundo plano
   - No aparece ninguna consola

2. **Notificaciones emergentes**
   - Al iniciar muestra la URL pública
   - Incluye credenciales de acceso

3. **Control desde bandeja**
   - Menú contextual completo
   - Doble clic para abrir
   - Opción de detener con confirmación

4. **Log de eventos**
   - Archivo `servidor.log` con timestamps
   - Útil para diagnóstico

5. **Seguridad**
   - Confirmación antes de detener
   - Previene cierres accidentales

---

## 🔧 Solución de Problemas

### El icono no aparece en la bandeja

**Causa**: Puede estar oculto en "mostrar iconos ocultos"

**Solución**:
1. Buscar la flecha `^` en la bandeja del sistema
2. Clic en la flecha para ver iconos ocultos
3. Encontrar el icono del sistema

### El servidor no inicia

**Verificar**:
1. Revisar el archivo `servidor.log`
2. Verificar que el puerto 3000 esté libre
3. Comprobar que `token.txt` existe (para ngrok)

**Comando de diagnóstico**:
```cmd
netstat -ano | findstr :3000
```

### Detener procesos manualmente

Si el script de detención no funciona:

```cmd
taskkill /IM ngrok.exe /F
taskkill /IM cargo.exe /F
```

---

## 📝 Comparación: Antes vs Ahora

| Característica | Script Anterior | Script Nuevo |
|----------------|----------------|--------------|
| **Ventana visible** | ✅ Sí (minimizada) | ❌ No |
| **Icono en bandeja** | ❌ No | ✅ Sí |
| **Menú contextual** | ❌ No | ✅ Sí |
| **Notificaciones** | ❌ No | ✅ Sí |
| **Ver estado** | ❌ No | ✅ Sí |
| **Ver log** | ❌ No | ✅ Sí |
| **Confirmación detener** | ❌ No | ✅ Sí |
| **Doble clic abrir** | ❌ No | ✅ Sí |

---

## 🎨 Personalización

### Cambiar texto de notificación

Editar `iniciar_servidor_bandeja.bat` línea con `BalloonTipText`

### Cambiar duración de notificación

Buscar `ShowBalloonTip(10000)` y cambiar milisegundos (10000 = 10 segundos)

### Cambiar texto del tooltip

Buscar `$notifyIcon.Text = 'Sistema de Temperaturas - Activo'`

---

## 🚨 Importante

1. **No eliminar el archivo BAT**: El VBS lo necesita para funcionar
2. **Mantener ambos archivos juntos**: VBS y BAT en la misma carpeta
3. **Log se sobreescribe**: Cada inicio reemplaza el log anterior
4. **Confirmar antes de detener**: Evita pérdida de datos

---

## 🔐 Seguridad

El sistema ahora es más seguro porque:

- ✅ No muestra credenciales en ventanas abiertas
- ✅ Requiere confirmación para detener
- ✅ Log privado en archivo local
- ✅ Notificaciones temporales que desaparecen

---

## 📦 Instalación en Inicio Automático (Opcional)

Para que el servidor inicie con Windows:

1. Presiona `Win + R`
2. Escribe: `shell:startup`
3. Copia `iniciar_servidor_oculto.vbs` a esa carpeta
4. Listo - el servidor iniciará al encender la PC

**O crear acceso directo**:
1. Clic derecho en `iniciar_servidor_oculto.vbs`
2. Crear acceso directo
3. Mover acceso directo a carpeta Inicio

---

## 📞 Soporte

Si tienes problemas:

1. Revisa `servidor.log`
2. Verifica que Rust esté instalado: `cargo --version`
3. Verifica que ngrok esté en la carpeta
4. Comprueba que `token.txt` existe

---

## ✅ Checklist de Funcionamiento

- [ ] Doble clic en `iniciar_servidor_oculto.vbs`
- [ ] Aparece notificación emergente con URL
- [ ] Icono visible en bandeja del sistema
- [ ] Clic derecho muestra menú contextual
- [ ] Doble clic en icono abre navegador
- [ ] "Ver estado" muestra información correcta
- [ ] "Ver log" abre archivo de log
- [ ] "Detener servidor" pide confirmación
- [ ] Al confirmar, todo se cierra correctamente

---

**Última actualización**: 2026-01-08
**Versión**: 2.0 - Sistema de Bandeja
**Estado**: ✅ Listo para producción
