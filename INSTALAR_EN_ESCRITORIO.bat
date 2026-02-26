@echo off
title Instalando Acceso Directo...
color 0B
cls

echo.
echo  Creando acceso directo en tu Escritorio...
echo.

REM Ejecutar el script de PowerShell
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0Scripts\crear_shortcut.ps1"

echo.
echo  Presiona cualquier tecla para salir.
pause >nul