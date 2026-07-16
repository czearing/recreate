# Release channel

Every validated push to `main` publishes native binaries to the
`recreate-main` GitHub Release.

Installed binaries query that release on every invocation. An update is applied
only when the release asset digest differs from the installed executable.

The current invocation then restarts with the updated binary. Active captures
never change versions midway through execution.
