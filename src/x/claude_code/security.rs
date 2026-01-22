use crate::x::claude_code::config::Config;
use anyhow::{Context, Result};
use llm_json::{RepairOptions, loads};
use regex::Regex;
use rust_i18n::t;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use dirs;

/// Represents a security warning for dangerous Claude Code settings
#[derive(Debug, Clone)]
pub struct SecurityWarning {
    pub config_path: String,
    pub config_item: String,
    pub reason: String,
    pub severity: SecurityWarningSeverity,
    pub matched_pattern: String,
    pub description: String,
    pub recommendation: String,
}

/// Severity levels for security warnings
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityWarningSeverity {
    Critical, // Can cause system damage or data loss
    High,     // Can compromise security or privacy
    Medium,   // Potentially risky operations
    Low,      // Minor security concerns
}

impl SecurityWarningSeverity {
    pub fn display_symbol(&self) -> &'static str {
        match self {
            SecurityWarningSeverity::Critical => "🚨",
            SecurityWarningSeverity::High => "⚠️",
            SecurityWarningSeverity::Medium => "⚡",
            SecurityWarningSeverity::Low => "ℹ️",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SecurityWarningSeverity::Critical => "CRITICAL",
            SecurityWarningSeverity::High => "HIGH",
            SecurityWarningSeverity::Medium => "MEDIUM",
            SecurityWarningSeverity::Low => "LOW",
        }
    }

    pub fn display_name_localized(&self) -> String {
        match self {
            SecurityWarningSeverity::Critical => {
                t!("claude_code.security.severity.critical").to_string()
            }
            SecurityWarningSeverity::High => t!("claude_code.security.severity.high").to_string(),
            SecurityWarningSeverity::Medium => {
                t!("claude_code.security.severity.medium").to_string()
            }
            SecurityWarningSeverity::Low => t!("claude_code.security.severity.low").to_string(),
        }
    }
}

/// Security checker for Claude Code settings
pub struct SecurityChecker {
    dangerous_patterns: Vec<String>,
    settings_files: Vec<String>,
    enabled: bool,
}

impl SecurityChecker {
    /// Create a new SecurityChecker from configuration
    pub fn from_config(config: &Config) -> Result<Self> {
        let security_config = config.security.as_ref();

        let enabled = security_config.and_then(|s| s.enabled).unwrap_or(true);

        let dangerous_patterns = security_config
            .and_then(|s| s.dangerous_patterns.clone())
            .unwrap_or_else(Self::default_dangerous_patterns);

        let settings_files = security_config
            .and_then(|s| s.claude_settings_files.clone())
            .unwrap_or_else(Self::default_settings_files);

        Ok(Self {
            dangerous_patterns,
            settings_files,
            enabled,
        })
    }

    /// Check Claude Code settings for dangerous permissions
    pub fn check_claude_settings(&self) -> Result<Vec<SecurityWarning>> {
        if !self.enabled {
            return Ok(vec![]);
        }

        let mut all_warnings = Vec::new();

        for file_pattern in &self.settings_files {
            if let Some(path) = self.resolve_settings_file_path(file_pattern) {
                match self.check_settings_file(&path) {
                    Ok(mut warnings) => all_warnings.append(&mut warnings),
                    Err(e) => {
                        // Log error but continue checking other files
                        eprintln!(
                            "{}",
                            t!(
                                "claude_code.security.parse_error",
                                path = path.display(),
                                error = e.to_string()
                            )
                        );
                    }
                }
            }
        }

        Ok(all_warnings)
    }

    /// Check a specific settings file for dangerous permissions
    fn check_settings_file(&self, path: &Path) -> Result<Vec<SecurityWarning>> {
        if !path.exists() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(path).with_context(|| {
            t!(
                "claude_code.security.read_settings_failed",
                path = path.display()
            )
        })?;

        // Try to parse JSON with repair capability
        let settings = match serde_json::from_str::<Value>(&content) {
            Ok(value) => value,
            Err(_) => {
                // Try to repair JSON
                match loads(&content, &RepairOptions::default()) {
                    Ok(repaired_value) => repaired_value,
                    Err(e) => {
                        anyhow::bail!(
                            "{}",
                            t!("claude_code.security.parse_json_failed", error = e)
                        );
                    }
                }
            }
        };

        Ok(self.extract_dangerous_permissions(&settings, &path.display().to_string()))
    }

