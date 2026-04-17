#[derive(Default)]
pub struct PluginRegistry {
    _private: (),
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn bootstrap() -> PluginRegistry {
    PluginRegistry::new()
}
