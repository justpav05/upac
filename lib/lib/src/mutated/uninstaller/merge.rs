// SPDX-FileCopyrightText: 2026 JustPav
// SPDX-FileCopyrightText: 2026 SmoothTeam
//
// SPDX-License-Identifier: LGPL-3.0-or-later WITH LGPL-3.0-linking-exception

use std::fs::create_dir_all;

use composefs::fsverity::FsVerityHashValue;
use composefs::repository::ImportContext;

use upac_abi::hook::{CancelToken, ProgressEventBuilder};

use crate::composefs::file::FileHandle;
use crate::composefs::overlay::apply_overlay_upper;
use crate::composefs::repository::commit_tree;
use crate::config::merge_config;
use crate::database::error::DeployRecordError;
use crate::database::record::DeployRecord;
use crate::deploy::Deploy;
use crate::deploy::digest::current_prefix_digest;
use crate::layout::deployment::CONFIG_DIR_NAME;
use crate::mutated::uninstaller::{CommitMessage, NewPrefixDigest, RemovedConfigPaths, Subject, UninstallError};
use crate::orchestrator::stage::{RollbackGuard, Stage, StageResult};
use crate::orchestrator::{Context, ctx_get, ctx_take};

pub struct MergeStage;

impl Stage<UninstallError> for MergeStage {
    fn run(
        &self, context: &mut Context, _cancel: &CancelToken, progress: ProgressEventBuilder,
    ) -> Result<(ProgressEventBuilder, StageResult, Box<dyn RollbackGuard>), UninstallError> {
        let removed_config_paths = ctx_take!(context, RemovedConfigPaths);

        let new_prefix = ctx_get!(context, NewPrefixDigest);
        let deploy = ctx_get!(context, Deploy);
        let subject = ctx_get!(context, Subject);
        let message = ctx_get!(context, CommitMessage);

        let repository = deploy.open_repository()?;

        let current_prefix = current_prefix_digest()?;
        let current_record_dir = deploy.deploy(&current_prefix);
        let current_record = DeployRecord::read(&current_record_dir)?;

        let base_config_layout = deploy.open_tree(&current_record.working_config)?;

        let mut live_config_layout = base_config_layout.clone();
        let config_upper_dir = current_record_dir.join(CONFIG_DIR_NAME).join("upper");
        let mut import_ctx = ImportContext::default();

        apply_overlay_upper(&repository, &mut live_config_layout, &config_upper_dir, &mut import_ctx)?;

        let mut new = base_config_layout.clone();

        for path in &removed_config_paths.0 {
            FileHandle::new(path).remove_in_tree(&mut new)?;
        }

        let merge_result = merge_config(&base_config_layout, &new, &live_config_layout, true)?;
        let new_config_digest = commit_tree(&repository, merge_result.tree)?.to_hex();

        let new_record_dir = deploy.deploy(&new_prefix.0);
        let mut record = match DeployRecord::read(&new_record_dir) {
            Ok(existing) => existing,
            Err(DeployRecordError::NotFound) => {
                create_dir_all(&new_record_dir).map_err(DeployRecordError::from)?;

                DeployRecord {
                    prefix_digest: new_prefix.0.clone(),
                    subject: subject.0.clone(),
                    message: message.0.clone(),
                    seq: DeployRecord::allocate_seq(&deploy.next_seq_path())?,
                    timestamp: DeployRecord::now_secs(),
                    config_history: Vec::new(),
                    working_config: String::new(),
                    pinned: false,
                }
            }
            Err(error) => return Err(error.into()),
        };

        let mut written = Vec::new();
        written.extend(record.update_working_config(
            &new_record_dir,
            new_config_digest,
            subject.0.clone(),
            message.0.clone(),
        )?);

        Ok((progress, StageResult::Advance, Box::new(written)))
    }
}
