#!/usr/bin/env python3
"""Build Mods Core as an importable Cinnabar WebAssembly component (dist/modscore.wasm)."""

from pathlib import Path
import os
import shlex
import subprocess

ROOT = Path(__file__).resolve().parents[1]
TARGET = "wasm32-unknown-unknown"


def main() -> None:
    wrapper = shlex.split(os.environ.get("CINNABAR_CARGO_WRAPPER", ""))
    cargo = [*wrapper, "cargo"]
    target = ROOT / "target"
    environment = dict(os.environ, CARGO_TARGET_DIR=str(target))
    # The screen (surface.escaped) and the icon are baked in with include_str!/include_bytes!.
    # Cargo only notices a changed file by its timestamp, and files unpacked from a zip keep the
    # zip's old timestamps, so an update could silently reuse the previous build. Dropping this
    # mod's own cached build makes every build start from the files on disk.
    for extra in ([], ["--target", TARGET]):
        subprocess.run(
            [*cargo, "clean", "-p", "modscore", "-p", "modscore-pack", "--release", *extra],
            cwd=ROOT,
            env=environment,
            check=False,
        )
    subprocess.run(
        [*cargo, "build", "--locked", "--release", "-p", "modscore", "--target", TARGET],
        cwd=ROOT,
        env=environment,
        check=True,
    )
    core = target / TARGET / "release/modscore.wasm"
    output = ROOT / "dist/modscore.wasm"
    subprocess.run(
        [*cargo, "run", "--locked", "--release", "-p", "modscore-pack", "--", str(core), str(output)],
        cwd=ROOT,
        env=environment,
        check=True,
    )
    print(f"Built {output}")


if __name__ == "__main__":
    main()
