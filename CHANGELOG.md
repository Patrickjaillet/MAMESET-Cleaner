# Journal des modifications

## v0.8.0
- Le scan détecte maintenant les archives réellement corrompues (et pas seulement les ROMs différentes de celles attendues), en relisant entièrement chaque fichier au lieu de se fier à une simple étiquette.
- Un fichier de référence MAME illisible ou vide affiche désormais un message d'erreur clair au lieu de produire un résultat trompeur.
- Ajout d'un bouton Annuler pendant un scan en cours.
- Après un nettoyage, le logiciel revérifie automatiquement que toutes les ROMs conservées sont bien présentes et intactes, et affiche le résultat de cette vérification.

## v0.7.0
- Ajout du nettoyage réel des ROMs en double détectées : elles sont déplacées vers la corbeille Windows par défaut, ou supprimées définitivement si l'utilisateur le choisit.
- Ajout d'une option de sauvegarde : les ROMs peuvent être copiées dans un dossier de sécurité avant tout nettoyage.
- Une confirmation est désormais toujours demandée avant de nettoyer, avec le nombre de ROMs concernées.
- Chaque nettoyage génère un rapport détaillé (JSON et CSV) listant les ROMs supprimées et conservées avec leur motif, enregistré automatiquement sur l'ordinateur.

## v0.6.0
- Le logiciel dispose maintenant d'une interface graphique complète avec 5 sections : Scan, Filtres, Résultats, Paramètres et À propos.
- La sélection des dossiers et fichiers (ROMs, DAT, catver.ini, languages.ini) se fait désormais par une fenêtre de sélection classique de Windows, depuis les Paramètres.
- Le scan affiche une barre de progression en temps réel et reste utilisable pendant l'analyse.
- Les critères de filtrage peuvent être réglés visuellement et appliqués en un clic.
- Les résultats s'affichent dans un tableau avec recherche et tri par nom ou par année, indiquant pour chaque jeu s'il est à garder ou à supprimer.
- Ajout d'un thème clair et d'un thème sombre, activable dans les Paramètres.

## v0.5.0
- Ajout du filtrage des jeux par genre, langue, région, statut de fonctionnement, fabricant, année et type (BIOS, mécanique, adulte).
- Plusieurs critères peuvent être combinés en même temps (par exemple : un genre précis ET une langue précise).
- Ajout de la sauvegarde et du chargement de profils de filtrage personnalisés, pour retrouver facilement ses réglages préférés.

## v0.4.0
- Ajout de la détection des jeux en double (un jeu principal et ses variantes).
- Pour chaque groupe de doublons, le logiciel choisit automatiquement le meilleur exemplaire à conserver selon la région préférée (par exemple Monde puis USA puis Europe puis Japon), l'état de fonctionnement du jeu, puis sa version la plus récente.
- La région préférée pourra être personnalisée par l'utilisateur.
- Cette étape prépare seulement la liste des jeux à garder et à retirer : aucune suppression de fichier n'a lieu à ce stade.

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
