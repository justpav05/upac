// SPDX-FileCopyrightText: 2026 JustPav
// SPDX-FileCopyrightText: 2026 SmoothTeam
//
// SPDX-License-Identifier: LGPL-3.0-or-later WITH LGPL-3.0-linking-exception

use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::os::raw::c_long;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::str::FromStr;

use efivar::VarManager;
use efivar::efi::{Variable, VariableFlags};

use nix::{ioctl_read, ioctl_write_ptr};

use uuid::Uuid;

use upac_abi::boot::Booter;

use crate::boot::{BOOT_NEXT_VAR, BOOT_ORDER_VAR, EFI_SYSFS_PATH, EFIVARFS_PATH, LOADER_INFO_VAR, SD_BOOT_LOADER_GUID};
use crate::error::UkiError;
use crate::grub::{GRUBENV_FALLBACK, GRUBENV_PRIMARY};
use crate::refind::{PREVIOUS_BOOT_GUID, PREVIOUS_BOOT_VAR};

const FS_IMMUTABLE_FL: c_long = 0x0000_0010;

ioctl_read!(fs_ioc_getflags, b'f', 1, c_long);
ioctl_write_ptr!(fs_ioc_setflags, b'f', 2, c_long);

pub struct Uki {
    manager: Box<dyn VarManager>,
}

impl Booter for Uki {
    type Error = UkiError;

    fn new() -> Result<Self, UkiError> {
        Ok(Self {
            manager: catch_unwind(AssertUnwindSafe(efivar::system))?,
        })
    }

    fn probes() -> bool {
        if !Path::new(EFI_SYSFS_PATH).exists() {
            return false;
        }
        if Path::new(GRUBENV_PRIMARY).exists() || Path::new(GRUBENV_FALLBACK).exists() {
            return false;
        }

        let Ok(manager) = catch_unwind(AssertUnwindSafe(efivar::system)) else {
            return false;
        };

        !efi_variable_exists(manager.as_ref(), LOADER_INFO_VAR, SD_BOOT_LOADER_GUID)
            && !efi_variable_exists(manager.as_ref(), PREVIOUS_BOOT_VAR, PREVIOUS_BOOT_GUID)
    }

    fn set_one_shot(&mut self, entry_name: &str) -> Result<(), UkiError> {
        let id = self.find_boot_id(entry_name)?;

        let variable = Variable::new(BOOT_NEXT_VAR);
        Self::clear_immutable(&variable);

        self.manager
            .write(&variable, VariableFlags::default(), &id.to_le_bytes())?;

        Ok(())
    }

    fn confirm_boot(&mut self, entry_name: &str) -> Result<(), UkiError> {
        let id = self.find_boot_id(entry_name)?;

        let mut order = self.manager.get_boot_order()?;
        order.retain(|&existing| existing != id);
        order.insert(0, id);

        Self::clear_immutable(&Variable::new(BOOT_ORDER_VAR));
        self.manager.set_boot_order(order)?;

        Ok(())
    }
}

impl Uki {
    fn find_boot_id(&self, slot_filename: &str) -> Result<u16, UkiError> {
        let slot_file_name = format!("{}.efi", slot_filename.to_lowercase());

        for (entry, _var) in self.manager.get_boot_entries()? {
            let entry = entry?;
            let matches = entry
                .entry
                .file_path_list
                .as_ref()
                .is_some_and(|list| list.file_path.path.to_lowercase().ends_with(&slot_file_name));

            if matches {
                return Ok(entry.id);
            }
        }

        Err(UkiError::EntryNotFound)
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

fn efi_variable_exists(manager: &dyn VarManager, name: &str, guid: &str) -> bool {
    let Ok(guid) = Uuid::from_str(guid) else {
        return false;
    };

    manager.exists(&Variable::new_with_vendor(name, guid)).unwrap_or(false)
}
