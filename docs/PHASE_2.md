# Phase 2: Sandboxed Projection v1

## What was implemented

### Profile System (`mirage-protocol`)
- Expanded `Profile` struct with typed fields: `timezone`, `locale`, `hostname`, `machine_id`, `dns`, `gps`.
- Added `load_profile(path)` — reads and deserialises a YAML file into a `Profile`.
- Added `serde_yaml` dependency.
- Sample profiles: `profiles/london-vpn.yaml`, `profiles/tokyo.yaml`.

### Provider Projection (`mirage-providers`)
All Phase 2-ready providers now implement `projected_value()` and `apply()`:

| Provider | `projected_value` | `apply` (sandbox effect) |
|---|---|---|
| `HostnameProvider` | reads `profile.hostname` | writes `/tmp/mirage-hostname`, bind-mounts over `/etc/hostname` |
| `TimezoneProvider` | reads `profile.timezone` | bind-mounts zoneinfo file over `/etc/localtime`; sets `TZ` env |
| `LocaleProvider` | reads `profile.locale` | sets `LANG` and `LC_ALL` env vars |
| `MachineIdProvider` | reads `profile.machine_id` | validates hex, writes `/tmp/mirage-machine-id`, bind-mounts over `/etc/machine-id` |

### Sandbox Runner (`mirage-core::runner`)
- `run_in_sandbox(app, args, profile)` — orchestrates the bwrap launch:
  - Each provider's `apply()` prepares its tmpfiles.
  - `build_bwrap_command()` assembles the full `bwrap` argument list:
    - `--bind / /` (real root passthrough)
    - `--unshare-uts` + `--hostname <value>`
    - `--bind <tmpfile> <target>` for machine-id, hostname, timezone
    - `--setenv TZ / LANG / LC_ALL`
    - `-- <app> [args]`

### CLI (`mirage-cli`)
New `mirage run` subcommand:
```
mirage run <app> --profile <path/to/profile.yaml> [-- <app-args>]
```
Example:
```bash
mirage run bash --profile profiles/london-vpn.yaml
mirage run /usr/bin/chromium --profile profiles/tokyo.yaml -- --no-sandbox
```

## Requirements
- `bwrap` must be installed: `sudo apt install bubblewrap`
- Profile YAML files live in `profiles/` at the workspace root.

## Next (Phase 3)
DNS namespace isolation, IPv6 suppression, network namespace management via `mirage-netns`.
