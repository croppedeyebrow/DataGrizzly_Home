# Checks the M0-01 route table for duplicate and forbidden paths.
# Exit 0: the table is consistent.
# Exit 1: a path collision or a forbidden route is present.

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$routeFile = Join-Path (Join-Path $PSScriptRoot '..') 'docs\contracts\routes.md'
if (-not (Test-Path -LiteralPath $routeFile)) {
    Write-Output 'FAIL missing docs/contracts/routes.md'
    exit 1
}

$lines = Get-Content -LiteralPath $routeFile -Encoding UTF8
$inTable = $false
$rows = @()

foreach ($line in $lines) {
    if ($line -eq '<!-- route-table -->') {
        $inTable = $true
        continue
    }
    if ($inTable -and $line.StartsWith('## ')) {
        break
    }
    if (-not $inTable -or -not $line.StartsWith('|')) {
        continue
    }

    $parts = $line.Trim().Trim('|').Split('|')
    $id = $parts[0].Trim()
    if ($id -eq 'ID' -or $id.StartsWith('-')) {
        continue
    }
    if ($parts.Length -lt 2) {
        Write-Output "FAIL row has no path: $line"
        exit 1
    }

    $path = $parts[1].Trim()
    if ($path.EndsWith('/') -and $path.Length -gt 1) {
        $path = $path.TrimEnd('/')
    }
    $rows += [pscustomobject]@{ Id = $id; Path = $path }
}

if ($rows.Count -eq 0) {
    Write-Output 'FAIL route table is empty'
    exit 1
}

$duplicates = $rows | Group-Object -Property Path | Where-Object { $_.Count -gt 1 }
if ($duplicates) {
    foreach ($duplicate in $duplicates) {
        $ids = ($duplicate.Group | ForEach-Object { $_.Id }) -join ','
        Write-Output "FAIL duplicate path $($duplicate.Name) ids=$ids"
    }
    exit 1
}

$forbidden = @('/lab', '/labs', '/experiments', '/signup', '/register', '/api/ask')
foreach ($row in $rows) {
    if ($forbidden -contains $row.Path) {
        Write-Output "FAIL forbidden path $($row.Path)"
        exit 1
    }
    $isAdmin = $row.Path -eq '/admin' -or $row.Path.StartsWith('/admin/')
    if ($row.Id.StartsWith('A') -and -not $isAdmin) {
        Write-Output "FAIL admin id $($row.Id) is outside /admin"
        exit 1
    }
    if ($row.Id.StartsWith('P') -and $isAdmin) {
        Write-Output "FAIL public id $($row.Id) uses /admin"
        exit 1
    }
}

$required = @(
    '/',
    '/projects/{slug}',
    '/projects/{slug}/logs',
    '/logs/{slug}',
    '/notes/{slug}',
    '/admin',
    '/admin/login',
    '/admin/imports/new'
)
$present = @{}
foreach ($row in $rows) {
    $present[$row.Path] = $true
}
foreach ($path in $required) {
    if (-not $present.ContainsKey($path)) {
        Write-Output "FAIL missing required path $path"
        exit 1
    }
}

Write-Output ("PASS route-table count=" + $rows.Count)
exit 0
