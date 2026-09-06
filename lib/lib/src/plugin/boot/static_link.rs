// SPDX-FileCopyrightText: 2026 JustPav
// SPDX-FileCopyrightText: 2026 SmoothTeam
//
// SPDX-License-Identifier: LGPL-3.0-or-later WITH LGPL-3.0-linking-exception

use upac_abi::boot::{ConfirmBootFn, EspLoaderSourceFn, InstallFn, ProbeFn, RegisterBootSlotsFn, SetOneShotFn};

use super::BootPlugin;

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

/// The boot plugins linked into this build, in probe order.
///
/// No ABI version check is performed here: these are compiled from the same source
/// tree by the same compiler, so `BOOT_ABI_VERSION` matches by construction.
#[allow(
    clippy::vec_init_then_push,
    reason = "each push is independently cfg-gated, vec![] can't express that"
)]
pub(super) fn static_plugins() -> Vec<(&'static str, BootPlugin)> {
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
