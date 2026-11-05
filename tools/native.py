#!/usr/bin/env python3
"""Build and package the exact-revision Horizon-native mod; Python is development-only."""
import argparse, json, os, pathlib, shutil, subprocess, sys, tarfile, tempfile, urllib.request
ROOT = pathlib.Path(__file__).resolve().parents[1]
COMPAT = json.loads((ROOT / "compatibility.json").read_text())
MANIFEST = json.loads((ROOT / "horizon-native.json").read_text())
BUNDLE = ROOT / "mixins" / MANIFEST["id"]
TARGET = ROOT / "target"

def run(*args, **kw):
    subprocess.run([str(a) for a in args], check=True, **kw)

def package():
    # One maintained packager. Cargo checks freshness and locked dependencies;
    # do not execute a potentially stale target/debug binary directly.
    result = subprocess.check_output([
        "cargo", "run", "--quiet", "--locked", "--manifest-path", str(ROOT / "Cargo.toml"),
        "--features", "native-tools", "--bin", "rusty-native", "--", "package", "--root", str(ROOT),
    ], encoding="utf-8")
    receipt = json.loads(result)
    print(json.dumps(receipt))
    return pathlib.Path(receipt["file"])

def source():
    # Each exact SDK pin owns its cache. Upgrades leave both older revision
    # caches and the legacy unqualified horizon-source tree intact.
    cache_name = "horizon-source-" + MANIFEST["horizon"]
    out = TARGET / cache_name
    if (out / ".horizon-revision").exists():
        assert (out / ".horizon-revision").read_text().strip() == MANIFEST["horizon"]
        return out
    TARGET.mkdir(exist_ok=True)
    # Cargo caches can restore workspace manifests without our completion
    # marker. Reconstruct that task-owned partial tree from a fresh extraction.
    with tempfile.TemporaryDirectory(prefix="native-source-", dir=TARGET) as scratch:
        scratch = pathlib.Path(scratch)
        archive = scratch / "source.tar.gz"
        urllib.request.urlretrieve("https://codeload.github.com/horizon-rust/horizon/tar.gz/" + MANIFEST["horizon"], archive)
        unpack = scratch / "unpack"
        unpack.mkdir()
        with tarfile.open(archive) as tar:
            tar.extractall(unpack, filter="data")
        tops = list(unpack.iterdir())
        assert len(tops) == 1 and tops[0].is_dir(), "expected one upstream source root"
        top = tops[0]
        for path in top.rglob("*"):
            if path.is_symlink(): path.unlink()
        if out.exists():
            assert not out.is_symlink() and out.resolve().is_relative_to(TARGET.resolve())
            assert TARGET.resolve().is_relative_to(ROOT) and out.name == cache_name
            shutil.rmtree(out)
        top.rename(out)
        (out / ".horizon-revision").write_text(MANIFEST["horizon"] + "\n")
    return out

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("action", choices=["package", "stage", "check", "build", "mesh-test", "verify-package"])
    p.add_argument("--horizon-tree", type=pathlib.Path)
    p.add_argument("--mixins-bin", type=pathlib.Path)
    p.add_argument("--resources", type=pathlib.Path)
    p.add_argument("--profile", choices=["performance", "dev"], default="performance", help="Native build profile; dev trades game speed for faster feature iteration")
    a = p.parse_args()
    if a.action == "package": package(); return
    tree = (a.horizon_tree or source()).resolve()
    if a.action == "verify-package":
        out = package()
        run("cargo", "run", "--locked", "--manifest-path", tree / "Cargo.toml", "-p", "cargo-horizon-mod", "--", "check", out, "--json")
        return
    exe = "horizon-mixins.exe" if os.name == "nt" else "horizon-mixins"
    tool = a.mixins_bin or TARGET / "mixins" / "bin" / exe
    if not tool.exists():
        run("cargo", "install", "--locked", "--git", COMPAT["horizon_mixins_repository"], "--rev", MANIFEST["mixins"], "--root", TARGET / "mixins", "horizon-mixins-cli")
    stage = TARGET / "stage"
    # The official tool cannot atomically replace an existing stage on Windows.
    # Apply into a fresh task-owned directory, then retain unchanged file timestamps.
    TARGET.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="native-apply-", dir=TARGET) as scratch:
        fresh = pathlib.Path(scratch) / "stage"
        run(tool.resolve(), "apply", "--workspace", tree, "--mods", BUNDLE.parent, "--out", fresh)
        stage.mkdir(exist_ok=True)
        assert stage.resolve().is_relative_to(TARGET.resolve())
        for old in stage.rglob("*"):
            if old.is_file() and not (fresh / old.relative_to(stage)).is_file(): old.unlink()
        for path in fresh.rglob("*"):
            if path.is_file():
                dest = stage / path.relative_to(fresh)
                if not dest.is_file() or dest.read_bytes() != path.read_bytes():
                    dest.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(path, dest)
    if a.action == "stage": return
    if a.action == "mesh-test":
        if not a.resources: p.error("mesh-test requires --resources (legally prepared Minecraft 26.3 assets)")
        # This exact SDK has an unrelated cfg(test) initializer missing rgb.
        # Correct that test fixture only in our generated stage; the native
        # production client and the pristine downloaded SDK are unchanged.
        from upstream_test_fixture import apply_stage
        correction = apply_stage(stage, MANIFEST["horizon"])
        print(json.dumps({"test_fixture_correction": correction}))
        env = dict(os.environ, HORIZON_MESH_TEST_RESOURCES=str(a.resources.resolve()))
        run("cargo", "test", "--profile", "fast", "--locked", "--manifest-path", stage / "Cargo.toml", "-p", "horizon-client-render", "--lib", "rusty_native_tests", "--", "--include-ignored", "--test-threads=1", env=env)
    else:
        args = ["cargo", a.action, "--locked", "--manifest-path", stage / "Cargo.toml", "-p", "horizon-client", "--bin", "horizon-client"]
        if a.action == "build":
            args += ["--profile", "fast"]
            if a.profile == "dev": args += ["--config", "profile.fast.package.horizon-client.opt-level=0"]
        run(*args)

if __name__ == "__main__": main()
