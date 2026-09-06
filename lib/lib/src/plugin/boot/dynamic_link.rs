// SPDX-FileCopyrightText: 2026 JustPav
// SPDX-FileCopyrightText: 2026 SmoothTeam
//
// SPDX-License-Identifier: LGPL-3.0-or-later WITH LGPL-3.0-linking-exception

use libloading::Library;

use upac_abi::BOOT_ABI_VERSION;
use upac_abi::boot::{
    AbiVersionFn, ConfirmBootFn, EspLoaderSourceFn, InstallFn, ProbeFn, RegisterBootSlotsFn, SetOneShotFn,
};

use super::BootPlugin;
use super::error::BootPluginError;

unsafe fn load_symbol<T: Copy>(library: &Library, name: &str) -> Result<T, BootPluginError> {
    unsafe { library.get::<T>(name.as_bytes()) }
        .map(|symbol| *symbol)
        .map_err(|_| BootPluginError::Symbol)
}

impl BootPlugin {
    pub(super) fn load(library_name: &str) -> Result<Self, BootPluginError> {
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
