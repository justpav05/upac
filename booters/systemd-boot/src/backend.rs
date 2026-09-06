// SPDX-FileCopyrightText: 2026 JustPav
// SPDX-FileCopyrightText: 2026 SmoothTeam
//
// SPDX-License-Identifier: LGPL-3.0-or-later WITH LGPL-3.0-linking-exception

use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::os::raw::c_long;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::str::FromStr;

use efivar::VarManager;
use efivar::efi::{Variable, VariableFlags};

use nix::{ioctl_read, ioctl_write_ptr};

use uuid::Uuid;

use upac_abi::boot::Booter;

use crate::boot::{
    EFIVARFS_PATH, LOADER_ENTRY_DEFAULT_VAR, LOADER_ENTRY_ONE_SHOT_VAR, LOADER_INFO_VAR, SD_BOOT_LOADER_GUID,
};
use crate::error::BlsError;
use crate::systemd_boot::SOURCE;

const FS_IMMUTABLE_FL: c_long = 0x0000_0010;

ioctl_read!(fs_ioc_getflags, b'f', 1, c_long);
ioctl_write_ptr!(fs_ioc_setflags, b'f', 2, c_long);

pub struct Bls {
    manager: Box<dyn VarManager>,
}

impl Booter for Bls {
    type Error = BlsError;

    fn new() -> Result<Self, BlsError> {
        Ok(Self {
            manager: catch_unwind(AssertUnwindSafe(efivar::system))?,
        })
    }

    fn probes() -> bool {
        let Ok(manager) = catch_unwind(AssertUnwindSafe(efivar::system)) else {
            return false;
        };
        let Ok(guid) = Uuid::from_str(SD_BOOT_LOADER_GUID) else {
            return false;
        };

        manager
            .exists(&Variable::new_with_vendor(LOADER_INFO_VAR, guid))
            .unwrap_or(false)
    }

    fn set_one_shot(&mut self, entry_name: &str) -> Result<(), BlsError> {
        self.write_loader_variable(LOADER_ENTRY_ONE_SHOT_VAR, entry_name)
    }

    fn confirm_boot(&mut self, entry_name: &str) -> Result<(), BlsError> {
        self.write_loader_variable(LOADER_ENTRY_DEFAULT_VAR, entry_name)
    }

    fn esp_loader_source() -> Option<&'static str> {
        Some(SOURCE)
    }

    fn register_boot_slots(
        &mut self, esp_partition_number: u32, esp_starting_lba: u64, esp_ending_lba: u64,
        esp_unique_partition_guid: [u8; 16], to_slot: &str, from_slot: &str,
    ) -> Result<(), BlsError> {
        let _ = (
            esp_partition_number,
            esp_starting_lba,
            esp_ending_lba,
            esp_unique_partition_guid,
            to_slot,
            from_slot,
        );

        Ok(())
    }

    fn install(&mut self, esp_mount_point: &str) -> Result<(), BlsError> {
        let _ = esp_mount_point;

        Ok(())
    }
}

impl Bls {
    fn write_loader_variable(&mut self, name: &str, entry_name: &str) -> Result<(), BlsError> {
        let guid = Uuid::from_str(SD_BOOT_LOADER_GUID)?;
        let variable = Variable::new_with_vendor(name, guid);

        Self::clear_immutable(&variable);

        self.manager
            .write(&variable, VariableFlags::default(), &encode_utf16_null(entry_name))?;

        Ok(())
    }

    fn clear_immutable(variable: &Variable) {
        let Ok(file) = OpenOptions::new()
            .read(true)
            .open(format!("{EFIVARFS_PATH}/{variable}"))
        else {
            return;
        };
        let fd = file.as_raw_fd();

        let mut flags: c_long = 0;
        if unsafe { fs_ioc_getflags(fd, &mut flags) }.is_err() {
            return;
        }

        if flags & FS_IMMUTABLE_FL != 0 {
            flags &= !FS_IMMUTABLE_FL;
            let _ = unsafe { fs_ioc_setflags(fd, &flags) };
        }
    }
}

fn encode_utf16_null(value: &str) -> Vec<u8> {
    let mut bytes: Vec<u8> = value.encode_utf16().flat_map(u16::to_le_bytes).collect();
    bytes.extend_from_slice(&[0x00, 0x00]);

    bytes
}
