# Mirage Profile Diversity Engine

## What is Tool-Level Homogeneity?
When an identity virtualization tool like Mirage generates mock data using fixed algorithms or default values, the resulting identities often share statistical patterns. Even if an observer cannot identify *who* you are, they can easily fingerprint *what tool* you are using. This meta-fingerprinting happens if all users have `1920x1080` screen resolution, use the same pool of `OUI` prefixes for MAC addresses, or have the exact same 5 fonts installed.

To solve this, Mirage uses the **Profile Diversity Engine** (`mirage-diversity`).

## What Mirage Generates
Instead of hardcoded defaults, the diversity engine samples from realistic population distributions based on your declared region and device class. It generates:
- **Hostname**: Using region-weighted first names and device popularity.
- **Machine ID**: Valid UUID4.
- **MAC Address**: Assigned from top market-share OUIs with the locally-administered bit properly set to `0`.
- **Screen Resolution**: Realistic combinations mapped to laptop/desktop/phone classes.
- **Font Sets**: OS base fonts + regional supplements + random application fonts.
- **Locale Combos**: Plausible sub-field variations (e.g. English LC_TIME on a German LANG).
- **Git Persona**: Regional names and popular regional email providers.

## Why Values Are Stable Per Profile
All these generated values are evaluated exactly **once** at profile creation time and stored in the encrypted Profile DB. They are **not re-rolled** each time you launch the sandbox. 
Why? Because real machines do not change their MAC address, machine-id, or font set every reboot. A machine-id that changes on every launch is an immediate red flag that the machine is a short-lived VM or container. By persisting these values, your profile ages naturally.

## Intentional Locale Variations
Real systems are rarely perfectly consistent. Developers or expats often have slight mismatches in their locale environment variables (e.g., `LANG=ja_JP.UTF-8` but `LC_NUMERIC=en_US.UTF-8`). The Diversity Engine introduces these variations intentionally about 20% of the time. Our internal Consistency Checker is aware of this logic and will not penalize the consistency score for these intentional variations.

## Known Homogeneity Risks (Not Fixed by Mirage)
While `mirage-diversity` handles userspace signals, some underlying signals cannot be spoofed without a full VM:
- **Kernel Version (`uname`)**: Visible to any binary inside the sandbox. If every Mirage user is running a specific Linux kernel version, this remains a fingerprint.
- **WireGuard / BoringTun Handshakes**: The handshake timing and packet structure differ between kernel WireGuard and userspace BoringTun.
- **CapEff Values**: The sandbox's capability set in `/proc/self/status` can be read by a process inside it, revealing that it runs inside `bwrap`.

## Network Visibility of MAC Addresses
**Note:** MAC addresses are L2-local. They are only visible to other devices on the same local network subnet. Remote services on the internet do not see your MAC address. The generated MAC is primarily useful when interacting with local captial portals or local network scanners inside an isolated netns.
