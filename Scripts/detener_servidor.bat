@echo off
chcp 65001 >nul
title Detener Sistema de Temperaturas
color 0C
echo.
echo ════════════════════════════════════════════════════
echo   DETENIENDO SERVICIOS
echo ════════════════════════════════════════════════════
echo.

echo Buscando procesos...
echo.

REM Detener servidor Rust
echo [1/2] Deteniendo servidor Rust...
for /f "tokens=5" %%a in ('netstat -ano ^| findstr :3000 ^| findstr LISTENING') do (
    taskkill /PID %%a /F
    echo Proceso %%a detenido.
)

REM Detener ngrok
echo [2/2] Deteniendo ngrok...
tasklist | findstr ngrok.exe >nul
if %errorlevel% equ 0 (
    taskkill /IM ngrok.exe /F
    echo ngrok detenido.
) else (
    echo ngrok no estaba corriendo.
)

echo.
echo ════════════════════════════════════════════════════
echo   SERVICIOS DETENIDOS
echo ════════════════════════════════════════════════════
echo.
pause