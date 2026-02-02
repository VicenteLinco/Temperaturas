# Script de Respaldo Automático de Base de Datos
# Este script copia la base de datos a la ruta de red especificada con la fecha actual.

$SourceFile = "C:\Users\VALTEK\Documents\GitHub\Temperaturas\datos.db"
$BackupDir = "\\10.4.172.71\Gestion Laboratorio\2026\TEMPERATURAS\Respaldo base de datos automatico"
$Date = Get-Date -Format "yyyy-MM-dd_HH-mm"
$BackupFile = "$BackupDir\datos_backup_$Date.db"

# Asegurarse de que el directorio de origen existe
if (-not (Test-Path $SourceFile)) {
    Write-Error "Error: No se encuentra el archivo de base de datos en $SourceFile"
    exit
}

# Realizar la copia
try {
    Copy-Item -Path $SourceFile -Destination $BackupFile -Force -ErrorAction Stop
    Write-Host "Respaldo completado con éxito: $BackupFile"
} catch {
    Write-Error "Error al realizar el respaldo: $($_.Exception.Message)"
}

