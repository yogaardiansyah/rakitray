use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::Path;
use tar::Archive as TarArchive;
use zip::ZipArchive;

use crate::models::{ArchiveFormat, InstallProgress, InstallState};
use crate::store::sandbox::{validate_safe_relative_path, validate_symlink_target};
use crate::store::strip::{should_strip_path, strip_components};
use crate::store::StoreError;

pub struct ExtractionSummary {
    pub extracted_files: u64,
    pub stripped_bytes: u64,
}

pub struct ExtractOptions<'a> {
    pub archive_path: &'a Path,
    pub format: ArchiveFormat,
    pub staging_dir: &'a Path,
    pub dest_dir: &'a Path,
    pub strip_comp: u8,
    pub strip_profile: &'a str,
    pub install_id: &'a str,
    pub catalog_id: &'a str,
}

pub struct LeanExtractor;

impl LeanExtractor {
    /// Extracts an archive into `staging_dir`, filters bloatware in memory,
    /// verifies Anti-Zip-Slip rules, and atomically renames `staging_dir` to `dest_dir` on success.
    pub fn extract<F>(
        options: ExtractOptions,
        mut on_progress: F,
    ) -> Result<ExtractionSummary, StoreError>
    where
        F: FnMut(InstallProgress) + Send + 'static,
    {
        let staging_dir = options.staging_dir;
        let dest_dir = options.dest_dir;

        // 1. Prepare staging directory
        if staging_dir.exists() {
            let _ = fs::remove_dir_all(staging_dir);
        }
        fs::create_dir_all(staging_dir).map_err(StoreError::Io)?;

        on_progress(InstallProgress {
            install_id: options.install_id.to_string(),
            catalog_id: options.catalog_id.to_string(),
            state: InstallState::ExtractingLean,
            bytes_downloaded: 0,
            total_bytes: 0,
            speed_bps: 0,
            eta_secs: None,
            stripped_bytes: 0,
            message: Some("Extracting lean package (filtering bloatware)...".to_string()),
            error: None,
        });

        let extract_result = match options.format {
            ArchiveFormat::TarGz => Self::extract_tar_gz(
                options.archive_path,
                staging_dir,
                options.strip_comp,
                options.strip_profile,
            ),
            ArchiveFormat::TarXz => Self::extract_tar_xz(
                options.archive_path,
                staging_dir,
                options.strip_comp,
                options.strip_profile,
            ),
            ArchiveFormat::Zip => Self::extract_zip(
                options.archive_path,
                staging_dir,
                options.strip_comp,
                options.strip_profile,
            ),
        };

        match extract_result {
            Ok(summary) => {
                // Ensure parent of dest_dir exists
                if let Some(parent) = dest_dir.parent() {
                    fs::create_dir_all(parent).map_err(StoreError::Io)?;
                }

                // If dest_dir already exists, remove it cleanly before atomic rename
                if dest_dir.exists() {
                    fs::remove_dir_all(dest_dir).map_err(StoreError::Io)?;
                }

                // Atomic rename staging_dir -> dest_dir
                fs::rename(staging_dir, dest_dir).map_err(|e| {
                    let _ = fs::remove_dir_all(staging_dir);
                    StoreError::Io(e)
                })?;

                let mb_saved = summary.stripped_bytes / (1024 * 1024);
                let message = if mb_saved > 0 {
                    format!(
                        "Stripped {} MB of bloatware (mysql-test, .pdb, docs)",
                        mb_saved
                    )
                } else {
                    "Lean extraction complete".to_string()
                };

                on_progress(InstallProgress {
                    install_id: options.install_id.to_string(),
                    catalog_id: options.catalog_id.to_string(),
                    state: InstallState::Registering,
                    bytes_downloaded: 0,
                    total_bytes: 0,
                    speed_bps: 0,
                    eta_secs: Some(0),
                    stripped_bytes: summary.stripped_bytes,
                    message: Some(message),
                    error: None,
                });

                Ok(summary)
            }
            Err(err) => {
                // Cleanup staging dir immediately on error
                let _ = fs::remove_dir_all(staging_dir);
                Err(err)
            }
        }
    }

    fn extract_tar_gz(
        archive_path: &Path,
        staging_dir: &Path,
        strip_comp: u8,
        strip_profile: &str,
    ) -> Result<ExtractionSummary, StoreError> {
        let file = File::open(archive_path).map_err(StoreError::Io)?;
        let gz = GzDecoder::new(BufReader::new(file));
        let mut archive = TarArchive::new(gz);
        Self::process_tar(&mut archive, staging_dir, strip_comp, strip_profile)
    }

