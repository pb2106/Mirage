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
