use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, env, fs, path::PathBuf, process::Command};

/// Host-level settings that can be applied globally or per host.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct HostSettings {
    pub cross: Option<bool>,
    pub package: Option<String>,
    pub target: Option<String>,
    pub release: Option<bool>,
}

/// Parsed `cargo-warp` configuration from disk.
#[derive(Debug, Default, Deserialize)]
pub struct WarpConfig {
    #[serde(default)]
    pub defaults: HostSettings,
    #[serde(default)]
    pub hosts: HashMap<String, HostSettings>,
}

/// CLI-provided values for the `warp` command.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CliWarpOptions {
    pub cross: Option<bool>,
    pub package: Option<String>,
    pub target: Option<String>,
    pub release: Option<bool>,
}

/// Final effective options used for building.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveWarpOptions {
    pub cross: bool,
    pub package: Option<String>,
    pub target: Option<String>,
    pub release: bool,
}

/// Returns the full path to the warp configuration file.
pub fn config_file_path() -> Result<PathBuf> {
    let base_path = config_base_dir(
        env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        env::var_os("HOME").map(PathBuf::from),
    )?;
    Ok(base_path.join("cargo-warp").join("config.toml"))
}

/// Loads the configuration file if it exists.
pub fn load_config_file() -> Result<Option<WarpConfig>> {
    let path = config_file_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file '{}'", path.display()))?;
    let config: WarpConfig = toml::from_str(&content)
        .with_context(|| format!("failed to parse '{}'", path.display()))?;

    Ok(Some(config))
}

/// Creates a starter config file at the default config path.
pub fn init_config_file(force: bool) -> Result<PathBuf> {
    let path = config_file_path()?;

    if path.exists() && !force {
        bail!(
            "config file '{}' already exists; rerun with --force to overwrite",
            path.display()
        );
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create '{}'", parent.display()))?;
    }

    fs::write(&path, starter_config())
        .with_context(|| format!("failed to write '{}'", path.display()))?;

    Ok(path)
}

/// Resolves effective `warp` options by combining CLI and config values.
pub fn resolve_warp_options(
    config: Option<&WarpConfig>,
    destination: &str,
    cli: CliWarpOptions,
) -> EffectiveWarpOptions {
    let destination_host = parse_destination_host(destination);
    let ssh_host = destination_host.as_deref().and_then(resolve_ssh_hostname);

    resolve_warp_options_for_hosts(
        config,
        destination_host.as_deref(),
        ssh_host.as_deref(),
        cli,
    )
}

fn starter_config() -> &'static str {
    r#"[defaults]
# release = true
# cross = false
# package = "my-bin"
# target = "aarch64-unknown-linux-gnu"

[hosts.mypc]
cross = true
target = "aarch64-unknown-linux-gnu"

[hosts."*.lab.local"]
release = true

[hosts."10.0.*"]
cross = false
"#
}

fn config_base_dir(xdg_config_home: Option<PathBuf>, home: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = xdg_config_home {
        if !path.as_os_str().is_empty() {
            return Ok(path);
        }
    }

    if let Some(path) = home {
        if !path.as_os_str().is_empty() {
            return Ok(path.join(".config"));
        }
    }

    bail!("could not determine config directory; set HOME or XDG_CONFIG_HOME")
}

fn parse_destination_host(destination: &str) -> Option<String> {
    if let Some(host) = parse_rsync_url_host(destination) {
        return Some(host);
    }

    let host_and_user = parse_scp_host(destination)?;
    if host_and_user.contains('/') {
        return None;
    }

    let host = host_and_user.rsplit('@').next().unwrap_or(host_and_user);
    if host.is_empty() {
        return None;
    }

    Some(strip_brackets(host).to_string())
}

fn resolve_ssh_hostname(host: &str) -> Option<String> {
    let output = Command::new("ssh").args(["-G", host]).output().ok()?;
    if !output.status.success() {
        return None;
    }

    parse_ssh_hostname_output(&output.stdout)
}

