//! Bounded, non-executing source-package validation and deterministic packaging.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

type Files = BTreeMap<String, Vec<u8>>;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const MAX_FILE: u64 = 4 * 1024 * 1024;
const MAX_TOTAL: usize = 32 * 1024 * 1024;
const MAX_ARCHIVE: u64 = 64 * 1024 * 1024;
const MAX_ENTRIES: usize = 2048;
const PREFIX: &str = "mixins/rusty/";

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u32,
    id: String,
    version: String,
    game: String,
    horizon: String,
    mixins: String,
}

fn ensure(ok: bool, reason: impl Into<String>) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(reason.into().into())
    }
}

fn normalize(bytes: &[u8]) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(bytes)?
        .strip_prefix('\u{feff}')
        .unwrap_or(std::str::from_utf8(bytes)?);
    ensure(!text.contains('\0'), "NUL in text asset")?;
    Ok(text.replace("\r\n", "\n").replace('\r', "\n").into_bytes())
}

fn safe_path(name: &str) -> Result<()> {
    ensure(
        name.is_ascii()
            && name.len() <= 240
            && !name.contains(['\\', ':', '<', '>', '"', '|', '?', '*']),
        format!("unsafe asset path: {name}"),
    )?;
    for part in name.split('/') {
        ensure(
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part.ends_with(['.', ' '])
                && !part.bytes().any(|b| b < 32 || b == 127),
            format!("unsafe asset path: {name}"),
        )?;
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        ensure(
            !matches!(
                stem.as_str(),
                "CON"
                    | "PRN"
                    | "AUX"
                    | "NUL"
                    | "COM1"
                    | "COM2"
                    | "COM3"
                    | "COM4"
                    | "COM5"
                    | "COM6"
                    | "COM7"
                    | "COM8"
                    | "COM9"
                    | "LPT1"
                    | "LPT2"
                    | "LPT3"
                    | "LPT4"
                    | "LPT5"
                    | "LPT6"
                    | "LPT7"
                    | "LPT8"
                    | "LPT9"
            ),
            format!("reserved asset path: {name}"),
        )?;
    }
    Ok(())
}

fn payload_path(name: &str) -> Result<()> {
    safe_path(name)?;
    let file = name.rsplit('/').next().unwrap_or("");
    ensure(
        name == "horizon-native.json"
            || (name.starts_with(PREFIX)
                && (name.ends_with(".rs")
                    || name.ends_with(".toml")
                    || file.starts_with("LICENSE")
                    || file.starts_with("NOTICE"))),
        format!("unexpected payload: {name}"),
    )
}

fn text<'a>(files: &'a Files, name: &str) -> Result<&'a str> {
    Ok(std::str::from_utf8(files.get(name).ok_or_else(|| {
        format!("required payload missing: {name}")
    })?)?)
}

