# Rakitray CLI Usage Guide (`rakit`)

`rakit` is Rakitray's headless CLI for reproducible, team-shared local development environments (`.rakitray.toml` / `.dev.toml`) without OCI containers.

## Quick Start

```bash
# 1. Browse available runtimes
rakit store list

# 2. Install specific runtime versions (dynamic official resolvers + catalog)
rakit install node@22.11.0
rakit install php@8.3
rakit install go@1.23
rakit install python@3.12
rakit install java@21
rakit install caddy@2.8.4

# 3. Verify installed runtimes
rakit scan

# 4. Initialize a project manifest (.rakitray.toml)
rakit init

# 5. Start, inspect, and stop the stack
rakit up
rakit status
rakit down
```

## Commands

| Command | Description |
| --- | --- |
| `rakit store list` | List catalog runtimes for the host OS/arch |
| `rakit install <engine>@<version>` | Download, verify SHA-256, extract, and register runtime (`node` resolves dynamically from `nodejs.org/dist/index.json`; omit `@version` for latest LTS) |
| `rakit scan` | Discover installed runtimes in `runtime/` and validate binaries + manifests |
| `rakit init` | Generate `.rakitray.toml` in the current project directory |
| `rakit up` | Boot project stack or global rack |
| `rakit status` | Show running services and ports |
| `rakit down` | Gracefully stop running stack (process-group cleanup) |
| `rakit env` | Print `export PATH="..."` for active `.rakitray.toml` (`eval "$(rakit env)"`) |
| `rakit remove <id>` / `rakit uninstall <id>` | Uninstall a runtime and delete its directory + receipt |
| `rakit --help` | Show CLI help and subcommand options |

## Shareable `.rakitray.toml` Example

Commit `.rakitray.toml` to Git so teammates share exact versions, ports, and isolated per-service configs (without touching system-wide files):

```toml
schema_version = 1
name = "my-app"

[services.php]
version = "8.3.12"
port = 9000

[services.php.config]
memory_limit = "512M"
upload_max_filesize = "64M"

[services.mariadb]
version = "11.4.3"
port = 3307

[services.mariadb.config]
max_connections = "200"
```

