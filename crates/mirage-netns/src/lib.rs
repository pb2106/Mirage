//! `mirage-netns` — Network namespace management for Phase 3.
//!
//! Provides root-level network isolation (veth pairs, NAT, IPv6 suppression)
//! for sandboxes.

use anyhow::{Context, Result};
use std::process::Command;

/// Represents an active network namespace created via `ip netns`.
pub struct Netns {
    pub name: String,
    pub veth_host: String,
    pub veth_guest: String,
    pub ip_guest: String,
    pub ip_host: String,
}

impl Netns {
    /// Creates a new network namespace and wires it up to the host with a veth pair.
    /// Requires `sudo` privileges (or running as root).
    pub fn create(id: u32, mac: Option<&str>) -> Result<Self> {
        let name = format!("mirage-{}", id);
        let veth_host = format!("vm-{}", id);
        let veth_guest = format!("vg-{}", id);
        
        // Use a static subnet for now (e.g. 10.200.1.X)
        // In a real scenario, this would use an IPAM / pool allocator to avoid collisions.
        let ip_host = "10.200.1.1".to_string();
        let ip_guest = "10.200.1.2".to_string();

        let ns = Self {
            name,
            veth_host,
            veth_guest,
            ip_host,
            ip_guest,
        };

        // 1. Create the netns
        run_sudo(&["ip", "netns", "add", &ns.name])?;

        // 2. Create veth pair
        run_sudo(&[
            "ip", "link", "add", &ns.veth_host, "type", "veth", "peer", "name", &ns.veth_guest,
        ])?;

        // 3. Move guest veth into netns
        run_sudo(&["ip", "link", "set", &ns.veth_guest, "netns", &ns.name])?;

        // 4. Configure host veth
        run_sudo(&["ip", "addr", "add", &format!("{}/24", ns.ip_host), "dev", &ns.veth_host])?;
        run_sudo(&["ip", "link", "set", &ns.veth_host, "up"])?;

        // 5. Configure guest veth
        ns.exec(&["ip", "addr", "add", &format!("{}/24", ns.ip_guest), "dev", &ns.veth_guest])?;
        if let Some(m) = mac {
            ns.exec(&["ip", "link", "set", "dev", &ns.veth_guest, "address", m])?;
        }
        ns.exec(&["ip", "link", "set", &ns.veth_guest, "up"])?;
        ns.exec(&["ip", "link", "set", "lo", "up"])?;

        // 6. Set default route in guest
        ns.exec(&["ip", "route", "add", "default", "via", &ns.ip_host])?;

        // 7. Suppress IPv6 inside the namespace
        ns.exec(&["sysctl", "-w", "net.ipv6.conf.all.disable_ipv6=1"])?;
        ns.exec(&["sysctl", "-w", "net.ipv6.conf.default.disable_ipv6=1"])?;

        // 8a. Allow unprivileged ICMP sockets (ping) inside the namespace.
        //     Each netns starts with ping_group_range = "1 0" (nobody allowed).
        //     Setting it to the full GID range lets any user run ping via
        //     SOCK_DGRAM IPPROTO_ICMP without needing CAP_NET_RAW.
        ns.exec(&["sysctl", "-w", "net.ipv4.ping_group_range=0 2147483647"])?;

        // 8. Enable IP forwarding on host & setup NAT
        run_sudo(&["sysctl", "-w", "net.ipv4.ip_forward=1"])?;
        run_sudo(&[
            "iptables", "-t", "nat", "-A", "POSTROUTING", "-s", "10.200.1.0/24", "-j", "MASQUERADE",
        ])?;
        run_sudo(&["iptables", "-A", "FORWARD", "-i", &ns.veth_host, "-j", "ACCEPT"])?;
        run_sudo(&["iptables", "-A", "FORWARD", "-o", &ns.veth_host, "-j", "ACCEPT"])?;

        Ok(ns)
    }

    /// Execute a command *inside* this network namespace via `ip netns exec`.
    pub fn exec(&self, args: &[&str]) -> Result<()> {
        let mut cmd = vec!["ip", "netns", "exec", &self.name];
        cmd.extend_from_slice(args);
        run_sudo(&cmd)
    }

    /// Tear down the namespace and associated NAT rules.
    pub fn destroy(&self) {
        // The veth pair is automatically destroyed when the netns is deleted,
        // but we explicitly remove the host side just in case.
        let _ = run_sudo(&["ip", "link", "delete", &self.veth_host]);
        let _ = run_sudo(&["ip", "netns", "delete", &self.name]);
        
        // Remove NAT and FORWARD rules
        let _ = run_sudo(&[
            "iptables", "-t", "nat", "-D", "POSTROUTING", "-s", "10.200.1.0/24", "-j", "MASQUERADE",
        ]);
        let _ = run_sudo(&["iptables", "-D", "FORWARD", "-i", &self.veth_host, "-j", "ACCEPT"]);
        let _ = run_sudo(&["iptables", "-D", "FORWARD", "-o", &self.veth_host, "-j", "ACCEPT"]);
    }
}

impl Drop for Netns {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// Helper to run a command with `sudo`.
fn run_sudo(args: &[&str]) -> Result<()> {
    let status = Command::new("sudo")
        .args(args)
        .status()
        .with_context(|| format!("Failed to execute sudo {:?}", args))?;

    if !status.success() {
        anyhow::bail!("Command `sudo {:?}` failed with status {}", args, status);
    }
    Ok(())
}