fn validate(files: &Files, expected: &serde_json::Value) -> Result<()> {
    ensure(files.len() <= MAX_ENTRIES, "too many payload entries")?;
    let mut total = 0usize;
    let mut names = BTreeSet::new();
    for (name, bytes) in files {
        payload_path(name)?;
        ensure(
            names.insert(name.to_ascii_lowercase()),
            format!("duplicate/case-colliding payload: {name}"),
        )?;
        ensure(
            bytes.len() as u64 <= MAX_FILE,
            format!("asset exceeds 4 MiB: {name}"),
        )?;
        total = total
            .checked_add(bytes.len())
            .ok_or("payload size overflow")?;
        ensure(total <= MAX_TOTAL, "payload exceeds 32 MiB")?;
        ensure(
            normalize(bytes)? == *bytes,
            format!("asset must be UTF-8 without BOM and use LF: {name}"),
        )?;
    }
    let m: Manifest = serde_json::from_str(text(files, "horizon-native.json")?)?;
    ensure(
        serde_json::to_value(&m)? == *expected,
        "package manifest differs from the required repository manifest",
    )?;
    ensure(
        m.schema == 1 && m.id == "rusty",
        "expected schema 1 and Rusty id",
    )?;
    ensure(
        m.version.split('.').count() == 3
            && m.version
                .split('.')
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
            && !m.game.trim().is_empty(),
        "invalid release version/game",
    )?;
    for pin in [&m.horizon, &m.mixins] {
        ensure(
            pin.len() == 40
                && pin
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
            "revision pins must be full lowercase SHA-1s",
        )?;
    }
    let mixin: toml::Value = toml::from_str(text(files, &format!("{PREFIX}horizon-mixin.toml"))?)?;
    ensure(
        mixin
            .get("mod")
            .and_then(|v| v.get("id"))
            .and_then(toml::Value::as_str)
            == Some(&m.id),
        "mixin id mismatch",
    )?;
    ensure(
        mixin
            .get("mod")
            .and_then(|v| v.get("version"))
            .and_then(toml::Value::as_str)
            == Some(&m.version),
        "mixin version mismatch",
    )?;
    ensure(
        mixin
            .get("compatibility")
            .and_then(|v| v.get("horizon_revision"))
            .and_then(toml::Value::as_str)
            == Some(&m.horizon),
        "mixin revision mismatch",
    )?;
    for (section, field) in [("module", "source"), ("injection", "helper")] {
        let entries = mixin
            .get(section)
            .and_then(toml::Value::as_array)
            .ok_or_else(|| format!("missing mixin {section} entries"))?;
        ensure(
            !entries.is_empty(),
            format!("empty mixin {section} entries"),
        )?;
        for entry in entries {
            let source = entry
                .get(field)
                .and_then(toml::Value::as_str)
                .ok_or_else(|| format!("missing {section}.{field}"))?;
            safe_path(source)?;
            ensure(
                source.ends_with(".rs") && files.contains_key(&format!("{PREFIX}{source}")),
                format!("missing or invalid mixin source: {source}"),
            )?;
        }
    }
    for (name, anchors) in [
        (
            "LICENSE.rs",
            &[
                "Apache License",
                "Version 2.0",
                "TERMS AND CONDITIONS",
                "END OF TERMS AND CONDITIONS",
            ][..],
        ),
        (
            "LICENSE-Camera.rs",
            &[
                "MIT License",
                "umbx-dot",
                "Permission is hereby granted, free of charge",
                "THE SOFTWARE IS PROVIDED",
            ][..],
        ),
        (
            "LICENSE-Horizon.rs",
            &[
                "MIT License",
                "Ludovic Marie",
                "Permission is hereby granted, free of charge",
                "THE SOFTWARE IS PROVIDED",
            ][..],
        ),
        (
            "NOTICE.rs",
            &[
                "Rusty",
                "Apache-2.0",
                "X To Xray",
                "RealCat1st",
                "umbx-dot",
                "Horizon",
                "Ludovic Marie",
            ][..],
        ),
    ] {
        let notice = text(files, &format!("{PREFIX}{name}"))?;
        ensure(
            anchors.iter().all(|s| notice.contains(s)),
            format!("incomplete license/notice: {name}"),
        )?;
    }
    Ok(())
}

fn encode(files: &Files) -> Result<Vec<u8>> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(6))
        .last_modified_time(zip::DateTime::from_date_and_time(2026, 1, 1, 0, 0, 0)?)
        .system(zip::System::Unix)
        .unix_permissions(0o644)
        .external_attributes(0o100644 << 16);
    for (name, bytes) in files {
        writer.start_file(name, options)?;
        writer.write_all(bytes)?;
    }
    Ok(writer.finish()?.into_inner())
}

