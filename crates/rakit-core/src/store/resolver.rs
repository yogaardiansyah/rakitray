use serde::Deserialize;

use crate::models::{ArchiveFormat, CatalogEntry, Platform, ServiceCategory};
use crate::store::StoreError;

#[derive(Debug, Deserialize)]
struct NodeDistEntry {
    version: String,
    #[serde(default)]
    lts: serde_json::Value,
}

fn node_platform_suffix(platform: Platform) -> Result<(&'static str, &'static str), StoreError> {
    match platform {
        Platform::LinuxX64 => Ok(("linux-x64", "tar.gz")),
        Platform::LinuxArm64 => Ok(("linux-arm64", "tar.gz")),
        Platform::MacosX64 => Ok(("darwin-x64", "tar.gz")),
        Platform::MacosArm64 => Ok(("darwin-arm64", "tar.gz")),
        Platform::WindowsX64 => Ok(("win-x64", "zip")),
    }
}

/// Resolves a runtime target (e.g. `node@22.11.0`, `node`, or `caddy@2.8.4`)
/// from the loaded catalog first, falling back to official dynamic resolvers.
pub async fn resolve_runtime(
    target: &str,
    platform: Platform,
    catalog: &[CatalogEntry],
) -> Result<CatalogEntry, StoreError> {
    let (engine, version_req) = match target.split_once('@') {
        Some((eng, ver)) => (eng.trim(), Some(ver.trim())),
        None => (target.trim(), None),
    };

    match engine {
        "node" | "nodejs" => return resolve_node(version_req, platform).await,
        "php" => return resolve_php(version_req, platform).await,
        "go" | "golang" => return resolve_go(version_req, platform).await,
        "java" | "jdk" => return resolve_java(version_req, platform).await,
        "python" => return resolve_python(version_req, platform).await,
        _ => {}
    }

    if let Some(entry) = catalog
        .iter()
        .find(|e| e.id == target && e.platform == platform)
    {
        return Ok(entry.clone());
    }

    Err(StoreError::Extraction(format!(
        "Unknown runtime '{}' for platform {:?}",
        target, platform
    )))
}

async fn resolve_node(
    version_req: Option<&str>,
    platform: Platform,
) -> Result<CatalogEntry, StoreError> {
    let client = reqwest::Client::builder()
        .user_agent("rakitray/0.2.0")
        .build()
        .map_err(|e| StoreError::Network(e.to_string()))?;

    let index_text = client
        .get("https://nodejs.org/dist/index.json")
        .send()
        .await
        .map_err(|e| StoreError::Network(e.to_string()))?
        .error_for_status()
        .map_err(|e| StoreError::Network(e.to_string()))?
        .text()
        .await
        .map_err(|e| StoreError::Network(e.to_string()))?;

    let index: Vec<NodeDistEntry> =
        serde_json::from_str(&index_text).map_err(|e| StoreError::Network(e.to_string()))?;

    let matched = match version_req {
        Some(req) => {
            let v_prefixed = if req.starts_with('v') {
                req.to_string()
            } else {
                format!("v{}", req)
            };
            index.into_iter().find(|e| {
                e.version == v_prefixed || e.version.starts_with(&format!("{}.", v_prefixed))
            })
        }
        None => index
            .into_iter()
            .find(|e| !e.lts.is_boolean() || e.lts.as_bool() == Some(true)),
    }
    .ok_or_else(|| StoreError::Extraction("Requested Node.js version not found".to_string()))?;

    let clean_version = matched.version.trim_start_matches('v').to_string();
    let (suffix, ext) = node_platform_suffix(platform)?;
    let filename = format!("node-v{}-{}.{}", clean_version, suffix, ext);
    let download_url = format!("https://nodejs.org/dist/v{}/{}", clean_version, filename);
    let shasums_url = format!("https://nodejs.org/dist/v{}/SHASUMS256.txt", clean_version);

    let shasums_text = client
        .get(&shasums_url)
        .send()
        .await
        .map_err(|e| StoreError::Network(e.to_string()))?
        .error_for_status()
        .map_err(|e| StoreError::Network(e.to_string()))?
        .text()
        .await
        .map_err(|e| StoreError::Network(e.to_string()))?;

    let sha256 = shasums_text
        .lines()
        .find_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let file = parts.next()?;
            if file == filename {
                Some(hash.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            StoreError::Extraction(format!(
                "SHA-256 not found for {} in SHASUMS256.txt",
                filename
            ))
        })?;

    Ok(CatalogEntry {
        id: format!("node@{}", clean_version),
        engine: "node".to_string(),
        category: ServiceCategory::Runtime,
        version: clean_version,
        platform,
        download_url,
        sha256,
        estimated_size_mb: 28,
        blueprint_id: "node".to_string(),
        archive_format: if ext == "zip" {
            ArchiveFormat::Zip
        } else {
            ArchiveFormat::TarGz
        },
        strip_components: 1,
        strip_profile: "default".to_string(),
    })
}

#[derive(Debug, Deserialize)]
struct StaticPhpEntry {
    name: String,
    is_dir: bool,
}

