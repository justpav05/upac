// SPDX-FileCopyrightText: 2026 JustPav
// SPDX-FileCopyrightText: 2026 SmoothTeam
//
// SPDX-License-Identifier: LGPL-3.0-or-later WITH LGPL-3.0-linking-exception

use std::str::from_utf8;

use upac_abi::BOOT_ABI_VERSION;
use upac_abi::boot::{Booter, CBootPluginRequest, CBootSlotsRequest};
use upac_abi::error::ErrorKind;
use upac_abi::types::{CBorrowed, CSlice};

use crate::backend::Uki;
use crate::error::UkiError;

mod backend;
mod error;

include!(concat!(env!("OUT_DIR"), "/layout.rs"));

/// # Safety
/// Touches no pointers — `unsafe extern "C"` only to match `upac_abi::boot::AbiVersionFn`.
#[cfg_attr(feature = "cdylib", unsafe(no_mangle))]
pub unsafe extern "C" fn abi_version() -> u32 {
    BOOT_ABI_VERSION
}

/// # Safety
/// Touches no pointers — `unsafe extern "C"` only to match `upac_abi::boot::ProbeFn`.
#[cfg_attr(feature = "cdylib", unsafe(no_mangle))]
pub unsafe extern "C" fn probe() -> i32 {
    i32::from(Uki::probes())
}

/// # Safety
/// Touches no pointers — `unsafe extern "C"` only to match `upac_abi::boot::EspLoaderSourceFn`.
#[cfg_attr(feature = "cdylib", unsafe(no_mangle))]
pub unsafe extern "C" fn esp_loader_source() -> CSlice {
    CSlice::from_slice(Uki::esp_loader_source().map(str::as_bytes))
}

/// # Safety
/// `request`, if non-null, must point to a valid, initialized `CBootPluginRequest` for the
/// duration of the call. `err_out`, if non-null, must point to writable `ErrorKind` storage.
#[cfg_attr(feature = "cdylib", unsafe(no_mangle))]
pub unsafe extern "C" fn set_one_shot(request: *const CBootPluginRequest, err_out: *mut ErrorKind) -> i32 {
    if request.is_null() {
        write_error(err_out, UkiError::InvalidRequest);
        return -1;
    }

    let result = entry_name_from_request(unsafe { &*request })
        .and_then(|entry_name| Uki::new().and_then(|mut uki| uki.set_one_shot(&entry_name)));

    match result {
        Ok(()) => 0,
        Err(error) => {
            write_error(err_out, error);
            -1
        }
    }
}

/// # Safety
/// `request`, if non-null, must point to a valid, initialized `CBootPluginRequest` for the
/// duration of the call. `err_out`, if non-null, must point to writable `ErrorKind` storage.
#[cfg_attr(feature = "cdylib", unsafe(no_mangle))]
pub unsafe extern "C" fn confirm_boot(request: *const CBootPluginRequest, err_out: *mut ErrorKind) -> i32 {
    if request.is_null() {
        write_error(err_out, UkiError::InvalidRequest);
        return -1;
    }

    let result = entry_name_from_request(unsafe { &*request })
        .and_then(|entry_name| Uki::new().and_then(|mut uki| uki.confirm_boot(&entry_name)));

    match result {
        Ok(()) => 0,
        Err(error) => {
            write_error(err_out, error);
            -1
        }
    }
}

/// # Safety
/// `request`, if non-null, must point to a valid, initialized `CBootSlotsRequest` for the
/// duration of the call. `err_out`, if non-null, must point to writable `ErrorKind` storage.
#[cfg_attr(feature = "cdylib", unsafe(no_mangle))]
pub unsafe extern "C" fn register_boot_slots(request: *const CBootSlotsRequest, err_out: *mut ErrorKind) -> i32 {
    if request.is_null() {
        write_error(err_out, UkiError::InvalidRequest);
        return -1;
    }

    let request = unsafe { &*request };

    let result = slots_from_request(request).and_then(|(to_slot, from_slot)| {
        Uki::new().and_then(|mut uki| {
            uki.register_boot_slots(
                request.esp_partition_number,
                request.esp_starting_lba,
                request.esp_ending_lba,
                request.esp_unique_partition_guid,
                &to_slot,
                &from_slot,
            )
        })
    });

    match result {
        Ok(()) => 0,
        Err(error) => {
            write_error(err_out, error);
            -1
        }
    }
}

/// # Safety
/// Touches no pointers — uki has nothing to install onto a pre-existing ESP, always succeeds
/// (its binary is copied from the source package tree via `esp_loader_source` instead).
#[cfg_attr(feature = "cdylib", unsafe(no_mangle))]
pub unsafe extern "C" fn install(_request: *const CBootPluginRequest, _err_out: *mut ErrorKind) -> i32 {
    0
}

fn entry_name_from_request(request: &CBootPluginRequest) -> Result<String, UkiError> {
    let bytes = unsafe { request.value.as_borrowed() };

    from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| UkiError::InvalidRequest)
}

fn slots_from_request(request: &CBootSlotsRequest) -> Result<(String, String), UkiError> {
    let to_bytes = unsafe { request.to_slot.as_borrowed() };
    let from_bytes = unsafe { request.from_slot.as_borrowed() };

    let to_slot = from_utf8(to_bytes)
        .map(str::to_owned)
        .map_err(|_| UkiError::InvalidRequest)?;
    let from_slot = from_utf8(from_bytes)
        .map(str::to_owned)
        .map_err(|_| UkiError::InvalidRequest)?;

    Ok((to_slot, from_slot))
}

fn write_error(err_out: *mut ErrorKind, error: UkiError) {
    if !err_out.is_null() {
        unsafe { *err_out = error.into() };
    }
}
