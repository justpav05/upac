// SPDX-FileCopyrightText: 2026 JustPav
// SPDX-FileCopyrightText: 2026 SmoothTeam
//
// SPDX-License-Identifier: LGPL-3.0-or-later WITH LGPL-3.0-linking-exception

use std::io::ErrorKind as IoErrorKind;
use std::path::Path;
use std::process::Command;

use upac_abi::boot::Booter;

use crate::error::GrubError;
use crate::grub::{
    GRUBENV_FALLBACK, GRUBENV_PRIMARY, REBOOT_BIN_FALLBACK, REBOOT_BIN_PRIMARY, SET_DEFAULT_BIN_FALLBACK,
    SET_DEFAULT_BIN_PRIMARY,
};

pub struct Grub;

impl Booter for Grub {
    type Error = GrubError;

    fn new() -> Result<Self, GrubError> {
        Ok(Grub)
    }

    fn probes() -> bool {
        Path::new(GRUBENV_PRIMARY).exists() || Path::new(GRUBENV_FALLBACK).exists()
    }

    fn set_one_shot(&mut self, entry_name: &str) -> Result<(), GrubError> {
        self.run_first_available([REBOOT_BIN_PRIMARY, REBOOT_BIN_FALLBACK], entry_name)
    }

    fn confirm_boot(&mut self, entry_name: &str) -> Result<(), GrubError> {
        self.run_first_available([SET_DEFAULT_BIN_PRIMARY, SET_DEFAULT_BIN_FALLBACK], entry_name)
    }

    fn register_boot_slots(
        &mut self, esp_partition_number: u32, esp_starting_lba: u64, esp_ending_lba: u64,
        esp_unique_partition_guid: [u8; 16], to_slot: &str, from_slot: &str,
    ) -> Result<(), GrubError> {
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
}

impl Grub {
    fn run_first_available(&self, candidates: [&str; 2], entry_name: &str) -> Result<(), GrubError> {
        for candidate in candidates {
            match Command::new(candidate).arg(entry_name).status() {
                Ok(status) if status.success() => return Ok(()),
                Ok(_) => return Err(GrubError::Unexpected),
                Err(error) if error.kind() == IoErrorKind::NotFound => continue,
                Err(error) => return Err(GrubError::from(error)),
            }
        }

        Err(GrubError::ToolNotFound)
    }
}