    /// Extract dangerous permissions from parsed settings
    fn extract_dangerous_permissions(
        &self,
        settings: &Value,
        config_path: &str,
    ) -> Vec<SecurityWarning> {
        let mut warnings = Vec::new();

        if let Some(permissions) = settings.get("permissions")
            && let Some(allow) = permissions.get("allow")
            && let Some(allow_array) = allow.as_array()
        {
            for (index, item) in allow_array.iter().enumerate() {
                if let Some(permission) = item.as_str()
                    && let Some((matched_pattern, (severity, description, recommendation))) =
                        self.get_dangerous_pattern_info(permission)
                {
                    warnings.push(SecurityWarning {
                        config_path: config_path.to_string(),
                        config_item: format!("permissions.allow[{}] = \"{}\"", index, permission),
                        reason: t!(
                            "claude_code.security.reason_pattern_detected",
                            pattern = permission
                        )
                        .to_string(),
                        severity,
                        matched_pattern,
                        description,
                        recommendation,
                    });
                }
            }
        }

        warnings
    }

    /// Get detailed information about a dangerous pattern
    fn get_dangerous_pattern_info(
        &self,
        permission: &str,
    ) -> Option<(String, (SecurityWarningSeverity, String, String))> {
        let permission_lower = permission.to_lowercase();

        for pattern in &self.dangerous_patterns {
            if let Some(matched) = self.matches_dangerous_pattern(&permission_lower, pattern) {
                let matched_str = matched.clone();
                return Some((matched, self.get_pattern_details(&matched_str)));
            }
        }

        None
    }

    /// Check if permission matches a dangerous pattern using regex for precision
    fn matches_dangerous_pattern(&self, permission: &str, pattern: &str) -> Option<String> {
        match pattern {
            // For "format", match only when it's a command/argument, not part of a variable name
            // e.g., matches "format C:" but not "LOGX_FORMAT"
            "format" => {
                // Use word boundary to avoid matching variable names like LOGX_FORMAT
                // Also match specific dangerous format commands like "format C:", "format /dev/xxx"
                let re = Regex::new(r"(?i)\bformat\s+[a-zA-Z:/\\]").ok()?;
                if re.is_match(permission) {
                    Some("format".to_string())
                } else {
                    None
                }
            }

            // For "mkfs", match only as a standalone command with options
            "mkfs" => {
                let re = Regex::new(r"(?i)\bmkfs(?:\.|\s+)").ok()?;
                if re.is_match(permission) {
                    Some("mkfs".to_string())
                } else {
                    None
                }
            }

            // For "rm -rf", match with proper context
            "rm -rf" => {
                // Match rm -rf followed by path (not just flags)
                let re = Regex::new(r"(?i)\brm\s+-rf\b").ok()?;
                if re.is_match(permission) {
                    Some("rm -rf".to_string())
                } else {
                    None
                }
            }

            // For "sudo rm", match dangerous sudo rm patterns
            "sudo rm" => {
                let re = Regex::new(r"(?i)\bsudo\s+rm\b").ok()?;
                if re.is_match(permission) {
                    Some("sudo rm".to_string())
                } else {
                    None
                }
            }

            // For "dd if=", match dd with input file specification
            "dd if=" => {
                let re = Regex::new(r"(?i)\bdd\s+if=").ok()?;
                if re.is_match(permission) {
                    Some("dd if=".to_string())
                } else {
                    None
                }
            }

            // Default: use simple contains for patterns that need flexibility
            _ => {
                if permission.contains(pattern) {
                    Some(pattern.to_string())
                } else {
                    None
                }
            }
        }
    }

