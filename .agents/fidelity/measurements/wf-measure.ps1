$dir  = "C:\Users\calebzearing\recreate\.agents\fidelity\measurements"
$root = "$dir\wf"
$edge = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"

$sites = @(
  @{k='reactdev';  u='https://react.dev/'; p=8841},
  @{k='sveltedev'; u='https://svelte.dev/'; p=8842},
  @{k='vuejs';     u='https://vuejs.org/'; p=8843},
  @{k='cern';      u='https://info.cern.ch/hypertext/WWW/TheProject.html'; p=8844},
  @{k='danluu';    u='https://danluu.com/'; p=8845},
  @{k='nprtext';   u='https://text.npr.org/'; p=8846}
)

function Ensure-Browser($prof) {
  try { $r = Invoke-WebRequest http://127.0.0.1:9388/json/version -UseBasicParsing -TimeoutSec 4; if ($r.StatusCode -eq 200) { return } } catch {}
  Start-Process $edge -ArgumentList "--headless=new","--remote-debugging-port=9388","--user-data-dir=$root\m-$prof","--no-first-run","--disable-gpu","--hide-scrollbars","--disable-dev-shm-usage","about:blank"
  Start-Sleep 10
}

foreach ($s in $sites) {
  $out = "$root\raw-$($s.k).json"
  if (Test-Path $out) { Write-Output "$($s.k) ALREADY MEASURED"; continue }
  if (-not (Test-Path "$root\$($s.k)\react\dist\index.html")) { Write-Output "$($s.k) NO DIST"; continue }
  for ($a = 1; $a -le 2; $a++) {
    if (Test-Path $out) { break }
    Ensure-Browser "$($s.k)$a"
    Write-Output "### measure $($s.k) attempt $a"
    (@{key=$s.k; sourceUrl=$s.u; recreationUrl="http://127.0.0.1:$($s.p)/"} | ConvertTo-Json -Compress) |
      Out-File -Encoding utf8 "$root\_site-$($s.k).json"
    Push-Location $dir
    node overlap-property-census.mjs http://127.0.0.1:9388 "$root\_site-$($s.k).json" $out 2>&1 | Select-Object -First 8 | ForEach-Object { "   $_" }
    Pop-Location
    if (-not (Test-Path $out)) {
      Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" |
        Where-Object { $_.CommandLine -like "*measurements\wf*" } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
      Start-Sleep 4
    }
  }
  Write-Output "$($s.k) => $(if (Test-Path $out) { 'MEASURED' } else { 'FAILED' })"
}
Write-Output "=== measure done ==="
