# Accesos Directos - Sistema de Temperaturas

**Fecha**: 2026-01-08
**Versión**: 2.1

---

## 🎯 Propósito

Facilitar el uso del sistema proporcionando accesos directos en la carpeta raíz que apuntan a los scripts en `Scripts/`.

---

## 📋 Accesos Directos Disponibles

### 1. `iniciar_servidor.lnk` ⭐

**Apunta a**: `Scripts/iniciar_servidor_oculto.vbs`

**Función**: Inicia el servidor en modo bandeja del sistema (oculto)

**Características**:
- ✅ Sin ventanas visibles
- ✅ Icono en bandeja del sistema
- ✅ Notificaciones emergentes
- ✅ Control desde menú contextual (clic derecho en icono)

**Uso**:
```
1. Doble clic en "iniciar_servidor.lnk"
2. Esperar ~15 segundos
3. Buscar icono en bandeja del sistema (T)
4. Clic derecho → Abrir en navegador
```

---

### 2. `detener_servidor.lnk`

**Apunta a**: `Scripts/detener_servidor_bandeja.vbs`

**Función**: Detiene el servidor que está ejecutándose en la bandeja

**Características**:
- ✅ Cierre limpio del servidor
- ✅ Libera puerto 3000
- ✅ Notificación de cierre

**Uso**:
```
1. Doble clic en "detener_servidor.lnk"
2. Servidor se cierra automáticamente
```

**Alternativa**: Clic derecho en icono de bandeja → "Detener servidor"

---

## 🔧 Detalles Técnicos

### Archivos .lnk

Los archivos `.lnk` son accesos directos de Windows que:
- Apuntan a los scripts reales en `Scripts/`
- Mantienen el directorio de trabajo correcto
- Incluyen descripción del propósito
- No ocupan espacio significativo (~1 KB cada uno)

### Ventajas

1. **Facilidad de uso**: Acceso inmediato desde la raíz
2. **Organización**: Scripts reales siguen en `Scripts/`
3. **Compatibilidad**: Funcionan en Windows Explorer
4. **Flexibilidad**: Se pueden copiar al escritorio

---

## 📁 Estructura

```
Temperaturas/
├── iniciar_servidor.lnk → Scripts/iniciar_servidor_oculto.vbs
├── detener_servidor.lnk → Scripts/detener_servidor_bandeja.vbs
└── Scripts/
    ├── iniciar_servidor_oculto.vbs  (archivo real)
    ├── detener_servidor_bandeja.vbs (archivo real)
    └── ... otros scripts
```

---

## 🎨 Personalización

### Crear Acceso Directo en Escritorio

Puedes usar el script incluido:
```
Scripts/CREAR_ACCESO_DIRECTO.bat
```

O manualmente:
1. Clic derecho en `iniciar_servidor.lnk`
2. Copiar
3. Clic derecho en escritorio
4. Pegar acceso directo

### Cambiar Icono

1. Clic derecho en el acceso directo
2. Propiedades
3. Cambiar icono
4. Seleccionar icono deseado

---

## ❓ Preguntas Frecuentes

### ¿Por qué accesos directos y no copiar los scripts?

- Mantiene la organización (scripts en `Scripts/`)
- Facilita actualizaciones (solo un archivo que actualizar)
- Evita duplicación de código

### ¿Puedo eliminar los accesos directos?

Sí, puedes eliminarlos sin afectar el funcionamiento. Los scripts originales siguen en `Scripts/`.

### ¿Funcionan en Linux/Mac?

No, los archivos `.lnk` son específicos de Windows. En Linux/Mac usa:
```bash
# Crear symlinks
ln -s Scripts/iniciar_servidor_oculto.vbs iniciar_servidor
ln -s Scripts/detener_servidor_bandeja.vbs detener_servidor
```

### ¿Los accesos directos se versionan en git?

Depende del `.gitignore`. Generalmente se excluyen (*.lnk) porque son específicos del sistema local.

---

## 🚀 Flujo de Trabajo Típico

### Inicio del Día

```
1. Doble clic → iniciar_servidor.lnk
2. Esperar notificación "Servidor iniciado"
3. Abrir navegador en http://localhost:3000
4. Trabajar normalmente
```

### Fin del Día

```
1. Doble clic → detener_servidor.lnk
   O
   Clic derecho en icono de bandeja → "Detener servidor"
```

---

## 📊 Beneficios

| Aspecto | Sin Accesos Directos | Con Accesos Directos |
|---------|---------------------|----------------------|
| **Clics para iniciar** | Navegar a Scripts/ → Doble clic | Doble clic directo |
| **Facilidad** | Media | Alta |
| **Organización** | Scripts en raíz | Scripts en carpeta |
| **Usuario nuevo** | Debe buscar | Ve accesos directos inmediatamente |

---

## ✅ Verificación

Accesos directos creados correctamente:

```bash
$ ls -lh *.lnk
-rwxr-xr-x 1 PC1 197121 991 ene.  8 11:41 iniciar_servidor.lnk
-rwxr-xr-x 1 PC1 197121 952 ene.  8 11:41 detener_servidor.lnk
```

**Total**: 2 accesos directos, ~2 KB

---

## 🎯 Conclusión

Los accesos directos mejoran la experiencia de usuario:
- ✅ Más fácil de usar para usuarios no técnicos
- ✅ Mantiene la organización del proyecto
- ✅ Acceso inmediato desde la raíz
- ✅ Compatible con Windows Explorer

**Recomendación**: Usar los accesos directos para uso diario, navegar a `Scripts/` solo para mantenimiento.

---

**Creación**: 2026-01-08
**Tipo**: Accesos directos (.lnk)
**Ubicación**: Carpeta raíz del proyecto
**Compatibilidad**: Windows
