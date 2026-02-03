
# Configuración
$rootPath = Resolve-Path "$PSScriptRoot\.."
$dbPath = Join-Path $rootPath "temperaturas.db"
$backupDir = Join-Path $rootPath "Backups"

# Crear carpeta de backups si no existe
if (!(Test-Path $backupDir)) {
    New-Item -ItemType Directory -Path $backupDir | Out-Null
}

# Calcular identificador de la semana actual (Año + Número de Semana)
# Ejemplo: 2026_Semana06
$culture = [System.Globalization.CultureInfo]::CurrentCulture
$weekNum = $culture.Calendar.GetWeekOfYear((Get-Date), [System.Globalization.CalendarWeekRule]::FirstFourDayWeek, [DayOfWeek]::Monday)
$year = (Get-Date).Year
$backupName = "Respaldo_${year}_Semana${weekNum}.db"
$targetFile = Join-Path $backupDir $backupName

# Lógica de verificación
if (Test-Path $dbPath) {
    if (!(Test-Path $targetFile)) {
        Write-Host " [AUTO-RESPALDO] Generando copia de seguridad de la Semana $weekNum..." -ForegroundColor Cyan
        Copy-Item $dbPath $targetFile
        Write-Host " [OK] Respaldo guardado en: Backups\$backupName" -ForegroundColor Green
        
        # Limpieza: Mantener solo los últimos 10 respaldos para no llenar el disco
        $oldFiles = Get-ChildItem $backupDir -Filter "Respaldo_*.db" | Sort-Object CreationTime -Descending | Select-Object -Skip 10
        if ($oldFiles) {
            foreach ($file in $oldFiles) {
                Remove-Item $file.FullName
            }
            Write-Host " [LIMPIEZA] Se eliminaron respaldos muy antiguos." -ForegroundColor Gray
        }
    } else {
        Write-Host " [AUTO-RESPALDO] Ya existe un respaldo para esta semana ($weekNum)." -ForegroundColor DarkGray
    }
} else {
    Write-Host " [ADVERTENCIA] No se encontró la base de datos para respaldar." -ForegroundColor Yellow
}
