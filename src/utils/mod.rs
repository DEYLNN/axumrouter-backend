use std::net::Ipv4Addr;

/// Try multiple endpoints to detect the public IP of this VPS.
/// Returns "unknown" if all fail.
pub async fn detect_public_ip() -> String {
    let endpoints = [
        "https://ifconfig.me",
        "https://api.ipify.org",
        "https://ipinfo.io/ip",
    ];
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .no_proxy()
        .build()
    {
        Ok(c) => c,
        Err(_) => return "unknown".to_string(),
    };
    for url in &endpoints {
        if let Ok(resp) = client.get(*url).send().await {
            if let Ok(text) = resp.text().await {
                let ip = text.trim().to_string();
                let parts: Vec<&str> = ip.split('.').collect();
                if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
                    return ip;
                }
            }
        }
    }
    "unknown".to_string()
}

/// Validate that a string looks like an IPv4 address.
pub fn is_valid_ipv4(s: &str) -> bool {
    s.parse::<Ipv4Addr>().is_ok()
}
