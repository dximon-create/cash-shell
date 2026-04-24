# cash — Conscious Adaptive Secure Host

A standalone, first-class shell written in Rust.

cash is not a wrapper over Bash or Zsh. It owns the process, the prompt, the environment, I/O, signals, and job control directly. It talks to the OS via `fork()` and `exec()`.

---

## Three Layers

**Layer 1 — The Shell**
Natural language commands. Teachable. Suggests corrections. Never fails silently.

**Layer 2 — Security Engine**
Audit log. Behaviour baseline. Anomaly detection. Agent registration. Permission walls. Network watching. Vault protection. Tamper detection. Trust scores. Sandbox mode. Runs on a separate thread. Never blocks the shell.

**Layer 3 — Agent Platform**
Native runtime for AI agents. Shared memory. Agent-to-agent messaging via Unix domain sockets. Agent marketplace. Token-efficient by design. Agents use CLI tools to act, AI APIs only to think.

---

## Built-in Commands

| Command | Description |
|---|---|
| `show [path]` | List directory contents |
| `go [path]` | Change directory (`go -` for previous) |
| `copy [-r] <src> <dst>` | Copy file or directory |
| `move <src> <dst>` | Move or rename |
| `remove [-rf] <targets>` | Delete (confirms destructive ops) |
| `teach <name> '<cmd>'` | Teach cash a natural language alias |
| `help [command]` | Show help |
| `exit [code]` | Exit the shell |

---

## Shell Features

- Pipes: `cmd1 | cmd2 | cmd3`
- Redirects: `>` `>>` `<` `2>`
- Environment variables: `VAR=value command`
- Natural language resolver: exact → pattern → fuzzy match
- Suggestion engine: typo detection, taught command hints
- Command history in `~/.cash/history.db`
- Audit log in `~/.cash/audit.db` (SHA-256 hashed, append-only)
- Tamper detection: watches `~/.cash/` for outside modification

---

## Build

**Requirements:** Rust 1.75+, Linux or macOS (WSL on Windows)

```bash
git clone https://github.com/dximon-create/cash-shell
cd cash-shell
cargo build --release
./target/release/cash
```

**Run tests:**
```bash
cargo test
```

---

## Status

Under active development. All 13 modules complete.

| Module | Status |
|---|---|
| 1. Scaffold | ✅ |
| 2. Read-eval loop | ✅ |
| 3. Executor (fork/exec/pipes) | ✅ |
| 4. Built-in commands | ✅ |
| 5. Memory Store | ✅ |
| 6. Resolver | ✅ |
| 7. Suggestion Engine | ✅ |
| 8. Audit Log | ✅ |
| 9. Tamper Detection | ✅ |
| 10. Security Engine | ✅ |
| 11. Agent Platform | ✅ |
| 12. Marketplace | ✅ |
| 13. Integration | ✅ |

---

© Personal Studio Limited — Confidential
