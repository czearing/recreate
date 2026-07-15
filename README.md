<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/czearing/recreate/main/.github/assets/recreate-hero-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/czearing/recreate/main/.github/assets/recreate-hero-light.svg">
    <img alt="Recreate. Capture the interface, keep the details." src="https://raw.githubusercontent.com/czearing/recreate/main/.github/assets/recreate-hero-light.svg" width="100%">
  </picture>
</div>

<div align="center">

<a href="https://github.com/czearing/recreate/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/czearing/recreate/ci.yml?branch=main&style=flat-square&label=CI&labelColor=171A24&color=6C7CFF"></a> <a href="https://www.npmjs.com/package/recreate-cli"><img alt="npm" src="https://img.shields.io/npm/v/recreate-cli?style=flat-square&label=npm&labelColor=171A24&color=6C7CFF"></a> <a href="https://www.npmjs.com/package/recreate-cli"><img alt="downloads" src="https://img.shields.io/npm/dm/recreate-cli?style=flat-square&label=downloads&labelColor=171A24&color=6C7CFF"></a> <a href="https://github.com/czearing/recreate/issues"><img alt="issues" src="https://img.shields.io/github/issues/czearing/recreate?style=flat-square&label=issues&labelColor=171A24&color=6C7CFF"></a> <a href="LICENSE"><img alt="license" src="https://img.shields.io/github/license/czearing/recreate?style=flat-square&label=license&labelColor=171A24&color=6C7CFF"></a>

[Get started](#get-started) · [Documentation](docs/reference.md) · [Report an issue](https://github.com/czearing/recreate/issues/new/choose) · [npm](https://www.npmjs.com/package/recreate-cli)

</div>

Recreate captures a live interface and turns it into a complete, portable reference. Structure, styling, assets, responsive behavior, and interactions stay together, ready to rebuild and verify.

## Get started

```bash
npx --yes recreate-cli@latest install
```

Recreate detects GitHub Copilot CLI and Claude Code, then installs a personal
`/recreate` skill for every client it finds.

| Client | Installed skill |
| --- | --- |
| GitHub Copilot CLI | `~/.copilot/skills/recreate/SKILL.md` |
| Claude Code | `~/.claude/skills/recreate/SKILL.md` |

Run it from Copilot or Claude:

```text
/recreate Capture https://example.com and rebuild it in this project.
```

Each run checks npm for the latest stable release. No reinstall is required.

Run Recreate directly at any time:

```bash
npx --yes --prefer-online recreate-cli@latest https://example.com
```

## One capture. The whole interface.

| | |
| --- | --- |
| **Structure** | Pages, sections, components, and content |
| **Visual system** | Layout, type, color, spacing, and assets |
| **Responsive behavior** | Desktop and mobile states captured together |
| **Interactions** | Navigation, menus, dialogs, focus, hover, and motion |

## Built for accurate recreation

Screenshots show how a page looked once. Recreate preserves the details that made it work, including the relationships between elements and the changes between states. The result is easier to understand, easier to reproduce, and easier to check.

## Links

- [Documentation](docs/reference.md)
- [Release channels](docs/release-channels.md)
- [Contributing](CONTRIBUTING.md)
- [Security](SECURITY.md)

## License

[MIT](LICENSE)
