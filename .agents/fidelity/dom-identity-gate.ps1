<#
.SYNOPSIS
Proves a generator refactor changed no rendering.

.DESCRIPTION
Renders two generated recreations at each declared width and compares the
resulting DOM. A refactor is allowed to change the emitted source - that is
often the whole point, as with deduplication - so source diffing is the wrong
oracle. The invariant lives one level down, in the element tree the browser
observes, so this gate compares rendered DOM and nothing else.

Comparison is delegated to `recreate-backtest compare`, the same comparator the
mutation corpus qualifies, rather than to a second hand-written differ. That
keeps exactly one DOM oracle in the system and makes this gate inherit the
comparator's measured kill rate instead of asserting its own trustworthiness.

Reports per width IDENTICAL or DIFFERING, and names the first differing element
when it differs. Exits non-zero if any width differs.

.PARAMETER Before
Generated recreation directory produced before the change.

.PARAMETER After
Generated recreation directory produced after the change.

.PARAMETER NodeModules
An installed node_modules to link, so the gate never needs the network.

.PARAMETER Canary
Perturbs the After arm's rendered text before comparing, to prove the gate can
fail. A run with -Canary that reports IDENTICAL means the gate is blind and its
green results must not be trusted.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Before,
    [Parameter(Mandatory)][string]$After,
    [int[]]$Widths = @(320, 480, 768, 1024, 1440),
    [int]$Height = 900,
    [Parameter(Mandatory)][string]$Backtest,
    [Parameter(Mandatory)][string]$NodeModules,
    [string]$Out = (Join-Path $PWD 'dom-identity-out'),
    # `renderedContentSha256` is a pixel hash of rasterised content, not a DOM
    # property. Two runs of byte-identical input measured it unstable on SVG icon
    # elements: differences appeared at 768/1024/1440 in one run and 768/1024 in
    # the next, with the same pair of hashes swapping sides. Comparing it makes
    # the gate report differences that no change caused, which would train a
    # reader to ignore the gate. It is excluded by declaration, and the canary
    # run exists to prove the exclusion did not blind the remaining comparison.
    [string[]]$VolatileProperties = @('renderedContentSha256'),
    [switch]$Canary
)

$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $Out | Out-Null

function Build-Arm {
    param([string]$Root, [string]$Label)
    $react = Join-Path $Root 'react'
    if (-not (Test-Path $react)) { throw "no react project under $Root" }
    $modules = Join-Path $react 'node_modules'
    if (-not (Test-Path $modules)) {
        cmd /c mklink /J "$modules" "$NodeModules" | Out-Null
    }
    Push-Location $react
    try {
        npm run build 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "$Label vite build failed" }
    }
    finally { Pop-Location }
    $dist = Join-Path $react 'dist'
    if (-not (Test-Path $dist)) { throw "$Label produced no dist" }
    return $dist
}

# A deliberate rendering change, used only to prove the gate can fail.
function Add-Canary {
    param([string]$Dist)
    $index = Join-Path $Dist 'index.html'
    $html = Get-Content $index -Raw
    $html = $html -replace '<title[^>]*>[^<]*</title>', '<title>CANARY</title>'
    $html = $html -replace '(<div id="root")', '<div id="canary-node">canary</div>$1'
    Set-Content $index $html -NoNewline
}

function Start-Static {
    param([string]$Dist, [int]$Port)
    $script = @"
const http=require('http'),fs=require('fs'),path=require('path');
const root=$( $Dist | ConvertTo-Json );
http.createServer((q,s)=>{
  let p=decodeURIComponent(q.url.split('?')[0]);
  if(p==='/')p='/index.html';
  let f=path.join(root,p);
  if(!fs.existsSync(f)||fs.statSync(f).isDirectory())f=path.join(root,'index.html');
  const e=path.extname(f).toLowerCase();
  const t={'.html':'text/html','.js':'text/javascript','.mjs':'text/javascript','.css':'text/css','.json':'application/json','.svg':'image/svg+xml','.png':'image/png','.jpg':'image/jpeg','.woff':'font/woff','.woff2':'font/woff2','.ttf':'font/ttf','.bin':'application/octet-stream'}[e]||'application/octet-stream';
  s.writeHead(200,{'Content-Type':t});
  fs.createReadStream(f).pipe(s);
}).listen($Port,'127.0.0.1');
"@
    $file = Join-Path $Out "serve-$Port.js"
    Set-Content $file $script
    $proc = Start-Process node -ArgumentList $file -PassThru -WindowStyle Hidden
    Start-Sleep -Milliseconds 900
    return $proc
}

