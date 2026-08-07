$dir = "C:\Users\calebzearing\recreate\.agents\fidelity\measurements"
$exe = "$dir\prop-target\release\recreate.exe"
$edge = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
$root = "$dir\property"
New-Item -ItemType Directory -Force -Path $root | Out-Null

$sites = @(
  @{k='phpman';    u='https://www.php.net/manual/en/function.array-map.php'},
  @{k='jquery';    u='https://jquery.com/'},
  @{k='mdn';       u='https://developer.mozilla.org/en-US/docs/Web/API/Element/getBoundingClientRect'},
  @{k='plaintext'; u='https://motherfuckingwebsite.com/'},
  @{k='litenews';  u='https://lite.cnn.com/'},
  @{k='paper';     u='https://arxiv.org/abs/1706.03762'}
)

function Ensure-Cap($port, $prof) {
  try { $r = Invoke-WebRequest "http://127.0.0.1:$port/json/version" -UseBasicParsing -TimeoutSec 4; if ($r.StatusCode -eq 200) { return } } catch {}
  Start-Process $edge -ArgumentList "--headless=new","--remote-debugging-port=$port","--user-data-dir=$root\$prof","--no-first-run","--disable-gpu","--hide-scrollbars","--disable-dev-shm-usage","about:blank"
  Start-Sleep 9
}

foreach ($s in $sites) {
  $out = "$root\$($s.k)"
  if (Test-Path "$out\spec.json") { Write-Output "skip $($s.k)"; continue }
  for ($a = 1; $a -le 3; $a++) {
    if (Test-Path "$out\spec.json") { break }
    Ensure-Cap 9355 "cap-prof-$($s.k)-$a"
    Write-Output "### capture $($s.k) attempt $a"
    & $exe capture $s.u --out $out --cdp-url "http://127.0.0.1:9355" 2>&1 | Select-Object -Last 3
    if (-not (Test-Path "$out\spec.json")) {
      Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" | Where-Object { $_.CommandLine -like "*9355*" } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
      Start-Sleep 3
    }
  }
  $ok = Test-Path "$out\spec.json"
  Write-Output "$($s.k) spec=$ok"
}
