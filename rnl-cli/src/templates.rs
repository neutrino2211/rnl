//! Embedded templates for project scaffolding

use handlebars::Handlebars;
use rust_embed::RustEmbed;
use anyhow::{Context, Result};
use std::collections::HashMap;

#[derive(RustEmbed)]
#[folder = "templates/"]
pub struct Templates;

/// Template renderer using Handlebars
pub struct TemplateRenderer<'a> {
    hbs: Handlebars<'a>,
}

impl<'a> TemplateRenderer<'a> {
    pub fn new() -> Result<Self> {
        let mut hbs = Handlebars::new();
        hbs.set_strict_mode(true);

        // Register all embedded templates
        for file in Templates::iter() {
            let filename = file.as_ref();
            if filename.ends_with(".hbs") {
                let content = Templates::get(filename)
                    .context(format!("Failed to load template: {}", filename))?;
                let template_content = std::str::from_utf8(content.data.as_ref())?;
                
                // Use filename without .hbs as template name
                let template_name = filename.trim_end_matches(".hbs");
                hbs.register_template_string(template_name, template_content)?;
            }
        }

        Ok(Self { hbs })
    }

    /// Render a template with the given data
    pub fn render(&self, template_name: &str, data: &serde_json::Value) -> Result<String> {
        self.hbs
            .render(template_name, data)
            .with_context(|| format!("Failed to render template: {}", template_name))
    }

    /// Get a static (non-template) file from the embedded assets
    pub fn get_static(path: &str) -> Option<Vec<u8>> {
        Templates::get(path).map(|f| f.data.to_vec())
    }
}

/// Template data for project scaffolding
#[derive(serde::Serialize)]
pub struct ProjectData {
    pub name: String,
    pub version: String,
    pub description: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub platforms: PlatformFlags,
}

#[derive(serde::Serialize)]
pub struct PlatformFlags {
    pub linux: bool,
    pub macos: bool,
    pub windows: bool,
}

impl ProjectData {
    pub fn new(name: &str, platforms: &[&str]) -> Self {
        Self {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: format!("{} - An RNL application", name),
            title: name.to_string(),
            width: 800,
            height: 600,
            platforms: PlatformFlags {
                linux: platforms.contains(&"linux"),
                macos: platforms.contains(&"macos"),
                windows: platforms.contains(&"windows"),
            },
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_data() {
        let data = ProjectData::new("my-app", &["linux", "macos"]);
        assert_eq!(data.name, "my-app");
        assert!(data.platforms.linux);
        assert!(data.platforms.macos);
        assert!(!data.platforms.windows);
    }
}
