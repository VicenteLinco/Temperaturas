@echo off
chcp 65001 >nul
echo.
echo ════════════════════════════════════════════════════
echo   Crear Acceso Directo en el Escritorio
echo ════════════════════════════════════════════════════
echo.

REM Obtener ruta del escritorio del usuario
for /f "tokens=3*" %%a in ('reg query "HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders" /v Desktop 2^>nul') do set DESKTOP=%%a %%b

REM Verificar que obtuvimos la ruta
if not defined DESKTOP (
    echo ERROR: No se pudo obtener la ruta del escritorio
    pause
    exit /b 1
)

echo Desktop encontrado: %DESKTOP%
echo.

REM Crear acceso directo usando PowerShell
echo Creando acceso directo...
powershell -Command "$WshShell = New-Object -ComObject WScript.Shell; $Shortcut = $WshShell.CreateShortcut('%DESKTOP%\Sistema de Temperaturas.lnk'); $Shortcut.TargetPath = '%~dp0iniciar_servidor_oculto.vbs'; $Shortcut.WorkingDirectory = '%~dp0'; $Shortcut.Description = 'Iniciar Sistema de Gestion de Temperaturas en segundo plano'; $Shortcut.Save()"

if %errorlevel% equ 0 (
    echo.
    echo ════════════════════════════════════════════════════
    echo   ✅ ACCESO DIRECTO CREADO CORRECTAMENTE
    echo ════════════════════════════════════════════════════
    echo.
    echo El acceso directo se creó en tu escritorio con el nombre:
    echo "Sistema de Temperaturas"
    echo.
    echo Ahora puedes hacer doble clic en ese acceso directo
    echo para iniciar el servidor en segundo plano.
    echo.
) else (
    echo.
    echo ════════════════════════════════════════════════════
    echo   ❌ ERROR AL CREAR ACCESO DIRECTO
    echo ════════════════════════════════════════════════════
    echo.
)

echo Presiona cualquier tecla para salir...
pause >nul
