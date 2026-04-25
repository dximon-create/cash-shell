// cash — DNS Lookup Tool

use std::net::ToSocketAddrs;

#[derive(Debug, Clone)]
pub struct DnsResult {
    pub domain:      String,
    pub record_type: String,
    pub value:       String,
}

/// Basic A record lookup using the system resolver.
pub fn lookup_a(domain: &str) -> Vec<DnsResult> {
    let addr = format!("{}:80", domain);
    match addr.to_socket_addrs() {
        Ok(addrs) => addrs.map(|a| DnsResult {
            domain:      domain.to_string(),
            record_type: "A".to_string(),
            value:       a.ip().to_string(),
        }).collect(),
        Err(e) => {
            eprintln!("dns: {}: {}", domain, e);
            vec![]
        }
    }
}

pub fn print_results(results: &[DnsResult]) {
    if results.is_empty() {
        println!("No records found.");
        return;
    }
    println!("\n{:<8} {}", "TYPE", "VALUE");
    println!("{}", "─".repeat(40));
    for r in results {
        println!("{:<8} {}", r.record_type, r.value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_returns_vec() {
        // We can't guarantee network in test env, just verify it doesn't panic.
        let results = lookup_a("localhost");
        let _ = results;
    }

    #[test]
    fn dns_result_has_fields() {
        let r = DnsResult {
            domain: "example.com".into(),
            record_type: "A".into(),
            value: "93.184.216.34".into(),
        };
        assert_eq!(r.record_type, "A");
    }
}
