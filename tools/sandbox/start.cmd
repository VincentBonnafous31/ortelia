@echo off
rem Runs inside Windows Sandbox at logon (see run-sandbox.ps1).
rem C:\ortelia holds ortelia.exe (read-only), C:\sandbox is this folder (read-write).

rem Turn off Smart App Control so that the unsigned executable can run.
reg add "HKLM\SYSTEM\CurrentControlSet\Control\CI\Policy" /v VerifiedAndReputablePolicyState /t REG_DWORD /d 0 /f
rem CiTool waits for Enter: feed it one, there is no keyboard at logon.
echo.| CiTool.exe -r

rem Start Ortelia, and copy its launcher to the desktop to start it again.
copy /y C:\sandbox\run-ortelia.cmd "%USERPROFILE%\Desktop\Ortelia.cmd"
call C:\sandbox\run-ortelia.cmd