    /// Get severity, description and recommendation for a dangerous pattern
    fn get_pattern_details(&self, pattern: &str) -> (SecurityWarningSeverity, String, String) {
        match pattern {
            // File system destruction - CRITICAL
            p if p.contains("rm -rf") => (
                SecurityWarningSeverity::Critical,
                t!("claude_code.security.pattern.rm_rf.description").to_string(),
                t!("claude_code.security.pattern.rm_rf.recommendation").to_string(),
            ),

            p if p.contains("sudo rm") => (
                SecurityWarningSeverity::Critical,
                t!("claude_code.security.pattern.sudo_rm.description").to_string(),
                t!("claude_code.security.pattern.sudo_rm.recommendation").to_string(),
            ),

            p if p.contains("dd if=") => (
                SecurityWarningSeverity::Critical,
                t!("claude_code.security.pattern.dd_if.description").to_string(),
                t!("claude_code.security.pattern.dd_if.recommendation").to_string(),
            ),

            p if p.contains("mkfs") || p.contains("format") => (
                SecurityWarningSeverity::Critical,
                t!("claude_code.security.pattern.mkfs_or_format.description").to_string(),
                t!("claude_code.security.pattern.mkfs_or_format.recommendation").to_string(),
            ),

            // Privilege escalation - HIGH
            p if p.contains("chmod 777") => (
                SecurityWarningSeverity::High,
                t!("claude_code.security.pattern.chmod_777.description").to_string(),
                t!("claude_code.security.pattern.chmod_777.recommendation").to_string(),
            ),

            p if p.contains("chown root") => (
                SecurityWarningSeverity::High,
                t!("claude_code.security.pattern.chown_root.description").to_string(),
                t!("claude_code.security.pattern.chown_root.recommendation").to_string(),
            ),

            // Remote code execution - HIGH
            p if p.contains("curl | sh") || p.contains("wget | sh") => (
                SecurityWarningSeverity::High,
                t!("claude_code.security.pattern.curl_or_wget_sh.description").to_string(),
                t!("claude_code.security.pattern.curl_or_wget_sh.recommendation").to_string(),
            ),

            p if p.contains("eval $(") || p.contains("exec $(") => (
                SecurityWarningSeverity::High,
                t!("claude_code.security.pattern.eval_or_exec.description").to_string(),
                t!("claude_code.security.pattern.eval_or_exec.recommendation").to_string(),
            ),

            // System configuration - MEDIUM
            p if p.contains("system(") => (
                SecurityWarningSeverity::Medium,
                t!("claude_code.security.pattern.system_call.description").to_string(),
                t!("claude_code.security.pattern.system_call.recommendation").to_string(),
            ),

            p if p.contains("crontab") => (
                SecurityWarningSeverity::Medium,
                t!("claude_code.security.pattern.crontab.description").to_string(),
                t!("claude_code.security.pattern.crontab.recommendation").to_string(),
            ),

            p if p.contains("systemctl") || p.contains("service") => (
                SecurityWarningSeverity::Medium,
                t!("claude_code.security.pattern.system_service.description").to_string(),
                t!("claude_code.security.pattern.system_service.recommendation").to_string(),
            ),

            // Network security - MEDIUM
            p if p.contains("iptables") || p.contains("ufw") || p.contains("firewall") => (
                SecurityWarningSeverity::Medium,
                t!("claude_code.security.pattern.firewall.description").to_string(),
                t!("claude_code.security.pattern.firewall.recommendation").to_string(),
            ),

            // Registry/Windows commands - HIGH
            p if p.contains("registry") || p.contains("reg add") => (
                SecurityWarningSeverity::High,
                t!("claude_code.security.pattern.registry.description").to_string(),
                t!("claude_code.security.pattern.registry.recommendation").to_string(),
            ),

            p if p.contains("net user") => (
                SecurityWarningSeverity::High,
                t!("claude_code.security.pattern.net_user.description").to_string(),
                t!("claude_code.security.pattern.net_user.recommendation").to_string(),
            ),

            // Command interpreters - MEDIUM
            p if p.contains("powershell -c") || p.contains("cmd /c") => (
                SecurityWarningSeverity::Medium,
                t!("claude_code.security.pattern.shell_exec.description").to_string(),
                t!("claude_code.security.pattern.shell_exec.recommendation").to_string(),
            ),

            // Python specific - MEDIUM
            p if p.contains("__import__('os').system") || p.contains("subprocess.call") => (
                SecurityWarningSeverity::Medium,
                t!("claude_code.security.pattern.python_system.description").to_string(),
                t!("claude_code.security.pattern.python_system.recommendation").to_string(),
            ),

            // Default for unknown patterns
            _ => (
                SecurityWarningSeverity::Medium,
                t!("claude_code.security.pattern.default.description").to_string(),
                t!("claude_code.security.pattern.default.recommendation").to_string(),
            ),
        }
    }

