mod linux;
mod settings;
#[cfg(test)]
mod tests;
mod types;

pub use self::types::{
    LinuxInstallResult, LinuxReleaseAssetInfo, LinuxReleaseInfo, UpdateReminderDecision,
    UpdateRuntimeInfo, UpdateSettings,
};

pub struct UpdateManager;
