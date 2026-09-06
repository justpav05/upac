// SPDX-FileCopyrightText: 2026 JustPav
// SPDX-FileCopyrightText: 2026 SmoothTeam
//
// SPDX-License-Identifier: LGPL-3.0-or-later WITH LGPL-3.0-linking-exception

use upac_types::{Dependency, PackageMeta};

#[cfg(any(feature = "dynamic-plugins", feature = "builtin-decoders"))]
use std::mem::MaybeUninit;
#[cfg(any(feature = "dynamic-plugins", feature = "builtin-decoders"))]
use std::str::from_utf8;

#[cfg(feature = "dynamic-plugins")]
use libloading::Library;

#[cfg(feature = "dynamic-plugins")]
use upac_abi::DECODER_ABI_VERSION;

#[cfg(feature = "dynamic-plugins")]
use upac_abi::decoder::AbiVersionFn;

#[cfg(any(feature = "dynamic-plugins", feature = "builtin-decoders"))]
use upac_abi::decoder::{CDecodeRequest, CDecodeResponse, DecodeFn};

#[cfg(any(feature = "dynamic-plugins", feature = "builtin-decoders"))]
use upac_abi::hook::CancelToken;

#[cfg(any(feature = "dynamic-plugins", feature = "builtin-decoders"))]
use upac_abi::types::{CBorrowed, CSlice};

#[cfg(any(feature = "dynamic-plugins", feature = "builtin-decoders"))]
use crate::plugin::decoder::error::DecoderError;

#[cfg(feature = "builtin-alpm")]
use upac_decoders_alpm::{decode as alpm_decode, manifest as alpm_manifest};

#[cfg(feature = "builtin-deb")]
use upac_decoders_deb::{decode as deb_decode, manifest as deb_manifest};

#[cfg(feature = "builtin-rpm")]
use upac_decoders_rpm::{decode as rpm_decode, manifest as rpm_manifest};

#[cfg(feature = "builtin-xbps")]
use upac_decoders_xbps::{decode as xbps_decode, manifest as xbps_manifest};

pub mod error;
pub mod manifest;
pub mod triggers;
pub mod unpack;

/// A package decoded by a decoder plugin.
///
/// Plain owned data — available in every build configuration, including ones
/// without `dynamic-plugins`/`builtin-decoders`, so that callers and error
/// types elsewhere in the crate keep compiling.
pub struct DecodedPackage {
    pub meta: PackageMeta,
    pub dependencies: Vec<Dependency>,
    pub declarative_triggers: Vec<String>,
}

#[cfg(feature = "dynamic-plugins")]
unsafe fn load_symbol<T: Copy>(library: &Library, name: &str) -> Result<T, DecoderError> {
    unsafe { library.get::<T>(name.as_bytes()) }
        .map(|symbol| *symbol)
        .map_err(|_| DecoderError::Symbol)
}

/// A decoder plugin, either loaded from a shared object at runtime (`dynamic-plugins`) or
/// compiled directly into this binary (`builtin-decoders`).
#[cfg(any(feature = "dynamic-plugins", feature = "builtin-decoders"))]
pub struct Decoder {
    decode: DecodeFn,

    #[cfg(feature = "dynamic-plugins")]
    _library: Option<Library>,
}

#[cfg(feature = "builtin-decoders")]
impl Decoder {
    fn from_static(decode: DecodeFn) -> Self {
        Decoder {
            decode,

            #[cfg(feature = "dynamic-plugins")]
            _library: None,
        }
    }
}

#[cfg(feature = "dynamic-plugins")]
impl Decoder {
    pub fn load(library_name: &str) -> Result<Self, DecoderError> {
        let library = unsafe { Library::new(library_name) }.map_err(|_| DecoderError::Load)?;

        let abi_version: AbiVersionFn = unsafe { load_symbol(&library, "abi_version")? };
        let decode: DecodeFn = unsafe { load_symbol(&library, "decode")? };

        let got = unsafe { abi_version() };
        if got != DECODER_ABI_VERSION {
            return Err(DecoderError::AbiMismatch {
                got,
                expected: DECODER_ABI_VERSION,
            });
        }

        Ok(Decoder {
            decode,
            _library: Some(library),
        })
    }
}

#[cfg(any(feature = "dynamic-plugins", feature = "builtin-decoders"))]
impl Decoder {
    pub fn decode(
        &self, package_path: &str, output_dir: &str, checksum: [u8; 32], cancel: &CancelToken,
    ) -> Result<DecodedPackage, DecoderError> {
        let request = CDecodeRequest::new(
            CSlice::from_borrowed(package_path.as_bytes()),
            CSlice::from_borrowed(output_dir.as_bytes()),
            checksum,
            cancel as *const CancelToken as *mut CancelToken,
        );

        let mut response = MaybeUninit::<CDecodeResponse>::uninit();

        let code = unsafe { (self.decode)(&request, response.as_mut_ptr()) };
        if code != 0 {
            return Err(DecoderError::Failed(code));
        }

        let response = unsafe { response.assume_init() };

        unsafe { response.validate() }?;

        let meta = PackageMeta::try_from(&response.meta)?;

        let dependencies = unsafe { response.dependencies.as_slice() }
            .iter()
            .map(Dependency::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        let declarative_triggers = unsafe { response.declarative_triggers.as_slice() }
            .iter()
            .map(|trigger| unsafe { trigger.as_borrowed() })
            .map(|bytes| from_utf8(bytes).map(str::to_owned))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| DecoderError::InvalidResponse)?;

        Ok(DecodedPackage {
            meta,
            dependencies,
            declarative_triggers,
        })
    }
}

/// The decoders compiled directly into this binary, keyed by format name with their claimed
/// extensions — mirrors `plugin::boot::static_plugins`, adapted for extension-based dispatch
/// (a decoder is selected by the package file's extension, not by a `probe()` call). No ABI
/// version check: compiled from the same source tree by the same compiler, so the decoder's own
/// `DECODER_ABI_VERSION` matches by construction.
#[cfg(feature = "builtin-decoders")]
#[allow(
    clippy::vec_init_then_push,
    reason = "each push is independently cfg-gated, vec![] can't express that"
)]
pub(crate) fn static_decoders() -> Vec<(&'static str, &'static [&'static str], Decoder)> {
    let mut decoders = Vec::new();

    #[cfg(feature = "builtin-alpm")]
    decoders.push((
        alpm_manifest::FORMAT,
        alpm_manifest::EXTENSIONS,
        Decoder::from_static(alpm_decode),
    ));

    #[cfg(feature = "builtin-deb")]
    decoders.push((
        deb_manifest::FORMAT,
        deb_manifest::EXTENSIONS,
        Decoder::from_static(deb_decode),
    ));

    #[cfg(feature = "builtin-rpm")]
    decoders.push((
        rpm_manifest::FORMAT,
        rpm_manifest::EXTENSIONS,
        Decoder::from_static(rpm_decode),
    ));

    #[cfg(feature = "builtin-xbps")]
    decoders.push((
        xbps_manifest::FORMAT,
        xbps_manifest::EXTENSIONS,
        Decoder::from_static(xbps_decode),
    ));

    decoders
}
