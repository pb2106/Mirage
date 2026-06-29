use anyhow::Result;
use clap::{Parser, Subcommand};
use mirage_core::audit::AuditEngine;
use mirage_providers::{
    bluetooth::BluetoothProvider, dns::DnsProvider, geoclue::GeoClueProvider,
    hostname::HostnameProvider, locale::LocaleProvider, machine_id::MachineIdProvider,
    network::{Ipv4Provider, Ipv6Provider}, timezone::TimezoneProvider, webrtc::WebRtcProvider,
    wifi::WifiProvider,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Mirage — Linux Identity Virtualization & Audit Platform"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Audit an application's visible identity signals
    Audit {
        /// Target process ID to audit (optional — audits host if omitted)
        pid: Option<u32>,
    },

    /// Launch an application inside a sandboxed identity profile
    Run {
        /// Path to the application binary to launch
        app: String,

        /// Path to the YAML profile file to use
        #[arg(short, long)]
        profile: PathBuf,

        /// Arguments to pass to the launched application
        #[arg(last = true)]
        args: Vec<String>,
    },
}

fn build_audit_engine() -> AuditEngine {
    let mut engine = AuditEngine::new();
    engine.register_provider(Box::new(HostnameProvider));
    engine.register_provider(Box::new(MachineIdProvider));
    engine.register_provider(Box::new(TimezoneProvider));
    engine.register_provider(Box::new(LocaleProvider));
    engine.register_provider(Box::new(Ipv4Provider));
    engine.register_provider(Box::new(Ipv6Provider));
    engine.register_provider(Box::new(WebRtcProvider));
    engine.register_provider(Box::new(DnsProvider));
    engine.register_provider(Box::new(GeoClueProvider));
    engine.register_provider(Box::new(WifiProvider));
    engine.register_provider(Box::new(BluetoothProvider));
    engine
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Audit { pid } => {
            println!("Starting Mirage Audit Engine...");
            if let Some(p) = pid {
                println!("Auditing PID: {}", p);
            } else {
                println!("Auditing host (no PID provided)");
            }

            let engine = build_audit_engine();
            let results = engine.run_audit(*pid)?;

            println!("\n[Audit Results]");
            for (kind, value) in &results {
                println!("{:?}: {:?}", kind, value);
            }

            println!("\n[Consistency Check]");
            let mut consistency_engine = mirage_core::consistency::ConsistencyEngine::new();
            consistency_engine.register_rule(Box::new(mirage_core::consistency::BasicNetworkRule));

            let rule_results = consistency_engine.evaluate(&results);
            for res in &rule_results {
                let status = if res.passed { "PASS" } else { "FAIL" };
                println!("[{}] {}: {}", status, res.rule_name, res.details);
            }

            let score = mirage_core::consistency::ConsistencyEngine::calculate_score(&rule_results);
            println!("Overall Consistency Score: {}/100", score);
        }

        Commands::Run { app, profile, args } => {
            // Load profile
            let profile = mirage_protocol::load_profile(profile)?;
            println!("Loaded profile: {}", profile.name);

            // Phase 2: launch via bwrap
            mirage_core::runner::run_in_sandbox(app, args, &profile)?;
        }
    }

    Ok(())
}
