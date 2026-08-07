$dir  = "C:\Users\calebzearing\recreate\.agents\fidelity\measurements"
$root = "$dir\wf"
$exe  = "$dir\wf-target\release\recreate.exe"
$edge = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
New-Item -ItemType Directory -Force -Path $root | Out-Null

$sites = @(
  @{k='wikipedia'; u='https://en.wikipedia.org/wiki/CSS'},
  @{k='reactdev';  u='https://react.dev/'},
  @{k='sveltedev'; u='https://svelte.dev/'},
  @{k='vuejs';     u='https://vuejs.org/'},
  @{k='cern';      u='https://info.cern.ch/hypertext/WWW/TheProject.html'},
  @{k='danluu';    u='https://danluu.com/'},
  @{k='nprtext';   u='https://text.npr.org/'}
)

function Ensure-Browser($prof) {
  try { $r = Invoke-WebRequest http://127.0.0.1:9377/json/version -UseBasicParsing -TimeoutSec 4; if ($r.StatusCode -eq 200) { return } } catch {}
  Start-Process $edge -ArgumentList "--headless=new","--remote-debugging-port=9377","--user-data-dir=$root\cap-$prof","--no-first-run","--disable-gpu","--hide-scrollbars","--disable-dev-shm-usage","about:blank"
  Start-Sleep 9
}

foreach ($s in $sites) {
  $out = "$root\$($s.k)"
  if (Test-Path "$out\capture.json") { Write-Output "$($s.k) ALREADY CAPTURED"; continue }
  for ($a = 1; $a -le 3; $a++) {
    if (Test-Path "$out\capture.json") { break }
    Ensure-Browser "$($s.k)-$a"
    Write-Output "### capture $($s.k) attempt $a"
    & $exe capture $s.u --out $out --cdp-url "http://127.0.0.1:9377" 2>&1 | Select-Object -Last 4
    if (-not (Test-Path "$out\capture.json")) {
      Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" |
        Where-Object { $_.CommandLine -like "*$root*" } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
      Start-Sleep 4
    }
  }
  if (Test-Path "$out\capture.json") {
    $mb = [math]::Round((Get-Item "$out\capture.json").Length/1MB,2)
    Write-Output "$($s.k) CAPTURED ${mb} MB"
  } else { Write-Output "$($s.k) FAILED ALL ATTEMPTS" }
}
Write-Output "=== capture phase done ==="
