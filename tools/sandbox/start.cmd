@echo off
rem Runs inside Windows Sandbox at logon (see run-sandbox.ps1).
rem C:\ortelia holds ortelia.exe (read-only), C:\sandbox is this folder (read-write).

rem Turn off Smart App Control so that the unsigned executable can run.
reg add "HKLM\SYSTEM\CurrentControlSet\Control\CI\Policy" /v VerifiedAndReputablePolicyState /t REG_DWORD /d 0 /f
rem CiTool waits for Enter: feed it one, there is no keyboard at logon.
echo.| CiTool.exe -r

rem Write the log into the shared folder so that it can be read from the host.
set ORTELIA_LOG_DIR=C:\sandbox\logs
set ORTELIA_LOG=debug
start "" C:\ortelia\ortelia.exe