    fn extract_tar_xz(
        archive_path: &Path,
        staging_dir: &Path,
        strip_comp: u8,
        strip_profile: &str,
    ) -> Result<ExtractionSummary, StoreError> {
        let file = File::open(archive_path).map_err(StoreError::Io)?;
        let mut decompressed = Vec::new();
        let mut reader = BufReader::new(file);
        lzma_rs::xz_decompress(&mut reader, &mut decompressed)
            .map_err(|e| StoreError::Extraction(format!("XZ decompress error: {}", e)))?;

        let mut archive = TarArchive::new(&decompressed[..]);
        Self::process_tar(&mut archive, staging_dir, strip_comp, strip_profile)
    }

    fn process_tar<R: Read>(
        archive: &mut TarArchive<R>,
        staging_dir: &Path,
        strip_comp: u8,
        strip_profile: &str,
    ) -> Result<ExtractionSummary, StoreError> {
        let mut extracted_files = 0u64;
        let mut stripped_bytes = 0u64;

        for entry_res in archive.entries().map_err(StoreError::Io)? {
            let mut entry = entry_res.map_err(StoreError::Io)?;
            let raw_path = entry.path().map_err(StoreError::Io)?.to_path_buf();
            let entry_size = entry.size();

            // Strip components
            let stripped_path = match strip_components(&raw_path, strip_comp) {
                Some(p) => p,
                None => continue, // was part of the stripped leading directories
            };

            // Anti-Zip-Slip validation
            let dest_path = validate_safe_relative_path(staging_dir, &stripped_path)?;

            // In-memory bloatware check
            if should_strip_path(&stripped_path, strip_profile) {
                stripped_bytes += entry_size;
                continue;
            }

            let entry_type = entry.header().entry_type();
            if entry_type.is_dir() {
                fs::create_dir_all(&dest_path).map_err(StoreError::Io)?;
            } else if entry_type.is_symlink() || entry_type.is_hard_link() {
                if let Some(target) = entry.link_name().map_err(StoreError::Io)? {
                    validate_symlink_target(staging_dir, &dest_path, &target)?;
                    #[cfg(unix)]
                    {
                        if let Some(parent) = dest_path.parent() {
                            fs::create_dir_all(parent).map_err(StoreError::Io)?;
                        }
                        let _ = fs::remove_file(&dest_path);
                        std::os::unix::fs::symlink(target, &dest_path).map_err(StoreError::Io)?;
                    }
                }
            } else if entry_type.is_file() {
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent).map_err(StoreError::Io)?;
                }

                let mut out_file = File::create(&dest_path).map_err(StoreError::Io)?;
                io::copy(&mut entry, &mut out_file).map_err(StoreError::Io)?;

                // Mask permissions: preserve user executable bit on Unix, drop setuid/setgid
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = entry.header().mode().unwrap_or(0o644) & 0o777;
                    fs::set_permissions(&dest_path, fs::Permissions::from_mode(mode))
                        .map_err(StoreError::Io)?;
                }

