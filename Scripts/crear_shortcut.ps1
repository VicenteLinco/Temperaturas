# Script para crear acceso directo en el Escritorio
$ErrorActionPreference = "Stop"

try {
    # 1. Obtener rutas
    $ProjectDir = (Get-Item "$PSScriptRoot\..").FullName
    $TargetScript = "$ProjectDir\Scripts\iniciar_servidor.bat"
    $IconPath = "$ProjectDir\Scripts\app_icon_96.png"
    $DesktopPath = [System.Environment]::GetFolderPath('Desktop')
    $ShortcutPath = "$DesktopPath\SISTEMA_TEMPERATURAS.lnk"

    # 2. Verificar que el destino existe
    if (-not (Test-Path $TargetScript)) {
        Write-Host "ERROR: No se encuentra el archivo $TargetScript" -ForegroundColor Red
        exit
    }

    # 3. Crear el objeto COM para el acceso directo
    $WshShell = New-Object -ComObject WScript.Shell
    $Shortcut = $WshShell.CreateShortcut($ShortcutPath)

    # 4. Configurar propiedades
    $Shortcut.TargetPath = $TargetScript
    $Shortcut.WorkingDirectory = "$ProjectDir\Scripts"
    $Shortcut.Description = "Iniciar Sistema de Temperaturas (Admin)"
    
    # Asignar icono si existe, si no, usa el default
    if (Test-Path $IconPath) {
        $Shortcut.IconLocation = $IconPath
    }

    # 5. Guardar
    $Shortcut.Save()

    Write-Host ""
    Write-Host " [EXITO] " -BackgroundColor Green -ForegroundColor White -NoNewline
    Write-Host " Acceso directo creado en: $ShortcutPath" -ForegroundColor Green
    Write-Host ""

} catch {
    Write-Host ""
    Write-Host " [ERROR] " -BackgroundColor Red -ForegroundColor White -NoNewline
    Write-Host " " + $_.Exception.Message -ForegroundColor Red
    Write-Host ""
}
