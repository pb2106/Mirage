#[cfg(test)]
mod consistency_tests {
    use mirage_core::consistency::{BasicNetworkRule, ConsistencyEngine, ConsistencyRule};
    use mirage_protocol::{SignalKind, SignalValue};
    use std::collections::HashMap;

    fn signals_with_matching_ips() -> HashMap<SignalKind, SignalValue> {
        let mut m = HashMap::new();
        m.insert(SignalKind::Ipv4, SignalValue::Ipv4("1.2.3.4".to_string()));
        m.insert(
            SignalKind::WebRtc,
            SignalValue::WebRtc {
                public_ips: vec!["1.2.3.4".to_string()],
                local_ips: vec!["192.168.1.1".to_string()],
            },
        );
        m
    }

    fn signals_with_mismatched_ips() -> HashMap<SignalKind, SignalValue> {
        let mut m = HashMap::new();
        m.insert(SignalKind::Ipv4, SignalValue::Ipv4("1.2.3.4".to_string()));
        m.insert(
            SignalKind::WebRtc,
            SignalValue::WebRtc {
                public_ips: vec!["9.9.9.9".to_string()],
                local_ips: vec!["192.168.1.1".to_string()],
            },
        );
        m
    }

    #[test]
    fn matching_ips_passes_rule() {
        let rule = BasicNetworkRule;
        let signals = signals_with_matching_ips();
        let result = rule.evaluate(&signals);
        assert!(result.passed, "matching IPs should pass: {}", result.details);
    }

    #[test]
    fn mismatched_ips_fails_rule() {
        let rule = BasicNetworkRule;
        let signals = signals_with_mismatched_ips();
        let result = rule.evaluate(&signals);
        assert!(!result.passed, "mismatched IPs should fail: {}", result.details);
    }

    #[test]
    fn score_is_100_when_all_rules_pass() {
        let mut engine = ConsistencyEngine::new();
        engine.register_rule(Box::new(BasicNetworkRule));

        let signals = signals_with_matching_ips();
        let results = engine.evaluate(&signals);
        let score = ConsistencyEngine::calculate_score(&results);
        assert_eq!(score, 100, "all rules passing should give score 100");
    }

    #[test]
    fn score_is_reduced_when_rule_fails() {
        let mut engine = ConsistencyEngine::new();
        engine.register_rule(Box::new(BasicNetworkRule));

        let signals = signals_with_mismatched_ips();
        let results = engine.evaluate(&signals);
        let score = ConsistencyEngine::calculate_score(&results);
        assert!(score < 100, "a failing rule should reduce the score");
        assert_eq!(score, 80, "BasicNetworkRule has score_impact=20, so score should be 80");
    }

    #[test]
    fn score_never_goes_below_zero() {
        use mirage_core::consistency::{ConsistencyRule, RuleResult};

        struct AlwaysFailRule { impact: i32 }
        impl ConsistencyRule for AlwaysFailRule {
            fn name(&self) -> &str { "AlwaysFail" }
            fn description(&self) -> &str { "Always fails for testing" }
            fn evaluate(&self, _: &HashMap<SignalKind, SignalValue>) -> RuleResult {
                RuleResult {
                    rule_name: self.name().to_string(),
                    description: self.description().to_string(),
                    passed: false,
                    score_impact: self.impact,
                    details: "always fails".to_string(),
                }
            }
        }

        let results: Vec<_> = (0..10)
            .map(|_| {
                let r = AlwaysFailRule { impact: 50 };
                r.evaluate(&HashMap::new())
            })
            .collect();

        let score = ConsistencyEngine::calculate_score(&results);
        assert_eq!(score, 0, "score must be floored at 0");
    }

    #[test]
    fn empty_signals_does_not_panic() {
        let rule = BasicNetworkRule;
        let result = rule.evaluate(&HashMap::new());
        // Should pass silently — no signals means no conflict detected
        assert!(result.passed);
    }
}
