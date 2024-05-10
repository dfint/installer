# installer

[![Build](https://github.com/dfint/installer/actions/workflows/build.yml/badge.svg)](https://github.com/dfint/installer/actions/workflows/build.yml)
[![Total downloads of all releases](https://img.shields.io/github/downloads/dfint/installer/total)](https://github.com/dfint/installer/releases)
[![Downloads of the latest release](https://img.shields.io/github/downloads/dfint/installer/latest/total)](https://github.com/dfint/installer/releases/latest)

Localization installer and updater for Dwarf Fortress. Installs localization for the chosen language, installs and checks updates of the hook (new [df-steam-hook-rs](https://github.com/dfint/df-steam-hook-rs)), its configs, translations, fonts.

## Usage

- Download a package from [releases](https://github.com/dfint/installer/releases/latest): "win" for Windows or "lin" for Linux
- Unpack...
  - ...to any directory, In this case you'll need to choose the Dwarf Fortress executable in the dialog after you run `dfint-installer`
  - ...to the DF directory, in this case the installer will find the DF executable automatically
- Run `dfint-installer`
- Choose localization language
- Press "Update" button
- Run the game in the usual way (run the game's executable or from the Steam client)

## DFHack

Starting from 0.2.0 version, the installer is compatible with DFHack ([50.13-r2](https://github.com/DFHack/dfhack/releases/tag/50.13-r2) and newer).

## Adding languages for Dwarf Fortress

If your language is missing in the dictionary drop-down list of the installer, please create an [issue](https://github.com/dfint/installer/issues).

It is desirable that there is at least a translation of the title menu and some other initial screens. You can participate in the translation of the game here:

[![Translate_Dwarf_Fortress](https://img.shields.io/badge/Translate_Dwarf_Fortress-blue?style=for-the-badge&logo=transifex)](https://app.transifex.com/dwarf-fortress-translation/dwarf-fortress-steam)

To add missing translations, you can join the localization project here: [translation of Dwarf Fortress 50.* on transifex.com](https://app.transifex.com/dwarf-fortress-translation/dwarf-fortress-steam/dashboard/)

## Localization of the installer

You can help with translation of the installer to your language: [translation of the installer on transifex.com](https://app.transifex.com/dwarf-fortress-translation/installer-3/)

There are only about 40 strings to translate. Once the translation to a language is finished, it can be added to the installer and will be available in its future releases.
