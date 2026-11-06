extern crate self as rusty_schematic_platform;
#[path = "../mixins/rusty/platform/lib.rs"]
mod platform;
pub use platform::{choose, commit};
// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
pub use rusty_horizon::rusty_schematic;
#[allow(dead_code)]
#[path = "../mixins/rusty/schematic_io.rs"]
mod io;
#[test]
fn cancellation_does_not_wait_for_worker_commit_io() {
    let dir = std::env::temp_dir().join(format!("rusty-commit-delay-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.schem");
    let token = io::cancellation();
    let worker_token = token.clone();
    let worker_path = path.clone();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        io::atomic_write_using(
            &worker_path,
            b"authorized",
            false,
            &worker_token,
            || {},
            |temp, path, replace| {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                commit(temp, path, replace)
            },
        )
    });
    entered_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();
    let cancel = std::thread::spawn(move || cancel_tx.send(io::cancel(&token)).unwrap());
    let outcome = cancel_rx.recv_timeout(std::time::Duration::from_millis(150));
    release_tx.send(()).unwrap();
    worker.join().unwrap().unwrap();
    cancel.join().unwrap();
    assert!(
        !outcome.unwrap(),
        "already committing must be reported truthfully"
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"authorized");
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
#[test]
fn atomic_unicode_save_requires_overwrite_and_cancel_preserves_original() {
    let dir = std::env::temp_dir().join(format!(
        "rusty-schematic-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("Costruzione_Русский_🧱.schem");
    let token = io::cancellation();
    io::atomic_write(&path, b"original", false, &token).unwrap();
    assert!(io::atomic_write(&path, b"unauthorized overwrite", false, &token).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    io::cancel(&token);
    assert!(io::atomic_write(&path, b"cancelled overwrite", true, &token).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    io::atomic_write(&path, b"confirmed overwrite", true, &io::cancellation()).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"confirmed overwrite");
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        1,
        "temporary files must be cleaned after every outcome"
    );
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn confirmed_overwrite_refuses_a_file_changed_while_export_is_preparing() {
    let dir = std::env::temp_dir().join(format!("rusty-schematic-race-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("selection.schem");
    std::fs::write(&path, b"old").unwrap();
    let live = io::cancellation();
    let worker_live = live.clone();
    let worker_path = path.clone();
    let newer_path = path.clone();
    let worker = std::thread::spawn(move || {
        io::atomic_write_using(
            &worker_path,
            b"prepared replacement",
            true,
            &worker_live,
            || {
                std::fs::write(&newer_path, b"a newer independently written file").unwrap();
            },
            commit,
        )
    });
    assert!(
        worker.join().unwrap().is_err(),
        "stale overwrite confirmation replaced newer file content"
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"a newer independently written file"
    );
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn cancellation_at_last_precommit_boundary_never_writes() {
    let dir = std::env::temp_dir().join(format!("rusty-precommit-cancel-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("saved.schem");
    std::fs::write(&path, b"original").unwrap();
    let token = io::cancellation();
    let result = io::atomic_write_using(
        &path,
        b"new",
        true,
        &token,
        || {
            assert!(io::cancel(&token));
        },
        |_, _, _| panic!("cancelled commit dispatched"),
    );
    assert!(result.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
