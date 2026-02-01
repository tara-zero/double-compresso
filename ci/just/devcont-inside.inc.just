# This is included into justfiles inside of devcontainer.

# GitHub runners replace image's HOME with some nonsense.
export HOME := "/root"

cicli := "python3 -m cicli.main --inside-devcontainer"

_devcontainer-post-start-command:
    rsync \
        --recursive \
        --update \
        --delete \
        --itemize-changes \
        --executability \
        --times \
        "/opt/host-tools/" \
        "{{ src_root_dir }}/.host-tools"
