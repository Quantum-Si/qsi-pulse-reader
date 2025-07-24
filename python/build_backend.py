"""
Custom build backend that copies rust-core before building.
"""

import os
import shutil
from pathlib import Path

from setuptools import build_meta

# Re-export the standard backend functions
get_requires_for_build_sdist = build_meta.get_requires_for_build_sdist

prepare_metadata_for_build_wheel = build_meta.prepare_metadata_for_build_wheel
get_requires_for_build_wheel = build_meta.get_requires_for_build_wheel

prepare_metadata_for_build_editable = build_meta.prepare_metadata_for_build_editable
get_requires_for_build_editable = build_meta.get_requires_for_build_editable


def _copy_out_of_tree():
    """Copy out-of-tree requirements before building."""
    python_dir = Path(__file__).parent
    root_dir = python_dir.parent
    rust_core_src = root_dir / "rust-core"
    rust_core_dst = python_dir / "rust-core"

    # This is a hack: if rust_core_src doesn't exist, but rust_core_dst does,
    # this means we're probably in an isolated directory and this function was
    # already run before the isolated directory was set up. If neither exist,
    # then something has gone wrong, so bail setup.
    if not rust_core_src.exists():
        if not rust_core_dst.exists():
            raise RuntimeError("rust-core could not be found!")
        else:
            return

    # Remove existing rust-core (symlink or directory)
    if rust_core_dst.is_symlink():
        rust_core_dst.unlink()
    elif rust_core_dst.exists():
        shutil.rmtree(rust_core_dst)

    os.mkdir(rust_core_dst)
    # Copy the rust-core directory
    shutil.copytree(rust_core_src / "src", rust_core_dst / "src")
    for file in ["Cargo.toml", "Cargo.lock"]:
        shutil.copy(rust_core_src / file, rust_core_dst / file)

    # Copy over the pulses.bin file for testing
    shutil.copy(root_dir / "example_files/pulses.bin", python_dir / "test/pulses.bin")


def build_sdist(sdist_directory, config_settings=None):
    _copy_out_of_tree()
    return build_meta.build_sdist(sdist_directory, config_settings)


def build_wheel(wheel_directory, config_settings=None, metadata_directory=None):
    _copy_out_of_tree()
    return build_meta.build_wheel(wheel_directory, config_settings, metadata_directory)


def build_editable(wheel_directory, config_settings=None, metadata_directory=None):
    _copy_out_of_tree()
    return build_meta.build_editable(wheel_directory, config_settings, metadata_directory)
