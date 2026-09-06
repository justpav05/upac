// SPDX-FileCopyrightText: 2026 JustPav
// SPDX-FileCopyrightText: 2026 SmoothTeam
//
// SPDX-License-Identifier: LGPL-3.0-or-later WITH LGPL-3.0-linking-exception

use std::mem::MaybeUninit;

use upac_abi::boot::{
    CBootPluginRequest, CBootSlotsRequest, ConfirmBootFn, EspLoaderSourceFn, InstallFn, ProbeFn, RegisterBootSlotsFn,
    SetOneShotFn,
};
use upac_abi::error::ErrorKind;
use upac_abi::types::{CBorrowed, CSlice};

use crate::plugin::boot::error::BootPluginError;

#[cfg(feature = "dynamic-plugins")]
use libloading::Library;

#[cfg(feature = "dynamic-plugins")]
use crate::plugin::boot::manifest::load_boot_plugin_manifests;

pub mod error;

#[cfg(feature = "dynamic-plugins")]
pub mod manifest;

#[cfg(feature = "dynamic-plugins")]
mod dynamic_link;

#[cfg(feature = "builtin-booters")]
mod static_link;

/// Resolves a boot plugin by loading shared objects described by on-disk manifests.
///
/// Built with `dynamic-plugins`: plugins are discovered at runtime from
/// `boot_plugins_dir`. Any plugin compiled in via `builtin-*` is still reachable
/// through [`static_link::static_plugins`], but on-disk manifests take part in the
/// same search.
#[cfg(feature = "dynamic-plugins")]
pub fn resolve_boot_plugin(
    boot_plugins_dir: &str, manifest_extension: &str, requested: Option<&str>,
) -> Result<BootPlugin, BootPluginError> {
    let manifests = load_boot_plugin_manifests(boot_plugins_dir, manifest_extension)?;

    match requested {
        Some(name) => {
            if let Some(manifest) = manifests.get(name) {
                return BootPlugin::load(&manifest.library);
            }

            #[cfg(feature = "builtin-booters")]
            if let Some((_, plugin)) = static_link::static_plugins()
                .into_iter()
                .find(|(plugin_name, _)| *plugin_name == name)
            {
                return Ok(plugin);
            }

            Err(BootPluginError::UnknownName(name.to_owned()))
        }
        None => {
            let mut claimants = Vec::new();
            for manifest in manifests.values() {
                let plugin = BootPlugin::load(&manifest.library)?;
                if plugin.probes() {
                    claimants.push(plugin);
                }
            }

            #[cfg(feature = "builtin-booters")]
            for (_, plugin) in static_link::static_plugins() {
                if plugin.probes() {
                    claimants.push(plugin);
                }
            }

            let mut claimants = claimants.into_iter();
            match (claimants.next(), claimants.next()) {
                (Some(plugin), None) => Ok(plugin),
                (None, _) => Err(BootPluginError::NoClaimant),
                (Some(_), Some(_)) => Err(BootPluginError::AmbiguousClaim),
            }
        }
    }
}

/// Resolves a boot plugin from the set compiled into this build.
///
/// Built without `dynamic-plugins`: this binary contains no code path that loads
/// executable objects from disk. `boot_plugins_dir` and `manifest_extension` are
/// accepted to keep the signature stable across build configurations, and ignored.
///
/// With no `builtin-*` feature enabled the candidate set is empty and every call
/// returns [`BootPluginError::NoClaimant`].
#[cfg(not(feature = "dynamic-plugins"))]
pub fn resolve_boot_plugin(
    _boot_plugins_dir: &str, _manifest_extension: &str, requested: Option<&str>,
) -> Result<BootPlugin, BootPluginError> {
    #[cfg(not(feature = "builtin-booters"))]
    {
        let _ = requested;
        Err(BootPluginError::NoClaimant)
    }

    #[cfg(feature = "builtin-booters")]
    {
        let plugins = static_link::static_plugins();

        match requested {
            Some(name) => plugins
                .into_iter()
                .find(|(plugin_name, _)| *plugin_name == name)
                .map(|(_, plugin)| plugin)
                .ok_or_else(|| BootPluginError::UnknownName(name.to_owned())),
            None => {
                let mut claimants = plugins.into_iter().filter(|(_, plugin)| plugin.probes());

                match (claimants.next(), claimants.next()) {
                    (Some((_, plugin)), None) => Ok(plugin),
                    (None, _) => Err(BootPluginError::NoClaimant),
                    (Some(_), Some(_)) => Err(BootPluginError::AmbiguousClaim),
                }
            }
        }
    }
}

pub struct BootPlugin {
    probe: ProbeFn,
    set_one_shot: SetOneShotFn,
    confirm_boot: ConfirmBootFn,
    esp_loader_source: EspLoaderSourceFn,
    register_boot_slots: RegisterBootSlotsFn,
    install: InstallFn,

    #[cfg(feature = "dynamic-plugins")]
    _library: Option<Library>,
}

impl BootPlugin {
    pub fn probes(&self) -> bool {
        unsafe { (self.probe)() == 1 }
    }

    pub fn set_one_shot(&self, entry_name: &str) -> Result<(), BootPluginError> {
        let request = CBootPluginRequest::new(CSlice::from_borrowed(entry_name.as_bytes()));
        let mut error = MaybeUninit::<ErrorKind>::uninit();

        let code = unsafe { (self.set_one_shot)(&request, error.as_mut_ptr()) };
        if code != 0 {
            return Err(BootPluginError::Reported(unsafe { error.assume_init() }));
        }

        Ok(())
    }

    pub fn confirm_boot(&self, entry_name: &str) -> Result<(), BootPluginError> {
        let request = CBootPluginRequest::new(CSlice::from_borrowed(entry_name.as_bytes()));
        let mut error = MaybeUninit::<ErrorKind>::uninit();

        let code = unsafe { (self.confirm_boot)(&request, error.as_mut_ptr()) };
        if code != 0 {
            return Err(BootPluginError::Reported(unsafe { error.assume_init() }));
        }

        Ok(())
    }

    pub fn esp_loader_source(&self) -> Option<String> {
        let slice = unsafe { (self.esp_loader_source)() };

        Option::<&str>::try_from(&slice).ok().flatten().map(str::to_owned)
    }

    pub fn register_boot_slots(
        &self, esp_partition_number: u32, esp_starting_lba: u64, esp_ending_lba: u64,
        esp_unique_partition_guid: [u8; 16], to_slot: &str, from_slot: &str,
    ) -> Result<(), BootPluginError> {
        let request = CBootSlotsRequest::new(
            esp_partition_number,
            esp_starting_lba,
            esp_ending_lba,
            esp_unique_partition_guid,
            CSlice::from_borrowed(to_slot.as_bytes()),
            CSlice::from_borrowed(from_slot.as_bytes()),
        );
        let mut error = MaybeUninit::<ErrorKind>::uninit();

        let code = unsafe { (self.register_boot_slots)(&request, error.as_mut_ptr()) };
        if code != 0 {
            return Err(BootPluginError::Reported(unsafe { error.assume_init() }));
        }

        Ok(())
    }

    pub fn install(&self, esp_mount_point: &str) -> Result<(), BootPluginError> {
        let request = CBootPluginRequest::new(CSlice::from_borrowed(esp_mount_point.as_bytes()));
        let mut error = MaybeUninit::<ErrorKind>::uninit();

        let code = unsafe { (self.install)(&request, error.as_mut_ptr()) };
        if code != 0 {
            return Err(BootPluginError::Reported(unsafe { error.assume_init() }));
        }

        Ok(())
    }
}
