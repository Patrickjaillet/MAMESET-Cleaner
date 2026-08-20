# MAMESET Cleaner

MAMESET Cleaner is a Windows desktop application that helps clean and organize a MAME ROM set — and, through downloadable plugins, ROM sets for 30 other console systems: detecting and removing duplicates (clones/parents), filtering by genre, language, region, working status, manufacturer and year, to produce a clean, consistent, minimal ROM set (1 game = 1 ROM).

![MAMESET Cleaner Filters view](docs/screenshots/filters_view.png)

## Features

- **Scan** the ROM folder (`.zip`, `.7z` files and uncompressed folders), detecting ROMs that are correct, missing, corrupted, or unreferenced
- **Duplicate detection** (1G1R logic: 1 game = 1 ROM) based on a customizable region priority (e.g. World > USA > Europe > Japan), working status and the most recent revision
- **Filtering** by genre, language, region, working status, manufacturer, year and content type (BIOS, mechanical, adult)
- **Cleanup** of duplicate ROMs: moved to the Windows Recycle Bin by default (or permanently deleted), with an optional backup beforehand and mandatory confirmation before any deletion
- **Detailed report** for every cleanup operation (JSON and CSV)
- **Automatic integrity check** of the set after a cleanup
- **French and English** interface, **light and dark** theme
- **Multi-console plugin system**: beyond its native MAME support, the application can load a plugin to clean ROM sets for other consoles. As of v2.0.0, 30 systems are available for download directly from the in-app "Plugins" section: NES/Famicom, SNES/Super Famicom, Nintendo 64, Game Boy/Game Boy Color, Game Boy Advance, Nintendo DS, GameCube/Wii, Master System, Mega Drive/Genesis, Game Gear, Sega CD, Saturn, Dreamcast, PlayStation, PlayStation 2, PSP, Neo Geo AES/MVS, Neo Geo Pocket, Atari 2600/5200/7800/Lynx/Jaguar, TurboGrafx-16/PC Engine, WonderSwan, 3DO, Commodore 64, Amiga, ZX Spectrum, Amstrad CPC and MSX/MSX2

![MAMESET Cleaner Results view](docs/screenshots/results_view.png)

## Installation

A Windows installer (Inno Setup) is available from version v1.0.0 onward. See the [releases](https://github.com/Patrickjaillet/MAMESET-Cleaner/releases) of this repository.

The application runs on Windows 10 and Windows 11 (64-bit).

## Building from source

See [docs/COMPILATION.md](docs/COMPILATION.md).

## License

This software is distributed under the **MIT** license. See the [LICENSE](LICENSE) file.

## Contact

- Email: sandefjord.development@proton.me
- Website: https://patrickjaillet.github.io/MAMESET-Cleaner
