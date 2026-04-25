// cash — ARP table reader

#[derive(Debug, Clone)]
pub struct ArpEntry {
    pub ip:        String,
    pub mac:       String,
    pub interface: String,
}

/// Read the ARP table from /proc/net/arp on Linux.
pub fn read_arp_table() -> Vec<ArpEntry> {
    let mut entries = Vec::new();

    #[cfg(target_os = "linux")]
    {
        let Ok(content) = std::fs::read_to_string("/proc/net/arp") else { return entries };
        for line in content.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 6 {
                entries.push(ArpEntry {
                    ip:        fields[0].to_string(),
                    mac:       fields[3].to_string(),
                    interface: fields[5].to_string(),
                });
            }
        }
    }

    entries
}

pub fn print_arp_table(entries: &[ArpEntry]) {
    if entries.is_empty() {
        println!("ARP table is empty or not available on this platform.");
        return;
    }
    println!("\n{:<18} {:<20} {}", "IP ADDRESS", "MAC ADDRESS", "INTERFACE");
    println!("{}", "─".repeat(50));
    for e in entries {
        println!("{:<18} {:<20} {}", e.ip, e.mac, e.interface);
    }
    println!("\n{} device(s) found on local network.", entries.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_arp_returns_vec() {
        let entries = read_arp_table();
        let _ = entries; // Just verify no panic
    }

    #[test]
    fn arp_entry_has_fields() {
        let e = ArpEntry {
            ip: "192.168.1.1".into(),
            mac: "aa:bb:cc:dd:ee:ff".into(),
            interface: "eth0".into(),
        };
        assert_eq!(e.ip, "192.168.1.1");
    }
}
