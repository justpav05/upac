// SPDX-FileCopyrightText: 2026 JustPav
// SPDX-FileCopyrightText: 2026 SmoothTeam
//
// SPDX-License-Identifier: LGPL-3.0-or-later WITH LGPL-3.0-linking-exception

use upac_macro::CNew;

use crate::error::ErrorKind;
use crate::types::CSlice;

pub type AbiVersionFn = unsafe extern "C" fn() -> u32;

pub type ProbeFn = unsafe extern "C" fn() -> i32;

pub type SetOneShotFn = unsafe extern "C" fn(request: *const CBootPluginRequest, err_out: *mut ErrorKind) -> i32;

pub type ConfirmBootFn = unsafe extern "C" fn(request: *const CBootPluginRequest, err_out: *mut ErrorKind) -> i32;

pub type EspLoaderSourceFn = unsafe extern "C" fn() -> CSlice;

pub type RegisterBootSlotsFn = unsafe extern "C" fn(request: *const CBootSlotsRequest, err_out: *mut ErrorKind) -> i32;

pub trait Booter: Sized {
    type Error;

    fn new() -> Result<Self, Self::Error>;
    fn probes() -> bool;
    fn set_one_shot(&mut self, entry_name: &str) -> Result<(), Self::Error>;
    fn confirm_boot(&mut self, entry_name: &str) -> Result<(), Self::Error>;

    fn esp_loader_source() -> Option<&'static str> {
        None
    }

    fn register_boot_slots(
        &mut self, esp_partition_number: u32, esp_starting_lba: u64, esp_ending_lba: u64,
        esp_unique_partition_guid: [u8; 16], to_slot: &str, from_slot: &str,
    ) -> Result<(), Self::Error> {
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

#[repr(C)]
#[derive(CNew)]
pub struct CBootPluginRequest {
    pub struct_size: usize,

    pub entry_name: CSlice,
}

#[repr(C)]
#[derive(CNew)]
pub struct CBootSlotsRequest {
    pub struct_size: usize,

    pub esp_partition_number: u32,
    pub esp_starting_lba: u64,
    pub esp_ending_lba: u64,
    pub esp_unique_partition_guid: [u8; 16],
    pub to_slot: CSlice,
    pub from_slot: CSlice,
}