    /// Resolve settings file path from pattern (handles ~ expansion)
    fn resolve_settings_file_path(&self, file_pattern: &str) -> Option<PathBuf> {
        let path = if let Some(stripped) = file_pattern.strip_prefix("~/") {
            if let Some(home_dir) = dirs::home_dir() {
                home_dir.join(stripped)
            } else {
                PathBuf::from(file_pattern)
            }
        } else {
            PathBuf::from(file_pattern)
        };

        Some(path)
    }

    /// Default dangerous command patterns
    fn default_dangerous_patterns() -> Vec<String> {
        vec![
            "rm -rf".to_string(),
            "sudo rm".to_string(),
            "dd if=".to_string(),
            "mkfs".to_string(),
            "format".to_string(),
            "chmod 777".to_string(),
            "chown root".to_string(),
            ">:|".to_string(),
            "curl | sh".to_string(),
            "wget | sh".to_string(),
            "eval $(".to_string(),
            "exec $(".to_string(),
            "system(".to_string(),
            "__import__('os').system".to_string(),
            "subprocess.call".to_string(),
            "powershell -c".to_string(),
            "cmd /c".to_string(),
            "registry".to_string(),
            "reg add".to_string(),
            "net user".to_string(),
            "crontab".to_string(),
            "systemctl".to_string(),
            "service".to_string(),
            "iptables".to_string(),
            "ufw".to_string(),
            "firewall".to_string(),
        ]
    }

