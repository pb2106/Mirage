#[cfg(test)]
mod audit_engine_tests {
    use mirage_core::audit::AuditEngine;
    use mirage_protocol::{SignalKind, SignalValue};
    use mirage_providers::{
        hostname::HostnameProvider,
        locale::LocaleProvider,
        machine_id::MachineIdProvider,
        timezone::TimezoneProvider,
    };

    fn build_engine() -> AuditEngine {
        let mut engine = AuditEngine::new();
        engine.register_provider(Box::new(HostnameProvider));
        engine.register_provider(Box::new(MachineIdProvider));
        engine.register_provider(Box::new(TimezoneProvider));
        engine.register_provider(Box::new(LocaleProvider));
        engine
    }

    #[test]
    fn audit_returns_results_for_all_registered_providers() {
        let engine = build_engine();
        let results = engine.run_audit(None).expect("audit should not fail");

        assert!(results.contains_key(&SignalKind::Hostname), "should have Hostname");
        assert!(results.contains_key(&SignalKind::MachineId), "should have MachineId");
        assert!(results.contains_key(&SignalKind::Timezone), "should have Timezone");
        assert!(results.contains_key(&SignalKind::Locale), "should have Locale");
    }

    #[test]
    fn hostname_is_non_empty_string() {
        let engine = {
            let mut e = AuditEngine::new();
            e.register_provider(Box::new(HostnameProvider));
            e
        };
        let results = engine.run_audit(None).expect("audit should not fail");
        match results.get(&SignalKind::Hostname) {
            Some(SignalValue::Hostname(h)) => {
                assert!(!h.is_empty(), "hostname must not be empty");
            }
            other => panic!("expected Hostname signal, got {:?}", other),
        }
    }

    #[test]
    fn machine_id_is_32_hex_chars() {
        let engine = {
            let mut e = AuditEngine::new();
            e.register_provider(Box::new(MachineIdProvider));
            e
        };
        let results = engine.run_audit(None).expect("audit should not fail");
        match results.get(&SignalKind::MachineId) {
            Some(SignalValue::MachineId(id)) => {
                let stripped = id.replace('-', "");
                assert!(
                    stripped.len() == 32 && stripped.chars().all(|c| c.is_ascii_hexdigit()),
                    "machine-id should be 32 hex chars, got: {}",
                    id
                );
            }
            other => panic!("expected MachineId signal, got {:?}", other),
        }
    }

    #[test]
    fn timezone_is_non_empty() {
        let engine = {
            let mut e = AuditEngine::new();
            e.register_provider(Box::new(TimezoneProvider));
            e
        };
        let results = engine.run_audit(None).expect("audit should not fail");
        match results.get(&SignalKind::Timezone) {
            Some(SignalValue::Timezone(tz)) => {
                assert!(!tz.is_empty(), "timezone must not be empty");
            }
            other => panic!("expected Timezone signal, got {:?}", other),
        }
    }

    #[test]
    fn audit_with_no_providers_returns_empty_map() {
        let engine = AuditEngine::new();
        let results = engine.run_audit(None).expect("empty audit should not fail");
        assert!(results.is_empty());
    }
}
