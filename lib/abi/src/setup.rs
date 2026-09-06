// SPDX-FileCopyrightText: 2026 JustPav
// SPDX-FileCopyrightText: 2026 SmoothTeam
//
// SPDX-License-Identifier: LGPL-3.0-or-later WITH LGPL-3.0-linking-exception

use upac_macro::{CNew, CValidate};

use crate::FsKind;
use crate::error::ErrorKind;
use crate::request::CRequestBase;
use crate::types::{CSlice, CVec, check_size};

#[repr(C)]
#[derive(CNew, CValidate)]
pub struct CSetupBase {
    pub struct_size: usize,

    pub base: CRequestBase,

    #[optional]
    pub mount_point: CSlice,
    #[non_empty]
    pub source: CSlice,
    pub empty_config: bool,
    pub pinned: bool,
    #[optional]
    pub boot_plugin: CSlice,
}

#[repr(C)]
#[derive(CValidate)]
pub struct CPartitionMount {
    pub struct_size: usize,

    #[non_empty]
    pub mount_path: CSlice,
    #[non_empty]
    pub device_path: CSlice,
    pub fs_kind: FsKind,
}

#[repr(C)]
#[derive(CValidate)]
pub struct CPartitionSpec {
    pub struct_size: usize,

    #[non_empty]
    pub mount_path: CSlice,
    pub size_mib: u64,
    pub fs_kind: FsKind,
}

#[repr(C)]
#[derive(CNew, CValidate)]
pub struct CSetupExistingRequest {
    pub struct_size: usize,

    pub base: CSetupBase,

    #[non_empty]
    pub esp_device: CSlice,
    #[non_empty]
    pub deploy_device: CSlice,
    pub deploy_fs: FsKind,
    pub extra_mounts: CVec<CPartitionMount>,
}

#[repr(C)]
#[derive(CValidate)]
pub struct CGptLayout {
    pub struct_size: usize,

    pub esp_size_mib: u64,
    pub deploy_fs: FsKind,
    pub deploy_size_mib: u64,
    pub extra_partitions: CVec<CPartitionSpec>,
    pub force_wipe: bool,
}

#[repr(C)]
#[derive(CValidate)]
pub struct CBtrfsOptions {
    pub struct_size: usize,

    pub node_size: u32,
    pub sector_size: u32,
}

#[repr(C)]
#[derive(CNew, CValidate)]
pub struct CSetupWholeDiskRequest {
    pub struct_size: usize,

    pub base: CSetupBase,

    #[non_empty]
    pub device_path: CSlice,
    pub gpt: CGptLayout,
    pub btrfs: CBtrfsOptions,
}
