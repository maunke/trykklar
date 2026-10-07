use crate::optional_content::OcConfig;

/// Content Walker Context
///
/// By setting the optional content default config, processors can set custom oc contexts.
#[derive(Debug, Clone, Default)]
pub struct WalkerContext {
    pub(crate) oc_default_config: Option<OcConfig>,
}

impl WalkerContext {
    /// Creates a new walker context.
    pub fn new() -> Self {
        Self::default()
    }
    /// Returns the walker context with given oc config.
    pub fn with_oc_default_config(mut self, oc_default_config: OcConfig) -> Self {
        self.oc_default_config = Some(oc_default_config);
        self
    }
}
