# Tests the Windows build of Ortelia in Windows Sandbox, in one command:
#   1. finds the CI run that built the branch (the last pushed commit), or offers to start one;
#   2. downloads ortelia.exe from that run (kept in tools\sandbox\build\<run id>);
#   3. copies the application into tools\sandbox\app, shared with the sandbox;
#   4. opens the sandbox, where Ortelia starts on that application (run-sandbox.ps1, start.cmd);
#   5. once the test is over, prints what the log says about it and the session file.
#
# Usage (from the repository root):
#   powershell -ExecutionPolicy Bypass -File tools\sandbox\test-windows.ps1 [options]
#     -Branch <name>  branch whose build is tested (default: the current branch)
#     -RunId <id>     CI run to take the executable from, instead of -Branch
#     -App <folder>   application to open, relative to the repository root
#                     (default: examples\annuaire)
#     -ResetSession   forget the session state left by the previous test (app.session.json)
#     -NoWait         do not wait for the end of the test (no log summary)
#
# In the sandbox, the "Ortelia" file on the desktop starts Ortelia again: close the window,
# then start it again to check that the session state is restored.
param(
    [string]$Branch,
    [long]$RunId,
    [string]$App = 'examples\annuaire',
    [switch]$ResetSession,
    [switch]$NoWait
)

$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
$repo = (Resolve-Path (Join-Path $here '..\..')).Path
$artifact = 'ortelia-windows-x86_64'

# Runs a native command and fails on a non-zero exit code (PowerShell does not, by itself).
function Invoke-Native {
    param([string]$Exe, [string[]]$Arguments)
    $output = & $Exe @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "'$Exe $($Arguments -join ' ')' failed (exit code $LASTEXITCODE)"
    }
    $output
}

# Parses the JSON output of gh (an array of lines in PowerShell). Windows PowerShell 5.1 emits a
# JSON array as one object: ForEach-Object enumerates its elements, as PowerShell 7 does.
function ConvertFrom-GhJson {
    param([object[]]$Lines)
    @(($Lines -join "`n") | ConvertFrom-Json | ForEach-Object { $_ })
}

# CI runs of the branch for one commit, newest first; pull request runs build no binaries.
function Get-BuildRuns {
    param([string]$Branch, [string]$Sha)
    $json = Invoke-Native gh @('run', 'list', '--workflow', 'ci.yml', '--branch', $Branch,
        '--limit', '50', '--json', 'databaseId,headSha,event,status,conclusion')
    @(ConvertFrom-GhJson $json | Where-Object { $_.event -ne 'pull_request' -and $_.headSha -eq $Sha })
}

function Confirm-Choice {
    param([string]$Question)
    (Read-Host "$Question [y/N]") -match '^(y|yes|o|oui)$'
}

