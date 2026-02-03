@echo off
SETLOCAL EnableDelayedExpansion
chcp 65001 >nul
title Sistema de Gestion de Temperaturas
color 0A
echo.
echo ════════════════════════════════════════════════════
echo   SISTEMA DE GESTION DE TEMPERATURAS
echo ════════════════════════════════════════════════════
echo.
echo Iniciando servicios...
echo.

REM Verificar si el servidor ya esta corriendo
netstat -ano | findstr :3000 >nul
if %errorlevel% equ 0 (
    echo [ADVERTENCIA] El servidor ya esta corriendo en el puerto 3000
    echo [ACCION] Deteniendo proceso anterior...
    for /f "tokens=5" %%a in ('netstat -ano ^| findstr :3000 ^| findstr LISTENING') do taskkill /PID %%a /F >nul 2>&1
    timeout /t 2 /nobreak >nul
)

REM Verificar si ngrok ya esta corriendo
tasklist | findstr ngrok.exe >nul
if %errorlevel% equ 0 (
    echo [ADVERTENCIA] ngrok ya esta corriendo
    echo [ACCION] Deteniendo proceso anterior...
    taskkill /IM ngrok.exe /F >nul 2>&1
    timeout /t 2 /nobreak >nul
)

echo.
echo [INFO] Verificando respaldos automáticos...
powershell -ExecutionPolicy Bypass -File "%~dp0auto_respaldo.ps1"

echo.
echo [1/4] Configurando ngrok...
set NGROK_TOKEN=
REM Intentar leer desde .env en la raiz
if exist "%~dp0..\.env" (
    for /f "tokens=2 delims==" %%a in ('findstr "NGROK_AUTHTOKEN=" "%~dp0..\.env"') do set NGROK_TOKEN=%%a
)
REM Si no esta en .env, intentar leer desde token.txt
if "%NGROK_TOKEN%"=="" (
    if exist "%~dp0token.txt" (
        set /p NGROK_TOKEN=<"%~dp0token.txt"
    )
)

if not "%NGROK_TOKEN%"=="" (
    echo [OK] Token encontrado, habilitando tunel publico.
    echo [INFO] Verificando actualizaciones de ngrok...
    "%~dp0ngrok.exe" update >nul 2>&1
    "%~dp0ngrok.exe" config add-authtoken %NGROK_TOKEN% >nul 2>&1
    set USE_NGROK=1
) else (
    echo [INFO] No se encontro token de ngrok, se iniciara en modo solo local.
    set USE_NGROK=0
)

echo [2/4] Iniciando servidor Rust...
cd /d "%~dp0.."
start "Servidor Rust" /min cargo run

echo [3/4] Esperando que el servidor este listo...
timeout /t 5 /nobreak >nul

if "%USE_NGROK%"=="1" GOTO NgrokPath
GOTO LocalPath

:NgrokPath
    echo [4/4] Iniciando ngrok...
    start "" /B "%~dp0ngrok.exe" http 3000
    
    echo.
    echo Esperando que ngrok se conecte...
    timeout /t 5 /nobreak >nul
    
    echo.
    echo ════════════════════════════════════════════════════
    echo   SERVICIOS INICIADOS CORRECTAMENTE
    echo ════════════════════════════════════════════════════
    echo.
    echo Obteniendo URL publica...
    echo.

    REM Obtener la URL de ngrok usando el nuevo script de PowerShell
    powershell -ExecutionPolicy Bypass -File "%~dp0get_ngrok_url.ps1"
GOTO EndNgrokIf

:LocalPath
    echo [4/4] Saltando inicio de ngrok (Modo Local).
    echo.
    echo ════════════════════════════════════════════════════
    echo   SERVIDOR INICIADO (SOLO LOCAL)
    echo ════════════════════════════════════════════════════
    echo.
    echo URL LOCAL: http://localhost:3000
    echo.
GOTO EndNgrokIf

:EndNgrokIf

echo ════════════════════════════════════════════════════
echo.
echo Abriendo URL en el navegador...

REM Abrir la URL en el navegador
powershell -Command "Start-Process http://localhost:3000"
echo "Navegador abierto con URL local."
GOTO EndOpenUrl

:EndOpenUrl

echo.
echo El navegador se ha abierto.
echo.
echo ════════════════════════════════════════════════════
echo   OPCIONES DE VISUALIZACION
echo ════════════════════════════════════════════════════
echo.
echo [1] Mantener ventana minimizada en barra de tareas (Normal)
echo [2] Ocultar en la bandeja del sistema (Reloj) - Recomendado
echo.
set /p "OPCION=Elige una opcion (1 o 2): "

if "%OPCION%"=="2" GOTO MinimizeTray
GOTO StandardMode

:MinimizeTray
    echo.
    echo [INFO] Configurando bandeja del sistema...
    echo El servidor seguira corriendo oculto.
    echo.
    echo IMPORTANTE:
    echo - Busca el icono de termometro en tu bandeja (cerca del reloj).
    echo - Haz doble click en el icono para abrir el navegador.
    echo - Click derecho para salir/cerrar todo.
    echo.
    echo Ocultando en 3 segundos...
    timeout /t 3 /nobreak >nul
    
    REM Ejecutar el helper de bandeja que oculta las ventanas y maneja el icono
    powershell -ExecutionPolicy Bypass -File "%~dp0tray_handler.ps1"
    
    REM Si el script de powershell termina (usuario da Salir), cerramos aqui tambien
    GOTO EndScript

:StandardMode
    echo.
    echo [INFO] Modo estandar seleccionado.
    echo IMPORTANTE: Mantener esta ventana abierta (minimizarla, no cerrarla).
    echo Si cierras esta ventana, los servicios se detendran.
    echo.
    echo ════════════════════════════════════════════════════
    echo.
    color 0C
    echo.
    echo Minimizando ventana en 5 segundos...
    timeout /t 5 /nobreak >nul

    REM Minimizar la ventana usando PowerShell (SW_MINIMIZE = 6)
    powershell -Command "$signature = '[DllImport(\"user32.dll\")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);'; $type = Add-Type -MemberDefinition $signature -Name Win32ShowWindow -Namespace Win32Functions -PassThru; $hwnd = (Get-Process -Id $PID).MainWindowHandle; if($hwnd -ne [IntPtr]::Zero) { $type::ShowWindow($hwnd, 6) }" 2>nul

    REM Mantener el script corriendo en segundo plano
    :loop
    timeout /t 60 /nobreak >nul
    goto loop

:EndScript
    exit
