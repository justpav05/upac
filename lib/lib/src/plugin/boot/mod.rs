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
use upac_abi::BOOT_ABI_VERSION;

#[cfg(feature = "dynamic-plugins")]
use upac_abi::boot::AbiVersionFn;

#[cfg(feature = "dynamic-plugins")]
use crate::plugin::boot::manifest::load_boot_plugin_manifests;

#[cfg(feature = "builtin-grub")]
use upac_boot_grub::{
    confirm_boot as grub_confirm_boot, esp_loader_source as grub_esp_loader_source, install as grub_install,
    probe as grub_probe, register_boot_slots as grub_register_boot_slots, set_one_shot as grub_set_one_shot,
};

#[cfg(feature = "builtin-systemd-boot")]
use upac_boot_systemd_boot::{
    confirm_boot as systemd_boot_confirm_boot, esp_loader_source as systemd_boot_esp_loader_source,
    install as systemd_boot_install, probe as systemd_boot_probe,
    register_boot_slots as systemd_boot_register_boot_slots, set_one_shot as systemd_boot_set_one_shot,
};

#[cfg(feature = "builtin-uki")]
use upac_boot_uki::{
    confirm_boot as uki_confirm_boot, esp_loader_source as uki_esp_loader_source, install as uki_install,
    probe as uki_probe, register_boot_slots as uki_register_boot_slots, set_one_shot as uki_set_one_shot,
};

#[cfg(feature = "builtin-refind")]
use upac_boot_refind::{
    confirm_boot as refind_confirm_boot, esp_loader_source as refind_esp_loader_source, install as refind_install,
    probe as refind_probe, register_boot_slots as refind_register_boot_slots, set_one_shot as refind_set_one_shot,
};

pub mod error;

#[cfg(feature = "dynamic-plugins")]
pub mod manifest;

#[cfg(feature = "builtin-booters")]
impl BootPlugin {
    fn from_static(
        probe: ProbeFn, set_one_shot: SetOneShotFn, confirm_boot: ConfirmBootFn, esp_loader_source: EspLoaderSourceFn,
        register_boot_slots: RegisterBootSlotsFn, install: InstallFn,
    ) -> Self {
        BootPlugin {
            probe,
            set_one_shot,
            confirm_boot,
            esp_loader_source,
            register_boot_slots,
            install,

            #[cfg(feature = "dynamic-plugins")]
            _library: None,
        }
    }
}

/// Resolves a boot plugin by loading shared objects described by on-disk manifests.
///
/// Built with `dynamic-plugins`: plugins are discovered at runtime from
/// `boot_plugins_dir`. Any plugin compiled in via `builtin-*` is still reachable
/// through [`static_plugins`], but on-disk manifests take part in the same search.
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
            if let Some((_, plugin)) = static_plugins()
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
            for (_, plugin) in static_plugins() {
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
        let plugins = static_plugins();

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

/// The boot plugins linked into this build, in probe order.
///
/// No ABI version check is performed here: these are compiled from the same source
/// tree by the same compiler, so [`BOOT_ABI_VERSION`] matches by construction.
#[cfg(feature = "builtin-booters")]
#[allow(
    clippy::vec_init_then_push,
    reason = "each push is independently cfg-gated, vec![] can't express that"
)]
fn static_plugins() -> Vec<(&'static str, BootPlugin)> {
    let mut plugins = Vec::new();

    #[cfg(feature = "builtin-uki")]
    plugins.push((
        "uki",
        BootPlugin::from_static(
            uki_probe,
            uki_set_one_shot,
            uki_confirm_boot,
            uki_esp_loader_source,
            uki_register_boot_slots,
            uki_install,
        ),
    ));

    #[cfg(feature = "builtin-systemd-boot")]
    plugins.push((
        "systemd-boot",
        BootPlugin::from_static(
            systemd_boot_probe,
            systemd_boot_set_one_shot,
            systemd_boot_confirm_boot,
            systemd_boot_esp_loader_source,
            systemd_boot_register_boot_slots,
            systemd_boot_install,
        ),
    ));

    #[cfg(feature = "builtin-grub")]
    plugins.push((
        "grub",
        BootPlugin::from_static(
            grub_probe,
            grub_set_one_shot,
            grub_confirm_boot,
            grub_esp_loader_source,
            grub_register_boot_slots,
            grub_install,
        ),
    ));

    #[cfg(feature = "builtin-refind")]
    plugins.push((
        "refind",
        BootPlugin::from_static(
            refind_probe,
            refind_set_one_shot,
            refind_confirm_boot,
            refind_esp_loader_source,
            refind_register_boot_slots,
            refind_install,
        ),
    ));

    plugins
}

#[cfg(feature = "dynamic-plugins")]
unsafe fn load_symbol<T: Copy>(library: &Library, name: &str) -> Result<T, BootPluginError> {
    unsafe { library.get::<T>(name.as_bytes()) }
        .map(|symbol| *symbol)
        .map_err(|_| BootPluginError::Symbol)
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

#[cfg(feature = "dynamic-plugins")]
impl BootPlugin {
    pub fn load(library_name: &str) -> Result<Self, BootPluginError> {
        let library = unsafe { Library::new(library_name) }.map_err(|_| BootPluginError::Load)?;

        let abi_version: AbiVersionFn = unsafe { load_symbol(&library, "abi_version")? };
        let probe: ProbeFn = unsafe { load_symbol(&library, "probe")? };
        let set_one_shot: SetOneShotFn = unsafe { load_symbol(&library, "set_one_shot")? };
        let confirm_boot: ConfirmBootFn = unsafe { load_symbol(&library, "confirm_boot")? };
        let esp_loader_source: EspLoaderSourceFn = unsafe { load_symbol(&library, "esp_loader_source")? };
        let register_boot_slots: RegisterBootSlotsFn = unsafe { load_symbol(&library, "register_boot_slots")? };
        let install: InstallFn = unsafe { load_symbol(&library, "install")? };

        let got = unsafe { abi_version() };
        if got != BOOT_ABI_VERSION {
            return Err(BootPluginError::AbiMismatch {
                got,
                expected: BOOT_ABI_VERSION,
            });
        }

        Ok(BootPlugin {
            probe,
            set_one_shot,
            confirm_boot,
            esp_loader_source,
            register_boot_slots,
            install,
            _library: Some(library),
        })
    }
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
