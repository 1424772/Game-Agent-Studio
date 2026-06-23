use crate::models::sanitize_error;
use std::net::IpAddr;

const MAX_MESSAGES: usize = 20;
const MAX_TOTAL_CHARS: usize = 40000;
pub const MAX_TOKENS_CAP: u32 = 32768;
const REQUEST_TIMEOUT_SECS: u64 = 60;

fn is_dev_mode() -> bool {
    cfg!(debug_assertions)
}

pub fn validate_base_url(base_url: &str) -> Result<(), String> {
    let trimmed = base_url.trim();

    if trimmed.is_empty() {
        return Err("Base URL must not be empty".to_string());
    }

    let parsed = url::Url::parse(trimmed).map_err(|e| {
        sanitize_error(format!("Invalid Base URL: {}", e))
    })?;

    let host = parsed.host_str().unwrap_or("");

    if host.is_empty() {
        return Err(sanitize_error(format!(
            "Base URL has no host: {}",
            trimmed
        )));
    }

    match parsed.scheme() {
        "https" => {}
        "http" => {
            if !is_allowed_localhost_for_mode(host, is_dev_mode()) {
                return Err(
                    "Base URL must use HTTPS. HTTP is only allowed for localhost in development mode."
                        .to_string(),
                );
            }
        }
        other => {
            return Err(sanitize_error(format!(
                "Unsupported protocol '{}'. Only https:// is allowed.",
                other
            )));
        }
    }

    validate_host_for_mode(host, is_dev_mode())?;

    Ok(())
}

fn is_allowed_localhost_for_mode(host: &str, allow_local_dev: bool) -> bool {
    if allow_local_dev {
        host == "localhost" || host == "127.0.0.1" || host == "::1"
    } else {
        false
    }
}

fn is_allowed_ip_for_mode(ip: IpAddr, allow_local_dev: bool) -> bool {
    if allow_local_dev {
        return ip.is_loopback();
    }
    !is_disallowed_ip(ip)
}

fn is_localhost_hostname(host: &str) -> bool {
    let lower = host.to_lowercase();
    lower == "localhost" || lower.starts_with("localhost.")
}

pub fn validate_host_for_mode(host: &str, allow_local_dev: bool) -> Result<(), String> {
    if host.is_empty() {
        return Err("Host must not be empty".to_string());
    }

    if allow_local_dev && is_exact_localhost_hostname(host) {
        return Ok(());
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if !is_allowed_ip_for_mode(ip, allow_local_dev) {
            return Err(sanitize_error(format!(
                "IP address not allowed: {}",
                host
            )));
        }
        return Ok(());
    }

    if is_localhost_hostname(host) {
        return Err(sanitize_error(format!(
            "Hostname not allowed: {}",
            host
        )));
    }

    Ok(())
}

fn is_exact_localhost_hostname(host: &str) -> bool {
    host == "localhost" || host == "127.0.0.1" || host == "::1"
}

fn validate_host(host: &str) -> Result<(), String> {
    validate_host_for_mode(host, is_dev_mode())
}

fn is_loopback(ip: IpAddr) -> bool {
    ip.is_loopback()
}

fn is_ipv6_link_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V6(v6) => {
            let segs = v6.segments();
            segs[0] & 0xFFC0 == 0xFE80
        }
        _ => false,
    }
}

fn is_disallowed_ip(ip: IpAddr) -> bool {
    is_loopback(ip)
        || is_ip_private(ip)
        || ip.is_unspecified()
        || is_ipv4_link_local(ip)
        || is_ipv6_link_local(ip)
}

fn is_ip_private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            (octets[0] == 10)
                || (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31)
                || (octets[0] == 192 && octets[1] == 168)
                || (octets[0] == 100 && octets[1] >= 64 && octets[1] <= 127)
        }
        IpAddr::V6(v6) => {
            let segs = v6.segments();
            (segs[0] & 0xFE00) == 0xFC00
        }
    }
}

fn is_ipv4_link_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            octets[0] == 169 && octets[1] == 254
        }
        _ => false,
    }
}

