# Starts ortelia.exe in Windows Sandbox.
#
# Usage: powershell -File tools\sandbox\run-sandbox.ps1 <folder containing ortelia.exe>
# Get the executable from the CI: gh run download <run id> -n ortelia-windows-x86_64 -D <folder>
#
# A .wsb file only accepts absolute paths, so it is generated here (sandbox.wsb, not versioned).
# Only one sandbox can run at a time: close any open one first.
param(
    [Parameter(Mandatory = $true)]
    [string]$ExeFolder
)

$ErrorActionPreference = 'Stop'
$exeFolder = (Resolve-Path $ExeFolder).Path
if (-not (Test-Path (Join-Path $exeFolder 'ortelia.exe'))) {
    throw "ortelia.exe not found in $exeFolder"
}
$here = $PSScriptRoot
New-Item -ItemType Directory -Force (Join-Path $here 'logs') | Out-Null

# vGPU is disabled: with it, ortelia.exe crashes in the sandbox's virtual graphics driver
# (access violation 0xC0000005). Without it, wgpu renders with WARP (software).
$wsb = @"
<Configuration>
  <vGPU>Disable</vGPU>
  <Networking>Disable</Networking>
  <MappedFolders>
    <MappedFolder>
      <HostFolder>$exeFolder</HostFolder>
      <SandboxFolder>C:\ortelia</SandboxFolder>
      <ReadOnly>true</ReadOnly>
    </MappedFolder>
    <MappedFolder>
      <HostFolder>$here</HostFolder>
      <SandboxFolder>C:\sandbox</SandboxFolder>
      <ReadOnly>false</ReadOnly>
    </MappedFolder>
  </MappedFolders>
  <LogonCommand>
    <Command>cmd.exe /c C:\sandbox\start.cmd</Command>
  </LogonCommand>
</Configuration>
"@
$wsbPath = Join-Path $here 'sandbox.wsb'
Set-Content -Path $wsbPath -Value $wsb -Encoding utf8
Start-Process $wsbPath
Write-Host "Sandbox started. Log: $(Join-Path $here 'logs')"
