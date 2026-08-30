use crate::error::Result;
use crate::extgstate::ExtGState;
use crate::font::Font;
use crate::{ColorSpace, Pattern, XObject};
use std::collections::HashMap;

// Contains the resolved results of a given resource dict related type
#[derive(Debug, Clone)]
pub(crate) struct ResolvedCache<T: Clone>(HashMap<Vec<u8>, Result<T>>);

impl<T: Clone> Default for ResolvedCache<T> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<T: Clone> ResolvedCache<T> {
    pub(crate) fn get_or_resolve(
        &mut self,
        key: &[u8],
        resolve: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        match self.0.get(key) {
            Some(cached) => cached.clone(),
            None => {
                let resolved = resolve();
                self.0.insert(key.to_vec(), resolved.clone());
                resolved
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct WalkerCache<'a> {
    pub(crate) color_space: ResolvedCache<ColorSpace>,
    pub(crate) font: ResolvedCache<Font>,
    pub(crate) xobject: ResolvedCache<XObject<'a>>,
    pub(crate) ext_gstate: ResolvedCache<ExtGState>,
    pub(crate) pattern: ResolvedCache<Pattern<'a>>,
}