fn is_allowed_ip(ip: IpAddr) -> bool {
    is_allowed_ip_for_mode(ip, is_dev_mode())
}

pub fn validate_max_tokens(max_tokens: u32) -> Result<(), String> {
    if max_tokens > MAX_TOKENS_CAP {
        return Err(format!(
            "max_tokens ({}) exceeds cap ({}).",
            max_tokens, MAX_TOKENS_CAP
        ));
    }
    Ok(())
}

pub fn validate_llm_request(
    message_count: usize,
    total_chars: usize,
    max_tokens: u32,
) -> Result<(), String> {
    if message_count > MAX_MESSAGES {
        return Err(format!(
            "Too many messages ({}). Maximum is {}.",
            message_count, MAX_MESSAGES
        ));
    }

    if total_chars > MAX_TOTAL_CHARS {
        return Err(format!(
            "Total message content too large ({} chars). Maximum is {}.",
            total_chars, MAX_TOTAL_CHARS
        ));
    }

    validate_max_tokens(max_tokens)?;

    Ok(())
}

pub fn build_reqwest_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .expect("Failed to build reqwest client")
}

/// Authoritative sorted list of allowed business commands. Must match lib.rs HANDLER_NAMES.
pub const ALLOWED_COMMANDS: &[&str] = &[
    "chunk_document",
    "create_agent_run",
    "create_document",
    "create_improvement_proposal",
    "create_project",
    "delete_project",
    "embed_pending_chunks",
    "export_json",
    "export_markdown",
    "get_agent_messages",
    "get_agent_run",
    "get_agent_runs",
    "get_agent_steps",
    "get_document_chunks",
    "get_events",
    "get_exports",
    "get_memory_versions",
    "get_model_config",
    "get_project",
    "get_project_memory",
    "get_retrieval_hit_excerpts",
    "get_retrieval_hits",
    "get_retrieval_runs",
    "get_user_preferences",
    "list_documents",
    "list_improvement_proposals",
    "list_projects",
    "log_event",
    "review_improvement_proposal",
    "run_llm_completion",
    "run_workflow",
    "save_agent_message",
    "save_agent_step",
    "save_model_config",
    "save_project_memory",
    "search_documents",
    "test_model_connection",
    "update_agent_message_content",
    "update_agent_run",
    "update_message_status",
    "update_user_preferences",
];

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Command manifest — parse generate_handler![] from lib.rs source
    // ============================================================

    #[test]
    fn command_manifest_matches_actual_handler() {
        let source = include_str!("../lib.rs");

        let handler_names = extract_handler_command_names(source);
        let mut handler_sorted = handler_names.clone();
        handler_sorted.sort();
        handler_sorted.dedup();

        let mut allowed: Vec<&str> = ALLOWED_COMMANDS.to_vec();
        allowed.sort();

        assert_eq!(
            allowed, handler_sorted,
            "ALLOWED_COMMANDS must exactly match generate_handler![] entries in lib.rs.\n\
             Handler has {} entries, ALLOWED_COMMANDS has {}.\n\
             If you added/removed/renamed a command in generate_handler![], update ALLOWED_COMMANDS.",
            handler_sorted.len(),
            allowed.len()
        );
    }

    fn extract_handler_command_names(source: &str) -> Vec<String> {
        let mut names = Vec::new();

        let start_marker = "generate_handler![";
        let start = source.find(start_marker)
            .expect("generate_handler![ not found in lib.rs");
        let after_start = &source[start + start_marker.len()..];

        let end = after_start.find(']')
            .expect("Closing ] of generate_handler![ not found");
        let block = &after_start[..end];

        for line in block.lines() {
            let trimmed = line.trim().trim_end_matches(',');
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }
            if let Some(last_seg) = trimmed.rsplit("::").next() {
                let name = last_seg.trim();
                if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                    names.push(name.to_string());
                }
            }
        }

        names
    }

    // ============================================================
    // scheme / empty / file / ftp
    // ============================================================

    #[test]
    fn allow_https_openai() {
        assert!(validate_base_url("https://api.openai.com").is_ok());
    }

    #[test]
    fn allow_https_deepseek() {
        assert!(validate_base_url("https://api.deepseek.com").is_ok());
    }

    #[test]
    fn allow_https_with_path() {
        assert!(validate_base_url("https://api.deepseek.com/v1").is_ok());
    }

    #[test]
    fn reject_https_empty_host() {
        assert!(validate_base_url("https://").is_err());
    }

    #[test]
    fn reject_file_scheme() {
        assert!(validate_base_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn reject_ftp_scheme() {
        assert!(validate_base_url("ftp://evil.com").is_err());
    }

    #[test]
    fn reject_javascript_scheme() {
        assert!(validate_base_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn reject_data_scheme() {
        assert!(validate_base_url("data:text/html,<script>alert(1)</script>").is_err());
    }

    #[test]
    fn reject_empty_string() {
        assert!(validate_base_url("").is_err());
    }

    #[test]
    fn reject_http_production() {
        assert!(validate_base_url("http://example.com").is_err());
    }

    // ============================================================
    // prefix bypass — all variants of localhost.* / 127.0.0.1.* rejected
    // ============================================================

    #[test]
    fn reject_http_localhost_evil_prefix() {
        assert!(validate_base_url("http://localhost.evil.com").is_err());
    }

    #[test]
    fn reject_http_127_0_0_1_evil_prefix() {
        assert!(validate_base_url("http://127.0.0.1.evil.com").is_err());
    }

    #[test]
    fn reject_https_localhost_evil_prefix() {
        assert!(
            validate_base_url("https://localhost.evil.com").is_err(),
            "https://localhost.evil.com must be rejected (localhost.* prefix)"
        );
    }

    #[test]
    fn reject_https_localhost_subdomain() {
        assert!(
            validate_base_url("https://localhost.anything.example.com").is_err(),
            "https://localhost.anything.example.com must be rejected"
        );
    }

    // ============================================================
    // validate_host_for_mode — explicit prod (allow_local_dev=false)
    // ============================================================

    fn assert_prod_rejects(host: &str) {
        assert!(
            validate_host_for_mode(host, false).is_err(),
            "prod should reject host: {}",
            host
        );
    }

    fn assert_prod_allows(host: &str) {
        assert!(
            validate_host_for_mode(host, false).is_ok(),
            "prod should allow host: {}",
            host
        );
    }

    fn assert_dev_allows(host: &str) {
        assert!(
            validate_host_for_mode(host, true).is_ok(),
            "dev should allow host: {}",
            host
        );
    }

    fn assert_dev_rejects(host: &str) {
        assert!(
            validate_host_for_mode(host, true).is_err(),
            "dev should reject host: {}",
            host
        );
    }

    #[test]
    fn prod_rejects_localhost() {
        assert_prod_rejects("localhost");
    }

    #[test]
    fn prod_rejects_localhost_sub() {
        assert_prod_rejects("localhost.evil.com");
    }

    #[test]
    fn prod_rejects_127_0_0_1() {
        assert_prod_rejects("127.0.0.1");
    }

    #[test]
    fn prod_rejects_ipv6_loopback() {
        assert_prod_rejects("::1");
    }

    #[test]
    fn prod_rejects_ipv4_private_10() {
        assert_prod_rejects("10.0.0.1");
    }

    #[test]
    fn prod_rejects_ipv4_private_172() {
        assert_prod_rejects("172.16.0.1");
    }

    #[test]
    fn prod_rejects_ipv4_private_192() {
        assert_prod_rejects("192.168.1.1");
    }

    #[test]
    fn prod_rejects_ipv6_private_fc00() {
        assert_prod_rejects("fc00::1");
    }

    #[test]
    fn prod_rejects_ipv6_private_fd00() {
        assert_prod_rejects("fd00::1");
    }

    #[test]
    fn prod_rejects_ipv4_link_local() {
        assert_prod_rejects("169.254.1.1");
    }

    #[test]
    fn prod_rejects_ipv6_link_local() {
        assert_prod_rejects("fe80::1");
    }

    #[test]
    fn prod_rejects_ipv4_unspecified() {
        assert_prod_rejects("0.0.0.0");
    }

    #[test]
    fn prod_rejects_ipv6_unspecified() {
        assert_prod_rejects("::");
    }

    #[test]
    fn prod_allows_public_api() {
        assert_prod_allows("api.openai.com");
        assert_prod_allows("api.deepseek.com");
    }

    // ============================================================
    // validate_host_for_mode — explicit dev (allow_local_dev=true)
    // ============================================================

    #[test]
    fn dev_allows_exact_localhost() {
        assert_dev_allows("localhost");
    }

    #[test]
    fn dev_allows_exact_127() {
        assert_dev_allows("127.0.0.1");
    }

    #[test]
    fn dev_allows_exact_ipv6_loopback() {
        assert_dev_allows("::1");
    }

    #[test]
    fn dev_rejects_localhost_evil() {
        assert_dev_rejects("localhost.evil.com");
    }

    #[test]
    fn dev_rejects_private() {
        assert_dev_rejects("192.168.1.1");
        assert_dev_rejects("10.0.0.1");
    }

    #[test]
    fn dev_rejects_link_local() {
        assert_dev_rejects("169.254.1.1");
        assert_dev_rejects("fe80::1");
    }

    #[test]
    fn dev_allows_public() {
        assert_dev_allows("api.openai.com");
    }

    // ============================================================
    // https://localhost in production context
    // ============================================================

    #[test]
    fn reject_https_localhost() {
        assert!(validate_base_url("https://localhost").is_err());
    }

    #[test]
    fn reject_https_localhost_with_port() {
        assert!(validate_base_url("https://localhost:8443").is_err());
    }

    #[test]
    fn reject_https_127_0_0_1() {
        assert!(validate_base_url("https://127.0.0.1").is_err());
    }

    #[test]
    fn reject_https_ipv6_loopback() {
        assert!(validate_base_url("https://[::1]").is_err());
    }

    #[test]
    fn reject_https_ipv6_link_local() {
        assert!(validate_base_url("https://[fe80::1]").is_err());
    }

    #[test]
    fn reject_https_ipv6_link_local_alt() {
        assert!(validate_base_url("https://[fe80::abcd:1]").is_err());
    }

    #[test]
    fn reject_https_ipv4_link_local() {
        assert!(validate_base_url("https://169.254.1.1").is_err());
    }

    #[test]
    fn reject_https_ipv4_private_192() {
        assert!(validate_base_url("https://192.168.1.1").is_err());
    }

    #[test]
    fn reject_https_ipv4_private_10() {
        assert!(validate_base_url("https://10.0.0.1").is_err());
    }

    #[test]
    fn reject_https_ipv4_private_172() {
        assert!(validate_base_url("https://172.16.0.1").is_err());
    }

    #[test]
    fn reject_https_ipv6_private_fc00() {
        assert!(validate_base_url("https://[fc00::1]").is_err());
    }

    #[test]
    fn reject_https_ipv6_private_fd00() {
        assert!(validate_base_url("https://[fd00::1]").is_err());
    }

    #[test]
    fn reject_https_ipv4_unspecified() {
        assert!(validate_base_url("https://0.0.0.0").is_err());
    }

    #[test]
    fn reject_https_ipv6_unspecified() {
        assert!(validate_base_url("https://[::]").is_err());
    }

    // ============================================================
    // IPv6 link-local detection unit
    // ============================================================

    #[test]
    fn is_ipv6_link_local_detection() {
        assert!(is_ipv6_link_local("fe80::1".parse().unwrap()));
        assert!(is_ipv6_link_local("fe80::abcd:1".parse().unwrap()));
        assert!(is_ipv6_link_local("feb0::1".parse().unwrap()));
        assert!(!is_ipv6_link_local("2001:db8::1".parse().unwrap()));
        assert!(!is_ipv6_link_local("::1".parse().unwrap()));
    }

    // ============================================================
    // dev mode helpers (cfg-gated, unit-testable)
    // ============================================================

    #[test]
    fn dev_is_allowed_localhost_exact() {
        assert!(is_allowed_localhost_for_mode("localhost", true));
        assert!(is_allowed_localhost_for_mode("127.0.0.1", true));
        assert!(is_allowed_localhost_for_mode("::1", true));
    }

    #[test]
    fn dev_is_allowed_localhost_rejects_evil() {
        assert!(!is_allowed_localhost_for_mode("localhost.evil.com", true));
        assert!(!is_allowed_localhost_for_mode("evil.com", true));
    }

    #[test]
    fn dev_is_allowed_ip_allows_loopback() {
        assert!(is_allowed_ip_for_mode("127.0.0.1".parse().unwrap(), true));
        assert!(is_allowed_ip_for_mode("::1".parse().unwrap(), true));
    }

    #[test]
    fn dev_is_allowed_ip_rejects_private() {
        assert!(!is_allowed_ip_for_mode("192.168.1.1".parse().unwrap(), true));
        assert!(!is_allowed_ip_for_mode("10.0.0.1".parse().unwrap(), true));
    }

    #[test]
    fn dev_is_allowed_ip_rejects_link_local() {
        assert!(!is_allowed_ip_for_mode("169.254.1.1".parse().unwrap(), true));
        assert!(!is_allowed_ip_for_mode("fe80::1".parse().unwrap(), true));
    }

    #[test]
    fn prod_is_allowed_ip_rejects_loopback() {
        assert!(!is_allowed_ip_for_mode("127.0.0.1".parse().unwrap(), false));
        assert!(!is_allowed_ip_for_mode("::1".parse().unwrap(), false));
    }

    #[test]
    fn prod_is_allowed_ip_allows_public() {
        assert!(is_allowed_ip_for_mode("8.8.8.8".parse().unwrap(), false));
        assert!(is_allowed_ip_for_mode("2001:db8::1".parse().unwrap(), false));
    }

    // ============================================================
    // max_tokens
    // ============================================================

    #[test]
    fn validate_max_tokens_under_cap() {
        assert!(validate_max_tokens(4096).is_ok());
        assert!(validate_max_tokens(32768).is_ok());
    }

    #[test]
    fn validate_max_tokens_over_cap() {
        assert!(validate_max_tokens(32769).is_err());
        assert!(validate_max_tokens(131072).is_err());
    }

    // ============================================================
    // sanitize_error coverage
    // ============================================================

    #[test]
    fn sanitize_bearer_token() {
        let input = "Authorization: Bearer sk-abc123def456ghij789".to_string();
        let output = crate::models::sanitize_error(input);
        assert!(!output.contains("sk-abc123"));
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn sanitize_authorization_header() {
        let input = "Authorization: secret-value-here".to_string();
        let output = crate::models::sanitize_error(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("secret-value-here"));
    }

    #[test]
    fn sanitize_api_key_param() {
        let input = "api_key=sk-mysecretkey123".to_string();
        let output = crate::models::sanitize_error(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("mysecretkey"));
    }

    #[test]
    fn sanitize_sk_prefix() {
        let input = "Error: sk-proj-1234567890abcdefghij".to_string();
        let output = crate::models::sanitize_error(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("sk-proj-1234"));
    }

    #[test]
    fn sanitize_clean_text_passes_through() {
        let input = "Normal error message without secrets".to_string();
        let output = crate::models::sanitize_error(input.clone());
        assert_eq!(input, output);
    }

    // ============================================================
    // LLM request validation
    // ============================================================

    #[test]
    fn validate_llm_request_limits() {
        assert!(validate_llm_request(1, 100, 4096).is_ok());
        assert!(validate_llm_request(20, 40000, 32768).is_ok());
        assert!(validate_llm_request(21, 100, 4096).is_err(), "too many messages");
        assert!(validate_llm_request(1, 50000, 4096).is_err(), "too many chars");
        assert!(validate_llm_request(1, 100, 32769).is_err(), "tokens over cap");
    }
}