fn parse_ssh_hostname_output(output: &[u8]) -> Option<String> {
    let stdout = std::str::from_utf8(output).ok()?;

    for line in stdout.lines() {
        let mut parts = line.split_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };
        if key.eq_ignore_ascii_case("hostname") {
            if let Some(value) = parts.next() {
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}

fn resolve_warp_options_for_hosts(
    config: Option<&WarpConfig>,
    destination_host: Option<&str>,
    resolved_host: Option<&str>,
    cli: CliWarpOptions,
) -> EffectiveWarpOptions {
    let global = config.map(|cfg| &cfg.defaults);
    let host = config.and_then(|cfg| select_host_settings(cfg, destination_host, resolved_host));

    EffectiveWarpOptions {
        cross: cli
            .cross
            .or_else(|| host.and_then(|settings| settings.cross))
            .or_else(|| global.and_then(|settings| settings.cross))
            .unwrap_or(false),
        package: cli
            .package
            .or_else(|| host.and_then(|settings| settings.package.clone()))
            .or_else(|| global.and_then(|settings| settings.package.clone())),
        target: cli
            .target
            .or_else(|| host.and_then(|settings| settings.target.clone()))
            .or_else(|| global.and_then(|settings| settings.target.clone())),
        release: cli
            .release
            .or_else(|| host.and_then(|settings| settings.release))
            .or_else(|| global.and_then(|settings| settings.release))
            .unwrap_or(false),
    }
}

fn select_host_settings<'a>(
    config: &'a WarpConfig,
    destination_host: Option<&str>,
    resolved_host: Option<&str>,
) -> Option<&'a HostSettings> {
    if let Some(host) = destination_host {
        if let Some(settings) = config.hosts.get(host) {
            return Some(settings);
        }
    }

    if let Some(host) = resolved_host {
        if let Some(settings) = config.hosts.get(host) {
            return Some(settings);
        }
    }

    if let Some(host) = destination_host {
        if let Some(settings) = best_wildcard_match(&config.hosts, host) {
            return Some(settings);
        }
    }

    if let Some(host) = resolved_host {
        if let Some(settings) = best_wildcard_match(&config.hosts, host) {
            return Some(settings);
        }
    }

    None
}

fn has_wildcards(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?')
}

fn wildcard_matches(pattern: &str, text: &str) -> bool {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    let mut pattern_index: usize = 0;
    let mut text_index: usize = 0;
    let mut star_index: Option<usize> = None;
    let mut last_star_match: usize = 0;

    while text_index < text_chars.len() {
        if pattern_index < pattern_chars.len()
            && (pattern_chars[pattern_index] == '?'
                || pattern_chars[pattern_index] == text_chars[text_index])
        {
            pattern_index += 1;
            text_index += 1;
            continue;
        }

        if pattern_index < pattern_chars.len() && pattern_chars[pattern_index] == '*' {
            star_index = Some(pattern_index);
            pattern_index += 1;
            last_star_match = text_index;
            continue;
        }

        if let Some(star_position) = star_index {
            pattern_index = star_position + 1;
            last_star_match += 1;
            text_index = last_star_match;
            continue;
        }

        return false;
    }

    while pattern_index < pattern_chars.len() && pattern_chars[pattern_index] == '*' {
        pattern_index += 1;
    }

    pattern_index == pattern_chars.len()
}

fn best_wildcard_match<'a>(
    hosts: &'a HashMap<String, HostSettings>,
    host: &str,
) -> Option<&'a HostSettings> {
    let mut best_pattern: Option<&str> = None;
    let mut best_score: (usize, usize) = (0, 0);
    let mut best_settings: Option<&HostSettings> = None;

    for (pattern, settings) in hosts {
        if !has_wildcards(pattern) || !wildcard_matches(pattern, host) {
            continue;
        }

        let score = wildcard_specificity(pattern);
        let pattern_str = pattern.as_str();
        let should_replace = match best_pattern {
            None => true,
            Some(current_best_pattern) => {
                score > best_score || (score == best_score && pattern_str < current_best_pattern)
            }
        };

        if should_replace {
            best_pattern = Some(pattern_str);
            best_score = score;
            best_settings = Some(settings);
        }
    }

    best_settings
}

fn wildcard_specificity(pattern: &str) -> (usize, usize) {
    (
        pattern
            .chars()
            .filter(|character| *character != '*' && *character != '?')
            .count(),
        pattern.chars().count(),
    )
}

fn strip_brackets(host: &str) -> &str {
    if let Some(stripped) = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        return stripped;
    }

    host
}

fn parse_scp_host(destination: &str) -> Option<&str> {
    let mut in_brackets = false;

    for (index, character) in destination.char_indices() {
        match character {
            '[' => in_brackets = true,
            ']' => in_brackets = false,
            ':' if !in_brackets => {
                return Some(&destination[..index]);
            }
            _ => {}
        }
    }

    None
}

