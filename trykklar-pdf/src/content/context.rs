use crate::ocg::OcConfig;

/// Content Walker Context
///
/// By setting the optional content default config, processors can set custom oc contexts.
#[derive(Debug, Clone)]
pub struct WalkerContext {
    pub(crate) oc_default_config: Option<OcConfig>,
}

impl WalkerContext {
    /// Creates a new walker context.
    pub fn new(oc_default_config: Option<OcConfig>) -> Self {
        Self { oc_default_config }
    }
}
