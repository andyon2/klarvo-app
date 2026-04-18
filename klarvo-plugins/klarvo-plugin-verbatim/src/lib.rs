use std::sync::Arc;

use klarvo_core::PluginRegistry;

mod provider;

pub use provider::Verbatim;

pub const ID: &str = "verbatim";

pub fn register(registry: &mut PluginRegistry) {
    registry.register_cleanup(ID, Arc::new(Verbatim::new()));
}
