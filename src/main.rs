// cash — Conscious Adaptive Secure Host
// © Personal Studio Limited — Confidential
//
// Three layers:
//   Layer 1 — Shell:           prompt, repl, parser, executor, built-ins,
//                               resolver, suggestion engine
//   Layer 2 — Security Engine: tamper detection, audit log, agent registry,
//                               vault, network watcher, trust scores
//   Layer 3 — Agent Platform:  runtime, shared memory, message bus,
//                               marketplace

mod agents;
mod toolkit;
mod marketplace;
mod security;
mod shell;
mod store;

fn main() -> anyhow::Result<()> {
    // Internal tracing — set CASH_LOG=debug to see output.
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("CASH_LOG"))
        .with_target(false)
        .init();

    tracing::debug!("cash starting — three layers initialising");

    // --- Layer 0: Store ---
    // Opens ~/.cash/, creates databases, loads config.
    let store = store::Store::open()?;
    tracing::debug!("store ready at {}", store.root.display());

    // --- Layer 3: Agent Platform ---
    // Initialise runtime before security engine so agents can register.
    let _agent_runtime = agents::AgentRuntime::new();
    tracing::debug!("agent runtime ready");

    // --- Layer 2: Security Engine ---
    // Spawns own thread. Never blocks the shell. Cannot be disabled.
    let sec = security::SecurityEngine::new(store.root.clone())?;
    sec.spawn();
    tracing::debug!("security engine spawned");

    // --- Layer 1: Shell ---
    // Takes over the process. Does not return until the user exits.
    let mut sh = shell::Shell::new(store)?;
    sh.run()?;

    tracing::debug!("cash exiting cleanly");
    Ok(())
}
