use crate::error::PluginError;
use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

pub struct IcloudContainer<R: Runtime> {
    _marker: std::marker::PhantomData<R>,
    _default_identifier: Option<String>,
}

// Safety: this desktop stub stores no runtime data, only a marker type.
unsafe impl<R: Runtime> Send for IcloudContainer<R> {}
unsafe impl<R: Runtime> Sync for IcloudContainer<R> {}

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    _api: PluginApi<R, C>,
    default_identifier: Option<String>,
) -> Result<IcloudContainer<R>, PluginError> {
    Ok(IcloudContainer {
        _marker: std::marker::PhantomData,
        _default_identifier: default_identifier,
    })
}