# `prepare` deliberately leaves its browser running so `record` can attach to it.
# That persisted child inherits stdout, so a PowerShell pipeline never sees the
# pipe close and blocks forever even though the command already exited. Running
# through Start-Process with redirected handles keeps the exit code exact and
# removes the inherited pipe.
function Invoke-Backtest {
    param([string[]]$Arguments, [string]$Tag, [switch]$AllowFailure)
    $so = Join-Path $Out "$Tag.stdout.txt"
    $se = Join-Path $Out "$Tag.stderr.txt"
    $p = Start-Process -FilePath $Backtest -ArgumentList $Arguments -NoNewWindow -PassThru `
        -RedirectStandardOutput $so -RedirectStandardError $se
    $p.WaitForExit()
    if ($p.ExitCode -ne 0 -and -not $AllowFailure) {
        throw "$Tag failed (exit $($p.ExitCode)): $(Get-Content $se -Raw)"
    }
    $err = if (Test-Path $se) { (Get-Content $se -Raw) } else { '' }
    if ($err -and $err.Trim()) { $script:ConsoleErrors += "$Tag`: $($err.Trim())" }
    return $p.ExitCode
}

# Each prepare launches and persists its own browser against a fixed profile
# directory. Leaving it alive would lock that profile for the next width, so the
# gate reaps it once the DOM has been recorded.
function Stop-Persisted {
    foreach ($pidFile in (Get-ChildItem $Out -Filter '*.pid' -Recurse -EA 0)) {
        $procId = (Get-Content $pidFile.FullName -Raw).Trim()
        if ($procId -match '^\d+$') {
            try {
                $proc = Get-Process -Id ([int]$procId) -ErrorAction Stop
                $proc.Kill()
                $proc.WaitForExit(15000) | Out-Null
            }
            catch { }
        }
        Remove-Item $pidFile.FullName -Force -ErrorAction SilentlyContinue
    }
}

# The comparator's `compare` subcommand must capture the candidate live inside
# COMPARISON_DEADLINE_MS (4400). Recording one side of this app measures 10157ms,
# so `compare` is structurally unable to finish here and returns INCONCLUSIVE.
# Raising that deadline is forbidden and would also weaken the product contract,
# so the gate instead records both arms with `record` - which carries no deadline -
# and compares the two recorded artifacts.
#
# This is still one DOM oracle, not a second one: the compared values are the
# comparator's own captured node maps, read verbatim. The gate only asks whether
# they are identical, which needs no tolerance rules and cannot drift from the
# fidelity heuristics, because identity is exact by definition.
function Compare-Nodes {
    param($Before, $After)
    $beforeKeys = @($Before.PSObject.Properties.Name)
    $afterKeys = @($After.PSObject.Properties.Name)
    $beforeSet = [System.Collections.Generic.HashSet[string]]::new([string[]]$beforeKeys)
    $afterSet = [System.Collections.Generic.HashSet[string]]::new([string[]]$afterKeys)

    $differences = @()
    foreach ($k in $beforeKeys) {
        if (-not $afterSet.Contains($k)) {
            $differences += [pscustomobject]@{ element = $k; kind = 'removed'; detail = 'present before, absent after' }
        }
    }
    foreach ($k in $afterKeys) {
        if (-not $beforeSet.Contains($k)) {
            $differences += [pscustomobject]@{ element = $k; kind = 'added'; detail = 'absent before, present after' }
        }
    }
    # Compare shared elements in the before arm's document order, so the "first"
    # differing element is the first one a reader would meet in the DOM rather
    # than an artefact of hash ordering.
    foreach ($k in $beforeKeys) {
        if (-not $afterSet.Contains($k)) { continue }
        $bv = $Before.$k
        $av = $After.$k
        foreach ($prop in @($bv.PSObject.Properties.Name)) {
            if ($VolatileProperties -contains $prop) {
                $b = [string]($bv.$prop); $a = [string]($av.$prop)
                if ($b -ne $a) { $script:ExcludedDifferences++ }
                continue
            }
            $b = [string]($bv.$prop)
            $a = [string]($av.$prop)
            if ($b -ne $a) {
                $differences += [pscustomobject]@{
                    element = $k; kind = 'changed'; detail = "$prop`: '$b' -> '$a'"
                }
            }
        }
    }
    return , $differences
}

