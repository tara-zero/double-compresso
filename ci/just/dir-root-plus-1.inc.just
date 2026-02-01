# This is included into ${SOURCE_ROOT}/*/ justfiles.

export PYTHONPATH := justfile_directory() + f"{{ slash }}..{{ slash }}ci" + if env("PYTHONPATH", "") == "" { "" } else { pathsep + env("PYTHONPATH") }
src_root_dir := ".."