fn php_platform_suffix(platform: Platform) -> Result<&'static str, StoreError> {
    match platform {
        Platform::LinuxX64 => Ok("cli-linux-x86_64.tar.gz"),
        Platform::LinuxArm64 => Ok("cli-linux-aarch64.tar.gz"),
        Platform::MacosX64 => Ok("cli-macos-x86_64.tar.gz"),
        Platform::MacosArm64 => Ok("cli-macos-aarch64.tar.gz"),
        Platform::WindowsX64 => Err(StoreError::Extraction(
            "Use official windows.php.net builds on Windows".to_string(),
        )),
    }
}

async fn resolve_php(
    version_req: Option<&str>,
    platform: Platform,
) -> Result<CatalogEntry, StoreError> {
    let suffix = php_platform_suffix(platform)?;
    let client = reqwest::Client::builder()
        .user_agent("rakitray/0.2.0")
        .build()
        .map_err(|e| StoreError::Network(e.to_string()))?;

    let text = client
        .get("https://dl.static-php.dev/static-php-cli/common/?format=json")
        .send()
        .await
        .map_err(|e| StoreError::Network(e.to_string()))?
        .error_for_status()
        .map_err(|e| StoreError::Network(e.to_string()))?
        .text()
        .await
        .map_err(|e| StoreError::Network(e.to_string()))?;

    let entries: Vec<StaticPhpEntry> =
        serde_json::from_str(&text).map_err(|e| StoreError::Network(e.to_string()))?;

    let mut versions: Vec<(String, String)> = entries
        .into_iter()
        .filter(|e| !e.is_dir && e.name.starts_with("php-") && e.name.ends_with(suffix))
        .filter_map(|e| {
            let rest = e.name.strip_prefix("php-")?;
            let ver = rest.strip_suffix(&format!("-{}", suffix))?;
            Some((ver.to_string(), e.name))
        })
        .collect();

    versions.sort_by(|a, b| a.0.cmp(&b.0));

    let (version, filename) = match version_req {
        Some(req) => versions
            .into_iter()
            .rfind(|(v, _)| v == req || v.starts_with(&format!("{}.", req))),
        None => versions.into_iter().last(),
    }
    .ok_or_else(|| StoreError::Extraction("Requested PHP version not found".to_string()))?;

    Ok(CatalogEntry {
        id: format!("php@{}", version),
        engine: "php".to_string(),
        category: ServiceCategory::Runtime,
        version,
        platform,
        download_url: format!(
            "https://dl.static-php.dev/static-php-cli/common/{}",
            filename
        ),
        sha256: String::new(),
        estimated_size_mb: 20,
        blueprint_id: "php".to_string(),
        archive_format: ArchiveFormat::TarGz,
        strip_components: 0,
        strip_profile: "default".to_string(),
    })
}

async fn http_get_text(url: &str) -> Result<String, StoreError> {
    reqwest::Client::builder()
        .user_agent("rakitray/0.2.0")
        .build()
        .map_err(|e| StoreError::Network(e.to_string()))?
        .get(url)
        .send()
        .await
        .map_err(|e| StoreError::Network(e.to_string()))?
        .error_for_status()
        .map_err(|e| StoreError::Network(e.to_string()))?
        .text()
        .await
        .map_err(|e| StoreError::Network(e.to_string()))
}

#[derive(Debug, Deserialize)]
struct GoRelease {
    version: String,
    stable: bool,
    files: Vec<GoFile>,
}

#[derive(Debug, Deserialize)]
struct GoFile {
    filename: String,
    os: String,
    arch: String,
    sha256: String,
    kind: String,
}

async fn resolve_go(
    version_req: Option<&str>,
    platform: Platform,
) -> Result<CatalogEntry, StoreError> {
    let (go_os, go_arch, ext) = match platform {
        Platform::LinuxX64 => ("linux", "amd64", "tar.gz"),
        Platform::LinuxArm64 => ("linux", "arm64", "tar.gz"),
        Platform::MacosX64 => ("darwin", "amd64", "tar.gz"),
        Platform::MacosArm64 => ("darwin", "arm64", "tar.gz"),
        Platform::WindowsX64 => ("windows", "amd64", "zip"),
    };

    let body = http_get_text("https://go.dev/dl/?mode=json&include=all").await?;
    let releases: Vec<GoRelease> =
        serde_json::from_str(&body).map_err(|e| StoreError::Network(e.to_string()))?;

    let matched = match version_req {
        Some(req) => {
            let clean = req.trim_start_matches("go");
            releases.into_iter().find(|r| {
                let v = r.version.trim_start_matches("go");
                v == clean || v.starts_with(&format!("{}.", clean))
            })
        }
        None => releases.into_iter().find(|r| r.stable),
    }
    .ok_or_else(|| StoreError::Extraction("Requested Go version not found".to_string()))?;

    let file = matched
        .files
        .into_iter()
        .find(|f| f.kind == "archive" && f.os == go_os && f.arch == go_arch)
        .ok_or_else(|| StoreError::Extraction("No Go archive for host platform".to_string()))?;

    let version = matched.version.trim_start_matches("go").to_string();
    Ok(CatalogEntry {
        id: format!("go@{}", version),
        engine: "go".to_string(),
        category: ServiceCategory::Runtime,
        version,
        platform,
        download_url: format!("https://go.dev/dl/{}", file.filename),
        sha256: file.sha256,
        estimated_size_mb: 70,
        blueprint_id: "go".to_string(),
        archive_format: if ext == "zip" {
            ArchiveFormat::Zip
        } else {
            ArchiveFormat::TarGz
        },
        strip_components: 1,
        strip_profile: "default".to_string(),
    })
}

