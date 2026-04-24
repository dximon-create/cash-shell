// cash — Conscious Adaptive Secure Host
// © Personal Studio Limited — Confidential

mod agents;
mod marketplace;
mod security;
mod shell;
mod store;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("CASH_LOG"))
        .with_target(false)
        .init();

    // Layer 0 — store
    let store = store::Store::open()?;

    // Layer 2 — security engine (own thread, never blocks shell)
    let sec = security::SecurityEngine::new(store.root.clone())?;
    sec.spawn();

    // Layer 1 — shell owns the process
    let mut sh = shell::Shell::new(store)?;
    sh.run()?;

    Ok(())
}
