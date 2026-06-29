use anyhow::{Context, Result};
use std::fs;
use std::collections::{HashSet, HashMap};

pub fn check_session_leak(suppress_warn: bool) -> Result<()> {
    // 1. Enumerate all PIDs under /proc/ that are children of a known mirage bwrap invocation.
    // We do this by finding processes whose cmdline starts with `bwrap` and `--unshare-uts`, 
    // or we can find all processes that are descendants of bwrap.
    let mut bwrap_pids = HashSet::new();
    let mut all_processes: HashMap<u32, u32> = HashMap::new(); // pid -> ppid
    let mut pid_to_name = HashMap::new();

    let proc_dir = fs::read_dir("/proc").context("failed to read /proc")?;
    for entry in proc_dir {
        let entry = entry?;
        let file_name = entry.file_name();
        let pid_str = file_name.to_string_lossy();
        if let Ok(pid) = pid_str.parse::<u32>() {
            let stat_path = format!("/proc/{}/stat", pid);
            if let Ok(stat_content) = fs::read_to_string(&stat_path) {
                // Parse ppid from stat: pid (comm) state ppid
                if let Some(r_paren) = stat_content.rfind(')') {
                    let parts: Vec<&str> = stat_content[r_paren + 1..].split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(ppid) = parts[1].parse::<u32>() {
                            all_processes.insert(pid, ppid);
                        }
                    }
                }
            }

            let comm_path = format!("/proc/{}/comm", pid);
            if let Ok(comm) = fs::read_to_string(&comm_path) {
                pid_to_name.insert(pid, comm.trim().to_string());
                if comm.trim() == "bwrap" {
                    // Check if it's a mirage bwrap by looking at cmdline
                    if let Ok(cmdline) = fs::read_to_string(format!("/proc/{}/cmdline", pid)) {
                        if cmdline.contains("/tmp/mirage-") || cmdline.contains("MIRAGE_PROFILE") {
                            bwrap_pids.insert(pid);
                        }
                    }
                }
            }
        }
    }

    // Now find all descendants of bwrap_pids
    let mut sandboxed_pids = HashSet::new();
    let mut changed = true;
    sandboxed_pids.extend(bwrap_pids.iter().copied());

    while changed {
        changed = false;
        for (pid, ppid) in &all_processes {
            if sandboxed_pids.contains(ppid) && !sandboxed_pids.contains(pid) {
                sandboxed_pids.insert(*pid);
                changed = true;
            }
        }
    }

    // Read active TCP connections for sandboxed processes
    let mut sandboxed_active = false;
    let mut sandboxed_leak_pid = 0;
    
    for pid in &sandboxed_pids {
        if has_active_tcp_connections(*pid) {
            sandboxed_active = true;
            sandboxed_leak_pid = *pid;
            break;
        }
    }

    // Read active TCP connections for host namespace (e.g. pid 1)
    let host_pid = 1;
    let host_active = has_active_tcp_connections(host_pid);

    if sandboxed_active && host_active && !suppress_warn {
        let comm = pid_to_name.get(&1).cloned().unwrap_or_else(|| "systemd".to_string());
        println!("\n[WARN] SESSION BOUNDARY LEAK RISK");
        println!("Sandboxed process {} (profile: unknown) and host process", sandboxed_leak_pid);
        println!("{} ({}) are both active on the network at the", host_pid, comm);
        println!("same time.");
        println!("A remote observer correlating connection timing, TLS");
        println!("fingerprints, or DNS queries may link the two identities.");
        println!("Consider closing host-side browser or app sessions before");
        println!("starting a profile session, or use `mirage shell` so all");
        println!("activity is contained within one session.\n");
    }

    Ok(())
}

fn has_active_tcp_connections(pid: u32) -> bool {
    let mut active = false;
    let tcp_path = format!("/proc/{}/net/tcp", pid);
    if let Ok(tcp_content) = fs::read_to_string(&tcp_path) {
        if check_established(&tcp_content) {
            active = true;
        }
    }
    
    let tcp6_path = format!("/proc/{}/net/tcp6", pid);
    if let Ok(tcp6_content) = fs::read_to_string(&tcp6_path) {
        if check_established(&tcp6_content) {
            active = true;
        }
    }
    
    active
}

fn check_established(net_content: &str) -> bool {
    // state field = 01 is ESTABLISHED
    for line in net_content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let state = parts[3];
            if state == "01" {
                return true;
            }
        }
    }
    false
}
