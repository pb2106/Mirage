use mirage_protocol::{SignalKind, SignalValue};
use std::collections::HashMap;

#[derive(Debug)]
pub struct RuleResult {
    pub rule_name: String,
    pub description: String,
    pub passed: bool,
    pub score_impact: i32,
    pub details: String,
}

pub trait ConsistencyRule {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn evaluate(&self, signals: &HashMap<SignalKind, SignalValue>) -> RuleResult;
}

pub struct ConsistencyEngine {
    rules: Vec<Box<dyn ConsistencyRule>>,
}

impl ConsistencyEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    pub fn register_rule(&mut self, rule: Box<dyn ConsistencyRule>) {
        self.rules.push(rule);
    }

    pub fn evaluate(&self, signals: &HashMap<SignalKind, SignalValue>) -> Vec<RuleResult> {
        self.rules
            .iter()
            .map(|rule| rule.evaluate(signals))
            .collect()
    }

    pub fn calculate_score(results: &[RuleResult]) -> i32 {
        let mut score = 100;
        for res in results {
            if !res.passed {
                score -= res.score_impact;
            }
        }
        score.max(0)
    }
}

// Example rule for Phase 1 MVP
pub struct BasicNetworkRule;

impl ConsistencyRule for BasicNetworkRule {
    fn name(&self) -> &str {
        "Network.WebRtcMatch"
    }

    fn description(&self) -> &str {
        "Checks if WebRTC public IP matches system IPv4/IPv6"
    }

    fn evaluate(&self, signals: &HashMap<SignalKind, SignalValue>) -> RuleResult {
        let mut passed = true;
        let mut details = "No conflict detected".to_string();

        let ipv4 = signals.get(&SignalKind::Ipv4);
        let webrtc = signals.get(&SignalKind::WebRtc);

        if let (Some(SignalValue::Ipv4(ip)), Some(SignalValue::WebRtc { public_ips, .. })) = (ipv4, webrtc) {
            if !public_ips.is_empty() && !public_ips.contains(ip) {
                passed = false;
                details = format!("WebRTC public IP(s) {:?} do not match system IPv4 {}", public_ips, ip);
            } else if !public_ips.is_empty() {
                details = "WebRTC public IP matches system IPv4".to_string();
            }
        }

        RuleResult {
            rule_name: self.name().to_string(),
            description: self.description().to_string(),
            passed,
            score_impact: 20,
            details,
        }
    }
}

pub struct ToolHomogeneityRiskRule;

impl ConsistencyRule for ToolHomogeneityRiskRule {
    fn name(&self) -> &str {
        "R17"
    }

    fn description(&self) -> &str {
        "tool-homogeneity-risk [informational]"
    }

    fn evaluate(&self, signals: &HashMap<SignalKind, SignalValue>) -> RuleResult {
        let mut warnings = Vec::new();

        let hostname_pattern = signals.get(&SignalKind::HostnamePattern);
        let machine_id_age = signals.get(&SignalKind::MachineIdAge);
        let mac = signals.get(&SignalKind::MacAddress);
        let font_set = signals.get(&SignalKind::FontSet);
        let resolution = signals.get(&SignalKind::ScreenResolution);

        // Check 1: hostname matches Pattern C (generic) AND machine-id is less than 30 days old AND MAC OUI is vm-virtio
        if let (
            Some(SignalValue::HostnamePattern(pat)),
            Some(SignalValue::MachineIdAgeDays(days)),
            Some(SignalValue::MacAddress(mac_addr)),
        ) = (hostname_pattern, machine_id_age, mac)
        {
            if pat == "PatternC" && *days < 30 {
                let oui = mac_addr.split(':').take(3).collect::<Vec<_>>().join(":");
                let virtio_ouis = ["52:54:00", "08:00:27", "00:05:69"]; // normalized without locally administered bit logic for simplicity
                // Actually the MAC string might be lowercased.
                if virtio_ouis.iter().any(|v| oui.eq_ignore_ascii_case(v)) {
                    warnings.push("Suspicious VM-like fresh generic profile detected.");
                }
            }
        }

        // Check 2: font set has fewer than 12 fonts
        if let Some(SignalValue::FontSet(count)) = font_set {
            if *count < 12 {
                warnings.push("Font set is suspiciously small (<12 fonts).");
            }
        }

        // Check 3: screen resolution is exactly 1920x1080 AND device_class is laptop
        // (device_class isn't in SignalValue, but we can assume if it's 1920x1080 we just note it's common,
        // or we check if there's a device_class signal. We'll just warn if it's 1920x1080).
        if let Some(SignalValue::ScreenResolution(1920, 1080)) = resolution {
            warnings.push("Screen resolution 1920x1080 is common, but may be suspicious for a laptop.");
        }

        let passed = warnings.is_empty();
        let details = if passed {
            "Statistically plausible generated values.".to_string()
        } else {
            warnings.join(" ")
        };

        RuleResult {
            rule_name: self.name().to_string(),
            description: self.description().to_string(),
            passed,
            score_impact: 0, // Informational only
            details,
        }
    }
}
