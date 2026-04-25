# cash — Conscious Adaptive Secure Host

A standalone, first-class shell written in Rust. Not a wrapper over Bash or Zsh.

---

## What makes cash different

| Feature | Bash | Zsh | Fish | cash |
|---|---|---|---|---|
| Natural language commands | ✗ | ✗ | ✗ | ✅ |
| Teachable aliases | ✗ | ✗ | ✗ | ✅ |
| Built-in security engine | ✗ | ✗ | ✗ | ✅ |
| Process + network monitor | ✗ | ✗ | ✗ | ✅ |
| Audit log (SHA-256 hashed) | ✗ | ✗ | ✗ | ✅ |
| Tamper detection | ✗ | ✗ | ✗ | ✅ |
| AI intent engine | ✗ | ✗ | ✗ | ✅ |
| Agent platform | ✗ | ✗ | ✗ | ✅ |
| Tool manager | ✗ | ✗ | ✗ | ✅ |
| Syntax highlighting | ✗ | Plugin | ✅ | ✅ |
| Tab completion | Basic | Good | ✅ | ✅ |
| Written in | C | C | C++ | **Rust** |

---

## Three Layers

**Layer 1 — The Shell**
Natural language. Teachable. Suggests corrections. Never fails silently.
Tab completion. Syntax highlighting. Ctrl+R history search. Job control.

**Layer 2 — Security Engine**
Process monitor. Network monitor. Audit log. Tamper detection.
Real-time alerts. Trust scores. Vault. Runs on separate thread. Cannot be disabled.

**Layer 3 — Agent Platform**
Native AI agent runtime. Shared memory. Message bus. Marketplace.
Agents think via AI APIs. Act via CLI tools.

---

## Quick Start

```bash
git clone https://github.com/dximon-create/cash-shell
cd cash-shell
cargo build --release
./target/release/cash
```

---

## Built-in Commands

```bash
show [path]              # list directory — dirs first, human sizes
go [path / ~ / -]        # change directory — go - returns to previous
copy [-r] <src> <dst>    # copy file or directory
move <src> <dst>         # move or rename
remove [-rf] <targets>   # delete — confirms destructive operations
teach <name> '<cmd>'     # teach cash a natural language alias
help [command]           # show help
exit                     # exit
```

## Natural Language

```bash
list files               # runs ls -la (exact match)
list                     # runs ls -la (pattern match)
lsit feles               # "did you mean list files?" (fuzzy)
scan my network          # discovers hosts and open ports
set up my router         # guides through router security check
check for intrusion      # reviews security logs and monitors
install nmap             # installs via apt/brew, registers in cash
```

## Security Commands

```bash
scan 192.168.1.0/24      # port scan with nmap
vuln http://target       # web vulnerability scan with nikto
dirs http://target       # directory discovery with gobuster
ssl example.com          # SSL certificate inspection
http example.com         # HTTP header inspection
```

## Ethical Hacking Toolkit

```bash
cash install nmap        # install security tools
cash install nikto
cash tools               # list installed tools
learn scan               # explains what scan does + ethical rules
lab start                # sandboxed practice environment
arp                      # show devices on local network
trace google.com         # traceroute
dns example.com          # DNS lookup
```

## Shell Features

```bash
ls | grep src            # pipes
echo hi > file.txt       # redirects
FOO=bar command          # environment variables
sleep 100 &              # background jobs
jobs                     # list background jobs
# Tab                    # completion
# Ctrl+R                 # history search
```

---

## Security Features

Every command is logged to `~/.cash/audit.db` with SHA-256 hash.
Tamper detection watches `~/.cash/` — any outside modification is flagged.
Process monitor detects suspicious processes (nmap, metasploit, chmod +s).
Network monitor logs every outbound connection.
Security engine runs on a supervised thread — restarts automatically if it crashes.

---

## Build

**Requirements:** Rust 1.75+, Linux / macOS / WSL2

```bash
cargo build --release
cargo test              # 164 tests
```

---

## Status

**164 tests passing. Active development.**

| Component | Status |
|---|---|
| Core shell | ✅ Complete |
| Security engine | ✅ Complete |
| Agent platform | ✅ Complete |
| Marketplace | ✅ Complete |
| Ethical toolkit | ✅ Complete |
| Tool manager | ✅ Complete |
| Tab completion | ✅ Complete |
| Syntax highlighting | ✅ Complete |
| AI intent engine | ✅ Complete |
| Job control | ✅ Complete |
| WhatsApp alerts | 🔲 Planned |
| Mac testing | 🔲 In progress |

---

© Personal Studio Limited
