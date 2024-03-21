# installer

[![Build](https://github.com/dfint/installer/actions/workflows/build.yml/badge.svg)](https://github.com/dfint/installer/actions/workflows/build.yml)
[![Total downloads of all releases](https://img.shields.io/github/downloads/dfint/installer/total)](https://github.com/dfint/installer/releases)
[![Downloads of the latest release](https://img.shields.io/github/downloads/dfint/installer/latest/total)](https://github.com/dfint/installer/releases/latest)

Localization installer and updater. Installs localization for the chosen language, installs and checks updates of the hook (new [df-steam-hook-rs](https://github.com/dfint/df-steam-hook-rs)), its configs, translations, fonts.

## Usage

- Download a package from [releases](https://github.com/dfint/installer/releases/latest): "win" for Windows or "lin" for Linux
- Unpack...
  - ...to any directory, In this case you'll need to choose the Dwarf Fortress executable in the dialog after you run `dfrus-installer`
  - ...to the DF directory, in this case the installer will find the DF executable automatically
- Run `dfrus-installer`
- Choose localization language
- Press "Update" button
- Run the game in the usual way (run the game's executable or from the Steam client)
