// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// File dialogs and bounded file work run on workers, never the render thread.
use super::rusty_schematic::{self as codec, Schematic};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
};
const PENDING: u8 = 0;
const CANCELLED: u8 = 1;
const COMMITTING: u8 = 2;
const FINISHED: u8 = 3;
pub type Cancellation = Arc<AtomicU8>;
pub fn cancellation() -> Cancellation {
    Arc::new(AtomicU8::new(PENDING))
}
static WORKER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
struct WorkerGuard;
impl Drop for WorkerGuard {
    fn drop(&mut self) {
        WORKER.store(false, std::sync::atomic::Ordering::Release);
    }
}
pub enum Output {
    Loaded(String, Schematic),
    Saved(String),
    Cancelled,
}
/// Nonblocking arbitration. False means an authorized commit already began;
/// it may finish, while the caller still invalidates scope/generation results.
pub fn cancel(token: &Cancellation) -> bool {
    match token.compare_exchange(PENDING, CANCELLED, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) | Err(CANCELLED) => true,
        Err(_) => false,
    }
}
pub fn begin(
    path: Option<PathBuf>,
    saving: Option<Schematic>,
    overwrite: bool,
    data_version: i32,
) -> (Cancellation, Receiver<Result<Output, String>>) {
    let token = cancellation();
    let worker = token.clone();
    let (tx, rx) = mpsc::channel();
    if WORKER.swap(true, std::sync::atomic::Ordering::AcqRel) {
        let _ = tx.send(Err(
            "Close the existing native file chooser before opening another".into(),
        ));
        return (token, rx);
    }
    thread::spawn(move || {
        let _guard = WorkerGuard;
        let result = (|| {
            let save = saving.is_some();
            let (path, replace) = if let Some(path) = path {
                (path, overwrite)
            } else {
                let Some(path) = rusty_schematic_platform::choose(save)? else {
                    return Ok(Output::Cancelled);
                };
                let replace = save && path.exists();
                (path, replace)
            };
            if worker.load(Ordering::Acquire) != PENDING {
                return Ok(Output::Cancelled);
            }
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if let Some(schematic) = saving {
                let bytes = codec::encode_sponge(&schematic, data_version)?;
                atomic_write(&path, &bytes, replace, &worker)?;
                Ok(Output::Saved(name))
            } else {
                let mut bytes = Vec::new();
                fs::File::open(&path)
                    .map_err(|e| format!("Cannot open {name}: {e}"))?
                    .take(32 * 1024 * 1024 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|e| format!("Cannot read {name}: {e}"))?;
                if bytes.len() > 32 * 1024 * 1024 {
                    return Err("File exceeds 32 MiB".into());
                }
                let mut schematic = codec::decode(&bytes)?;
                schematic.name = name.chars().filter(|c| !c.is_control()).take(256).collect();
                Ok(Output::Loaded(name, schematic))
            }
        })();
        let _ = tx.send(result);
    });
    (token, rx)
}

pub fn atomic_write(
    path: &Path,
    bytes: &[u8],
    replace: bool,
    live: &Cancellation,
) -> Result<(), String> {
    atomic_write_using(
        path,
        bytes,
        replace,
        live,
        || {},
        rusty_schematic_platform::commit,
    )
}
/// Hooks isolate a blocked filesystem commit in deterministic tests; the normal
/// worker passes the actual platform commit and never locks game-thread state.
pub fn atomic_write_using(
    path: &Path,
    bytes: &[u8],
    replace: bool,
    live: &Cancellation,
    before_commit: impl FnOnce(),
    commit: impl FnOnce(&Path, &Path, bool) -> Result<(), String>,
) -> Result<(), String> {
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("Export exceeds 32 MiB".into());
    }
    fn destination_stamp(
        path: &Path,
    ) -> Result<Option<(u64, std::time::SystemTime, Option<std::time::SystemTime>)>, String> {
        match fs::metadata(path) {
            Ok(m) => {
                if !m.is_file() {
                    return Err("Export destination is not a regular file".into());
                }
                Ok(Some((
                    m.len(),
                    m.modified().map_err(|e| e.to_string())?,
                    m.created().ok(),
                )))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("Cannot inspect export destination: {e}")),
        }
    }
    let original = destination_stamp(path)?;
    if original.is_some() && !replace {
        return Err("Export destination already exists; overwrite was not confirmed".into());
    }
    let parent = path.parent().ok_or("Save path has no parent")?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temp = parent.join(format!(
        ".rusty-schematic-{}-{stamp}.tmp",
        std::process::id()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| format!("Cannot create export: {e}"))?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| format!("Cannot finish export: {e}"))?;
        drop(file);
        before_commit();
        if live.load(Ordering::Acquire) != PENDING {
            return Err("Export cancelled".into());
        }
        if destination_stamp(path)? != original {
            return Err(
                "Export destination changed after overwrite confirmation; choose it again".into(),
            );
        }
        live.compare_exchange(PENDING, COMMITTING, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "Export cancelled before commit")?;
        let result = commit(&temp, path, replace);
        live.store(FINISHED, Ordering::Release);
        result
    })();
    if temp.exists() {
        let _ = fs::remove_file(&temp);
    }
    result
}
pub fn transform(
    model: Arc<Schematic>,
    rotation: u8,
    mirror: bool,
) -> (Cancellation, Receiver<Result<Output, String>>) {
    let token = cancellation();
    let (tx, rx) = mpsc::channel();
    if WORKER.swap(true, std::sync::atomic::Ordering::AcqRel) {
        let _ = tx.send(Err(
            "Close the existing native file chooser before transforming".into(),
        ));
        return (token, rx);
    }
    thread::spawn(move || {
        let _guard = WorkerGuard;
        let _ = tx.send(
            model
                .transformed(rotation, mirror)
                .map(|m| Output::Loaded("Transformed schematic".into(), m)),
        );
    });
    (token, rx)
}