fn decode(bytes: &[u8]) -> Result<Files> {
    ensure(bytes.len() as u64 <= MAX_ARCHIVE, "archive exceeds 64 MiB")?;
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    ensure(archive.len() <= MAX_ENTRIES, "too many ZIP entries")?;
    archive.validate_header_consistency()?;
    ensure(
        !archive.has_overlapping_entries()?,
        "overlapping ZIP entries",
    )?;
    let mut files = Files::new();
    let mut seen = BTreeSet::new();
    let mut total = 0usize;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = std::str::from_utf8(entry.name_raw())?.to_owned();
        payload_path(&name)?;
        ensure(
            seen.insert(name.to_ascii_lowercase()),
            format!("duplicate/case-colliding ZIP entry: {name}"),
        )?;
        ensure(
            !entry.encrypted()
                && !entry.is_dir()
                && entry
                    .unix_mode()
                    .is_none_or(|mode| mode & 0o170000 == 0 || mode & 0o170000 == 0o100000),
            format!("encrypted/nonregular ZIP entry: {name}"),
        )?;
        ensure(
            matches!(
                entry.compression(),
                zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
            ),
            "unsupported compression",
        )?;
        ensure(
            entry.size() <= MAX_FILE,
            format!("asset exceeds 4 MiB: {name}"),
        )?;
        total = total
            .checked_add(entry.size() as usize)
            .ok_or("payload size overflow")?;
        ensure(total <= MAX_TOTAL, "payload exceeds 32 MiB")?;
        let mut data = Vec::with_capacity(entry.size() as usize);
        (&mut entry).take(MAX_FILE + 1).read_to_end(&mut data)?;
        ensure(
            data.len() as u64 <= MAX_FILE && data.len() as u64 == entry.size(),
            "ZIP size mismatch",
        )?;
        ensure(
            normalize(&data)? == data,
            format!("noncanonical UTF-8/LF asset: {name}"),
        )?;
        files.insert(name, data);
    }
    Ok(files)
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let meta = fs::symlink_metadata(path)?;
    ensure(
        meta.is_file() && !meta.is_symlink(),
        format!("expected regular file: {}", path.display()),
    )?;
    ensure(
        meta.len() <= limit,
        format!("file too large: {}", path.display()),
    )?;
    let mut data = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut data)?;
    ensure(data.len() as u64 <= limit, "file grew beyond limit")?;
    Ok(data)
}

fn expected_manifest(root: &Path) -> Result<serde_json::Value> {
    let expected: Manifest = serde_json::from_slice(&normalize(&read_bounded(
        &root.join("horizon-native.json"),
        MAX_FILE,
    )?)?)?;
    let compat: serde_json::Value = serde_json::from_slice(&normalize(&read_bounded(
        &root.join("compatibility.json"),
        MAX_FILE,
    )?)?)?;
    ensure(
        compat["game"] == expected.game
            && compat["horizon_revision"] == expected.horizon
            && compat["horizon_mixins_revision"] == expected.mixins,
        "compatibility.json game/revision pins mismatch",
    )?;
    let cargo: toml::Value = toml::from_str(std::str::from_utf8(&normalize(&read_bounded(
        &root.join("Cargo.toml"),
        MAX_FILE,
    )?)?)?)?;
    ensure(
        cargo
            .get("package")
            .and_then(|p| p.get("version"))
            .and_then(toml::Value::as_str)
            == Some(&expected.version),
        "Cargo package version mismatch",
    )?;
    ensure(
        cargo
            .get("package")
            .and_then(|p| p.get("license"))
            .and_then(toml::Value::as_str)
            == Some("Apache-2.0"),
        "Cargo package license must remain Apache-2.0",
    )?;
    serde_json::to_value(expected).map_err(Into::into)
}

fn collect(root: &Path) -> Result<Files> {
    fn walk(root: &Path, dir: &Path, files: &mut Files, scanned: &mut usize) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            *scanned += 1;
            ensure(*scanned <= 8192, "source tree exceeds entry limit")?;
            let path = entry.path();
            let kind = entry.file_type()?;
            ensure(
                !kind.is_symlink(),
                format!("symlink in source payload: {}", path.display()),
            )?;
            if kind.is_dir() {
                walk(root, &path, files, scanned)?;
            } else if kind.is_file() {
                let name = path
                    .strip_prefix(root)?
                    .to_str()
                    .ok_or("non UTF-8 path")?
                    .replace('\\', "/");
                let file = name.rsplit('/').next().unwrap_or("");
                if name.ends_with(".rs")
                    || name.ends_with(".toml")
                    || file.starts_with("LICENSE")
                    || file.starts_with("NOTICE")
                {
                    payload_path(&name)?;
                    files.insert(name, normalize(&read_bounded(&path, MAX_FILE)?)?);
                    ensure(
                        files.len() <= MAX_ENTRIES
                            && files.values().map(Vec::len).sum::<usize>() <= MAX_TOTAL,
                        "source payload exceeds size/count limit",
                    )?;
                }
            } else {
                return Err(format!("nonregular source: {}", path.display()).into());
            }
        }
        Ok(())
    }
    let mut files = Files::new();
    files.insert(
        "horizon-native.json".into(),
        normalize(&read_bounded(&root.join("horizon-native.json"), MAX_FILE)?)?,
    );
    let bundle = root.join("mixins/rusty");
    ensure(
        !fs::symlink_metadata(root.join("mixins"))?.is_symlink(),
        "mixins directory cannot be a symlink",
    )?;
    ensure(
        fs::symlink_metadata(&bundle)?.is_dir() && !fs::symlink_metadata(&bundle)?.is_symlink(),
        "source bundle must be a regular directory",
    )?;
    walk(root, &bundle, &mut files, &mut 0)?;
    Ok(files)
}

