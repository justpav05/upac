// SPDX-FileCopyrightText: 2026 JustPav
// SPDX-FileCopyrightText: 2026 SmoothTeam
//
// SPDX-License-Identifier: LGPL-3.0-or-later WITH LGPL-3.0-linking-exception

use std::str::from_utf8;

use upac_abi::BOOT_ABI_VERSION;
use upac_abi::boot::{Booter, CBootPluginRequest};
use upac_abi::error::ErrorKind;
use upac_abi::types::{CBorrowed, CSlice};

use crate::backend::Grub;
use crate::error::GrubError;

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
    i32::from(Grub::probes())
}

/// # Safety
/// Touches no pointers — `unsafe extern "C"` only to match `upac_abi::boot::EspLoaderSourceFn`.
#[cfg_attr(feature = "cdylib", unsafe(no_mangle))]
pub unsafe extern "C" fn esp_loader_source() -> CSlice {
    CSlice::from_slice(Grub::esp_loader_source().map(str::as_bytes))
}

/// # Safety
/// `request`, if non-null, must point to a valid, initialized `CBootPluginRequest` for the
/// duration of the call. `err_out`, if non-null, must point to writable `ErrorKind` storage.
#[cfg_attr(feature = "cdylib", unsafe(no_mangle))]
pub unsafe extern "C" fn set_one_shot(request: *const CBootPluginRequest, err_out: *mut ErrorKind) -> i32 {
    if request.is_null() {
        write_error(err_out, GrubError::InvalidRequest);
        return -1;
    }

    let result = entry_name_from_request(unsafe { &*request })
        .and_then(|entry_name| Grub::new().and_then(|mut grub| grub.set_one_shot(&entry_name)));

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
        write_error(err_out, GrubError::InvalidRequest);
        return -1;
    }

    let result = entry_name_from_request(unsafe { &*request })
        .and_then(|entry_name| Grub::new().and_then(|mut grub| grub.confirm_boot(&entry_name)));

    match result {
        Ok(()) => 0,
        Err(error) => {
            write_error(err_out, error);
            -1
        }
    }
}

fn entry_name_from_request(request: &CBootPluginRequest) -> Result<String, GrubError> {
    let bytes = unsafe { request.entry_name.as_borrowed() };

    from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| GrubError::InvalidRequest)
}

fn write_error(err_out: *mut ErrorKind, error: GrubError) {
    if !err_out.is_null() {
        unsafe { *err_out = error.into() };
    }
}
