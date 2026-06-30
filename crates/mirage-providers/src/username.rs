use crate::IdentityProvider;
use anyhow::{Context, Result};
use mirage_protocol::{Profile, SignalKind, SignalValue};
use mirage_sandbox::SandboxHandle;
use std::fs;

pub struct UsernameProvider;

impl IdentityProvider for UsernameProvider {
    fn signal_kind(&self) -> SignalKind {
        SignalKind::Username
    }

    fn real_value(&self) -> Result<SignalValue> {
        let user = std::env::var("USER").unwrap_or_else(|_| "nobody".to_string());
        Ok(SignalValue::Username(user))
    }

    fn projected_value(&self, profile: &Profile) -> Result<SignalValue> {
        let fake = profile
            .username
            .as_deref()
            .unwrap_or("devuser");
        Ok(SignalValue::Username(fake.to_string()))
    }

    fn apply(&self, ns: &SandboxHandle, profile: &Profile) -> Result<()> {
        let fake_user = match &profile.username {
            Some(u) => u,
            None => return Ok(()),
        };

        let real_uid = unsafe { nix::libc::getuid() };
        let passwd_contents = fs::read_to_string("/etc/passwd")
            .context("Failed to read /etc/passwd")?;
        
        let mut new_passwd = String::new();
        let mut found = false;
        
        for line in passwd_contents.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 6 && parts[2] == real_uid.to_string() {
                // Format: username:password:uid:gid:gecos:homedir:shell
                // Replace username (parts[0]) and homedir (parts[5])
                let fake_home = format!("/home/{}", fake_user);
                new_passwd.push_str(&format!("{}:{}:{}:{}:{}:{}:{}\n", 
                    fake_user, parts[1], parts[2], parts[3], parts[4], fake_home, parts[6]
                ));
                found = true;
            } else {
                new_passwd.push_str(line);
                new_passwd.push('\n');
            }
        }

        if found {
            fs::write(ns.tmp_dir.join("passwd"), new_passwd)
                .context("Failed to write fake passwd")?;
        }
        
        Ok(())
    }
}
