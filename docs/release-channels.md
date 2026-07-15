# Release channels and automatic updates

## Channels

| Channel | npm tag | Purpose |
| --- | --- | --- |
| Beta | `beta` | Every successful push to `main` |
| Stable | `latest` | Versioned GitHub releases |
| Pinned | Exact SemVer | Reproducible projects and rollback |

Beta releases append the GitHub run, attempt, and commit to the stable package
version. Published package versions are immutable.

## Automatic updates

Automated CLI launches should select a release channel:

```text
npx --yes recreate-cli@beta https://example.com
```

The package tag is resolved when a new MCP process starts. An active process
keeps its current version until the next agent session, preventing mid-capture
changes. Channel-based launches require registry access; use an installed exact
version when a machine must work offline.

Use `@latest` for stable updates or an exact version to pin:

```text
recreate-cli@latest
recreate-cli@0.1.0
```

## Release flow

1. CI runs syntax checks, tests, and a package dry run.
2. A successful `main` build publishes a unique prerelease with the `beta` tag.
3. Release Please maintains the stable version PR and changelog.
4. Merging that PR creates a GitHub release and publishes the exact version
   with the `latest` tag.

Each stable GitHub release includes the exact tarball published to npm.

## Recovery

npm never replaces an existing version. To recover from a bad
release, move the channel tag back to a known version or pin the MCP
registration to that version. Do not delete the broken version because it is
part of the release audit trail.

```powershell
npm dist-tag add recreate-cli@0.1.0-beta.42.1.sha-abcdef0 beta
```

Automatic updates must remain optional. Set a fixed package version in the MCP
configuration when a project requires a frozen toolchain.

Public installs require no package login. Releases use npm Trusted Publishing
and provenance from GitHub Actions.