Push-Location $repo
try {
    # 1. The CI run.
    if ($RunId) {
        $run = (ConvertFrom-GhJson (Invoke-Native gh @('run', 'view', $RunId, '--json',
                    'headSha,headBranch,event,status,conclusion')))[0]
        Write-Host "CI run $RunId : branch $($run.headBranch), commit $($run.headSha.Substring(0, 7))"
        if ($run.event -eq 'pull_request') {
            throw "run $RunId was started by a pull request: it built no executable"
        }
        if ($run.status -ne 'completed') {
            Invoke-Native gh @('run', 'watch', $RunId, '--exit-status') | Out-Host
        } elseif ($run.conclusion -ne 'success') {
            throw "run $RunId ended with '$($run.conclusion)'"
        }
    } else {
        if (-not $Branch) {
            $Branch = (Invoke-Native git @('rev-parse', '--abbrev-ref', 'HEAD')).Trim()
        }
        Invoke-Native git @('fetch', '--quiet', 'origin', $Branch) | Out-Null
        $sha = (Invoke-Native git @('rev-parse', "origin/$Branch")).Trim()
        $short = $sha.Substring(0, 7)
        Write-Host "Branch $Branch, last pushed commit: $short"
        $head = (Invoke-Native git @('rev-parse', 'HEAD')).Trim()
        $current = (Invoke-Native git @('rev-parse', '--abbrev-ref', 'HEAD')).Trim()
        if ($current -eq $Branch -and $head -ne $sha) {
            Write-Warning "the local branch differs from origin/$Branch (commits not pushed?): commit $short is tested"
        }

        $runs = Get-BuildRuns $Branch $sha
        $done = $runs | Where-Object { $_.status -eq 'completed' -and $_.conclusion -eq 'success' } |
            Select-Object -First 1
        $running = $runs | Where-Object { $_.status -ne 'completed' } | Select-Object -First 1
        if ($done) {
            $RunId = $done.databaseId
            Write-Host "CI run $RunId built commit $short"
        } elseif ($running) {
            $RunId = $running.databaseId
            Write-Host "CI run $RunId is building commit ${short}: waiting for its end"
            Invoke-Native gh @('run', 'watch', $RunId, '--exit-status') | Out-Host
        } else {
            Write-Host "No CI run has built commit $short (pull request runs build no executable)."
            if (-not (Confirm-Choice "Start the CI on $Branch (about 15 minutes)?")) {
                Write-Host 'Nothing to test.'
                exit 1
            }
            # The new run is the one that did not exist before the dispatch.
            $before = @(Get-BuildRuns $Branch $sha | ForEach-Object { $_.databaseId })
            Invoke-Native gh @('workflow', 'run', 'ci.yml', '--ref', $Branch) | Out-Host
            for ($i = 0; $i -lt 30 -and -not $RunId; $i++) {
                Start-Sleep -Seconds 3
                $new = Get-BuildRuns $Branch $sha | Where-Object {
                    $_.event -eq 'workflow_dispatch' -and $before -notcontains $_.databaseId
                } | Select-Object -First 1
                if ($new) { $RunId = $new.databaseId }
            }
            if (-not $RunId) { throw 'the CI run started does not show up in gh run list' }
            Write-Host "CI run $RunId started: waiting for its end"
            Invoke-Native gh @('run', 'watch', $RunId, '--exit-status') | Out-Host
        }
    }

    # 2. The executable.
    $build = Join-Path $here "build\$RunId"
    if (Test-Path (Join-Path $build 'ortelia.exe')) {
        Write-Host "Executable already downloaded: $build"
    } else {
        Write-Host "Downloading $artifact from run $RunId"
        & gh run download $RunId -n $artifact -D $build
        if ($LASTEXITCODE -ne 0) {
            throw "download failed: the artifact may have expired (30 days); start the CI again"
        }
    }

    # 3. The application, copied without its session file: the sandbox keeps its own.
    $appSource = Join-Path $repo $App
    if (-not (Test-Path (Join-Path $appSource 'app.json'))) {
        throw "no app.json in $appSource"
    }
    $appCopy = Join-Path $here 'app'
    New-Item -ItemType Directory -Force $appCopy | Out-Null
    Get-ChildItem $appCopy | Where-Object { $ResetSession -or $_.Name -notlike '*.session.json' } |
        Remove-Item -Recurse -Force
    Get-ChildItem $appSource | Where-Object { $_.Name -notlike '*.session.json' } |
        Copy-Item -Destination $appCopy -Recurse -Force
    $sessionFile = Join-Path $appCopy 'app.session.json'
    if (Test-Path $sessionFile) {
        Write-Host "Session state of the previous test kept (-ResetSession to forget it)"
    }

    # 4. The sandbox.
    $open = Get-Process -Name WindowsSandbox, WindowsSandboxClient, WindowsSandboxRemoteSession `
        -ErrorAction SilentlyContinue
    if ($open -and -not (Confirm-Choice 'A sandbox seems to be open, and only one can run: continue anyway?')) {
        exit 1
    }
    $logFile = Join-Path $here ("logs\ortelia.{0}.log" -f (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd'))
    $logStart = 0
    if (Test-Path $logFile) { $logStart = @(Get-Content $logFile).Count }
    & (Join-Path $here 'run-sandbox.ps1') $build
    Write-Host 'The sandbox starts in one or two minutes, then opens Ortelia (software rendering: slow).'
    Write-Host 'The "Ortelia" file on its desktop starts Ortelia again.'
    Write-Host 'Closing the sandbox erases its content; the log stays in tools\sandbox\logs.'
    if ($NoWait) { exit 0 }

    # 5. The summary, once the user is done.
    Read-Host 'Press Enter once the test is over (the sandbox can stay open)' | Out-Null
    $lines = @()
    if (Test-Path $logFile) { $lines = @(Get-Content $logFile | Select-Object -Skip $logStart) }
    Write-Host ''
    Write-Host "=== log of this test: $logFile"
    $starts = @($lines | Where-Object { $_ -match ' INFO ortelia: starting Ortelia' }).Count
    $shown = @($lines | Where-Object { $_ -match 'main window shown' }).Count
    Write-Host "Ortelia started $starts time(s), main window shown $shown time(s)"
    Write-Host ''
    Write-Host '=== warnings and errors'
    $problems = @($lines | Where-Object { $_ -match '^\S+\s+(WARN|ERROR)\s' })
    if ($problems) { $problems | ForEach-Object { Write-Host $_ } } else { Write-Host '(none)' }
    Write-Host ''
    Write-Host "=== session file: $sessionFile"
    if (Test-Path $sessionFile) { Get-Content $sessionFile | ForEach-Object { Write-Host $_ } }
    else { Write-Host '(absent: Ortelia was not closed normally yet)' }
} finally {
    Pop-Location
}
