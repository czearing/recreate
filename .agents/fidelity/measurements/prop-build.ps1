$dir = "C:\Users\calebzearing\recreate\.agents\fidelity\measurements"
$root = "$dir\property"
$shared = "C:\Users\calebzearing\recreate\live-r15\react\node_modules"

$keys = @('phpman','jquery','godev','plaintext','litenews','paper')
foreach ($k in $keys) {
  $rj = "$root\$k\react"
  if (-not (Test-Path $rj)) { Write-Output "$k MISSING react"; continue }
  if (Test-Path "$rj\dist\index.html") { Write-Output "$k dist ok"; continue }
  if (-not (Test-Path "$rj\node_modules")) {
    cmd /c mklink /J "$rj\node_modules" "$shared" | Out-Null
  }
  Push-Location $rj
  npm run build 2>&1 | Select-Object -Last 3
  Pop-Location
  Write-Output "$k dist=$(Test-Path "$rj\dist\index.html")"
}

