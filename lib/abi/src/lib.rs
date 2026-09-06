// SPDX-FileCopyrightText: 2026 JustPav
// SPDX-FileCopyrightText: 2026 SmoothTeam
//
// SPDX-License-Identifier: LGPL-3.0-or-later WITH LGPL-3.0-linking-exception

use self::error::ErrorKind;

pub mod boot;
pub mod decoder;
pub mod error;
pub mod hook;
pub mod memory;
pub mod package;
pub mod request;
pub mod response;
pub mod setup;
pub mod types;

pub const LIB_ABI_VERSION: u32 = 2;
pub const BOOT_ABI_VERSION: u32 = 2;
pub const DECODER_ABI_VERSION: u32 = 2;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDiffKind {
    Added = 0,
    Removed = 1,
    Modified = 2,
}

impl FileDiffKind {
    pub fn from_u8(version: u8) -> Result<FileDiffKind, ErrorKind> {
        match version {
            0 => Ok(FileDiffKind::Added),
            1 => Ok(FileDiffKind::Removed),
            2 => Ok(FileDiffKind::Modified),
            _ => Err(ErrorKind::InvalidEntry),
        }
    }
}

// A package's own metadata can be Added/Removed/Modified — or unchanged while
// one of its own files changed underneath it (e.g. a hand-edited is_user file),
// which FileDiffKind's three variants can't represent. Kept separate rather
// than adding a fourth variant to FileDiffKind, since every file-level
// consumer (DiffPrefixFileEntry/DiffConfigFileEntry/DiffUntrackedFileEntry) is
// already a complete, correct 3-way split — a package-only concept doesn't
// belong there.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageDiffKind {
    Added = 0,
    Removed = 1,
    Modified = 2,
    FilesChanged = 3,
}

impl PackageDiffKind {
    pub fn from_u8(version: u8) -> Result<PackageDiffKind, ErrorKind> {
        match version {
            0 => Ok(PackageDiffKind::Added),
            1 => Ok(PackageDiffKind::Removed),
            2 => Ok(PackageDiffKind::Modified),
            3 => Ok(PackageDiffKind::FilesChanged),
            _ => Err(ErrorKind::InvalidEntry),
        }
    }
}

// Distinguishes which tree a DiffPrefixFileEntry/DiffUntrackedFileEntry came
// from when both /usr and /etc changes are folded into one list (the combined
// diff command). Standalone diff_prefix/diff_config don't need it to
// disambiguate (the command itself already implies the axis), but reuse the
// same entry types and set it to a fixed value.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffFileSource {
    Prefix = 0,
    Config = 1,
}

impl DiffFileSource {
    pub fn from_u8(version: u8) -> Result<DiffFileSource, ErrorKind> {
        match version {
            0 => Ok(DiffFileSource::Prefix),
            1 => Ok(DiffFileSource::Config),
            _ => Err(ErrorKind::InvalidEntry),
        }
    }
}

// Filesystem chosen for the deployment partition (needs fs-verity support, see
// doc chapter 3 §(4)) or any extra mount upac-setup formats/mounts. Appending
// a new variant later (e.g. bcachefs) is a plain additive change here — every
// consumer already goes through from_u8, so nothing needs pre-reserving.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsKind {
    Ext4 = 0,
    Btrfs = 1,
    Xfs = 2,
}

impl FsKind {
    pub fn from_u8(version: u8) -> Result<FsKind, ErrorKind> {
        match version {
            0 => Ok(FsKind::Ext4),
            1 => Ok(FsKind::Btrfs),
            2 => Ok(FsKind::Xfs),
            _ => Err(ErrorKind::InvalidEntry),
        }
    }
}

impl AsRef<str> for FsKind {
    fn as_ref(&self) -> &str {
        match self {
            FsKind::Ext4 => "ext4",
            FsKind::Btrfs => "btrfs",
            FsKind::Xfs => "xfs",
        }
    }
}
