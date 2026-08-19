# Compilation de MAMESET Cleaner

Ce document explique comment compiler MAMESET Cleaner depuis les sources, sur Windows 10/11 (x64).

## Prérequis

- **Rust** (édition 2021) via [rustup](https://rustup.rs), avec la cible `x86_64-pc-windows-msvc`
- **Visual Studio Build Tools** (composant "Outils de génération C++") — requis par la chaîne de compilation `msvc` de Rust sur Windows
- **Inno Setup 7** — uniquement nécessaire pour générer l'installateur (`installer/inno_setup_script.iss`), pas pour compiler le logiciel lui-même

Vérifier l'installation :

```powershell
rustc --version
cargo --version
```

## Compilation en mode développement

```powershell
cargo build
```

Le binaire est généré dans `target\debug\mameset_cleaner.exe`. En mode développement, les journaux (`tracing`) restent visibles dans le terminal.

## Compilation en mode release (optimisé)

```powershell
cargo build --release
```

Le binaire final est généré dans `target\release\mameset_cleaner.exe`. Ce mode :
- active les optimisations complètes (`opt-level = 3`, LTO, un seul codegen-unit) ;
- retire les symboles de débogage (`strip`) pour réduire la taille du fichier ;
- masque la fenêtre de console au démarrage (aucune console ne s'affiche au double-clic) ;
- intègre l'icône de l'application (`assets/icons/app.ico`) dans l'exécutable.

## Lancer les tests

```powershell
cargo test
```

Inclut les tests unitaires (dans chaque module de `src/`) et les tests d'intégration (`test/*.rs`), soit plus de 60 tests couvrant le parsing, le scan, le dédoublonnage, le filtrage et le nettoyage.

## Génération de l'installateur Windows

1. Compiler le binaire en mode release (`cargo build --release`).
2. Ouvrir `installer/inno_setup_script.iss` avec Inno Setup 7 (ou lancer `ISCC.exe installer\inno_setup_script.iss` en ligne de commande).
3. L'installateur généré (`MAMESET-Cleaner-Setup-vX.Y.Z.exe`) est déposé dans `installer/output/`.

L'installateur cible Windows 10 et Windows 11 (x64), utilise l'icône de l'application, et propose une installation ainsi qu'une désinstallation silencieuses (`/VERYSILENT`).

## Structure du projet

```
MAMESET-Cleaner/
├── src/            Code source Rust (cœur métier + interface)
├── ui/             Interface utilisateur Slint (.slint)
├── assets/         Icônes et fichiers de traduction (i18n)
├── test/           Tests d'intégration
├── installer/      Script Inno Setup 7
└── docs/           Documentation (dont ce fichier)
```