#[derive(Debug, Deserialize)]
struct AdoptiumRelease {
    binaries: Vec<AdoptiumBinary>,
    version_data: AdoptiumVersionData,
}

#[derive(Debug, Deserialize)]
struct AdoptiumBinary {
    package: AdoptiumPackage,
}

#[derive(Debug, Deserialize)]
struct AdoptiumPackage {
    link: String,
    checksum: String,
}

#[derive(Debug, Deserialize)]
struct AdoptiumVersionData {
    semver: String,
}

async fn resolve_java(
    version_req: Option<&str>,
    platform: Platform,
) -> Result<CatalogEntry, StoreError> {
    let major = version_req
        .and_then(|v| v.split('.').next())
        .filter(|s| !s.is_empty())
        .unwrap_or("21");
    let (os, arch, ext) = match platform {
        Platform::LinuxX64 => ("linux", "x64", ArchiveFormat::TarGz),
        Platform::LinuxArm64 => ("linux", "aarch64", ArchiveFormat::TarGz),
        Platform::MacosX64 => ("mac", "x64", ArchiveFormat::TarGz),
        Platform::MacosArm64 => ("mac", "aarch64", ArchiveFormat::TarGz),
        Platform::WindowsX64 => ("windows", "x64", ArchiveFormat::Zip),
    };

    let url = format!(
        "https://api.adoptium.net/v3/assets/feature_releases/{}/ga?architecture={}&os={}&image_type=jdk&page_size=1",
        major, arch, os
    );
    let body = http_get_text(&url).await?;
    let releases: Vec<AdoptiumRelease> =
        serde_json::from_str(&body).map_err(|e| StoreError::Network(e.to_string()))?;
    let release = releases
        .into_iter()
        .next()
        .ok_or_else(|| StoreError::Extraction(format!("Java {} JDK not found", major)))?;
    let pkg = release
        .binaries
        .into_iter()
        .next()
        .ok_or_else(|| StoreError::Extraction("No JDK binary in Adoptium release".to_string()))?
        .package;

    let version = release
        .version_data
        .semver
        .split('+')
        .next()
        .unwrap_or(major)
        .to_string();

    Ok(CatalogEntry {
        id: format!("java@{}", version),
        engine: "java".to_string(),
        category: ServiceCategory::Runtime,
        version,
        platform,
        download_url: pkg.link,
        sha256: pkg.checksum,
        estimated_size_mb: 195,
        blueprint_id: "java".to_string(),
        archive_format: ext,
        strip_components: 1,
        strip_profile: "default".to_string(),
    })
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

async fn resolve_python(
    version_req: Option<&str>,
    platform: Platform,
) -> Result<CatalogEntry, StoreError> {
    let target_triple = match platform {
        Platform::LinuxX64 => "x86_64-unknown-linux-gnu-install_only.tar.gz",
        Platform::LinuxArm64 => "aarch64-unknown-linux-gnu-install_only.tar.gz",
        Platform::MacosX64 => "x86_64-apple-darwin-install_only.tar.gz",
        Platform::MacosArm64 => "aarch64-apple-darwin-install_only.tar.gz",
        Platform::WindowsX64 => "x86_64-pc-windows-msvc-install_only.tar.gz",
    };

    let body = http_get_text(
        "https://api.github.com/repos/astral-sh/python-build-standalone/releases/latest",
    )
    .await?;
    let release: GithubRelease =
        serde_json::from_str(&body).map_err(|e| StoreError::Network(e.to_string()))?;

    let prefix = match version_req {
        Some(v) => format!("cpython-{}", v),
        None => "cpython-3.12.".to_string(),
    };

    let asset = release
        .assets
        .into_iter()
        .find(|a| a.name.starts_with(&prefix) && a.name.ends_with(target_triple))
        .ok_or_else(|| StoreError::Extraction("Requested Python version not found".to_string()))?;

    let version = asset
        .name
        .strip_prefix("cpython-")
        .and_then(|s| s.split('+').next())
        .unwrap_or("3.12")
        .to_string();

    Ok(CatalogEntry {
        id: format!("python@{}", version),
        engine: "python".to_string(),
        category: ServiceCategory::Runtime,
        version,
        platform,
        download_url: asset.browser_download_url,
        sha256: String::new(),
        estimated_size_mb: 30,
        blueprint_id: "python".to_string(),
        archive_format: ArchiveFormat::TarGz,
        strip_components: 1,
        strip_profile: "default".to_string(),
    })
}
