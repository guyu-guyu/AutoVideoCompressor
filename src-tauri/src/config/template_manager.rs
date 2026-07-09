/// Resolves template names to ffmpeg params. Mirrors TemplateManager (C++).
#[derive(Default, Clone)]
pub struct TemplateManager {
    templates: Vec<(String, String)>, // (name, params), order-preserving
}

impl TemplateManager {
    pub fn new() -> Self {
        TemplateManager::default()
    }

    pub fn set_templates(&mut self, tmpls: Vec<(String, String)>) {
        self.templates = tmpls;
    }

    /// Empty → empty; matching name → its params; otherwise return input verbatim.
    pub fn resolve(&self, name_or_params: &str) -> String {
        if name_or_params.is_empty() {
            return String::new();
        }
        for (name, params) in &self.templates {
            if name == name_or_params {
                return params.clone();
            }
        }
        name_or_params.to_string()
    }

    pub fn names(&self) -> Vec<String> {
        self.templates.iter().map(|(n, _)| n.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_by_name_or_passthrough() {
        let mut tm = TemplateManager::new();
        tm.set_templates(vec![("H.265".into(), "-c:v libx265".into())]);
        assert_eq!(tm.resolve("H.265"), "-c:v libx265");
        assert_eq!(tm.resolve("-c:v libx264"), "-c:v libx264");
        assert_eq!(tm.resolve(""), "");
    }

    #[test]
    fn names_are_ordered() {
        let mut tm = TemplateManager::new();
        tm.set_templates(vec![("a".into(), "1".into()), ("b".into(), "2".into())]);
        assert_eq!(tm.names(), vec!["a".to_string(), "b".to_string()]);
    }
}
