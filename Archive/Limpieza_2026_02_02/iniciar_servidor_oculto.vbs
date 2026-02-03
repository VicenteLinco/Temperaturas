Set WshShell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
strPath = fso.GetParentFolderName(WScript.ScriptFullName)
REM Ejecuta el script de bandeja de forma oculta (0)
WshShell.Run chr(34) & strPath & "\iniciar_servidor_bandeja.bat" & chr(34), 0
Set WshShell = Nothing