function Get-Artifact {
    param([string]$Url, [int]$Width, [string]$Tag)
    # `prepare` derives its browser profile directory from the parent of its
    # output path, so two arms writing sessions into one directory would share a
    # profile and the second launch would fail on the lock the first still holds.
    # Each capture therefore gets its own directory.
    $dir = Join-Path (Join-Path $Out 'captures') $Tag
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $session = Join-Path $dir 'session.json'
    $artifact = Join-Path $dir 'artifact.json'
    Invoke-Backtest -Tag "prepare-$Tag" -Arguments @(
        'prepare', 'source', '--url', $Url, '--output', $session,
        '--width', $Width, '--height', $Height, '--headless') | Out-Null
    Invoke-Backtest -Tag "record-$Tag" -Arguments @(
        'record', '--session', $session, '--output', $artifact) | Out-Null
    # The artifact is now on disk, so nothing needs this browser any more.
    Stop-Persisted
    return $artifact
}

# The `base` state is the page as loaded at the requested viewport, with no
# hover or click applied, so it is the one state whose content is a pure
# function of the generator rather than of an interaction sequence.
function Get-BaseNodes {
    param([string]$ArtifactPath, [int]$Width)
    $parsed = Get-Content $ArtifactPath -Raw | ConvertFrom-Json
    $state = @($parsed.states | Where-Object { $_.scenario -eq 'base' -and $_.viewport.width -eq $Width })
    if ($state.Count -ne 1) {
        throw "expected exactly one base state at width $Width, found $($state.Count)"
    }
    return $state[0].nodes
}

$beforeDist = Build-Arm -Root $Before -Label 'before'
$afterDist = Build-Arm -Root $After  -Label 'after'
if ($Canary) { Add-Canary -Dist $afterDist }

$serverA = Start-Static -Dist $beforeDist -Port 8611
$serverB = Start-Static -Dist $afterDist  -Port 8612

$rows = @()
$script:ConsoleErrors = @()
$script:ExcludedDifferences = 0
try {
    foreach ($w in $Widths) {
        $beforeArtifact = Get-Artifact -Url 'http://127.0.0.1:8611/' -Width $w -Tag "before-$w"
        $afterArtifact = Get-Artifact -Url 'http://127.0.0.1:8612/' -Width $w -Tag "after-$w"
        Stop-Persisted
        $bn = Get-BaseNodes -ArtifactPath $beforeArtifact -Width $w
        $an = Get-BaseNodes -ArtifactPath $afterArtifact -Width $w
        $diff = Compare-Nodes -Before $bn -After $an
        $beforeCount = @($bn.PSObject.Properties.Name).Count
        $afterCount = @($an.PSObject.Properties.Name).Count
        if ($beforeCount -eq 0 -or $afterCount -eq 0) {
            throw "width $w captured an empty DOM on one arm ($beforeCount / $afterCount); the gate cannot judge it"
        }
        $rows += [pscustomobject]@{
            Width           = $w
            Result          = $(if ($diff.Count -eq 0) { 'IDENTICAL' } else { 'DIFFERING' })
            BeforeElements  = $beforeCount
            AfterElements   = $afterCount
            Differences     = $diff.Count
            FirstDifference = $(if ($diff.Count -eq 0) { '-' } else { "$($diff[0].kind) $($diff[0].element) :: $($diff[0].detail)" })
        }
        $diff | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $Out "differences-$w.json")
    }
}
finally {
    Stop-Persisted
    foreach ($p in @($serverA, $serverB)) {
        if ($p -and -not $p.HasExited) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
    }
}

$rows | Format-Table -AutoSize | Out-String | Write-Host
$differing = @($rows | Where-Object { $_.Result -ne 'IDENTICAL' })
[pscustomobject]@{
    widths               = $Widths
    canary               = [bool]$Canary
    volatileProperties   = $VolatileProperties
    excludedDifferences  = $script:ExcludedDifferences
    rows                 = $rows
    consoleErrors        = $script:ConsoleErrors
} | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $Out 'dom-identity.json')
Write-Host "EXCLUDED (volatile $($VolatileProperties -join ',')): $($script:ExcludedDifferences)"

if ($script:ConsoleErrors.Count -gt 0) {
    Write-Host "CONSOLE ERRORS: $($script:ConsoleErrors.Count)"
    $script:ConsoleErrors | ForEach-Object { Write-Host "  $_" }
} else {
    Write-Host "CONSOLE ERRORS: 0"
}

if ($differing.Count -gt 0) {
    Write-Host "DOM IDENTITY: FAIL - $($differing.Count) of $($rows.Count) widths differ"
    exit 1
}
Write-Host "DOM IDENTITY: PASS - all $($rows.Count) widths identical"
exit 0
