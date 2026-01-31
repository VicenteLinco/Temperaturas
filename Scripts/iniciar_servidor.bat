@echo off
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
echo [1/4] Configurando ngrok...
set NGROK_TOKEN=
REM Intentar leer desde .env en la raiz
if exist "%~dp0..\.env" (
    for /f "tokens=2 delims==" %%a in ('findstr /B "NGROK_AUTHTOKEN=" "%~dp0..\.env"') do set NGROK_TOKEN=%%a
)
REM Si no esta en .env, intentar leer desde token.txt
if "%NGROK_TOKEN%"=="" (
    if exist "%~dp0token.txt" (
        set /p NGROK_TOKEN=<"%~dp0token.txt"
    )
)

if not "%NGROK_TOKEN%"=="" (
    echo [OK] Token encontrado, habilitando tunel publico.
    "%~dp0ngrok.exe" config add-authtoken %NGROK_TOKEN% >nul 2>&1
    set USE_NGROK=1
) else (
    echo [INFO] No se encontro token de ngrok, se iniciara en modo solo local.
    set USE_NGROK=0
)

echo [2/4] Iniciando servidor Rust...
cd /d "%~dp0.."
start /B cargo run >nul 2>&1

echo [3/4] Esperando que el servidor este listo...
timeout /t 5 /nobreak >nul

if %USE_NGROK%==1 (
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

    REM Obtener la URL de ngrok usando PowerShell
    powershell -Command "$response = try { Invoke-RestMethod -Uri 'http://localhost:4040/api/tunnels' -ErrorAction SilentlyContinue } catch { $null }; if($response) { $url = $response.tunnels[0].public_url; Write-Host ''; Write-Host 'URL PUBLICA: ' -NoNewline -ForegroundColor Green; Write-Host $url -ForegroundColor Cyan; Write-Host ''; Write-Host 'Servidor local: http://localhost:3000' -ForegroundColor Gray; Write-Host ''; } else { Write-Host 'Error al conectar con ngrok. Revisa el token.' -ForegroundColor Red; }"
) else (
    echo [4/4] Saltando inicio de ngrok (Modo Local).
    echo.
    echo ════════════════════════════════════════════════════
    echo   SERVIDOR INICIADO (SOLO LOCAL)
    echo ════════════════════════════════════════════════════
    echo.
    echo URL LOCAL: http://localhost:3000
    echo.
)

echo ════════════════════════════════════════════════════
echo.
echo Presiona cualquier tecla para abrir la URL en el navegador...
pause >nul

REM Abrir la URL en el navegador
powershell -Command "$response = Invoke-RestMethod -Uri 'http://localhost:4040/api/tunnels'; $url = $response.tunnels[0].public_url; Start-Process $url"

echo.
echo El navegador se ha abierto con la URL publica.
echo.
echo IMPORTANTE: Mantener esta ventana abierta.
echo Si cierras esta ventana, los servicios se detendran.
echo.
echo ════════════════════════════════════════════════════
echo.
echo.
color 0C
echo.
echo Minimizando en 10 segundos...
timeout /t 10 /nobreak >nul

REM Minimizar la ventana usando PowerShell
powershell -Command "$signature = '[DllImport(\"user32.dll\")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);'; $type = Add-Type -MemberDefinition $signature -Name Win32ShowWindow -Namespace Win32Functions -PassThru; $hwnd = (Get-Process -Id $PID).MainWindowHandle; if($hwnd -ne [IntPtr]::Zero) { $type::ShowWindow($hwnd, 6) }" 2>nul

REM Mantener el script corriendo en segundo plano
:loop
timeout /t 60 /nobreak >nul
goto loop
