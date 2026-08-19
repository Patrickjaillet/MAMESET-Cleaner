# Journal des modifications

## v0.3.0
- Ajout de l'analyse du dossier de ROMs de l'utilisateur (fichiers `.zip`, `.7z` et dossiers non compressés), y compris dans les sous-dossiers.
- Le logiciel compare désormais chaque ROM trouvée avec la liste de référence MAME et indique si elle est correcte, corrompue, manquante ou non reconnue.
- L'analyse s'exécute en parallèle sur plusieurs cœurs du processeur pour rester rapide même sur de très gros sets de ROMs.

## v0.2.0
- Ajout de la lecture des fichiers de référence MAME : liste officielle des jeux (DAT), catégories de jeux et langues des jeux.
- Le logiciel peut désormais reconnaître automatiquement le nom, la description, l'année, le fabricant, le statut (jeu principal ou variante), l'état de fonctionnement, le genre et la langue de chaque jeu.
- Ajout de la vérification de la disponibilité d'une nouvelle version du fichier de référence MAME.

## v0.1.0
- Première mise en place du logiciel MAMESET Cleaner.
- Ajout de la fenêtre principale avec un menu de navigation (Scan, Filtres, Résultats, Paramètres, À propos).
- Ajout de la prise en charge du français et de l'anglais dans l'interface.
- Ajout de l'enregistrement des préférences (chemins des ROMs, du fichier DAT, des fichiers de catégories et de langues).