                extracted_files += 1;
            }
        }

        Ok(ExtractionSummary {
            extracted_files,
            stripped_bytes,
        })
    }

    fn extract_zip(
        archive_path: &Path,
        staging_dir: &Path,
        strip_comp: u8,
        strip_profile: &str,
    ) -> Result<ExtractionSummary, StoreError> {
        let file = File::open(archive_path).map_err(StoreError::Io)?;
        let mut zip = ZipArchive::new(file)
            .map_err(|e| StoreError::Extraction(format!("Invalid zip archive: {}", e)))?;

        let mut extracted_files = 0u64;
        let mut stripped_bytes = 0u64;

        for i in 0..zip.len() {
            let mut file = zip
                .by_index(i)
                .map_err(|e| StoreError::Extraction(format!("Zip read error: {}", e)))?;

            let raw_path = match file.enclosed_name() {
                Some(p) => p.to_path_buf(),
                None => {
                    return Err(StoreError::PathTraversal(format!(
                        "Zip enclosed_name traversal check failed on: {}",
                        file.name()
                    )))
                }
            };
            let entry_size = file.size();

            let stripped_path = match strip_components(&raw_path, strip_comp) {
                Some(p) => p,
                None => continue,
            };

            let dest_path = validate_safe_relative_path(staging_dir, &stripped_path)?;

            if should_strip_path(&stripped_path, strip_profile) {
                stripped_bytes += entry_size;
                continue;
            }

            if file.is_dir() {
                fs::create_dir_all(&dest_path).map_err(StoreError::Io)?;
            } else {
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent).map_err(StoreError::Io)?;
                }

                let mut out_file = File::create(&dest_path).map_err(StoreError::Io)?;
                io::copy(&mut file, &mut out_file).map_err(StoreError::Io)?;

                #[cfg(unix)]
                {
                    if let Some(mode) = file.unix_mode() {
                        use std::os::unix::fs::PermissionsExt;
                        let safe_mode = mode & 0o777;
                        fs::set_permissions(&dest_path, fs::Permissions::from_mode(safe_mode))
                            .map_err(StoreError::Io)?;
                    }
                }

                extracted_files += 1;
            }
        }

        Ok(ExtractionSummary {
            extracted_files,
            stripped_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_tar_gz_extract_and_strip() {
        let temp_dir = std::env::temp_dir().join("rakit_test_extract_targz");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);

        let tar_gz_path = temp_dir.join("test_pkg.tar.gz");
        let staging_dir = temp_dir.join("staging");
        let dest_dir = temp_dir.join("dest");

        // Build a mock tar.gz containing useful files and bloatware
        {
            let file = File::create(&tar_gz_path).unwrap();
            let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            let mut tar = tar::Builder::new(gz);

            let useful_data = b"binary-content-here";
            let mut h1 = tar::Header::new_gnu();
            h1.set_path("root-dir/bin/mariadbd").unwrap();
            h1.set_size(useful_data.len() as u64);
            h1.set_mode(0o755);
            h1.set_cksum();
            tar.append(&h1, &useful_data[..]).unwrap();

            let pdb_data = b"debug-pdb-symbols";
            let mut h2 = tar::Header::new_gnu();
            h2.set_path("root-dir/bin/mariadbd.pdb").unwrap();
            h2.set_size(pdb_data.len() as u64);
            h2.set_mode(0o644);
            h2.set_cksum();
            tar.append(&h2, &pdb_data[..]).unwrap();

            let test_data = b"mysql-test-suite-files";
            let mut h3 = tar::Header::new_gnu();
            h3.set_path("root-dir/mysql-test/suite.pl").unwrap();
            h3.set_size(test_data.len() as u64);
            h3.set_mode(0o644);
            h3.set_cksum();
            tar.append(&h3, &test_data[..]).unwrap();

            tar.finish().unwrap();
        }

        let summary = LeanExtractor::extract(
            ExtractOptions {
                archive_path: &tar_gz_path,
                format: ArchiveFormat::TarGz,
                staging_dir: &staging_dir,
                dest_dir: &dest_dir,
                strip_comp: 1, // strip root-dir
                strip_profile: "default",
                install_id: "inst1",
                catalog_id: "mariadb@11.4",
            },
            |_| {},
        )
        .unwrap();

        assert_eq!(summary.extracted_files, 1);
        assert!(summary.stripped_bytes > 0);
        // mariadbd exists in destination
        assert!(dest_dir.join("bin/mariadbd").exists());
        // pdb and mysql-test were stripped
        assert!(!dest_dir.join("bin/mariadbd.pdb").exists());
        assert!(!dest_dir.join("mysql-test").exists());
        // staging directory was cleanly removed/renamed
        assert!(!staging_dir.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_zip_path_traversal_aborts_and_cleans_staging() {
        let temp_dir = std::env::temp_dir().join("rakit_test_extract_traversal");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);

        let zip_path = temp_dir.join("evil.zip");
        let staging_dir = temp_dir.join("staging");
        let dest_dir = temp_dir.join("dest");

        {
            let file = File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            // write traversal entry
            zip.start_file("../evil.txt", options).unwrap();
            zip.write_all(b"evil data").unwrap();
            zip.finish().unwrap();
        }

        let res = LeanExtractor::extract(
            ExtractOptions {
                archive_path: &zip_path,
                format: ArchiveFormat::Zip,
                staging_dir: &staging_dir,
                dest_dir: &dest_dir,
                strip_comp: 0,
                strip_profile: "default",
                install_id: "inst_evil",
                catalog_id: "evil@1.0",
            },
            |_| {},
        );

        assert!(matches!(res, Err(StoreError::PathTraversal(_))));
        assert!(!staging_dir.exists());
        assert!(!dest_dir.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
