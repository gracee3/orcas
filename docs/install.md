# Installation

## Overview

TT can be installed from a release tarball, a Debian package, or a source build. The Linux-first layout uses local executables plus XDG-backed config, data, and runtime directories.

The build produces three executables:

1. `tt` for the operator CLI.
2. `ttd` for the daemon service.

## Install From Release Tarball

Download the release tarball for your platform, then extract it and run the binaries from the unpacked tree.

```bash
tar -xzf tt-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
cd tt-v0.1.0-x86_64-unknown-linux-gnu
./bin/tt doctor
./bin/ttd
```

To make the binaries available on your `PATH`, install them into a bin directory.

```bash
mkdir -p ~/.local/bin
install -m 0755 bin/tt ~/.local/bin/tt
install -m 0755 bin/ttd ~/.local/bin/ttd
```

For a system-wide install, use `/usr/local/bin` instead of `~/.local/bin`.

```bash
sudo install -m 0755 bin/tt /usr/local/bin/tt
sudo install -m 0755 bin/ttd /usr/local/bin/ttd
```

## Install Via `.deb`

Install the package with `dpkg -i`.

```bash
sudo dpkg -i ./tt_0.1.0_amd64.deb
```

The package installs the executables into `/usr/bin`, the daemon user unit as `tt-daemon.service`, and package documentation under `/usr/share/doc/tt`.

After installation, manage the daemon with the user service manager so it shares the same XDG paths as the CLI.

```bash
systemctl --user enable --now tt-daemon.service
systemctl --user status tt-daemon.service
```

## Build From Source

Install Rust with `rustup` and build from the repository root.

```bash
rustup toolchain install stable
rustup default stable
make build
```

Install the binaries into your preferred prefix.

```bash
sudo make install
make install-user
```

The default source build target is `x86_64-unknown-linux-gnu`. Override `TARGET` if you are cross-building.

## Systemd Setup

Install the user unit file, reload the user manager, and enable the daemon.

```bash
make install-systemd
systemctl --user daemon-reload
systemctl --user enable --now tt-daemon.service
systemctl --user status tt-daemon.service
```

`make install-systemd` rewrites `ExecStart` to the current `PREFIX`/`BINDIR` choice. If you copy the checked-in unit manually instead of using the Makefile target, update the `ExecStart` path before enabling it.

## Uninstall

Remove locally installed binaries and the user unit file, then reload the user manager.

```bash
sudo make uninstall
make uninstall-systemd
systemctl --user daemon-reload
```

If you installed to `~/.local/bin`, remove the files directly.

```bash
rm -f ~/.local/bin/tt
rm -f ~/.local/bin/ttd
```

If you installed system-wide without the Makefile targets, remove the binaries from `/usr/local/bin` and delete `tt-daemon.service` from the user systemd unit directory in use on your host.
