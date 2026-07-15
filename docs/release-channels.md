# Release channels and automatic updates

## Channels

| Channel | npm tag | Purpose |
| --- | --- | --- |
| Internal beta | `beta` | Every successful push to `main` |
| Stable | `latest` | Versioned GitHub releases |
| Pinned | Exact SemVer | Reproducible projects and rollback |

Beta releases append the GitHub run, attempt, and commit to the stable package
version. Published package versions are immutable.

## Automatic updates

Claude and Copilot registrations should launch Recreate through a channel:

```text
npx --yes @czearing/recreate@beta
```

The package tag is resolved when a new MCP process starts. An active process
keeps its current version until the next agent session, preventing mid-capture
changes. Channel-based launches require registry access; use an installed exact
version when a machine must work offline.

Use `@latest` for stable updates or an exact version to pin:

```text
@czearing/recreate@latest
@czearing/recreate@0.1.0
```

## Release flow

1. CI runs syntax checks, tests, and a package dry run.
2. A successful `main` build publishes a unique prerelease with the `beta` tag.
3. Release Please maintains the stable version PR and changelog.
4. Merging that PR creates a GitHub release and publishes the exact version
   with the `latest` tag.

## Recovery

GitHub Packages never replaces an existing version. To recover from a bad
release, move the channel tag back to a known version or pin the MCP
registration to that version. Do not delete the broken version because it is
part of the release audit trail.

```powershell
npm dist-tag add @czearing/recreate@0.1.0-beta.42.1.sha-abcdef0 beta `
  --registry=https://npm.pkg.github.com
```

Automatic updates must remain optional. Set a fixed package version in the MCP
configuration when a project requires a frozen toolchain.

## Authentication

Private GitHub Packages require a classic personal access token with
`read:packages`. Store it only in the user's npm configuration or credential
manager. Never place a package token in a repository MCP configuration.
