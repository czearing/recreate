param(
  [int[]]$Counts,
  [string]$Tag = "run"
)
$ErrorActionPreference = "Continue"
$exe = "C:\Users\calebzearing\recreate\.agents\fidelity\measurements\ceiling-target\release\recreate.exe"
$base = "C:\Users\calebzearing\recreate\.agents\fidelity\measurements\ceiling"
$results = @()
foreach ($n in $Counts) {
  $out = Join-Path $base "out-$n"
  if (Test-Path $out) { Remove-Item -Recurse -Force $out }
  $url = "http://127.0.0.1:8811/n/$n"
  $log = Join-Path $base "cap-$n.log"
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $p = Start-Process -FilePath $exe -ArgumentList "capture", $url, "--out", $out, "--cdp-url", "http://127.0.0.1:9344", "--spec-only" -NoNewWindow -Wait -PassThru -RedirectStandardOutput $log -RedirectStandardError "$log.err"
  $sw.Stop()
  $spec = Join-Path $out "spec.json"
  $specBytes = if (Test-Path $spec) { (Get-Item $spec).Length } else { 0 }
  $files = if (Test-Path $out) { (Get-ChildItem -Recurse -File $out).Count } else { 0 }
  $err = if (Test-Path "$log.err") { (Get-Content "$log.err" -Raw) } else { "" }
  $msgSize = 0
  $cap = 0
  if ($err -match "Message too long: (\d+) > (\d+)") { $msgSize = [int64]$Matches[1]; $cap = [int64]$Matches[2] }
  $results += [pscustomobject]@{
    nodes = $n
    exitCode = $p.ExitCode
    ms = [int]$sw.Elapsed.TotalMilliseconds
    specBytes = $specBytes
    filesWritten = $files
    cdpMessageBytes = $msgSize
    cdpCapBytes = $cap
    errorText = ($err.Trim() -split "`n" | Select-Object -First 3) -join " | "
  }
  Write-Output ("n={0} exit={1} spec={2} files={3} cdpMsg={4} ms={5}" -f $n, $p.ExitCode, $specBytes, $files, $msgSize, $sw.Elapsed.TotalMilliseconds)
}
$results | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $base "ladder-$Tag.json")
