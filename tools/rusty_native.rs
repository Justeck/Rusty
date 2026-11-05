mod native_package;

use std::{ffi::OsString, path::PathBuf};

const HELP: &str = "Rusty native release tools (structural checks; no SDK/game execution)\n\nUsage: rusty-native <package|check|verify-package> [PACKAGE] [--root PATH] [--out-dir PATH]\n\n  package                  Validate source, write deterministic .hnmod and SHA256SUMS\n  check                    Validate source manifests, pins, references and notices\n  verify-package PACKAGE   Validate an existing archive without extraction/execution\n\nDefaults: --root is this Cargo project; --out-dir is ROOT/dist.\nSDK hook/type checks remain available through python tools/native.py verify-package.\n";

fn run(args: impl Iterator<Item = OsString>) -> native_package::Result<()> {
    let mut args = args.peekable();
    let Some(action) = args.next() else {
        return Err(HELP.into());
    };
    if action == "--help" || action == "-h" {
        print!("{HELP}");
        return Ok(());
    }
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut out = None;
    let mut package = None;
    while let Some(arg) = args.next() {
        if arg == "--root" {
            root = PathBuf::from(args.next().ok_or("--root requires a path")?);
        } else if arg == "--out-dir" {
            out = Some(PathBuf::from(
                args.next().ok_or("--out-dir requires a path")?,
            ));
        } else if action == "verify-package"
            && package.is_none()
            && !arg.to_string_lossy().starts_with('-')
        {
            package = Some(PathBuf::from(arg));
        } else {
            return Err(format!("unexpected argument: {}\n{HELP}", arg.to_string_lossy()).into());
        }
    }
    let receipt = match action.to_str() {
        Some("package") => {
            native_package::package(&root, &out.unwrap_or_else(|| root.join("dist")))?
        }
        Some("check") if out.is_none() => native_package::check(&root)?,
        Some("verify-package") if out.is_none() => native_package::verify(
            &root,
            &package.ok_or("verify-package requires an existing .hnmod path")?,
        )?,
        _ => return Err(HELP.into()),
    };
    println!("{}", serde_json::to_string(&receipt)?);
    Ok(())
}

fn main() {
    if let Err(error) = run(std::env::args_os().skip(1)) {
        eprintln!("rusty-native: {error}");
        std::process::exit(1);
    }
}
