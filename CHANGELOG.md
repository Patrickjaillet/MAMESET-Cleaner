# Changelog

## v1.6.0
- Added support for nine more systems — Neo Geo AES/MVS, Neo Geo Pocket/Pocket Color, Atari 2600, Atari 5200/7800, Atari Lynx, Atari Jaguar, TurboGrafx-16/PC Engine, WonderSwan/WonderSwan Color and 3DO — each installable from the "Plugins" section once published to the repository.

## v1.5.0
- Added support for three Sony systems — PlayStation, PlayStation 2 and PSP — each installable from the "Plugins" section once published to the repository.

## v1.4.0
- Added support for six Sega systems — Master System, Mega Drive/Genesis, Game Gear, Sega CD/Mega-CD, Sega Saturn and Dreamcast — each installable from the "Plugins" section once published to the repository.

## v1.3.0
- Added support for seven Nintendo systems — NES/Famicom, SNES/Super Famicom, Nintendo 64, Game Boy/Game Boy Color, Game Boy Advance, Nintendo DS and GameCube/Wii — each installable from the "Plugins" section once published to the repository.
- Added a "Système actif" (active system) selector in Settings, to switch between MAME and any installed console plugin before scanning.

## v1.2.0
- Added a new "Plugins" section to the interface, where support for other consoles can be browsed, installed, updated and removed directly from the project's GitHub repository.
- Every downloaded plugin is verified against its expected checksum before being installed, so a corrupted or tampered download is automatically rejected.
- Downloads show a real-time progress bar without freezing the interface.

## v1.1.0
- Laid the internal groundwork for supporting consoles beyond MAME through downloadable plugins. This release has no new visible feature on its own; the "Plugins" section introduced in v1.2.0 is what makes it usable.

## v1.0.0
- First stable release of MAMESET Cleaner.
- The software has been fully tested (more than 60 automated checks) and reviewed to leave no known issues.
- Complete documentation for users (installation guide, feature overview, screenshots) and for developers who want to build the software themselves.

## v0.9.0
- Added an icon for the application and its installer.
- Fixed an important bug: a black console window used to appear at startup alongside the main window; this no longer happens.
- Prepared the Windows installer (finalized once Inno Setup was available).
- Added a build guide for developers (`docs/COMPILATION.md`).

## v0.8.0
- The scan now detects archives that are actually corrupted (not just ROMs that differ from what was expected), by fully re-reading each file instead of trusting a simple label.
- An unreadable or empty MAME reference file now shows a clear error message instead of producing a misleading result.
- Added a Cancel button during an ongoing scan.
- After a cleanup, the software automatically re-checks that all kept ROMs are still present and intact, and displays the result of this check.

## v0.7.0
- Added real cleanup of detected duplicate ROMs: they are moved to the Windows Recycle Bin by default, or permanently deleted if the user chooses to.
- Added a backup option: ROMs can be copied to a safety folder before any cleanup.
- A confirmation is now always required before cleaning up, showing the number of ROMs affected.
- Every cleanup generates a detailed report (JSON and CSV) listing deleted and kept ROMs with their reason, automatically saved on the computer.

## v0.6.0
- The software now has a complete graphical interface with 5 sections: Scan, Filters, Results, Settings and About.
- Selecting folders and files (ROMs, DAT, catver.ini, languages.ini) is now done through a standard Windows file picker, from Settings.
- The scan shows a real-time progress bar and remains usable during the analysis.
- Filter criteria can be set visually and applied with one click.
- Results are shown in a table with search and sorting by name or year, indicating for each game whether it should be kept or removed.
- Added a light theme and a dark theme, selectable in Settings.

## v0.5.0
- Added filtering of games by genre, language, region, working status, manufacturer, year and type (BIOS, mechanical, adult).
- Several criteria can be combined at the same time (for example: a specific genre AND a specific language).
- Added saving and loading of custom filter profiles, to easily retrieve your preferred settings.

## v0.4.0
- Added detection of duplicate games (a main game and its variants).
- For each group of duplicates, the software automatically picks the best copy to keep based on the preferred region (for example World, then USA, then Europe, then Japan), the game's working status, then its most recent revision.
- The preferred region can be customized by the user.
- This step only prepares the list of games to keep and remove: no file is deleted at this stage.

## v0.3.0
- Added scanning of the user's ROM folder (`.zip`, `.7z` files and uncompressed folders), including subfolders.
- The software now compares every ROM found against the MAME reference list and indicates whether it is correct, corrupted, missing, or unrecognized.
- The scan runs in parallel across several processor cores to stay fast even on very large ROM sets.

## v0.2.0
- Added reading of MAME reference files: the official game list (DAT), game categories, and game languages.
- The software can now automatically recognize the name, description, year, manufacturer, status (main game or variant), working status, genre and language of each game.
- Added a check for the availability of a newer version of the MAME reference file.

## v0.1.0
- First version of MAMESET Cleaner.
- Added the main window with a navigation menu (Scan, Filters, Results, Settings, About).
- Added support for French and English in the interface.
- Added saving of preferences (ROM folder path, DAT file, category and language files).
