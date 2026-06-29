# Mirage D-Bus Proxy

## Overview

`mirage-dbus-proxy` starts a private `dbus-daemon` and registers fake handlers
for the D-Bus interfaces listed below. The sandbox's `/run/dbus/system_bus_socket`
is bind-mounted to this private daemon's socket, and `DBUS_SESSION_BUS_ADDRESS` /
`DBUS_SYSTEM_BUS_ADDRESS` are overridden in the sandbox environment.

This means that any application inside the sandbox that queries these interfaces
receives profile-consistent fake values instead of real host values.

## Intercepted Interfaces

| Interface | What is faked | What is NOT faked |
|---|---|---|
| `org.freedesktop.GeoClue2` | GPS latitude, longitude, accuracy | — |
| `org.freedesktop.hostname1` | Hostname, StaticHostname, MachineID, Chassis, HardwareModel | OsPrettyName (returns a generic Ubuntu string) |
| `org.freedesktop.timedate1` | Timezone | Wall-clock time (TimeUSec). See note below. |
| `org.freedesktop.locale1` | LANG, LC_TIME, LC_NUMERIC (full LocaleCombo) | X11 keyboard layout |
| `org.freedesktop.login1` | ListSessions (returns empty), ListUsers (returns empty) | Inhibitor locks, power management, seat control |
| `org.freedesktop.Accounts` | All calls → AccessDenied | (entire interface blocked) |
| `org.freedesktop.UPower` | Device model string, serial number | Battery percentage, charge state, time-to-empty |

## Wall-Clock Time

Mirage does **not** spoof wall-clock time. Only the `Timezone` identifier is
projected. The reasons:

1. Faking `TimeUSec` creates visible clock drift for the user (timestamps on
   files, browser TLS, etc. all break).
2. The real time is already correct once the timezone is set — applications
   display the correct local time when they render `TimeUSec` through the
   profile timezone.

## Abstract Unix Socket Isolation

Abstract Unix domain sockets (names beginning with a null byte, e.g.
`unix:abstract=dbus-...`) exist in the **kernel network namespace**, not on the
filesystem. `bwrap`'s filesystem overlay does not cover them.

This is the primary way the D-Bus proxy can be bypassed: if the host sets
`DBUS_SESSION_BUS_ADDRESS` to an abstract socket, the proxy is never consulted.

Mirage closes this gap with two layers:

1. **`--unshare-net`** is always passed to `bwrap` (unless `isolate_network: true`
   is also set, in which case the full netns wrapping supersedes it). Abstract
   sockets are scoped per network namespace in Linux, so the host's abstract
   sockets are completely invisible inside the sandbox.

2. **`DBUS_SESSION_BUS_ADDRESS` and `DBUS_SYSTEM_BUS_ADDRESS`** are explicitly
   overridden to `unix:path=<proxy-socket>` via `--setenv`. This handles the
   case where a library reads the environment variable without consulting
   `/run/dbus/system_bus_socket`.

## What is Not Intercepted in `login1`

The following `login1` functionality is intentionally **not** intercepted because
the user's own applications depend on it:

- `Inhibit` / `ListInhibitors` — power management and suspend inhibitor locks.
- `PowerOff`, `Reboot`, `Suspend`, `Hibernate` — the sandbox cannot issue these
  anyway (they require privilege), but the proxy does not intercept the calls.
- `GetSeat`, `GetSession` — calls for the user's own seat/session data. These
  are blocked because they would reveal real session identity.

If an application inside the sandbox strictly requires `login1` session
information, it will see an empty session list. This is the safe default.
