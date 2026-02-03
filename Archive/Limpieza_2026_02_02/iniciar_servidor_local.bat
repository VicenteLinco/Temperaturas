@echo off
setlocal

:: Asegurar que estamos en la carpeta raiz del proyecto
cd /d "%~dp0"
cd ..

echo ==========================================
echo   SISTEMA DE TEMPERATURAS - MODO LOCAL
echo ==========================================
echo Directorio actual: %CD%
echo.

if not exist "Cargo.toml" (
    echo [ERROR] No se encontro Cargo.toml en %CD%
    echo Asegurate de que el script este dentro de la carpeta 'Scripts'.
    pause
    exit /b 1
)

echo [1/2] Liberando puerto 3000...
powershell -Command "Get-NetTCPConnection -LocalPort 3000 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }"

where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] No se encontro 'cargo'. Por favor, instala Rust desde https://rustup.rs/
    pause
    exit /b 1
)

echo [2/2] Iniciando servidor Rust...
echo (Si es la primera vez, la compilacion puede tardar un momento)
echo.
echo >>> El sistema estara disponible en: http://localhost:3000
echo >>> Presiona Ctrl+C para detener el servidor.
echo.
cargo run --color always
if %errorlevel% neq 0 (
    echo.
    echo [ERROR] El servidor se detuvo o no pudo compilar. Revisa los mensajes arriba.
)
pause
pause