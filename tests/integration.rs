// cash — Integration Tests

use tempfile::TempDir;

#[test]
fn teach_and_resolve_roundtrip() {
    use cash::store::{Store, memory};
    use cash::store::config::Config;
    use cash::shell::resolver::{resolve, ResolveResult, MatchKind};

    let dir = TempDir::new().unwrap();
    let store = Store { root: dir.path().to_path_buf() };
    memory::init(&store).unwrap();
    memory::teach(&store, "list files", "ls -la").unwrap();

    match resolve("list files", &store, &Config::default()) {
        ResolveResult::Resolved { match_kind, command, .. } => {
            assert_eq!(match_kind, MatchKind::Exact);
            assert_eq!(command, "ls -la");
        }
        other => panic!("expected Resolved, got {:?}", other),
    }
}

#[test]
fn audit_records_and_verifies() {
    use cash::store::{Store, audit, audit::CommandKind};

    let dir = TempDir::new().unwrap();
    let store = Store { root: dir.path().to_path_buf() };
    audit::init(&store).unwrap();
    audit::record(&store, "sess-1", "ls -la", "/home", 0, CommandKind::System).unwrap();
    let tampered = audit::verify_all(&store).unwrap();
    assert!(tampered.is_empty());
}

#[test]
fn agent_runtime_shared_memory() {
    use cash::agents::{AgentRuntime, Agent, AgentPermission};

    let rt = AgentRuntime::new();
    rt.register(Agent::new("a1", "A1", "1.0", vec![
        AgentPermission::SharedMemoryRead,
        AgentPermission::SharedMemoryWrite,
    ])).unwrap();

    rt.mem_write("a1", "key", "value").unwrap();
    assert_eq!(rt.mem_read("a1", "key").unwrap(), Some("value".to_string()));
}

#[test]
fn marketplace_install_and_list() {
    use cash::marketplace::{Installer, verifier::{Verifier, AgentManifest}};
    use cash::store::Store;

    let dir = TempDir::new().unwrap();
    let store = Store { root: dir.path().to_path_buf() };
    let installer = Installer::new(store).unwrap();
    let data = b"agent binary";
    let mut manifest = AgentManifest::new("my-agent", "My Agent", "1.0.0", "main");
    manifest.checksum = Verifier::checksum(data);
    installer.install(&manifest, data).unwrap();
    assert_eq!(installer.list().unwrap().len(), 1);
}

#[test]
fn security_vault_set_get_delete() {
    use cash::security::Vault;

    let dir = TempDir::new().unwrap();
    let vault = Vault::new(dir.path());
    vault.set("token", "secret-value").unwrap();
    assert_eq!(vault.get("token").unwrap(), Some("secret-value".to_string()));
    vault.delete("token").unwrap();
    assert_eq!(vault.get("token").unwrap(), None);
}
