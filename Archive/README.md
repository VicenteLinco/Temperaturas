# Archive - Archivos Archivados

Esta carpeta contiene archivos que no son necesarios para el funcionamiento del sistema pero se mantienen por referencia histórica.

## Contenido

### Certificados SSL (No usados actualmente)
- `cert.pem` - Certificado SSL de prueba
- `key.pem` - Clave privada SSL de prueba

**Nota**: El sistema usa Cloudflare Tunnel o ngrok para HTTPS, por lo que estos certificados no son necesarios.

### Herramientas de Túnel
- `ngrok.exe` - Ejecutable de ngrok (31.7 MB)
- `ngrok.zip` - Archivo comprimido de ngrok (11.1 MB)

**Nota**: Cloudflare Tunnel es la opción recomendada actualmente.

### Archivos Temporales
- `cookies.txt` - Archivo temporal de cookies
- `token.txt` - Token temporal
- `nul` - Archivo vacío temporal

### Logs
- `servidor.log` - Log del servidor (se genera automáticamente en cada ejecución)

**Nota**: Los logs nuevos se generan en la carpeta raíz durante la ejecución. Este archivo es una copia de referencia.

---

## Limpieza

Estos archivos pueden ser eliminados de forma segura si:
- No planeas usar ngrok
- No necesitas los certificados SSL locales
- No necesitas logs históricos

Para liberar espacio, puedes eliminar:
```bash
rm -rf Archive/
```

**Espacio ocupado**: ~43 MB

---

## Fecha de archivo

Archivado: 2026-01-08
Por: Reorganización del proyecto v2.1