fn parse_rsync_url_host(destination: &str) -> Option<String> {
    let rest = destination.strip_prefix("rsync://")?;
    let authority = rest.split('/').next()?;
    if authority.is_empty() {
        return None;
    }

    let host = authority.rsplit('@').next().unwrap_or(authority);
    if host.is_empty() {
        return None;
    }

    Some(strip_brackets(host).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(
        cross: Option<bool>,
        package: Option<&str>,
        target: Option<&str>,
        release: Option<bool>,
    ) -> HostSettings {
        HostSettings {
            cross,
            package: package.map(|value| value.to_string()),
            target: target.map(|value| value.to_string()),
            release,
        }
    }

    #[test]
    fn test_config_base_dir_prefers_xdg_config_home() {
        let path = config_base_dir(
            Some(PathBuf::from("/tmp/custom-config")),
            Some(PathBuf::from("/home/matthew")),
        )
        .expect("Expected XDG config directory to be selected");

        assert_eq!(path, PathBuf::from("/tmp/custom-config"));
    }

    #[test]
    fn test_config_base_dir_falls_back_to_home_config() {
        let path = config_base_dir(None, Some(PathBuf::from("/home/matthew")))
            .expect("Expected HOME fallback");

        assert_eq!(path, PathBuf::from("/home/matthew/.config"));
    }

    #[test]
    fn test_parse_destination_host_user_host_path() {
        let host = parse_destination_host("dev@mypc:/tmp/bin");

        assert_eq!(host, Some("mypc".to_string()));
    }

    #[test]
    fn test_parse_destination_host_host_path() {
        let host = parse_destination_host("mypc:/tmp/bin");

        assert_eq!(host, Some("mypc".to_string()));
    }

    #[test]
    fn test_parse_destination_host_rsync_url() {
        let host = parse_destination_host("rsync://build.box.local/tmp/bin");

        assert_eq!(host, Some("build.box.local".to_string()));
    }

    #[test]
    fn test_parse_destination_host_ipv4() {
        let host = parse_destination_host("deployer@192.168.1.50:/tmp/bin");

        assert_eq!(host, Some("192.168.1.50".to_string()));
    }

    #[test]
    fn test_parse_ssh_hostname_output_extracts_hostname() {
        let output = b"host mypc\nuser matthew\nhostname 10.0.1.20\nport 22\n";

        assert_eq!(
            parse_ssh_hostname_output(output),
            Some("10.0.1.20".to_string())
        );
    }

    #[test]
    fn test_wildcard_match_star() {
        assert!(wildcard_matches("*.lab.local", "build.lab.local"));
        assert!(!wildcard_matches("*.lab.local", "build.prod.local"));
    }

    #[test]
    fn test_wildcard_match_question() {
        assert!(wildcard_matches("web-??", "web-01"));
        assert!(!wildcard_matches("web-??", "web-010"));
    }

    #[test]
    fn test_wildcard_match_mixed_pattern() {
        assert!(wildcard_matches("10.0.*.?", "10.0.12.3"));
        assert!(!wildcard_matches("10.0.*.?", "10.1.12.3"));
    }

    #[test]
    fn test_select_best_wildcard_by_specificity() {
        let mut hosts: HashMap<String, HostSettings> = HashMap::new();
        hosts.insert(
            "*.lab.local".to_string(),
            settings(Some(true), None, None, None),
        );
        hosts.insert(
            "build-*.lab.local".to_string(),
            settings(Some(false), None, None, None),
        );

        let selected =
            best_wildcard_match(&hosts, "build-01.lab.local").expect("Expected wildcard match");

        assert_eq!(selected.cross, Some(false));
    }

    #[test]
    fn test_resolve_host_prefers_exact_over_wildcard() {
        let mut config = WarpConfig::default();
        config.hosts.insert(
            "*.lab.local".to_string(),
            settings(Some(false), None, None, None),
        );
        config.hosts.insert(
            "build.lab.local".to_string(),
            settings(Some(true), None, None, None),
        );

        let selected = select_host_settings(&config, Some("build.lab.local"), None)
            .expect("Expected exact host match");

        assert_eq!(selected.cross, Some(true));
    }

    #[test]
    fn test_resolve_host_prefers_alias_over_resolved_host() {
        let mut config = WarpConfig::default();
        config
            .hosts
            .insert("alias".to_string(), settings(Some(true), None, None, None));
        config.hosts.insert(
            "10.0.1.20".to_string(),
            settings(Some(false), None, None, None),
        );

        let selected = select_host_settings(&config, Some("alias"), Some("10.0.1.20"))
            .expect("Expected alias host match");

        assert_eq!(selected.cross, Some(true));
    }

    #[test]
    fn test_merge_precedence_cli_over_host_over_defaults() {
        let mut config = WarpConfig::default();
        config.defaults = settings(
            Some(false),
            Some("default-pkg"),
            Some("x86_64"),
            Some(false),
        );
        config.hosts.insert(
            "mypc".to_string(),
            settings(Some(true), Some("host-pkg"), Some("aarch64"), Some(true)),
        );

        let cli = CliWarpOptions {
            cross: Some(false),
            package: None,
            target: Some("armv7".to_string()),
            release: None,
        };

        let resolved = resolve_warp_options_for_hosts(Some(&config), Some("mypc"), None, cli);

        assert_eq!(
            resolved,
            EffectiveWarpOptions {
                cross: false,
                package: Some("host-pkg".to_string()),
                target: Some("armv7".to_string()),
                release: true,
            }
        );
    }
}
