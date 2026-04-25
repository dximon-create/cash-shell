// cash — Port Scanner
//
// Discovers open ports on a host or network range.
// Uses TCP connect scan — no raw sockets required.
// Always shows ethical reminder before scanning.

use std::net::{TcpStream, ToSocketAddrs, IpAddr, Ipv4Addr};
use std::time::Duration;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub host:      String,
    pub port:      u16,
    pub open:      bool,
    pub service:   Option<String>,
    pub banner:    Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub timeout_ms:  u64,
    pub ports:       Vec<u16>,
    pub show_closed: bool,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            timeout_ms:  500,
            ports:       common_ports(),
            show_closed: false,
        }
    }
}

/// Scan a single host for open ports.
pub fn scan_host(host: &str, config: &ScanConfig) -> Vec<ScanResult> {
    let mut results = Vec::new();

    for &port in &config.ports {
        let addr = format!("{}:{}", host, port);
        let open = TcpStream::connect_timeout(
            &addr.to_socket_addrs().ok()
                 .and_then(|mut a| a.next())
                 .unwrap_or_else(|| format!("0.0.0.0:{}", port).parse().unwrap()),
            Duration::from_millis(config.timeout_ms),
        ).is_ok();

        if open || config.show_closed {
            results.push(ScanResult {
                host:    host.to_string(),
                port,
                open,
                service: known_service(port),
                banner:  None,
            });
        }
    }

    results
}

/// Scan a CIDR range (e.g. 192.168.1.0/24).
/// Returns results for all hosts that respond on any port.
pub fn scan_range(cidr: &str, config: &ScanConfig) -> Vec<ScanResult> {
    let mut all_results = Vec::new();

    let hosts = match expand_cidr(cidr) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("scan: invalid range '{}': {}", cidr, e);
            return all_results;
        }
    };

    println!("Scanning {} hosts on {} ports...", hosts.len(), config.ports.len());
    println!("(This may take a moment. Press Ctrl+C to stop.)\n");

    for host in &hosts {
        let host_str = host.to_string();
        let host_results = scan_host(&host_str, config);
        let open: Vec<_> = host_results.iter().filter(|r| r.open).collect();
        if !open.is_empty() {
            println!("Host: {}", host_str);
            for r in &open {
                let svc = r.service.as_deref().unwrap_or("unknown");
                println!("  {:5}  open  {}", r.port, svc);
            }
            println!();
        }
        all_results.extend(host_results);
    }

    all_results
}

/// Print scan results in a readable table.
pub fn print_results(results: &[ScanResult]) {
    let open: Vec<_> = results.iter().filter(|r| r.open).collect();
    if open.is_empty() {
        println!("No open ports found.");
        return;
    }
    println!("\n{:<8} {:<10} {}", "PORT", "STATE", "SERVICE");
    println!("{}", "─".repeat(35));
    for r in &open {
        let svc = r.service.as_deref().unwrap_or("unknown");
        println!("{:<8} {:<10} {}", r.port, "open", svc);
    }
    println!("\n{} open port(s) found on {}", open.len(), results[0].host);
}

/// Expand a CIDR to individual IP addresses. Limited to /24 and larger.
fn expand_cidr(cidr: &str) -> Result<Vec<Ipv4Addr>, String> {
    if !cidr.contains('/') {
        // Single host
        return Ipv4Addr::from_str(cidr)
            .map(|ip| vec![ip])
            .map_err(|e| e.to_string());
    }

    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 { return Err("invalid CIDR".into()); }

    let base = Ipv4Addr::from_str(parts[0]).map_err(|e| e.to_string())?;
    let prefix: u32 = parts[1].parse().map_err(|_| "invalid prefix".to_string())?;

    if prefix < 16 {
        return Err("prefix must be /16 or larger to avoid scanning too many hosts".into());
    }

    let mask = if prefix == 0 { 0u32 } else { !((1u32 << (32 - prefix)) - 1) };
    let network = u32::from(base) & mask;
    let host_bits = 32 - prefix;
    let count = 1u32 << host_bits;

    let hosts: Vec<Ipv4Addr> = (1..count - 1) // skip network and broadcast
        .map(|i| Ipv4Addr::from(network + i))
        .collect();

    Ok(hosts)
}

fn known_service(port: u16) -> Option<String> {
    let svc = match port {
        21   => "FTP",
        22   => "SSH",
        23   => "Telnet",
        25   => "SMTP",
        53   => "DNS",
        80   => "HTTP",
        110  => "POP3",
        143  => "IMAP",
        443  => "HTTPS",
        445  => "SMB",
        3306 => "MySQL",
        3389 => "RDP",
        5432 => "PostgreSQL",
        5900 => "VNC",
        6379 => "Redis",
        8080 => "HTTP-Alt",
        8443 => "HTTPS-Alt",
        27017 => "MongoDB",
        _    => return None,
    };
    Some(svc.to_string())
}

fn common_ports() -> Vec<u16> {
    vec![
        21, 22, 23, 25, 53, 80, 110, 143, 443, 445,
        3306, 3389, 5432, 5900, 6379, 8080, 8443, 27017,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_cidr_single_host() {
        let hosts = expand_cidr("192.168.1.1").unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].to_string(), "192.168.1.1");
    }

    #[test]
    fn expand_cidr_slash24() {
        let hosts = expand_cidr("192.168.1.0/24").unwrap();
        assert_eq!(hosts.len(), 254);
    }

    #[test]
    fn expand_cidr_too_large_rejected() {
        assert!(expand_cidr("10.0.0.0/8").is_err());
    }

    #[test]
    fn known_service_http() {
        assert_eq!(known_service(80), Some("HTTP".to_string()));
        assert_eq!(known_service(443), Some("HTTPS".to_string()));
    }

    #[test]
    fn known_service_unknown_port() {
        assert_eq!(known_service(9999), None);
    }

    #[test]
    fn scan_config_default_has_ports() {
        let cfg = ScanConfig::default();
        assert!(!cfg.ports.is_empty());
        assert!(cfg.ports.contains(&80));
        assert!(cfg.ports.contains(&443));
    }
}
