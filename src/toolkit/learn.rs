// cash — Learning Mode
//
// Explains every hacking tool before it runs.
// Designed for beginners learning ethical hacking safely.
// Shows: what it does, why hackers use it, ethical rules, legal note.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ToolExplanation {
    pub name:        String,
    pub what:        String,
    pub why_hackers: String,
    pub ethical:     String,
    pub legal:       String,
    pub example:     String,
}

impl ToolExplanation {
    pub fn print(&self) {
        println!("\n╔══ cash learn: {} ══", self.name);
        println!("║");
        println!("║  WHAT IT DOES");
        println!("║  {}", self.what);
        println!("║");
        println!("║  WHY HACKERS USE IT");
        println!("║  {}", self.why_hackers);
        println!("║");
        println!("║  ETHICAL RULE");
        println!("║  {}", self.ethical);
        println!("║");
        println!("║  LEGAL NOTE");
        println!("║  {}", self.legal);
        println!("║");
        println!("║  EXAMPLE");
        println!("║  {}", self.example);
        println!("╚══\n");
    }
}

pub struct LearningMode {
    explanations: HashMap<String, ToolExplanation>,
}

impl LearningMode {
    pub fn new() -> Self {
        let mut explanations = HashMap::new();

        explanations.insert("scan".into(), ToolExplanation {
            name: "scan".into(),
            what: "Discovers devices on a network and finds which ports (services) are open.\nA port is like a door — open ports mean services are running there.".into(),
            why_hackers: "Hackers scan networks to map targets before attacking.\nDefenders scan to find unexpected open ports (security gaps).".into(),
            ethical: "Only scan networks you OWN or have WRITTEN PERMISSION to scan.\nYour home network: OK. School/work/public WiFi: NOT OK without permission.".into(),
            legal: "In the UK, unauthorised scanning can violate the Computer Misuse Act 1990.\nIn the US, it can violate the CFAA. Always get permission in writing.".into(),
            example: "scan 192.168.1.0/24        (scan your home network)\nscan 192.168.1.1 --ports 80,443,22".into(),
        });

        explanations.insert("trace".into(), ToolExplanation {
            name: "trace".into(),
            what: "Shows the path packets take from your computer to a destination.\nEach hop is a router. You see delays at each step.".into(),
            why_hackers: "Used to map network topology and find slow/broken links.\nHelps identify network infrastructure between you and a target.".into(),
            ethical: "Traceroute is generally safe and passive — you're just following packets.\nDon't use results to plan attacks on infrastructure.".into(),
            legal: "Generally legal everywhere — you're only observing your own traffic.\nSome countries restrict certain network probing tools.".into(),
            example: "trace google.com\ntrace 192.168.1.1".into(),
        });

        explanations.insert("dns".into(), ToolExplanation {
            name: "dns".into(),
            what: "Looks up DNS records for a domain — IP addresses, mail servers, name servers.\nDNS is like a phone book for the internet.".into(),
            why_hackers: "DNS enumeration reveals infrastructure — subdomains, mail servers, IPs.\nCan expose internal hostnames if DNS is misconfigured.".into(),
            ethical: "DNS lookups are public and passive — anyone can do them.\nDon't use found information to attack the target.".into(),
            legal: "DNS lookups are legal everywhere — it's public information.\nUsing the information to attack is illegal.".into(),
            example: "dns google.com\ndns google.com --type MX\ndns google.com --type NS".into(),
        });

        explanations.insert("whois".into(), ToolExplanation {
            name: "whois".into(),
            what: "Looks up registration information for a domain or IP address.\nShows owner, registrar, creation date, name servers.".into(),
            why_hackers: "Reveals domain owner contact info, registration dates.\nHelps identify the organisation behind an IP range.".into(),
            ethical: "WHOIS is public information — legal and passive.\nDon't use to harass or stalk domain owners.".into(),
            legal: "Legal everywhere. WHOIS data is public by design.\nGDPR has limited some personal data in WHOIS records.".into(),
            example: "whois google.com\nwhois 8.8.8.8".into(),
        });

        explanations.insert("arp".into(), ToolExplanation {
            name: "arp".into(),
            what: "Shows the ARP table — maps IP addresses to MAC addresses on your local network.\nEvery device has a unique MAC address.".into(),
            why_hackers: "Reveals all devices currently connected to your local network.\nARP poisoning is an attack technique (not covered here).".into(),
            ethical: "Reading ARP table is passive — only shows your local network.\nARP poisoning is an attack — never do it without permission.".into(),
            legal: "Reading your own ARP table is legal everywhere.\nARP poisoning on others' networks is illegal.".into(),
            example: "arp\narp --watch    (update every 5 seconds)".into(),
        });

        explanations.insert("lab".into(), ToolExplanation {
            name: "lab".into(),
            what: "Starts a sandboxed practice environment.\nAll commands run in isolation — cannot reach the real internet.\nPerfect for practising without risk.".into(),
            why_hackers: "Professional hackers use isolated labs to test tools safely.\nCTF (Capture The Flag) competitions use similar environments.".into(),
            ethical: "Always practise in a lab first before using on real networks.\nDocument everything you learn.".into(),
            legal: "Lab mode is completely safe and legal — it's your own isolated environment.".into(),
            example: "lab start\nlab stop\nlab status".into(),
        });

        Self { explanations }
    }

    pub fn explain(&self, tool: &str) -> Option<&ToolExplanation> {
        self.explanations.get(tool)
    }

    pub fn list_tools(&self) {
        println!("\ncash ethical toolkit — available tools:\n");
        let mut tools: Vec<&str> = self.explanations.keys().map(|s| s.as_str()).collect();
        tools.sort();
        for tool in tools {
            if let Some(exp) = self.explanations.get(tool) {
                let first_line = exp.what.lines().next().unwrap_or("");
                println!("  {:10}  {}", tool, first_line);
            }
        }
        println!("\nType 'learn <tool>' for full explanation.");
        println!("Type '<tool> --help' for usage.\n");
    }
}

impl Default for LearningMode {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_tools_have_explanations() {
        let lm = LearningMode::new();
        for tool in &["scan", "trace", "dns", "whois", "arp", "lab"] {
            assert!(lm.explain(tool).is_some(), "missing explanation for {}", tool);
        }
    }

    #[test]
    fn explanation_has_required_fields() {
        let lm = LearningMode::new();
        let exp = lm.explain("scan").unwrap();
        assert!(!exp.what.is_empty());
        assert!(!exp.ethical.is_empty());
        assert!(!exp.legal.is_empty());
        assert!(!exp.example.is_empty());
    }

    #[test]
    fn unknown_tool_returns_none() {
        let lm = LearningMode::new();
        assert!(lm.explain("nonexistent").is_none());
    }
}
