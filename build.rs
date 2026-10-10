// Windows release hardening: embed icon + manifest + version stamp.
// winres already seeds FileVersion/ProductVersion/ProductName from Cargo,
// but we set them explicitly from CARGO_PKG_VERSION per spec.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=resources/icon.ico");
    println!("cargo:rerun-if-changed=resources/app.manifest");
    println!("cargo:rerun-if-changed=Cargo.toml");
    #[cfg(windows)]
    {
        build_windows_resource()?;
    }
    Ok(())
}

#[cfg(windows)]
fn build_windows_resource() -> Result<(), Box<dyn std::error::Error>> {
    let version = env!("CARGO_PKG_VERSION");
    let numeric = version_u64(version);
    let mut res = winres::WindowsResource::new();
    res.set_icon("resources/icon.ico")
        .set_manifest_file("resources/app.manifest")
        .set_version_info(winres::VersionInfo::FILEVERSION, numeric)
        .set_version_info(winres::VersionInfo::PRODUCTVERSION, numeric)
        .set("FileVersion", version)
        .set("ProductVersion", version)
        .set("FileDescription", "bd-neon")
        .set("ProductName", "bd-neon")
        .set("InternalName", "bd-neon")
        .set("OriginalFilename", "bd-neon.exe");
    let rc_file = write_neutral_resource_file(&res)?;
    res.set_resource_file(&rc_file);
    res.compile()?;
    Ok(())
}

// winres emits no LANGUAGE statement, so rc.exe stamps every resource —
// including RT_MANIFEST — with en-US (0x0409). The SxS loader has no
// cross-language fallback, so the exe fails with 14001
// (ERROR_SXS_CANT_GEN_ACTCTX) on non-en-US Windows. Force neutral (0)
// language by prepending the statement to winres's generated .rc and
// compiling that file instead. Version stamping, icon, and manifest
// selection stay with winres; only the language override is ours.
#[cfg(windows)]
fn write_neutral_resource_file(
    res: &winres::WindowsResource,
) -> Result<String, Box<dyn std::error::Error>> {
    let rc_path = std::path::PathBuf::from(std::env::var("OUT_DIR")?).join("resource.rc");
    res.write_resource_file(&rc_path)?;
    let generated = std::fs::read_to_string(&rc_path)?;
    std::fs::write(&rc_path, format!("LANGUAGE 0, 0\n{generated}"))?;
    Ok(rc_path.to_string_lossy().into_owned())
}

// Parse "major.minor.patch..." into the u64 FILEVERSION/PRODUCTVERSION
// layout: MAJOR << 48 | MINOR << 32 | PATCH << 16 | RELEASE.
// Non-numeric suffixes (e.g. "-beta") stop the current component;
// missing components default to 0; components saturate at u16::MAX.
#[cfg(windows)]
fn version_u64(version: &str) -> u64 {
    let mut parts = version.split('.').map(numeric_prefix_u16);
    let major: u16 = parts.next().unwrap_or_default();
    let minor: u16 = parts.next().unwrap_or_default();
    let patch: u16 = parts.next().unwrap_or_default();
    let release: u16 = parts.next().unwrap_or_default();
    // Shifts cannot overflow: each part is u16 widened to u64.
    combination(major, minor, patch, release)
}

#[cfg(windows)]
#[allow(
    clippy::arithmetic_side_effects,
    reason = "shifts of u16-wide values into u64 cannot overflow"
)]
fn combination(major: u16, minor: u16, patch: u16, release: u16) -> u64 {
    u64::from(major) << 48 | u64::from(minor) << 32 | u64::from(patch) << 16 | u64::from(release)
}

#[cfg(windows)]
fn numeric_prefix_u16(text: &str) -> u16 {
    let mut value: u16 = 0;
    for byte in text.bytes() {
        let digit = match byte.checked_sub(b'0') {
            Some(offset) if offset <= 9 => u16::from(offset),
            Some(_) | None => break,
        };
        if let Some(next) = value
            .checked_mul(10)
            .and_then(|scaled| scaled.checked_add(digit))
        {
            value = next;
        } else {
            value = u16::MAX;
            break;
        }
    }
    value
}
