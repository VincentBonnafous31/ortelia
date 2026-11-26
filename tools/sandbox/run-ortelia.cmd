@echo off
rem Starts Ortelia inside Windows Sandbox (see start.cmd), with a debug log in the shared folder,
rem so that it can be read from the host. Copied to the sandbox desktop, as "Ortelia".
set ORTELIA_LOG_DIR=C:\sandbox\logs
set ORTELIA_LOG=debug
rem test-windows.ps1 copies the application to test into C:\sandbox\app.
if exist C:\sandbox\app\app.json (
    start "" C:\ortelia\ortelia.exe C:\sandbox\app
) else (
    start "" C:\ortelia\ortelia.exe
)
