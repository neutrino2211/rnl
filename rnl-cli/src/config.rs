//! Configuration parsing for rnl.toml

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Root configuration structure for rnl.toml
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub project: ProjectConfig,
    pub window: WindowConfig,
    #[serde(default)]
    pub platforms: PlatformConfigs,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub elements: ElementsConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowConfig {
    pub title: String,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
}

fn default_width() -> u32 {
    800
}
fn default_height() -> u32 {
    600
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PlatformConfigs {
    #[serde(default)]
    pub linux: Option<LinuxConfig>,
    #[serde(default)]
    pub macos: Option<MacOSConfig>,
    #[serde(default)]
    pub windows: Option<WindowsConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LinuxConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_gtk4")]
    pub toolkit: String,
    #[serde(default = "default_cpp")]
    pub lang: String,
    #[serde(default)]
    pub min_version: Option<String>,
}

fn default_true() -> bool {
    true
}
fn default_gtk4() -> String {
    "gtk4".to_string()
}
fn default_cpp() -> String {
    "cpp".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MacOSConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_appkit")]
    pub toolkit: String,
    #[serde(default = "default_swift")]
    pub lang: String,
    #[serde(default)]
    pub min_version: Option<String>,
}

fn default_appkit() -> String {
    "appkit".to_string()
}
fn default_swift() -> String {
    "swift".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_winui3")]
    pub toolkit: String,
    #[serde(default = "default_csharp")]
    pub lang: String,
    #[serde(default)]
    pub min_version: Option<String>,
}

fn default_winui3() -> String {
    "winui3".to_string()
}
fn default_csharp() -> String {
    "csharp".to_string()
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BuildConfig {
    #[serde(default = "default_true")]
    pub bundle_embed: bool,
    #[serde(default = "default_true")]
    pub minify: bool,
    #[serde(default)]
    pub sourcemaps: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ElementsConfig {
    #[serde(default)]
    pub custom: Vec<String>,
}

impl Config {
    /// Load configuration from rnl.toml in the given directory
    pub fn load(project_dir: &Path) -> Result<Self> {
        let config_path = project_dir.join("rnl.toml");
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", config_path.display()))
    }

    /// Save configuration to rnl.toml
    pub fn save(&self, project_dir: &Path) -> Result<()> {
        let config_path = project_dir.join("rnl.toml");
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    /// Create a default configuration for a new project
    pub fn default_for_project(name: &str, platforms: &[&str]) -> Self {
        Config {
            project: ProjectConfig {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                description: format!("{} - An RNL application", name),
            },
            window: WindowConfig {
                title: name.to_string(),
                width: 800,
                height: 600,
            },
            platforms: PlatformConfigs {
                linux: if platforms.contains(&"linux") {
                    Some(LinuxConfig {
                        enabled: true,
                        toolkit: "gtk4".to_string(),
                        lang: "cpp".to_string(),
                        min_version: Some("22.04".to_string()),
                    })
                } else {
                    None
                },
                macos: if platforms.contains(&"macos") {
                    Some(MacOSConfig {
                        enabled: true,
                        toolkit: "appkit".to_string(),
                        lang: "swift".to_string(),
                        min_version: Some("12.0".to_string()),
                    })
                } else {
                    None
                },
                windows: if platforms.contains(&"windows") {
                    Some(WindowsConfig {
                        enabled: true,
                        toolkit: "winui3".to_string(),
                        lang: "csharp".to_string(),
                        min_version: Some("10.0.17763.0".to_string()),
                    })
                } else {
                    None
                },
            },
            build: BuildConfig {
                bundle_embed: true,
                minify: true,
                sourcemaps: false,
            },
            elements: ElementsConfig::default(),
        }
    }

    /// Get enabled platforms
    pub fn enabled_platforms(&self) -> Vec<&str> {
        let mut platforms = Vec::new();
        if let Some(ref linux) = self.platforms.linux {
            if linux.enabled {
                platforms.push("linux");
            }
        }
        if let Some(ref macos) = self.platforms.macos {
            if macos.enabled {
                platforms.push("macos");
            }
        }
        if let Some(ref windows) = self.platforms.windows {
            if windows.enabled {
                platforms.push("windows");
            }
        }
        platforms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default_for_project("test-app", &["linux", "macos"]);
        assert_eq!(config.project.name, "test-app");
        assert!(config.platforms.linux.is_some());
        assert!(config.platforms.macos.is_some());
        assert!(config.platforms.windows.is_none());
    }

    #[test]
    fn test_enabled_platforms() {
        let config = Config::default_for_project("test-app", &["linux", "windows"]);
        let platforms = config.enabled_platforms();
        assert!(platforms.contains(&"linux"));
        assert!(platforms.contains(&"windows"));
        assert!(!platforms.contains(&"macos"));
    }
}
