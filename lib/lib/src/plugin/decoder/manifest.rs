// SPDX-FileCopyrightText: 2026 JustPav
// SPDX-FileCopyrightText: 2026 SmoothTeam
//
// SPDX-License-Identifier: LGPL-3.0-or-later WITH LGPL-3.0-linking-exception

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::str::FromStr;

use mime::Mime;

use serde::Deserialize;

use crate::plugin::decoder::error::DecoderError;

#[derive(Debug, Clone, Deserialize)]
pub struct DecoderManifest {
    pub format: String,
    pub extensions: Vec<String>,
    pub library: String,
    pub mime: String,
}

pub fn load_decoder_manifests(
    decoders_dir: &str, manifest_extension: &str,
) -> Result<HashMap<String, DecoderManifest>, DecoderError> {
    let mut manifests = HashMap::new();

    let dir = match fs::read_dir(decoders_dir) {
        Ok(dir) => dir,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(manifests),
        Err(error) => return Err(error.into()),
    };

    for entry in dir {
        let path = entry?.path();

        if path.extension().and_then(|extension| extension.to_str()) != Some(manifest_extension) {
            continue;
        }

        let raw = fs::read_to_string(&path)?;
        let manifest: DecoderManifest = toml::from_str(&raw)?;

        Mime::from_str(&manifest.mime)?;

        if manifests.contains_key(&manifest.format) {
            return Err(DecoderError::DuplicateFormat(manifest.format));
        }

        manifests.insert(manifest.format.clone(), manifest);
    }

    Ok(manifests)
}