#[derive(Serialize)]
pub struct Receipt {
    pub file: Option<PathBuf>,
    pub sha256: Option<String>,
    pub size: usize,
    pub entries: usize,
    pub version: String,
    pub validation: &'static str,
}

pub fn check(root: &Path) -> Result<Receipt> {
    let expected = expected_manifest(root)?;
    let files = collect(root)?;
    validate(&files, &expected)?;
    Ok(Receipt {
        file: None,
        sha256: None,
        size: files.values().map(Vec::len).sum(),
        entries: files.len(),
        version: expected["version"].as_str().unwrap().into(),
        validation: "source-structural",
    })
}

pub fn verify(root: &Path, path: &Path) -> Result<Receipt> {
    let expected = expected_manifest(root)?;
    let bytes = read_bounded(path, MAX_ARCHIVE)?;
    let files = decode(&bytes)?;
    validate(&files, &expected)?;
    Ok(Receipt {
        file: Some(path.to_owned()),
        sha256: Some(format!("{:x}", Sha256::digest(&bytes))),
        size: bytes.len(),
        entries: files.len(),
        version: expected["version"].as_str().unwrap().into(),
        validation: "archive-structural",
    })
}

pub fn package(root: &Path, out_dir: &Path) -> Result<Receipt> {
    let expected = expected_manifest(root)?;
    let files = collect(root)?;
    validate(&files, &expected)?;
    let bytes = encode(&files)?;
    // Validate the actual bytes before publishing; never extract or execute package contents.
    validate(&decode(&bytes)?, &expected)?;
    let name = format!("rusty-{}.hnmod", expected["version"].as_str().unwrap());
    fs::create_dir_all(out_dir)?;
    let path = out_dir.join(&name);
    let digest = format!("{:x}", Sha256::digest(&bytes));
    fs::write(&path, &bytes)?;
    fs::write(out_dir.join("SHA256SUMS"), format!("{digest}  {name}\n"))?;
    Ok(Receipt {
        file: Some(path),
        sha256: Some(digest),
        size: bytes.len(),
        entries: files.len(),
        version: expected["version"].as_str().unwrap().into(),
        validation: "archive-structural",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, write::SimpleFileOptions};

    fn manifest() -> serde_json::Value {
        serde_json::json!({"schema":1,"id":"rusty","version":"1.2.0","game":"26.3","horizon":"1111111111111111111111111111111111111111","mixins":"2222222222222222222222222222222222222222"})
    }
    fn fixture() -> Files {
        let mut f = Files::new();
        f.insert(
            "horizon-native.json".into(),
            serde_json::to_vec(&manifest()).unwrap(),
        );
        let prefix = "mixins/rusty/";
        for (name, data) in [
            (
                "horizon-mixin.toml",
                "[mod]\nid='rusty'\nversion='1.2.0'\n[compatibility]\nhorizon_revision='1111111111111111111111111111111111111111'\n[[module]]\nsource='runtime.rs'\n[[injection]]\nhelper='hook.rs'\n",
            ),
            ("runtime.rs", "pub fn run() {}\n"),
            ("hook.rs", "{}\n"),
            (
                "LICENSE.rs",
                "Apache License\nVersion 2.0\nTERMS AND CONDITIONS\nEND OF TERMS AND CONDITIONS\n",
            ),
            (
                "LICENSE-Camera.rs",
                "MIT License\nCopyright 2026 umbx-dot\nPermission is hereby granted, free of charge\nTHE SOFTWARE IS PROVIDED\n",
            ),
            (
                "LICENSE-Horizon.rs",
                "MIT License\nCopyright Ludovic Marie\nPermission is hereby granted, free of charge\nTHE SOFTWARE IS PROVIDED\n",
            ),
            (
                "NOTICE.rs",
                "Rusty Apache-2.0 X To Xray RealCat1st umbx-dot Horizon Ludovic Marie\n",
            ),
        ] {
            f.insert(format!("{prefix}{name}"), data.as_bytes().to_vec());
        }
        f
    }
    fn raw_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in entries {
            writer
                .start_file(
                    *name,
                    SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
                )
                .unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
    #[test]
    fn canonicalizes_bom_crlf_and_lone_cr_without_changing_unicode() {
        assert_eq!(
            normalize(b"\xef\xbb\xbf// caf\xc3\xa9\r\nnext\rlast\n").unwrap(),
            "// café\nnext\nlast\n".as_bytes()
        );
    }
    #[test]
    fn invalid_utf8_and_nul_are_rejected() {
        assert!(normalize(b"\xff").is_err());
        assert!(normalize(b"a\0b").is_err());
    }
    #[test]
    fn valid_release_roundtrips_with_fixed_sorted_metadata() {
        let files = fixture();
        validate(&files, &manifest()).unwrap();
        let bytes = encode(&files).unwrap();
        assert_eq!(decode(&bytes).unwrap(), files);
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut previous = String::new();
        for i in 0..archive.len() {
            let entry = archive.by_index(i).unwrap();
            assert!(entry.name().unwrap().as_ref() > previous.as_str());
            previous = entry.name().unwrap().into_owned();
            assert_eq!(entry.unix_mode(), Some(0o100644));
            assert_eq!(
                entry.last_modified().unwrap(),
                zip::DateTime::from_date_and_time(2026, 1, 1, 0, 0, 0).unwrap()
            );
            assert_eq!(entry.compression(), zip::CompressionMethod::Deflated);
        }
    }
    #[test]
    fn repeated_builds_have_identical_bytes() {
        assert_eq!(encode(&fixture()).unwrap(), encode(&fixture()).unwrap());
    }
    #[test]
    fn rejects_manifest_identity_version_schema_and_pin_mismatches() {
        for (key, value) in [
            ("id", serde_json::json!("another")),
            ("version", serde_json::json!("2.0.0")),
            ("schema", serde_json::json!(2)),
            (
                "horizon",
                serde_json::json!("3333333333333333333333333333333333333333"),
            ),
            (
                "mixins",
                serde_json::json!("3333333333333333333333333333333333333333"),
            ),
            ("game", serde_json::json!("26.4")),
        ] {
            let mut files = fixture();
            let mut m = manifest();
            m[key] = value;
            files.insert(
                "horizon-native.json".into(),
                serde_json::to_vec(&m).unwrap(),
            );
            assert!(validate(&files, &manifest()).is_err(), "{key}");
        }
    }
    #[test]
    fn rejects_mixin_version_and_revision_mismatches() {
        for (before, after) in [
            ("1.2.0", "1.9.0"),
            (
                "1111111111111111111111111111111111111111",
                "3333333333333333333333333333333333333333",
            ),
        ] {
            let mut files = fixture();
            let name = "mixins/rusty/horizon-mixin.toml";
            files.insert(
                name.into(),
                String::from_utf8(files[name].clone())
                    .unwrap()
                    .replace(before, after)
                    .into_bytes(),
            );
            assert!(validate(&files, &manifest()).is_err());
        }
    }
    #[test]
    fn refuses_missing_or_empty_required_licenses_and_notices() {
        for name in [
            "LICENSE.rs",
            "LICENSE-Camera.rs",
            "LICENSE-Horizon.rs",
            "NOTICE.rs",
        ] {
            let mut files = fixture();
            let key = format!("mixins/rusty/{name}");
            files.remove(&key);
            assert!(validate(&files, &manifest()).is_err(), "missing {name}");
            files.insert(key, b"/* placeholder */\n".to_vec());
            assert!(validate(&files, &manifest()).is_err(), "empty {name}");
        }
    }
    #[test]
    fn rejects_missing_module_or_injection_source() {
        for name in ["runtime.rs", "hook.rs"] {
            let mut files = fixture();
            files.remove(&format!("mixins/rusty/{name}"));
            assert!(validate(&files, &manifest()).is_err());
        }
    }
    #[test]
    fn rejects_traversal_absolute_windows_and_non_payload_paths() {
        for name in [
            "../escape.rs",
            "/absolute.rs",
            "C:/drive.rs",
            "mixins/rusty/../evil.rs",
            "mixins\\rusty\\evil.rs",
            "mixins/rusty/a//b.rs",
            "mixins/rusty/a:stream.rs",
            "mixins/other/evil.rs",
            "mixins/rusty/run.exe",
        ] {
            let mut files = fixture();
            files.insert(name.into(), b"text\n".to_vec());
            assert!(validate(&files, &manifest()).is_err(), "{name}");
            assert!(
                decode(&raw_zip(&[(name, b"text\n")])).is_err(),
                "ZIP {name}"
            );
        }
    }
    #[test]
    fn rejects_case_colliding_archive_entries() {
        let bytes = raw_zip(&[("mixins/rusty/a.rs", b"a"), ("mixins/rusty/A.rs", b"b")]);
        assert!(decode(&bytes).is_err());
    }
    #[test]
    fn rejects_invalid_zip_and_excessive_entry_size() {
        assert!(decode(b"not a zip").is_err());
        let huge = vec![b'x'; 4 * 1024 * 1024 + 1];
        assert!(decode(&raw_zip(&[("mixins/rusty/huge.rs", &huge)])).is_err());
    }
    #[test]
    fn rejects_noncanonical_text_in_existing_archives() {
        assert!(decode(&raw_zip(&[("mixins/rusty/a.rs", b"a\r\n")])).is_err());
        assert!(decode(&raw_zip(&[("mixins/rusty/a.rs", b"\xef\xbb\xbftext\n")])).is_err());
        assert!(decode(&raw_zip(&[("mixins/rusty/a.rs", b"\xff")])).is_err());
    }

    #[test]
    fn rejects_duplicate_names_even_when_zip_index_deduplicates() {
        let mut bytes = raw_zip(&[("mixins/rusty/a.rs", b"a"), ("mixins/rusty/b.rs", b"b")]);
        let needle = b"mixins/rusty/b.rs";
        for i in 0..bytes.len() - needle.len() {
            if &bytes[i..i + needle.len()] == needle {
                bytes[i + needle.len() - 4] = b'a';
            }
        }
        assert!(decode(&bytes).is_err());
    }

    #[test]
    fn rejects_zip_crc_corruption() {
        let mut bytes = raw_zip(&[("mixins/rusty/a.rs", b"UNIQUE-RUST-PAYLOAD")]);
        let offset = bytes
            .windows(19)
            .position(|b| b == b"UNIQUE-RUST-PAYLOAD")
            .unwrap();
        bytes[offset] = b'X';
        assert!(decode(&bytes).is_err());
    }

    #[test]
    fn rejects_total_decompressed_size_and_entry_count_limits() {
        let data = vec![b'x'; 4 * 1024 * 1024];
        let names: Vec<String> = (0..9).map(|i| format!("mixins/rusty/{i}.rs")).collect();
        let entries: Vec<_> = names
            .iter()
            .map(|s| (s.as_str(), data.as_slice()))
            .collect();
        assert!(decode(&raw_zip(&entries)).is_err());
        let names: Vec<String> = (0..2049).map(|i| format!("mixins/rusty/{i}.rs")).collect();
        let entries: Vec<_> = names.iter().map(|s| (s.as_str(), &b""[..])).collect();
        assert!(decode(&raw_zip(&entries)).is_err());
    }

    struct Root(PathBuf);
    impl Root {
        fn fixture() -> Self {
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!(
                "rusty-native-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            for (name, bytes) in fixture() {
                let path = root.join(name);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, bytes).unwrap();
            }
            fs::write(
                root.join("Cargo.toml"),
                "[package]\nname='test-fixture'\nversion='1.2.0'\nlicense='Apache-2.0'\n",
            )
            .unwrap();
            fs::write(root.join("compatibility.json"), serde_json::to_vec(&serde_json::json!({"game":"26.3","horizon_revision":"1111111111111111111111111111111111111111","horizon_mixins_revision":"2222222222222222222222222222222222222222"})).unwrap()).unwrap();
            Self(root)
        }
    }
    impl Drop for Root {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn source_packaging_normalizes_text_filters_extras_and_writes_valid_checksum() {
        let root = Root::fixture();
        fs::write(
            root.0.join("mixins/rusty/runtime.rs"),
            b"\xef\xbb\xbfpub fn run() {}\r\n",
        )
        .unwrap();
        fs::write(root.0.join("mixins/rusty/scratch.bin"), b"\xff\0").unwrap();
        let output = root.0.join("release");
        let receipt = package(&root.0, &output).unwrap();
        let archive = receipt.file.unwrap();
        assert_eq!(archive.file_name().unwrap(), "rusty-1.2.0.hnmod");
        let bytes = fs::read(&archive).unwrap();
        let files = decode(&bytes).unwrap();
        assert_eq!(files["mixins/rusty/runtime.rs"], b"pub fn run() {}\n");
        assert!(!files.contains_key("mixins/rusty/scratch.bin"));
        assert_eq!(files.len(), 8);
        assert_eq!(
            fs::read_to_string(output.join("SHA256SUMS")).unwrap(),
            format!("{}  rusty-1.2.0.hnmod\n", receipt.sha256.unwrap())
        );
        assert_eq!(verify(&root.0, &archive).unwrap().entries, 8);
        assert_eq!(check(&root.0).unwrap().entries, 8);
        package(&root.0, &output).unwrap();
        assert_eq!(fs::read(archive).unwrap(), bytes);
    }

    #[test]
    fn rejects_cargo_and_compatibility_drift_and_malformed_manifests() {
        let root = Root::fixture();
        let cargo = root.0.join("Cargo.toml");
        let good = fs::read(&cargo).unwrap();
        for bad in [
            "[workspace]\n",
            "[package]\nversion='1.9.0'\nlicense='Apache-2.0'\n",
            "[package]\nversion='1.2.0'\nlicense='MIT'\n",
        ] {
            fs::write(&cargo, bad).unwrap();
            assert!(check(&root.0).is_err());
        }
        fs::write(cargo, good).unwrap();
        let compat = root.0.join("compatibility.json");
        let good = fs::read(&compat).unwrap();
        for key in ["game", "horizon_revision", "horizon_mixins_revision"] {
            let mut value: serde_json::Value = serde_json::from_slice(&good).unwrap();
            value[key] = serde_json::json!("wrong");
            fs::write(&compat, serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(check(&root.0).is_err(), "{key}");
        }
    }

    #[test]
    fn verifies_existing_archive_instead_of_rebuilding_or_executing_it() {
        let root = Root::fixture();
        let archive = root.0.join("existing.hnmod");
        fs::write(&archive, encode(&fixture()).unwrap()).unwrap();
        fs::write(
            root.0.join("mixins/rusty/runtime.rs"),
            b"invalid current source \xff",
        )
        .unwrap();
        assert!(check(&root.0).is_err());
        assert_eq!(verify(&root.0, &archive).unwrap().entries, 8);
        let original = fs::read(&archive).unwrap();
        let mut files = fixture();
        files.insert("horizon-native.json".into(), b"{}".to_vec());
        fs::write(&archive, encode(&files).unwrap()).unwrap();
        assert!(verify(&root.0, &archive).is_err());
        assert_ne!(fs::read(archive).unwrap(), original);
        assert!(!root.0.join("dist").exists());
    }
}
