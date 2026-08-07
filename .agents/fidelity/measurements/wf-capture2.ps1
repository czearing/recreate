$dir  = "C:\Users\calebzearing\recreate\.agents\fidelity\measurements"
$root = "$dir\wf"
$exe  = "$dir\wf-target\release\recreate.exe"
$edge = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"

$sites = @(
  @{k='wikipedia'; u='https://en.wikipedia.org/wiki/CSS'},
  @{k='sveltedev'; u='https://svelte.dev/'},
  @{k='vuejs';     u='https://vuejs.org/'},
  @{k='danluu';    u='https://danluu.com/'},
  @{k='nprtext';   u='https://text.npr.org/'}
)

function Done($k) { Test-Path "$root\$k\react\src\App.jsx" }

function Ensure-Browser($prof) {
  try { $r = Invoke-WebRequest http://127.0.0.1:9377/json/version -UseBasicParsing -TimeoutSec 4; if ($r.StatusCode -eq 200) { return } } catch {}
  Start-Process $edge -ArgumentList "--headless=new","--remote-debugging-port=9377","--user-data-dir=$root\p-$prof","--no-first-run","--disable-gpu","--hide-scrollbars","--disable-dev-shm-usage","about:blank"
  Start-Sleep 10
}

foreach ($s in $sites) {
  if (Done $s.k) { Write-Output "$($s.k) ALREADY OK"; continue }
  for ($a = 1; $a -le 2; $a++) {
    if (Done $s.k) { break }
    Ensure-Browser "$($s.k)$a"
    Write-Output "### capture $($s.k) attempt $a"
    & $exe capture $s.u --out "$root\$($s.k)" --cdp-url "http://127.0.0.1:9377" 2>&1 |
      Select-Object -Last 6 | ForEach-Object { "   $_" } | Tee-Object -FilePath "$root\err-$($s.k).txt" -Append
    if (-not (Done $s.k)) {
      Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" |
        Where-Object { $_.CommandLine -like "*measurements\wf*" } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
      Start-Sleep 4
    }
  }
  Write-Output "$($s.k) => $(if (Done $s.k) { 'OK' } else { 'FAILED' })"
}
Write-Output "=== done ==="
