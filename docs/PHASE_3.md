# Phase 3: Network Namespace Isolation

## What was implemented

### DNS Provider (`mirage-providers/src/dns.rs`)
- Implemented `projected_value` to return DNS servers from the profile.
- Implemented `apply` to generate a custom `/etc/resolv.conf` in the per-run temporary directory.
- Runner bind-mounts this custom `resolv.conf` over the host's `/etc/resolv.conf` using `bwrap`.

### Network Namespace Manager (`mirage-netns`)
- Created `mirage-netns` crate to manage root-level Linux network namespaces via `ip netns`.
- **Veth Pair**: Sets up a virtual ethernet link between the host (`vm-<pid>`) and the sandbox (`vg-<pid>`).
- **IP Addressing**: Assigns a static `/24` subnet (e.g., `10.200.1.X`) to the bridge.
- **Routing & NAT**: Adds a default route inside the namespace and configures `iptables MASQUERADE` and `ip_forward=1` on the host to provide internet access to the sandbox.
- **IPv6 Suppression**: Modifies the `sysctl` flags (`net.ipv6.conf.all.disable_ipv6=1`) inside the namespace to completely prevent IPv6 leaks.

### Sandbox Runner Integration (`mirage-core/src/runner.rs`)
- Added `isolate_network` boolean to the `Profile` protocol.
- When `isolate_network` is true, the runner instantiates `Netns::create()` (prompting for `sudo` to run the `ip` commands).
- Wraps the `bwrap` execution inside `sudo ip netns exec mirage-<pid> sudo -u <user> bwrap ...` to ensure the sandbox runs inside the isolated network namespace *while still running as the unprivileged user*.
- Netns resources (veth interfaces, netns, iptables rules) are automatically cleaned up when the `Netns` struct goes out of scope (via the `Drop` trait).

## Next (Phase 4)
- Spoofing dynamic hardware identifiers (MAC addresses, CPU details).
- Hooking/Spoofing GeoClue D-Bus endpoints for location spoofing via `mirage-dbus-proxy`.
