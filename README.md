<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/czearing/recreate/main/.github/assets/recreate-hero-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/czearing/recreate/main/.github/assets/recreate-hero-light.svg">
    <img alt="Recreate. Capture the interface, keep the details." src="https://raw.githubusercontent.com/czearing/recreate/main/.github/assets/recreate-hero-light.svg" width="100%">
  </picture>
</div>

<div align="center">

<a href="https://github.com/czearing/recreate/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/czearing/recreate/ci.yml?branch=main&style=flat-square&label=CI&labelColor=171A24&color=6C7CFF"></a>
<a href="https://github.com/czearing/recreate/releases/tag/recreate-main"><img alt="release" src="https://img.shields.io/github/v/release/czearing/recreate?include_prereleases&style=flat-square&label=release&labelColor=171A24&color=6C7CFF"></a>
<a href="https://github.com/czearing/recreate/issues"><img alt="issues" src="https://img.shields.io/github/issues/czearing/recreate?style=flat-square&label=issues&labelColor=171A24&color=6C7CFF"></a>
<a href="LICENSE"><img alt="license" src="https://img.shields.io/github/license/czearing/recreate?style=flat-square&label=license&labelColor=171A24&color=6C7CFF"></a>

</div>

Recreate captures one rendered page and turns it into local assets, responsive
evidence, animation data, and clean React components.

## Install

### npm

```bash
npm install --global recreate-cli
recreate install
```

The npm command installs a small launcher that downloads the validated native
binary for the current platform. The native CLI then keeps itself updated from
the `recreate-main` release.

### Windows

```powershell
$dir = "$HOME\.recreate\bin"
New-Item -ItemType Directory -Force $dir | Out-Null
curl.exe -L https://github.com/czearing/recreate/releases/download/recreate-main/recreate-windows-x86_64.exe -o "$dir\recreate.exe"
& "$dir\recreate.exe" install
```

### macOS and Linux

```bash
mkdir -p "$HOME/.recreate/bin"
# Download the matching binary from the recreate-main release, then:
chmod +x "$HOME/.recreate/bin/recreate"
"$HOME/.recreate/bin/recreate" install
```

The installer adds a personal `/recreate` skill for GitHub Copilot CLI and
Claude Code and installs the independent backtester beside the native CLI.
Users access it only through `recreate backtest`; its companion process and
installation path are internal details. The native binary checks GitHub
Releases for validated updates on every invocation.

## Capture

Open authenticated or access-controlled sources in Recreate's visible browser:

```bash
recreate open https://example.com
```

After access is complete, instrument and reload that same authenticated target
to capture startup motion:

```bash
recreate capture --reuse --reload
```

Recreate remembers the tab opened by `recreate open`; target IDs and browser
debugging addresses are internal details. Advanced overrides are documented in
[`docs/reference.md`](docs/reference.md).

Compare any source and recreation URLs, including localhost:

```bash
recreate backtest run --source https://example.com --recreation http://localhost:8080
```

Add `--focus "toolbar"` to isolate one semantic region while debugging.

Recreate writes:

- `spec.json`
- `acceptance.json`
- source screenshots for each viewport
- local assets and fonts
- `react/` with semantic component folders, JSX, and CSS

## Verify

Start the generated React project, then compare it with the captured evidence:

```bash
recreate verify --spec recreate-output/spec.json --url http://127.0.0.1:5173
```

See [the reference](docs/reference.md) for the full command surface.

## License

[MIT](LICENSE)
