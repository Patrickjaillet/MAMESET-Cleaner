# MAMESET Cleaner

MAMESET Cleaner est un logiciel de bureau pour Windows qui aide à nettoyer et organiser un set de ROMs MAME : détection et suppression des doublons (clones/parents), filtrage par genre, langue, région, statut de fonctionnement, fabricant et année, pour obtenir un set de ROMs propre, cohérent et minimal (1 jeu = 1 ROM).

![Vue Filtres de MAMESET Cleaner](docs/screenshots/filters_view.png)

## Fonctionnalités

- **Scan** du dossier de ROMs (fichiers `.zip`, `.7z` et dossiers non compressés), avec détection des ROMs correctes, manquantes, corrompues ou non référencées
- **Détection des doublons** (logique 1G1R : 1 jeu = 1 ROM) selon une priorité régionale personnalisable (ex. Monde > USA > Europe > Japon), le statut de fonctionnement et la version la plus récente
- **Filtrage** par genre, langue, région, statut de fonctionnement, fabricant, année et type de contenu (BIOS, mécanique, adulte)
- **Nettoyage** des ROMs en double : déplacement vers la corbeille Windows par défaut (ou suppression définitive), avec sauvegarde préalable optionnelle et confirmation obligatoire avant toute suppression
- **Rapport détaillé** de chaque nettoyage (JSON et CSV)
- **Vérification automatique** de l'intégrité du set après un nettoyage
- Interface en **français et en anglais**, thème **clair et sombre**

![Vue Résultats de MAMESET Cleaner](docs/screenshots/results_view.png)

## Installation

Un installateur Windows (Inno Setup) est fourni à partir de la version v1.0.0. Voir les [releases](https://github.com/Patrickjaillet/MAMESET-Cleaner/releases) du dépôt.

Le logiciel fonctionne sur Windows 10 et Windows 11 (64 bits).

## Compilation depuis les sources

Voir [docs/COMPILATION.md](docs/COMPILATION.md).

## Licence

Ce logiciel est distribué sous licence **MIT**. Voir le fichier [LICENSE](LICENSE).

## Contact

- E-mail : sandefjord.development@proton.me
- Site web : https://patrickjaillet.github.io/MAMESET-Cleaner
