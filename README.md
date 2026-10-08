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

<!-- dark mode -->
<img src="./screenshots/screenshot-01-dark.png#gh-dark-mode-only" alt="Image Selection">
<!-- light mode -->
<img src="./screenshots/screenshot-01-light.png#gh-light-mode-only" alt="Image Selection">

### Device Selection

<!-- dark mode -->
<img src="./screenshots/screenshot-02-dark.png#gh-dark-mode-only" alt="Device Selection">
<!-- light mode -->
<img src="./screenshots/screenshot-02-light.png#gh-light-mode-only" alt="Device Selection">

The list will also dynamically refresh as devices are added and removed

<!-- dark mode -->
<img src="./screenshots/device-monitoring-dark.gif#gh-dark-mode-only" alt="Device monitoring">
<!-- light mode -->
<img src="./screenshots/device-monitoring-light.gif#gh-light-mode-only" alt="Device monitoring">

### Device Flashing

<!-- dark mode -->
<img src="./screenshots/screenshot-03-dark.png#gh-dark-mode-only" alt="Flashing Devices">
<!-- light mode -->
<img src="./screenshots/screenshot-03-light.png#gh-light-mode-only" alt="Flashing Devices">

### Summary

<!-- dark mode -->
<img src="./screenshots/screenshot-05-dark.png#gh-dark-mode-only" alt="Summary">
<!-- light mode -->
<img src="./screenshots/screenshot-05-light.png#gh-light-mode-only" alt="Summary">

## Translators

Translators are welcome to submit translations directly as a pull request to this project. It is generally expected that your pull requests will contain a single commit for each language that was added or improved, using a syntax like so:

```
i18n(eo): Add Esperanto language support
```

Translation files can be found [here](./i18n/). We are using [Project Fluent](https://projectfluent.org) for our translations, which should be easier than working with gettext.
