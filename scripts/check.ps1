# Current-stage checks. Windows PowerShell, repository root.
# Exit 0: every executed check passed, including git diff --check.
# Exit 3: web install, build, and typecheck passed, but there is no Git repository.
#         git diff --check was not run and is not a pass.
# Any other exit code comes from the failing command.

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
# Keep Astro from opening an install prompt when a package cannot be resolved.
$env:CI = 'true'
Set-Location -LiteralPath (Join-Path $PSScriptRoot '..')

function Invoke-Native {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [string[]]$Command
    )

    Write-Output "CHECK $Name"
    $executable = $Command[0]
    $arguments = @()
    if ($Command.Length -gt 1) {
        $arguments = $Command[1..($Command.Length - 1)]
    }

    & $executable @arguments
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        Write-Output "FAIL $Name exit=$code"
        exit $code
    }

    Write-Output "PASS $Name"
}

Invoke-Native -Name 'yarn-install' -Command @('corepack', 'yarn', 'install', '--immutable')
Invoke-Native -Name 'web-build' -Command @('corepack', 'yarn', 'workspace', '@datagrizzly/web', 'build')
Invoke-Native -Name 'web-check' -Command @('corepack', 'yarn', 'workspace', '@datagrizzly/web', 'check')
Invoke-Native -Name 'env-classification' -Command @('node', 'scripts/check-m0-env.mjs')
Invoke-Native -Name 'content-fixtures' -Command @('node', 'scripts/check-m0-content.mjs')
Invoke-Native -Name 'layout-home' -Command @('node', 'scripts/check-m1-layout.mjs')
Invoke-Native -Name 'project-pages' -Command @('node', 'scripts/check-m1-projects.mjs')
Invoke-Native -Name 'research-pages' -Command @('node', 'scripts/check-m1-research.mjs')
Invoke-Native -Name 'seo-pages' -Command @('node', 'scripts/check-m1-seo.mjs')

$freeGiB = [math]::Round((Get-PSDrive -Name C).Free / 1GB, 2)
Write-Output "DISK_FREE_GIB=$freeGiB"

$gitDir = Join-Path (Get-Location) '.git'
if (-not (Test-Path -LiteralPath $gitDir)) {
    Write-Output 'GIT_CHECK=DEFERRED'
    Write-Output 'No Git repository. S03 and S04 remain deferred, so git diff --check was not run and is not a pass.'
    exit 3
}

Invoke-Native -Name 'git-diff-check' -Command @('git', 'diff', '--check')
Write-Output 'CHECK_OK'
exit 0
