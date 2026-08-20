# Changelog

## v3.5.0
- The Filters view's genre, language, region and manufacturer fields are no longer free-text boxes you have to type exact values into — they're now checklists populated with the actual values found in your loaded ROM set, so you can just pick what you want.

## v3.4.0
- Redesigned the About screen: it now shows the app icon, clickable links to the project website, GitHub releases, and contact email, plus a short credits section for the open cataloging projects the app relies on (No-Intro, TOSEC, Redump, MAME).

## v3.3.0
- The sidebar now shows a smoothly animated accent indicator that slides to the active section, and a subtle divider separates the core workflow (Scan, Filters, Results, Settings) from Plugins and About.
- Fixed the Results table so its columns resize to fit the window instead of overflowing off the edge when the window is small.
- The Scan and Results views now show a proper designed placeholder ("Aucune analyse effectuée" / "Aucun résultat pour l'instant") instead of a bare sentence before you've run a scan.

## v3.2.0
- Added smooth animations throughout the interface: buttons, sidebar items, toggles and choice controls now transition instead of snapping instantly; views fade in when switching between sections; progress bars (scanning, plugin downloads) animate smoothly instead of jumping; text fields highlight more clearly when focused; cards and dialogs now have subtle shadows for better visual depth.

## v3.1.1
- Fixed a bug where scanning would fail with a confusing error if the active system (selected in Settings) pointed to a plugin that was no longer installed. The application now automatically falls back to MAME in that case and explains why, instead of getting stuck.

## v3.1.0
- Removed the dark theme in favor of a single, more consistent visual design (the dark theme had grown inconsistent over time, with several elements not actually changing color when toggled).
- Laid the groundwork for further visual improvements to the interface, coming in upcoming releases.

## v3.0.0
- First stable release with full multi-console catalog coverage: 87 systems, spanning every major Nintendo, Sega, Sony, SNK and Atari console plus dozens of classic 8/16-bit computers and handhelds, all available for download from the "Plugins" section.
- The project website now lists every supported system, grouped by manufacturer.

## v2.9.0
- Added support for 15 more systems — Apple II, Apple IIGS, Atari ST, Sharp X1, Sharp X68000, Fujitsu FM-7, Acorn BBC Micro, Acorn Electron, Acorn Archimedes, Oric-1/Atmos, Dragon 32/64, SAM Coupé, Enterprise 128, NEC PC-88 and NEC PC-98 — available for download from the "Plugins" section.
- The "Plugins" section now groups systems by manufacturer and has a search box, and the "Système actif" selector in Settings does the same, now that the catalog has grown to 87 systems.

## v2.6.0
- Added support for 20 more systems — Neo Geo CD, PC-FX, TurboGrafx-CD/PC Engine CD, CD-i, Amiga CD32, CDTV, Jaguar CD, FM Towns, Playdia, Pippin, PlayStation 3, Xbox, Xbox 360, Wii U, Atari 8-bit, Commodore VIC-20, Commodore 128, Commodore Plus/4, Sinclair QL and Sord M5 — available for download from the "Plugins" section.

## v2.3.0
- Added support for 18 more systems — ColecoVision, Mattel Intellivision, GCE Vectrex, Magnavox Odyssey²/Videopac, Fairchild Channel F, Bally Astrocade, Emerson Arcadia 2001, Interton VC 4000, RCA Studio II, Watara Supervision, Tiger Game.com, Nokia N-Gage, Tapwave Zodiac, VTech CreatiVision, VTech V.Smile, APF MP1000, Casio PV-1000 and Casio Loopy — available for download from the "Plugins" section.

## v2.1.0
- Added support for four more systems — Virtual Boy, Pokémon Mini, SG-1000 and Sega Pico — available for download from the "Plugins" section.

## v2.0.0
- First stable release with multi-console support: 30 systems — every NES/Famicom through Commodore 64/Amiga/ZX Spectrum/Amstrad CPC/MSX plugin added since v1.3.0 — are now actually available for download from the "Plugins" section, alongside the built-in MAME support.
- Fixed a bug where, after installing and using one console plugin, switching to a different one in the same session could fail to actually switch to it.

## v1.8.0
- Strengthened the plugin system: a plugin can no longer claim another system's identity (including the built-in MAME system), and a plugin that encounters an internal error while reading a reference database now reports it cleanly instead of being able to affect the rest of the application.

## v1.7.0
- Added support for five classic computers — Commodore 64, Commodore Amiga, ZX Spectrum, Amstrad CPC and MSX/MSX2 — each installable from the "Plugins" section once published to the repository.

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
