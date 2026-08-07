$dir = "C:\Users\calebzearing\recreate\.agents\fidelity\measurements"
$root = "$dir\property"
$edge = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"

$sites = @(
  @{k='phpman';    u='https://www.php.net/manual/en/function.array-map.php'; p=8821},
  @{k='jquery';    u='https://jquery.com/'; p=8822},
  @{k='godev';     u='https://go.dev/'; p=8823},
  @{k='plaintext'; u='https://motherfuckingwebsite.com/'; p=8824},
  @{k='litenews';  u='https://lite.cnn.com/'; p=8825},
  @{k='paper';     u='https://arxiv.org/abs/1706.03762'; p=8826}
)

function Ensure-Browser($prof) {
  try { $r = Invoke-WebRequest http://127.0.0.1:9366/json/version -UseBasicParsing -TimeoutSec 4; if ($r.StatusCode -eq 200) { return } } catch {}
  Start-Process $edge -ArgumentList "--headless=new","--remote-debugging-port=9366","--user-data-dir=$root\m-$prof","--no-first-run","--disable-gpu","--hide-scrollbars","--disable-dev-shm-usage","about:blank"
  Start-Sleep 9
}

foreach ($s in $sites) {
  $out = "$root\raw-$($s.k).json"
  if (-not (Test-Path "$root\$($s.k)\react\dist\index.html")) { Write-Output "$($s.k) NO DIST - skipped"; continue }
  for ($a = 1; $a -le 3; $a++) {
    if (Test-Path $out) { break }
    Ensure-Browser "$($s.k)-$a"
    Write-Output "### $($s.k) attempt $a"
    (@{key=$s.k; sourceUrl=$s.u; recreationUrl="http://127.0.0.1:$($s.p)/"} | ConvertTo-Json -Compress) | Out-File -Encoding utf8 "$root\_site-$($s.k).json"
    Push-Location $dir
    node overlap-property-census.mjs http://127.0.0.1:9366 "$root\_site-$($s.k).json" $out 2>&1 | Select-Object -First 10
    Pop-Location
    if (-not (Test-Path $out)) {
      Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" | Where-Object { $_.CommandLine -like "*9366*" } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
      Start-Sleep 3
    }
  }
}

