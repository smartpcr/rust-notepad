/// Trait for editor validation plugins.
pub trait EditorPlugin {
    fn name(&self) -> &'static str;
    fn supports_extension(&self, ext: &str) -> bool;
    fn validate(&self, text: &str) -> anyhow::Result<()>;
}

/// Validates JSON documents.
pub struct JsonPlugin;

impl EditorPlugin for JsonPlugin {
    fn name(&self) -> &'static str {
        "JSON Validator"
    }
    fn supports_extension(&self, ext: &str) -> bool {
        ext.eq_ignore_ascii_case("json")
    }
    fn validate(&self, text: &str) -> anyhow::Result<()> {
        serde_json::from_str::<serde_json::Value>(text)?;
        Ok(())
    }
}

/// Validates XML documents.
pub struct XmlPlugin;

impl EditorPlugin for XmlPlugin {
    fn name(&self) -> &'static str {
        "XML Validator"
    }
    fn supports_extension(&self, ext: &str) -> bool {
        ext.eq_ignore_ascii_case("xml")
    }
    fn validate(&self, text: &str) -> anyhow::Result<()> {
        quick_xml::de::from_str::<serde_json::Value>(text)?;
        Ok(())
    }
}

/// Run all plugins that support the given extension and return diagnostic messages.
pub fn run_plugins(plugins: &[Box<dyn EditorPlugin>], ext: &str, content: &str) -> Vec<String> {
    let mut messages = Vec::new();
    for plugin in plugins {
        if plugin.supports_extension(ext) {
            match plugin.validate(content) {
                Ok(_) => messages.push(format!("{}: OK", plugin.name())),
                Err(err) => messages.push(format!("{}: {}", plugin.name(), err)),
            }
        }
    }
    messages
}

/// Create the default set of built-in plugins.
pub fn default_plugins() -> Vec<Box<dyn EditorPlugin>> {
    vec![Box::new(JsonPlugin), Box::new(XmlPlugin)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_plugin_validates_valid_json() {
        let plugin = JsonPlugin;
        assert!(plugin.validate(r#"{"key": "value"}"#).is_ok());
    }

    #[test]
    fn json_plugin_rejects_invalid_json() {
        let plugin = JsonPlugin;
        assert!(plugin.validate("not json").is_err());
    }

    #[test]
    fn xml_plugin_supports_xml_extension() {
        let plugin = XmlPlugin;
        assert!(plugin.supports_extension("xml"));
        assert!(plugin.supports_extension("XML"));
        assert!(!plugin.supports_extension("json"));
    }

    #[test]
    fn run_plugins_returns_messages() {
        let plugins = default_plugins();
        let messages = run_plugins(&plugins, "json", r#"{"a": 1}"#);
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("OK"));
    }

    #[test]
    fn run_plugins_skips_unsupported() {
        let plugins = default_plugins();
        let messages = run_plugins(&plugins, "rs", "fn main() {}");
        assert!(messages.is_empty());
    }
}
