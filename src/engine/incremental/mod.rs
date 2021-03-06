mod fs_hash;

use crate::domain::Target;
use crate::engine::incremental::fs_hash::file_hashes_eq;
use anyhow::{Context, Error, Result};
use fs_hash::compute_file_hashes_in_paths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(PartialEq)]
pub enum IncrementalRunResult<T> {
    Skipped,
    Run(T),
}

pub struct IncrementalRunner<'a> {
    checksum_dir: &'a Path,
}

impl<'a> IncrementalRunner<'a> {
    pub fn new(checksum_dir: &'a Path) -> Self {
        Self { checksum_dir }
    }

    pub fn run<T, F>(&self, target: &Target, function: F) -> Result<IncrementalRunResult<Result<T>>>
    where
        F: Fn() -> Result<T>,
    {
        if self.files_have_not_changed_since_last_successful_execution(target)? {
            return Ok(IncrementalRunResult::Skipped);
        }

        self.remove_target_checksums(&target)?;

        let result = function();

        if result.is_ok() {
            if let Some(target_checksums) = compute_target_checksums(target)? {
                self.write_target_checksums(&target, &target_checksums)?;
            }
        }

        Ok(IncrementalRunResult::Run(result))
    }

    fn get_checksum_file_path(&self, target: &Target) -> PathBuf {
        self.checksum_dir.join(format!("{}.checksum", target.name))
    }

    fn files_have_not_changed_since_last_successful_execution(
        &self,
        target: &Target,
    ) -> Result<bool> {
        let saved_checksums = self
            .read_target_checksums(target)
            .with_context(|| format!("Failed to read saved checksums for {}", target.name))?;

        match saved_checksums {
            Some(saved_checksums) => saved_checksums.eq_fs_checksum(target).with_context(|| {
                format!(
                    "Failed to compare saved checksums with filesystem checksums for {}",
                    target.name
                )
            }),
            _ => Ok(false),
        }
    }

    fn read_target_checksums(&self, target: &Target) -> Result<Option<TargetChecksums>> {
        // Might want to check for some errors like permission denied.
        fs::create_dir(&self.checksum_dir).ok();

        let file_path = self.get_checksum_file_path(target);
        if file_path.exists() {
            let file = File::open(&file_path)
                .with_context(|| format!("Failed to open checksum file {}", file_path.display()))?;
            match bincode::deserialize_from(file) {
                Ok(checksums) => Ok(Some(checksums)),
                Err(e) => {
                    log::trace!(
                        "{} - Dropping corrupted checksum file (Error: {})",
                        &target.name,
                        e
                    );
                    self.remove_target_checksums(&target)?;
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    fn remove_target_checksums(&self, target: &Target) -> Result<()> {
        let checksum_file = &self.get_checksum_file_path(target);
        if checksum_file.exists() {
            fs::remove_file(&checksum_file).with_context(|| {
                format!("Failed to delete checksum file {}", checksum_file.display())
            })?;
        }
        Ok(())
    }

    fn write_target_checksums(&self, target: &Target, checksums: &TargetChecksums) -> Result<()> {
        let file_path = self.get_checksum_file_path(target);
        let file = File::create(&file_path)
            .with_context(|| format!("Failed to create checksum file {}", file_path.display()))?;
        bincode::serialize_into(file, checksums)
            .with_context(|| format!("Failed to serialize checksums for {}", target.name))
    }

    pub fn clean_checksums(&self, targets: &[Target]) -> Result<()> {
        if targets.is_empty() {
            self.remove_checksum_dir()
        } else {
            for target in targets.iter() {
                self.remove_target_checksums(target)?;
            }
            Ok(())
        }
    }

    pub fn remove_checksum_dir(&self) -> Result<()> {
        match fs::remove_dir_all(self.checksum_dir) {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => {
                return Err(Error::new(e).context(format!(
                    "Failed to remove checksum directory {}",
                    self.checksum_dir.display()
                )));
            }
        }

        Ok(())
    }
}

fn compute_target_checksums(target: &Target) -> Result<Option<TargetChecksums>> {
    if target.input_paths.is_empty() {
        Ok(None)
    } else {
        Ok(Some(TargetChecksums {
            inputs: compute_file_hashes_in_paths(&target.input_paths)?,
            outputs: compute_file_hashes_in_paths(&target.output_paths)?,
        }))
    }
}

#[derive(Serialize, Deserialize, PartialEq)]
struct TargetChecksums {
    inputs: HashMap<PathBuf, u64>,
    outputs: HashMap<PathBuf, u64>,
}

impl TargetChecksums {
    fn eq_fs_checksum(&self, target: &Target) -> Result<bool> {
        Ok(file_hashes_eq(&target.input_paths, &self.inputs)?
            && file_hashes_eq(&target.output_paths, &self.outputs)?)
    }
}
