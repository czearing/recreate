$dir = "C:\Users\calebzearing\recreate\.agents\fidelity\measurements"
$edge = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"

function Ensure-Browser($profile) {
  try { $r = Invoke-WebRequest http://127.0.0.1:9333/json/version -UseBasicParsing -TimeoutSec 4; if ($r.StatusCode -eq 200) { return } } catch {}
  Start-Process $edge -ArgumentList "--headless=new","--remote-debugging-port=9333","--user-data-dir=$dir\multi-site\$profile","--no-first-run","--disable-gpu","--hide-scrollbars","--disable-dev-shm-usage","about:blank"
  Start-Sleep 9
}

$sites = @(
  @{k='legacy'; u='https://news.ycombinator.com/'; p=8805},
  @{k='table';  u='https://datatables.net/examples/styling/bootstrap5.html'; p=8806},
  @{k='docs';   u='https://docs.python.org/3/library/json.html'; p=8807}
)

foreach ($s in $sites) {
  $out = "$dir\multi-site\raw-$($s.k).json"
  for ($attempt = 1; $attempt -le 3; $attempt++) {
    if (Test-Path $out) { break }
    Ensure-Browser "prof-$($s.k)-$attempt"
    Write-Output "### $($s.k) attempt $attempt"
    Push-Location $dir
    (@{key=$s.k; sourceUrl=$s.u; recreationUrl="http://127.0.0.1:$($s.p)/"} | ConvertTo-Json -Compress) | Out-File -Encoding utf8 "$dir\multi-site\_site-$($s.k).json"
    node overlap-multi-site.mjs http://127.0.0.1:9333 "$dir\multi-site\_site-$($s.k).json" $out 2>&1 | Select-Object -First 9
    Pop-Location
  }
}
