//! `MCP_TOKEN`: whether `/mcp` is mounted, and the bearer it requires.

use crate::error::{Result, TurfOpsError};
use serde::Deserialize;

/// The MCP endpoint at `/mcp`. `token` unset = the endpoint is not mounted.
#[derive(Clone, Default, Deserialize)]
pub struct McpConfig {
    pub token: Option<String>,
}

// @spec MCP-CFG-004
impl std::fmt::Debug for McpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpConfig")
            .field("token", &self.token.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

/// Shortest bearer token accepted for `/mcp`; the gateway in front of it is open.
const MIN_MCP_TOKEN_LEN: usize = 32;

/// An empty value means "not configured" (compose passes unset keys as ""); a token
/// that is set but short is a config error rather than a silently weak endpoint.
// @spec MCP-CFG-001, MCP-CFG-002, MCP-CFG-003
pub fn parse_token(raw: Option<String>) -> Result<Option<String>> {
    match raw.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) {
        Some(token) if token.len() < MIN_MCP_TOKEN_LEN => Err(TurfOpsError::Config(format!(
            "MCP_TOKEN must be at least {MIN_MCP_TOKEN_LEN} characters"
        ))),
        token => Ok(token),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // @spec MCP-CFG-001
    #[test]
    fn mcp_token_unset_or_empty_is_not_configured() {
        assert_eq!(parse_token(None).unwrap(), None);
        assert_eq!(parse_token(Some(String::new())).unwrap(), None);
        assert_eq!(parse_token(Some("  ".into())).unwrap(), None);
        assert_eq!(parse_token(Some(" \t\n".into())).unwrap(), None);
    }

    // @spec MCP-CFG-002
    #[test]
    fn mcp_token_shorter_than_32_is_rejected() {
        for short in [
            "x".repeat(31),
            format!("  {}  ", "x".repeat(31)),
            "x".into(),
        ] {
            match parse_token(Some(short)) {
                Err(TurfOpsError::Config(msg)) => {
                    assert_eq!(msg, "MCP_TOKEN must be at least 32 characters")
                }
                other => panic!("expected a configuration error, got ok={}", other.is_ok()),
            }
        }
    }

    // @spec MCP-CFG-003
    #[test]
    fn mcp_token_of_32_or_more_is_kept_trimmed() {
        let token = "x".repeat(32);
        assert_eq!(
            parse_token(Some(format!(" {token}\n"))).unwrap(),
            Some(token)
        );
    }

    // @spec MCP-CFG-004
    #[test]
    fn mcp_config_debug_redacts_the_token() {
        let cfg = McpConfig {
            token: Some("secret-secret-secret-secret-secret".into()),
        };
        let printed = format!("{cfg:?}");
        assert!(!printed.contains("secret"));
        assert!(printed.contains("[REDACTED]"), "{printed}");
    }
}
