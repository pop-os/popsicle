# Popsicle

Popsicle is a Linux utility for flashing multiple USB devices in parallel, written in [Rust](https://www.rust-lang.org/).

## Build Dependencies

Building the COSMIC front end requires the D-Bus development files, usually named `libdbus-1-dev`, in addition to Rust's `cargo` utility.

For those who need to vendor Cargo's crate dependencies which are fetched from [Crates.io](https://crates.io/), you will need to install [cargo-vendor](https://github.com/alexcrichton/cargo-vendor), and then run `just vendor`.

## Installation Instructions

 A [justfile](https://github.com/casey/just) is included for simply building and installing all required files into the system. You may either build both the CLI and COSMIC UI workspace, just the CLI workspace, or just the UI workspace.

- `just cli && sudo just install-cli` will build and install just the CLI workspace
- `just ui && sudo just install-ui` will build and install just the COSMIC UI workspace
- `just && sudo just install` will build and install both the CLI and COSMIC UI workspaces

## Screenshots

### Image Selection

![Image Selection](./screenshots/screenshot-01.png)

### Device Selection

![Device Selection](./screenshots/screenshot-02.png)

The list will also dynamically refresh as devices are added and removed

![GIF Demo](./screenshots/device-monitoring.gif)

### Device Flashing

![Flashing Devices](./screenshots/screenshot-03.png)
![Flashing Devices](./screenshots/screenshot-04.png)

### Summary

![Summary](./screenshots/screenshot-05.png)

## Translators

Translators are welcome to submit translations directly as a pull request to this project. It is generally expected that your pull requests will contain a single commit for each language that was added or improved, using a syntax like so:

```
i18n(eo): Add Esperanto language support
```

Translation files can be found [here](./i18n/). We are using [Project Fluent](https://projectfluent.org) for our translations, which should be easier than working with gettext.
