use crate::{DynReleaseProvider, Error, Result, config::InstallConfig};

#[derive(Clone)]
pub(super) struct ReleaseRouteState {
    pub(super) provider: Option<DynReleaseProvider>,
    pub(super) install: InstallConfig,
}

impl ReleaseRouteState {
    pub(super) fn provider(&self) -> Result<DynReleaseProvider> {
        self.provider.clone().ok_or(Error::NotConfigured)
    }
}
