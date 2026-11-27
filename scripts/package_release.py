"""Build and smoke-test a relocatable release archive for the host platform."""

import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
WINDOWS = os.name == "nt"
BINARY = "hello-world.exe" if WINDOWS else "hello-world"
LAUNCHER = "hello-world.cmd" if WINDOWS else "hello-world"


def archive_bundle(bundle, archive):
    if WINDOWS:
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as handle:
            for path in sorted(bundle.rglob("*")):
                if path.is_file():
                    handle.write(path, path.relative_to(bundle.parent))
    else:
        with tarfile.open(archive, "w:gz") as handle:
            handle.add(bundle, arcname=bundle.name)


def smoke_test(archive, folder, bundle_name):
    if WINDOWS:
        with zipfile.ZipFile(archive) as handle:
            handle.extractall(folder)
    else:
        with tarfile.open(archive) as handle:
            handle.extractall(folder, filter="data")
    launcher = folder / bundle_name / LAUNCHER
    command = ["cmd.exe", "/d", "/c", "call", str(launcher)] if WINDOWS else [str(launcher)]
    environment = dict(os.environ, HELLO_WORLD_ROOT=str(folder / "invalid-root"))
    result = subprocess.run(command, cwd=folder, env=environment, capture_output=True, check=True)
    if result.stdout.replace(b"\r\n", b"\n") != b"hello world\n":
        raise RuntimeError(f"unexpected packaged stdout: {result.stdout!r}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--name", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    suffix = ".zip" if WINDOWS else ".tar.gz"
    archive = args.output / f"{args.name}{suffix}"

    with tempfile.TemporaryDirectory(prefix="hello-world-release-") as temp:
        root = Path(temp)
        bundle = root / args.name
        shutil.copytree(ROOT / "src", bundle / "src")
        (bundle / "bin").mkdir()
        shutil.copy2(args.binary, bundle / "bin" / BINARY)
        shutil.copy2(ROOT / "README.md", bundle)
        shutil.copy2(ROOT / "CHANGELOG.md", bundle)
        launcher = bundle / LAUNCHER
        if WINDOWS:
            launcher.write_text(
                '@echo off\nsetlocal\nset "HELLO_WORLD_ROOT=%~dp0."\n"%~dp0bin\\hello-world.exe" %*\n',
                encoding="ascii",
            )
        else:
            launcher.write_text(
                '#!/bin/sh\nset -eu\n'
                'root="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"\n'
                'HELLO_WORLD_ROOT="$root" exec "$root/bin/hello-world" "$@"\n',
                encoding="ascii",
            )
            launcher.chmod(0o755)

        archive_bundle(bundle, archive)
        smoke_test(archive, root / "extracted", bundle.name)

    with archive.open("rb") as handle:
        digest = hashlib.file_digest(handle, "sha256").hexdigest()
    archive.with_name(f"{archive.name}.sha256").write_bytes(f"{digest}  {archive.name}\n".encode("ascii"))
    print(archive)


if __name__ == "__main__":
    main()