    /// Default Claude Code settings files to check
    fn default_settings_files() -> Vec<String> {
        vec![
            ".claude/settings.local.json".to_string(),
            ".claude/settings.json".to_string(),
            "~/.claude/settings.json".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_dangerous_pattern_info() {
        let checker = SecurityChecker {
            dangerous_patterns: vec!["rm -rf".to_string(), "curl | sh".to_string()],
            settings_files: vec![],
            enabled: true,
        };

        // Test matching patterns
        assert!(
            checker
                .get_dangerous_pattern_info("Bash(rm -rf /tmp/*)")
                .is_some()
        );
        assert!(
            checker
                .get_dangerous_pattern_info("Bash(curl | sh script.sh)")
                .is_some()
        );

        // Test non-matching patterns
        assert!(checker.get_dangerous_pattern_info("Bash(ls -la)").is_none());

        // Test case insensitivity and sudo variants
        assert!(
            checker
                .get_dangerous_pattern_info("Bash(sudo rm -rf /file)")
                .is_some()
        );
    }

    #[test]
    fn test_extract_dangerous_permissions() {
        let checker = SecurityChecker {
            dangerous_patterns: vec!["rm -rf".to_string()],
            settings_files: vec![],
            enabled: true,
        };

        let settings_json = r#"
        {
            "permissions": {
                "allow": [
                    "Bash(ls -la)",
                    "Bash(rm -rf /tmp/*)",
                    "WebSearch"
                ]
            }
        }
        "#;

        let settings: Value = serde_json::from_str(settings_json).unwrap();
        let warnings = checker.extract_dangerous_permissions(&settings, "/test/path");

        assert_eq!(warnings.len(), 1);
        assert_eq!(
            warnings[0].config_item,
            "permissions.allow[1] = \"Bash(rm -rf /tmp/*)\""
        );
        assert_eq!(warnings[0].config_path, "/test/path");
        assert_eq!(warnings[0].matched_pattern, "rm -rf");
        assert_eq!(warnings[0].severity, SecurityWarningSeverity::Critical);
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(SecurityWarningSeverity::Critical.display_symbol(), "🚨");
        assert_eq!(SecurityWarningSeverity::Critical.display_name(), "CRITICAL");
        assert_eq!(SecurityWarningSeverity::High.display_symbol(), "⚠️");
        assert_eq!(SecurityWarningSeverity::High.display_name(), "HIGH");
        assert_eq!(SecurityWarningSeverity::Medium.display_symbol(), "⚡");
        assert_eq!(SecurityWarningSeverity::Medium.display_name(), "MEDIUM");
        assert_eq!(SecurityWarningSeverity::Low.display_symbol(), "ℹ️");
        assert_eq!(SecurityWarningSeverity::Low.display_name(), "LOW");
    }

    #[test]
    fn test_pattern_details() {
        let checker = SecurityChecker {
            dangerous_patterns: vec![
                "rm -rf".to_string(),
                "curl | sh".to_string(),
                "chmod 777".to_string(),
            ],
            settings_files: vec![],
            enabled: true,
        };

        // Test critical severity
        let (severity, description, recommendation) = checker.get_pattern_details("rm -rf");
        assert_eq!(severity, SecurityWarningSeverity::Critical);
        assert!(!description.is_empty());
        assert!(!recommendation.is_empty());

        // Test high severity
        let (severity, description, recommendation) = checker.get_pattern_details("curl | sh");
        assert_eq!(severity, SecurityWarningSeverity::High);
        assert!(!description.is_empty());
        assert!(!recommendation.is_empty());

        // Test high severity for chmod
        let (severity, description, recommendation) = checker.get_pattern_details("chmod 777");
        assert_eq!(severity, SecurityWarningSeverity::High);
        assert!(!description.is_empty());
        assert!(!recommendation.is_empty());
    }

    #[test]
    fn test_format_pattern_precision() {
        let checker = SecurityChecker {
            dangerous_patterns: vec!["format".to_string(), "mkfs".to_string()],
            settings_files: vec![],
            enabled: true,
        };

        // Should NOT match variable names containing "format"
        assert!(
            checker
                .get_dangerous_pattern_info("Bash(LOGX_FORMAT=console just run-python:*)")
                .is_none(),
            "LOGX_FORMAT should not be matched as dangerous"
        );
        assert!(
            checker
                .get_dangerous_pattern_info("Bash(OUTPUT_FORMAT=json command)")
                .is_none(),
            "OUTPUT_FORMAT should not be matched as dangerous"
        );

        // Should match actual format commands
        assert!(
            checker
                .get_dangerous_pattern_info("Bash(format C: /q)")
                .is_some(),
            "format C: should be matched as dangerous"
        );
        assert!(
            checker
                .get_dangerous_pattern_info("Bash(format /dev/sda)")
                .is_some(),
            "format /dev/sda should be matched as dangerous"
        );

        // Should match mkfs commands
        assert!(
            checker
                .get_dangerous_pattern_info("Bash(mkfs.ext4 /dev/sda1)")
                .is_some(),
            "mkfs.ext4 should be matched as dangerous"
        );
        assert!(
            checker
                .get_dangerous_pattern_info("Bash(mkfs -t ext4 /dev/sda1)")
                .is_some(),
            "mkfs -t should be matched as dangerous"
        );

        // Should NOT match harmless commands
        assert!(
            checker
                .get_dangerous_pattern_info("Bash(formatted=123)")
                .is_none(),
            "formatted= should not be matched as dangerous"
        );
    }
}
