$root   = "C:\Users\calebzearing\recreate\.agents\fidelity\measurements\wf"
$shared = "C:\Users\calebzearing\recreate\live-r15\react\node_modules"

foreach ($k in 'reactdev','sveltedev','vuejs','cern','danluu','nprtext') {
  $rj = "$root\$k\react"
  if (-not (Test-Path "$rj\src\App.jsx")) { Write-Output "$k NO SOURCE"; continue }
  if (Test-Path "$rj\dist\index.html")    { Write-Output "$k ALREADY BUILT"; continue }
  if (-not (Test-Path "$rj\node_modules")) { cmd /c mklink /J "$rj\node_modules" "$shared" | Out-Null }
  Push-Location $rj
  npm run build 2>&1 | Select-Object -Last 3 | ForEach-Object { "   $_" }
  Pop-Location
  Write-Output "$k => $(if (Test-Path "$rj\dist\index.html") { 'BUILT' } else { 'BUILD FAILED' })"
}
Write-Output "=== build done ==="
