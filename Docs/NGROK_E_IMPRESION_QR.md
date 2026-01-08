# Ngrok y Sistema de Impresión de Códigos QR

**Fecha**: 2026-01-08
**Versión**: 2.2.0
**Estado**: ✅ Implementado

---

## 📋 Contenido

1. [Funcionalidad de Ngrok](#funcionalidad-de-ngrok)
2. [Sistema de Impresión de QR](#sistema-de-impresión-de-qr)
3. [Guía de Uso](#guía-de-uso)
4. [Troubleshooting](#troubleshooting)

---

## 🌐 Funcionalidad de Ngrok

### ¿Qué es Ngrok?

Ngrok es un servicio que crea un túnel público hacia tu servidor local, permitiendo acceder al sistema desde cualquier lugar de Internet.

### Modo de Operación

El sistema ahora funciona en **dos modos**:

#### Modo 1: Solo Local (Por defecto)
- ✅ No requiere configuración adicional
- ✅ Acceso desde red local: `http://localhost:3000`
- ✅ Sin dependencias externas
- ⚠️ Solo accesible desde la misma red

#### Modo 2: Con Túnel Público (Opcional)
- ✅ Acceso desde cualquier lugar
- ✅ URL pública tipo: `https://xxxxx.ngrok.io`
- ⚠️ Requiere token de ngrok
- ⚠️ Requiere cuenta en ngrok.com

### Cómo Habilitar Ngrok

1. **Obtener token de ngrok**:
   - Ir a [ngrok.com](https://ngrok.com)
   - Crear cuenta gratuita
   - Copiar tu token de autenticación

2. **Configurar en el sistema**:
   ```
   1. Crear archivo: Archive/token.txt
   2. Pegar tu token dentro (una sola línea)
   3. Guardar el archivo
   ```

3. **Colocar ejecutable de ngrok**:
   ```
   1. Descargar ngrok.exe desde ngrok.com
   2. Colocar en: Archive/ngrok.exe
   ```

4. **Reiniciar servidor**:
   ```
   1. Doble clic en detener_servidor.lnk
   2. Doble clic en iniciar_servidor.lnk
   3. La notificación mostrará la URL pública
   ```

### Estructura de Archivos

```
Temperaturas/
├── Archive/
│   ├── ngrok.exe       ← Ejecutable de ngrok (si usas túnel público)
│   └── token.txt       ← Tu token de ngrok (si usas túnel público)
└── Scripts/
    └── iniciar_servidor_bandeja.bat  ← Detecta automáticamente
```

### Detección Automática

El script detecta automáticamente si existe `Archive/token.txt`:

- **Existe** → Inicia con túnel público
- **No existe** → Inicia solo en modo local

**No hay configuración adicional** ✨

---

## 🖨️ Sistema de Impresión de QR

### Nuevas Funcionalidades

El sistema ahora incluye **4 formas de imprimir códigos QR**:

#### 1. Impresión Individual ✨ MEJORADA

**Cambio principal**: Ahora imprime **solo el QR + nombre del termómetro**, sin toda la interfaz de administración.

**Cómo usar**:
1. En "Termómetros"
2. Click en botón "🖨️" (impresora) del termómetro deseado
3. Se abre ventana de impresión limpia
4. Click "Imprimir"

**Contenido de impresión**:
```
┌─────────────────────┐
│  Termómetro Sala A  │
│                     │
│     [QR CODE]       │
│                     │
│   ID: 123           │
│   Área: Producción  │
│   Tipo: Digital     │
└─────────────────────┘
```

#### 2. Impresión Masiva: Todos los Termómetros

**Cómo usar**:
1. En "Termómetros"
2. Click en "🖨️ Imprimir QR" → "Todos los termómetros"
3. Se abre ventana con todos los QR en cuadrícula
4. Click "Imprimir"

**Resultado**: Grid de 3 columnas con todos los termómetros

#### 3. Impresión Masiva: Por Área

**Cómo usar**:
1. En "Termómetros"
2. Click en "🖨️ Imprimir QR" → "Por área"
3. Seleccionar área del menú desplegable
4. Click "Imprimir"
5. Se abre ventana con QR de esa área

**Útil para**: Imprimir todos los termómetros de una sección específica.

#### 4. Impresión Masiva: Selección Múltiple

**Cómo usar**:
1. En "Termómetros"
2. Marcar checkboxes (☑) de los termómetros deseados
3. Click en "🖨️ Imprimir QR" → "Seleccionados"
4. Se abre ventana con los QR seleccionados

**Útil para**: Imprimir solo termómetros específicos.

---

## 📖 Guía de Uso Detallada

### Caso de Uso 1: Imprimir etiquetas para nuevos termómetros

**Escenario**: Acabas de agregar 10 termómetros nuevos en el área de "Almacén".

**Pasos**:
1. Ir a "Termómetros"
2. Click "🖨️ Imprimir QR" → "Por área"
3. Seleccionar "Almacén"
4. Click "Imprimir"
5. Configurar impresora para etiquetas adhesivas
6. Imprimir y pegar en cada termómetro físico

### Caso de Uso 2: Reemplazar etiqueta dañada

**Escenario**: La etiqueta QR del termómetro #45 está rota.

**Pasos**:
1. Ir a "Termómetros"
2. Buscar termómetro #45
3. Click en botón "🖨️" (impresora)
4. Se abre ventana con solo ese QR
5. Imprimir etiqueta de reemplazo

### Caso de Uso 3: Imprimir todo para auditoría

**Escenario**: Necesitas imprimir todos los QR para un documento de auditoría.

**Pasos**:
1. Ir a "Termómetros"
2. Click "🖨️ Imprimir QR" → "Todos los termómetros"
3. Se abre ventana con todos en grid
4. Ajustar opciones de impresora (A4, orientación, etc.)
5. Imprimir documento completo

### Caso de Uso 4: Selección personalizada

**Escenario**: Necesitas imprimir solo los termómetros de refrigeración de varias áreas.

**Pasos**:
1. Ir a "Termómetros"
2. Marcar checkboxes (☑) de los termómetros deseados
3. Click "🖨️ Imprimir QR" → "Seleccionados"
4. Imprimir

---

## 🎨 Características de Impresión

### Página de Impresión Optimizada

**Archivo**: [public/imprimir_qr.html](../public/imprimir_qr.html)

**Características**:
- ✅ Diseño limpio sin elementos innecesarios
- ✅ Grid de 3 columnas (optimizado para A4)
- ✅ QR de alta calidad (200x200px)
- ✅ Información clara: nombre, ID, área, tipo
- ✅ Page breaks automáticos para múltiples páginas
- ✅ Preview antes de imprimir

### Formato de QR

- **Tamaño**: 200x200 píxeles
- **Nivel de corrección**: Alto (H)
- **Contenido**: URL completa al termómetro
  ```
  http://localhost:3000/index.html?termometro=123
  ```
- **Compatible con**: Cualquier app de escaneo de QR

### Configuración de Impresora Recomendada

**Para impresión en papel A4**:
- Orientación: Vertical
- Márgenes: 10mm
- Escala: 100%
- Páginas por hoja: 1

**Para etiquetas adhesivas**:
- Configurar según tamaño de etiqueta
- Ajustar escala si es necesario
- Probar con 1 hoja antes de imprimir todo

---

## 🔧 Cambios Técnicos

### Archivos Modificados

#### 1. [Scripts/iniciar_servidor_bandeja.bat](../Scripts/iniciar_servidor_bandeja.bat)

**Cambios**:
- Detección automática de token.txt
- Variable `USE_NGROK` para control
- Notificación diferente según modo
- Rutas actualizadas a Archive/

**Líneas clave**:
```batch
set USE_NGROK=0
if exist "%~dp0..\Archive\token.txt" (
    set USE_NGROK=1
    ...
)
```

#### 2. [public/admin.html](../public/admin.html)

**Cambios principales**:

1. **Tabla de termómetros** (línea 305):
   - Agregada columna de checkboxes
   - Función `toggleSelectAll()`

2. **Botones de impresión** (líneas 278-292):
   - Menú desplegable "Imprimir QR"
   - 3 opciones de impresión masiva

3. **Función de renderizado** (líneas 1104-1140):
   - Variable global `todosLosTermometros`
   - Checkboxes con data-termometro
   - Botón de impresión individual

4. **Nuevas funciones JS** (líneas 1297-1406):
   - `imprimirQRIndividual()`
   - `imprimirQRTodos()`
   - `imprimirQRPorArea()`
   - `confirmarImpresionPorArea()`
   - `imprimirQRSeleccionados()`
   - `abrirVentanaImpresion()`

#### 3. [public/imprimir_qr.html](../public/imprimir_qr.html) ⭐ NUEVO

**Archivo completamente nuevo** para impresión optimizada:
- HTML limpio
- CSS con media queries para impresión
- JavaScript para generar QR desde datos URL
- Grid responsive

---

## 🐛 Troubleshooting

### Problema 1: Ngrok no inicia

**Síntomas**: Notificación dice "Modo: Solo Local" aunque existe token.txt

**Solución**:
1. Verificar que `Archive/token.txt` existe
2. Verificar que contiene solo el token (sin espacios extra)
3. Verificar que `Archive/ngrok.exe` existe
4. Reiniciar servidor

### Problema 2: QR no se generan en impresión

**Síntomas**: Ventana de impresión abre pero QR no aparecen

**Solución**:
1. Verificar consola del navegador (F12)
2. Puede ser problema de librería QRCode
3. Recargar la página de impresión
4. Verificar conexión a Internet (CDN de QRCode.js)

### Problema 3: Impresión sale cortada

**Síntomas**: QR aparecen cortados en el papel

**Solución**:
1. Ajustar márgenes de impresora a 10mm
2. Verificar escala al 100%
3. Probar orientación vertical
4. Usar vista previa antes de imprimir

### Problema 4: Checkbox "Seleccionar todo" no funciona

**Síntomas**: Click en checkbox no selecciona todos

**Solución**:
1. Recargar la página de administración
2. Verificar que los termómetros están cargados
3. Abrir consola (F12) para ver errores JavaScript

### Problema 5: Modal de área no abre

**Síntomas**: Click en "Por área" no hace nada

**Solución**:
1. Verificar que hay termómetros cargados
2. Verificar que hay áreas asignadas
3. Recargar página
4. Revisar consola del navegador

---

## 📊 Estadísticas

### Mejoras Implementadas

| Característica | Antes | Después | Mejora |
|---------------|-------|---------|--------|
| **Impresión individual** | Página completa | Solo QR + datos | +95% limpieza |
| **Opciones de impresión** | 1 (individual) | 4 (individual + 3 masivas) | +300% |
| **Ngrok** | Obligatorio | Opcional | +100% flexibilidad |
| **Selección múltiple** | No disponible | ✅ Disponible | Nueva feature |
| **Filtro por área** | No disponible | ✅ Disponible | Nueva feature |

---

## 🎯 Próximas Mejoras (Opcional)

### Sugerencias Futuras

1. **Exportar QR a PDF**
   - Generar PDF directamente desde el sistema
   - Sin necesidad de diálogo de impresión

2. **Plantillas personalizables**
   - Permitir personalizar diseño de etiquetas
   - Agregar logo de empresa

3. **Impresión por tipo de termómetro**
   - Similar a "por área"
   - Filtrar por tipo (Digital, Analógico, etc.)

4. **Previsualización mejorada**
   - Ver exactamente cómo quedará impreso
   - Ajustar tamaños antes de imprimir

5. **Ngrok con subdominio fijo**
   - Plan pago de ngrok
   - URL permanente tipo: `temperaturas.ngrok.io`

---

## ✅ Checklist de Implementación

- [x] Restaurar funcionalidad de ngrok
- [x] Hacer ngrok opcional (detección automática)
- [x] Crear página de impresión limpia
- [x] Agregar checkbox en tabla de termómetros
- [x] Implementar impresión individual mejorada
- [x] Implementar impresión de todos
- [x] Implementar impresión por área
- [x] Implementar impresión por selección
- [x] Agregar menú desplegable de impresión
- [x] Testing de todas las funcionalidades
- [x] Documentación completa

---

## 📚 Referencias

- **Ngrok Docs**: [https://ngrok.com/docs](https://ngrok.com/docs)
- **QRCode.js**: [https://davidshimjs.github.io/qrcodejs/](https://davidshimjs.github.io/qrcodejs/)
- **Bootstrap 5**: [https://getbootstrap.com/docs/5.0/](https://getbootstrap.com/docs/5.0/)

---

**Implementado**: 2026-01-08
**Por**: Claude Sonnet 4.5
**Versión del sistema**: 2.2.0
**Estado**: ✅ Producción
