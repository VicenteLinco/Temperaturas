@echo off
REM -- AUTO-ELEVACION A ADMINISTRADOR --
>nul 2>&1 "%SYSTEMROOT%\system32\cacls.exe" "%SYSTEMROOT%\system32\config\system"
if '%errorlevel%' NEQ '0' (
    echo Solicitando permisos de Administrador...
    goto UACPrompt
) else ( goto gotAdmin )

:UACPrompt
    echo Set UAC = CreateObject^("Shell.Application"^) > "%temp%\getadmin.vbs"
    echo UAC.ShellExecute "%~s0", "", "", "runas", 1 >> "%temp%\getadmin.vbs"
    "%temp%\getadmin.vbs"
    exit /B

:gotAdmin
    if exist "%temp%\getadmin.vbs" ( del "%temp%\getadmin.vbs" )
    pushd "%CD%"
    CD /D "%~dp0"
REM --------------------------------------

title Sistema de Temperaturas
color 0A
cls

echo ========================================================
echo   SISTEMA DE GESTION DE TEMPERATURAS
echo ========================================================
echo.

REM --- 1. LIMPIEZA INTELIGENTE ---
echo [1/4] Liberando puertos...

:CHECK_PORT
netstat -ano | findstr :3000 | findstr LISTENING >nul
if %errorlevel% equ 0 (
    echo     - Puerto 3000 ocupado. Intentando liberar...
    
    REM Matar por nombre
    taskkill /F /IM "sistema-temperaturas.exe" >nul 2>&1
    
    REM Matar por PID específico
    for /f "tokens=5" %%a in ('netstat -ano ^| findstr :3000 ^| findstr LISTENING') do taskkill /PID %%a /F >nul 2>&1
    
    echo     - Esperando 2 segundos...
    timeout /t 2 /nobreak >nul
    
    REM Volver a comprobar
    goto :CHECK_PORT
)

echo     - Puerto 3000 LIBRE.

taskkill /F /IM "ngrok.exe" >nul 2>&1
taskkill /F /IM "cargo.exe" >nul 2>&1

REM --- 2. RED Y SEGURIDAD ---
set "NGROK_TOKEN="
if exist "%~dp0..\.env" for /f "tokens=2 delims==" %%a in ('findstr "NGROK_AUTHTOKEN=" "%~dp0..\.env"') do set "NGROK_TOKEN=%%a"
if "%NGROK_TOKEN%"=="" if exist "%~dp0token.txt" set /p NGROK_TOKEN=<"%~dp0token.txt"

echo [2/4] Configurando red...
if not "%NGROK_TOKEN%"=="" (
    echo     - Iniciando Ngrok...
    start "Ngrok Tunnel" /MIN "%~dp0ngrok.exe" http 3000
    echo     - Seguridad: Acceso remoto se cortara en 40 min.
    start /B powershell -Command "Start-Sleep -Seconds 2400; Stop-Process -Name ngrok -ErrorAction SilentlyContinue"
    set "MODO=REMOTO"
) else (
    echo     - Modo LOCAL.
    set "MODO=LOCAL"
)

REM --- 3. EJECUCION DIRECTA ---
echo [3/4] Iniciando...
cd /d "%~dp0.."

set EXE_PATH="target\debug\sistema-temperaturas.exe"

if exist %EXE_PATH% (
    echo     - Ejecutable encontrado. Iniciando...
    %EXE_PATH%
) else (
    echo     - No se encontro ejecutable. Compilando...
    cargo run
)

echo.
echo ========================================================
echo  SERVIDOR DETENIDO
echo ========================================================
echo.
pause