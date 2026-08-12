<p align="center">
  <img src="https://raw.githubusercontent.com/eltonfabricio10/bigtube/main/assets/banner.png" alt="BigTube Banner" width="100%">
</p>

<p align="center">
  <a href="../README.md">English</a> · <a href="README.pt-BR.md">Português (BR)</a> · <a href="README.es.md">Español</a> · <b>Français</b>
</p>

# 🎬 BigTube

> **Le téléchargeur multimédia ultime pour Linux**

**BigTube** est une application de bureau moderne, rapide et élégante, développée en **Rust** avec **GTK4**, **Libadwaita** et **GStreamer**. Conçu pour celles et ceux qui n'acceptent rien de moins que la perfection lorsqu'ils téléchargent du contenu depuis Internet, BigTube transforme la complexité de `yt-dlp` en un outil intuitif et puissant — un binaire natif et rapide.

---

## 📸 Captures d'écran

#### 🔍 Gestionnaire de recherche
<p align="center">
  <img src="screenshots/01-main.png" alt="BigTube — Gestionnaire de recherche" width="80%">
</p>

#### 🎚️ Sélecteur de format &nbsp;·&nbsp; ⚙️ Paramètres
<p align="center">
  <img src="screenshots/04-formats.png" alt="Sélecteur de qualité vidéo et audio côte à côte" width="48%">
  &nbsp;
  <img src="screenshots/02-settings.png" alt="Paramètres" width="48%">
</p>

#### 🔄 Convertisseur multimédia &nbsp;·&nbsp; 💖 Dons
<p align="center">
  <img src="screenshots/03-converter.png" alt="Convertisseur multimédia intégré" width="48%">
  &nbsp;
  <img src="screenshots/05-donations.png" alt="Fenêtre de dons" width="30%">
</p>

---

## ✨ Fonctionnalités

### 🔍 Recherche et découverte
- **Recherche YouTube intégrée** - Recherchez sans ouvrir de navigateur, avec un filtre de type : **Vidéos**, **En direct**, **Chaînes** ou **Playlists**
- **Recherche YouTube Music native** - Musique uniquement (sans podcasts), via l'API de YouTube Music, filtrée par **Titres**, **Albums**, **Artistes** ou **Playlists** ; les titres arrivent en audio et les clips en vidéo
- **Liens directs** - Prise en charge de plus de 400 sites via URL
- **Ouvrir les conteneurs** - Ouvrez une chaîne, un album, un artiste ou une playlist dans une fenêtre modale listant toutes ses vidéos/pistes, avec **Tout lire**, **Tout télécharger** et un mode de sélection pour ne télécharger que les éléments cochés
- **Playlists par lien** - Collez un lien de playlist YouTube (`playlist?list=` ou `watch?v=...&list=`) et la recherche liste jusqu’aux 500 premières vidéos (limite pour préserver la fluidité des très grandes playlists)
- **Suggestions de recherche** - Historique local et autocomplétion en ligne pendant la saisie, avec navigation complète au clavier (↑/↓ pour se déplacer, Entrée pour choisir, Échap pour fermer)

### ⬇️ Téléchargements avancés
| Fonctionnalité | Description |
|---------|-------------|
| **Qualité vidéo** | 4K (2160p), 2K (1440p), 1080p, 720p, 480p, 360p, 240p, 144p |
| **Formats audio** | MP3, M4A, Opus, FLAC, WAV, AAC avec extraction haute qualité |
| **Métadonnées** | Intégration automatique des tags, de l'album et de l'artiste |
| **Sous-titres** | Intégration et/ou enregistrement comme fichiers sidecar, manuels + auto-générés — choisissez le mode et les langues par vidéo directement dans la boîte de dialogue des formats (par défaut, suit les Paramètres) |
| **Planification** | Mettez des téléchargements en file pour plus tard, ponctuels ou selon un horaire récurrent |
| **SponsorBlock** | Ignore les segments sponsorisés dans la vidéo — marquez-les comme chapitres ou retirez-les du fichier (utilise la base [SponsorBlock](https://sponsor.ajay.app/)) |
| **Concurrence** | Plusieurs téléchargements simultanés avec fragments parallèles configurables |
| **Reprise** | Reprise des téléchargements interrompus |

### 🔄 Convertisseur multimédia
- Conversion vidéo vers vidéo (MP4, MKV, WebM)
- Extraction et conversion audio (MP3, M4A, Opus, FLAC, WAV, AAC)
- Fusion multi-pistes des sous-titres : chaque sidecar à côté du fichier (`video.srt`, `video.en.srt`, `video.pt-BR.vtt`…) est intégré comme piste distincte étiquetée avec sa langue, avec un sélecteur par piste lorsqu'il y en a plusieurs
- File d'attente de conversion par lot
- Progression en temps réel avec estimation du temps restant (ETA)
- Écritures sûres : les conversions vont dans un fichier temporaire caché qui ne prend le nom final qu'à la fin — un plantage ou une annulation ne laisse jamais un « résultat » à moitié écrit, et convertir un fichier vers son propre format demande **Remplacer / Conserver les deux** (l'original n'est remplacé qu'après une conversion réussie)

### 📺 Lecteur intégré
- Moteur de lecture **GStreamer** (natif, intégré à GTK4)
- Aperçu vidéo léger en 360p avant le téléchargement — obtenez la pleine qualité via **Télécharger**
- Navigation dans la playlist (Précédent / Lecture-Pause / **Arrêt** / Suivant), barre de progression (seek) et un curseur de volume qui règle le propre flux de l'application dans le mélangeur du système (PulseAudio/PipeWire) — le **niveau de volume est mémorisé** entre les sessions
- Le bouton haut-parleur ouvre un popover contenant le **muet** à côté du curseur (une seule icône dans la barre, pas de haut-parleurs en double), et son glyphe suit le niveau actuel
- Un bouton de **répétition** alternant entre trois modes : désactivé (s'arrête à la fin de la file), répéter tout (boucle sur la playlist), répéter la piste (rejoue la piste actuelle) — le mode est lui aussi mémorisé
- Fenêtre vidéo détachable, avec ses propres commandes sur la vidéo, y compris le volume ; **Espace** bascule lecture/pause, **F11** bascule le plein écran, **Échap** quitte le plein écran/ferme
- **Favoris** — marquez n'importe quelle piste avec le cœur sur les lignes de résultats et de playlists ; ouvrez la liste des favoris depuis la barre du lecteur pour lire, retirer ou vider les éléments marqués

### ⌨️ Raccourcis clavier
| Raccourci | Action |
|-----------|--------|
| **Ctrl+L** | Focalise le champ de recherche |
| **Ctrl+1…4** | Change entre les onglets principaux |
| **Espace** (fenêtre vidéo) | Lecture / pause |
| **F11** (fenêtre vidéo) | Bascule le plein écran |
| **Échap** (fenêtre vidéo) | Quitte le plein écran, puis ferme |

### 🎨 Personnalisation de l'apparence
| Mode | Description |
|------|-------------|
| **Thème** | Clair / Sombre / Suivre le système |
| **Couleurs** | 16 schémas de couleurs (Default Blue, Modern Violet, Emerald Green, Sunburst Orange, Vibrant Rose, Nordic Cyan, Nordic Snow, Gruvbox Retro, Catppuccin Mocha, Dracula Dark, Tokyo Night, Rosé Pine, Solarized Dark, Monokai Pro, Cyberpunk Neon, BigTube Brand) |
| **Style** | Interface moderne avec effet glassmorphism |

### 📊 Gestion
- Historique des téléchargements
- Historique des conversions
- Historique des recherches
- Téléchargements planifiés
- Favoris
- Option pour effacer automatiquement les données à la fermeture

---

## 🛠️ Technologies

| Technologie | Rôle |
|------------|------|
| **Rust 2021** | Cœur de l'application (binaire natif) |
| **GTK4 + Libadwaita** | Interface native GNOME |
| **GStreamer** | Moteur de lecture |
| **yt-dlp** | Moteur de téléchargement |
| **FFmpeg** | Conversion multimédia |
| **Cargo** | Compilation et gestion des dépendances |

> Le projet est un workspace Cargo composé de trois crates : **`bigtube-core`** (logique/moteur), **`bigtube-cli`** (binaire `bigtube` sans interface) et **`bigtube-gui`** (interface graphique `bigtube-gui`).

---

## 🚀 Installation

### Arch Linux (AUR) — recommandé
Paquet binaire précompilé (`bigtube-bin`) : s'installe rapidement, **sans rien compiler** sur votre machine.
```bash
yay -S bigtube-bin
# ou
paru -S bigtube-bin
```

### Debian / Ubuntu (.deb)
Téléchargez le `.deb` depuis la [dernière version](https://github.com/eltonfabricio10/bigtube/releases/latest) et installez-le (les dépendances sont résolues automatiquement) :
```bash
sudo apt install ./bigtube_*_amd64.deb
```
> Compilé sur Ubuntu 24.04, il nécessite donc **Ubuntu 24.04+** ou **Debian 13+** (GTK ≥ 4.12, libadwaita ≥ 1.5).

### Fedora (.rpm)
Téléchargez le `.rpm` depuis la [dernière version](https://github.com/eltonfabricio10/bigtube/releases/latest) et installez-le :
```bash
sudo dnf install ./bigtube-*.x86_64.rpm
```
> Compilé sur Fedora 40 (nécessite **Fedora 40+**). `ffmpeg` (extraction audio/conversion) se trouve dans [RPM Fusion](https://rpmfusion.org/) — activez-le et lancez `sudo dnf install ffmpeg` pour ces fonctions.

### AppImage (portable, toute distribution)
Téléchargez `BigTube-*-x86_64.AppImage` depuis la [dernière version](https://github.com/eltonfabricio10/bigtube/releases/latest), rendez-le exécutable et lancez-le :
```bash
chmod +x BigTube-*-x86_64.AppImage
./BigTube-*-x86_64.AppImage
```
> Embarque GTK4/libadwaita et les plugins GStreamer (y compris le sink gtk4 du lecteur), donc il fonctionne sur tout système x86_64 quelle que soit la version de GTK de la distribution. `ffmpeg` et `yt-dlp` sont utilisés à l'exécution s'ils sont présents ; l'application télécharge `yt-dlp` dans son propre dossier de données au premier lancement.
>
> **Remarque :** l'AppImage nécessite **glibc ≥ 2.41** (Debian 13+, Ubuntu 25.10+, Fedora 42+, ou une distribution rolling comme Arch/openSUSE Tumbleweed). Sur les systèmes plus anciens, utilisez les paquets `.deb`/`.rpm`/AUR.

### Compilation depuis les sources (Cargo)
Nécessite la chaîne d'outils Rust (`rustup`) et les dépendances système listées ci-dessous.
```bash
# Cloner le dépôt
git clone https://github.com/eltonfabricio10/bigtube.git
cd bigtube/rust

# Compiler en mode release
cargo build --release --locked

# Les binaires se trouvent dans rust/target/release/
./target/release/bigtube-gui      # interface graphique
./target/release/bigtube --help   # mode sans interface (CLI)
```

Pour installer à l'échelle du système depuis une compilation locale :
```bash
sudo install -Dm755 target/release/bigtube-gui /usr/bin/bigtube-gui
sudo install -Dm755 target/release/bigtube     /usr/bin/bigtube
sudo install -Dm644 ../assets/bigtube.svg /usr/share/icons/hicolor/scalable/apps/bigtube.svg
sudo install -Dm644 ../assets/bigtube.png /usr/share/icons/hicolor/512x512/apps/bigtube.png
sudo install -Dm644 packaging/io.github.eltonfabricio10.bigtube.desktop /usr/share/applications/io.github.eltonfabricio10.bigtube.desktop
```

---

## ⌨️ Ligne de commande

BigTube fournit **deux binaires** :

| Binaire | Rôle |
|--------|------|
| `bigtube-gui` | Ouvre l'interface graphique |
| `bigtube` | Mode sans interface (téléchargement directement depuis le terminal, sans interface graphique) |

### Interface graphique
```bash
bigtube-gui      # ouvre la fenêtre BigTube
```

### Mode sans interface (`bigtube`)
```bash
bigtube -d <URL> [options]
```

| Option | Description |
|--------|-------------|
| `-d, --download URL` | Télécharge l'URL directement depuis le terminal, sans ouvrir la fenêtre |
| `-o, --output DIR` | Dossier de destination pour ce `--download` uniquement (par défaut : dossier configuré ; ne modifie pas le réglage de l'interface) |
| `--audio-only` | Avec `--download`, extrait l'audio au format MP3 (prioritaire sur `--format`) |
| `--format FMT` | Avec `--download`, sélecteur de format personnalisé pour `yt-dlp -f` |
| `--ext EXT` | Avec `--format`, le conteneur/l'extension de sortie (par défaut : `mp4`) — utilisez par ex. `m4a`/`opus` pour des sélecteurs audio uniquement |
| `--yt-dlp-version` | Affiche la version de `yt-dlp` incluse |
| `--version` | Affiche la version de BigTube |
| `--help` | Affiche l'aide |

### Exemples
```bash
bigtube-gui                                      # opens the GUI
bigtube -d https://youtube.com/watch?v=...       # headless download
bigtube -d <url> -o ~/Music --audio-only         # headless MP3 audio
bigtube -d <url> --format "bestvideo+bestaudio"  # custom format
bigtube -d <url> --format bestaudio --ext m4a    # audio-only selector, correct extension
```

---

## 📁 Structure des dossiers

| Emplacement | Contenu |
|----------|----------|
| `~/.config/bigtube/` | Paramètres et historiques |
| `~/.config/bigtube/config.json` | Paramètres de l'application |
| `~/.config/bigtube/history.json` | Historique des téléchargements |
| `~/.config/bigtube/search_history.json` | Historique des recherches |
| `~/.config/bigtube/converter_history.json` | Historique des conversions |
| `~/.config/bigtube/scheduled_downloads.json` | Téléchargements planifiés |
| `~/.config/bigtube/favorites.json` | Favoris |
| `~/.local/share/bigtube/bin/` | Binaires inclus (`yt-dlp`, `deno`) |
| `~/.cache/bigtube/thumbnails/` | Cache des miniatures |
| `~/Downloads/BigTube/` | Dossier de téléchargement par défaut |
| `~/Downloads/BigTube/Converted/` | Dossier de sortie du convertisseur par défaut |

---

## ⚙️ Paramètres disponibles

Les préférences sont enregistrées dans `~/.config/bigtube/config.json`. Lorsque le fichier n'existe pas ou est corrompu, BigTube recrée la configuration avec des valeurs par défaut. Les chemins vides ou les options désactivées font simplement revenir l'application à son comportement par défaut.

> La page des paramètres est organisée en groupes dans cet ordre : **Apparence**, **Recherche**, **Téléchargements**, **Performance**, **Post-traitement**, **Sous-titres**, **Convertisseur multimédia**, **Réseau et avancé**, **Système** et **Stockage**. Les préférences du lecteur (volume, mode de répétition) se règlent directement dans la barre du lecteur et sont enregistrées automatiquement.

### Apparence
| Paramètre | Par défaut | Explication |
|---------|---------|-------------|
| **Thème de l'interface** | Suivre le système | Définit si l'interface utilise le thème du système, force un thème clair ou force un thème sombre. |
| **Schéma de couleurs** | Default Blue | Modifie la palette/couleur d'accentuation de l'interface. Options : Default Blue, Modern Violet, Emerald Green, Sunburst Orange, Vibrant Rose, Nordic Cyan, Nordic Snow, Gruvbox Retro, Catppuccin Mocha, Dracula Dark, Tokyo Night, Rosé Pine, Solarized Dark, Monokai Pro, Cyberpunk Neon et BigTube Brand. |

### Recherche
| Paramètre | Par défaut | Explication |
|---------|---------|-------------|
| **Enregistrer l'historique des recherches** | Activé | Stocke vos recherches localement dans `search_history.json`, ce qui vous permet de réutiliser des requêtes précédentes. |
| **Activer les suggestions de recherche** | Activé | Affiche des suggestions au fur et à mesure de la saisie, en utilisant l'historique local des recherches. Naviguez avec ↑/↓, choisissez avec Entrée, fermez avec Échap. |
| **Suggestions en ligne** | Activé | Récupère aussi des complétions d'autocomplétion en ligne pendant la saisie (en plus de l'historique local). |
| **Nombre maximal de suggestions** | 10 | Définit combien de suggestions peuvent apparaître en même temps. Accepte des valeurs de 1 à 50. |
| **Effacer l'historique des recherches** | Action manuelle | Supprime toutes les entrées enregistrées de l'historique des recherches. Ne supprime pas les fichiers téléchargés. |
| **Nombre maximal de résultats de recherche** | 15 | Définit combien de résultats BigTube demande à `yt-dlp` pour les recherches textuelles. Accepte des valeurs de 5 à 100. |

### Téléchargements
| Paramètre | Par défaut | Explication |
|---------|---------|-------------|
| **Dossier de téléchargement** | `~/Downloads/BigTube/` | Définit l'emplacement où les fichiers téléchargés sont enregistrés. L'application crée le dossier si nécessaire. |
| **Qualité préférée** | Demander à chaque fois | Définit le format par défaut pour les nouveaux téléchargements. Peut demander à chaque téléchargement, télécharger la meilleure vidéo, choisir 4K, 2K, 1080p, 720p, 480p, 360p, 240p, 144p, ou télécharger uniquement l'audio au format MP3, M4A, Opus, FLAC, WAV ou AAC. |
| **Enregistrer l'historique des téléchargements** | Activé | Conserve un enregistrement local des téléchargements dans `history.json`, utilisé par la vue historique/liste. |
| **Nombre maximal d'entrées d'historique** | 100 | Combien d'entrées de téléchargement conserver dans la liste. Accepte des valeurs de 10 à 1000. |
| **Supprimer une fois terminé** | Désactivé | Supprime automatiquement de la liste les téléchargements terminés. |
| **Supprimer si annulé** | Désactivé | Supprime automatiquement de la liste les téléchargements annulés. |

#### Options de qualité
| Option | Explication |
|--------|-------------|
| **Demander à chaque fois** | Affiche le choix de qualité/format au moment du téléchargement. |
| **Meilleure qualité (MKV)** | Télécharge la meilleure combinaison vidéo et audio disponible et fusionne le résultat. |
| **4K, 2K, 1080p, 720p, 480p, 360p, 240p, 144p** | Privilégie la vidéo MP4/AVC à la résolution choisie avec l'audio M4A ; si ce format exact n'existe pas, `yt-dlp` utilise la meilleure alternative compatible définie dans le préréglage. |
| **Audio (MP3)** | Extrait uniquement l'audio, le convertit en MP3 haute qualité et tente d'intégrer la miniature. |
| **Audio (M4A)** | Télécharge uniquement l'audio, en privilégiant le codec/conteneur M4A. |
| **Audio (Opus / FLAC / WAV / AAC)** | Extrait uniquement l'audio et le convertit au format choisi à la plus haute qualité. |

### Performance
| Paramètre | Par défaut | Explication |
|---------|---------|-------------|
| **Téléchargements simultanés** | 3 | Contrôle combien de vidéos peuvent être téléchargées en même temps. Accepte des valeurs de 1 à 10. |
| **Fragments simultanés** | 16 | Définit combien de fragments parallèles `yt-dlp` utilise par téléchargement. Accepte des valeurs de 1 à 16. Des valeurs plus élevées peuvent accélérer les téléchargements segmentés mais augmentent aussi l'utilisation du réseau. |
| **Limite de vitesse** | 0 Ko/s | Limite la vitesse de téléchargement en Ko/s. `0` signifie aucune limite. Accepte des valeurs de 0 à 100000. |
| **Accélérer avec aria2c** | Activé | Utilise `aria2c` comme téléchargeur externe de `yt-dlp` pour des téléchargements multi-connexions plus rapides et reprenables. Ne s'active que si `aria2c` est installé ; sinon, le téléchargement est normal. |

### Post-traitement
| Paramètre | Par défaut | Explication |
|---------|---------|-------------|
| **Ajouter des métadonnées** | Désactivé | Tente d'intégrer l'artiste, l'album, la pochette et d'autres métadonnées dans les fichiers téléchargés. Nécessite `ffmpeg` ; s'il n'est pas installé, l'application ignore cette étape. |
| **SponsorBlock** | Désactivé | Ignore les segments sponsorisés dans la vidéo via la base SponsorBlock. « Marquer les chapitres » ajoute des repères (non destructif) ; « Retirer les segments » les coupe du fichier. Nécessite `ffmpeg`. |
| **Commande de post-traitement** | Vide | Exécute une commande après le téléchargement à l'aide de `yt-dlp --exec`. Utilisez `{}` dans la commande pour représenter le fichier téléchargé. |

### Sous-titres
| Paramètre | Par défaut | Explication |
|---------|---------|-------------|
| **Sous-titres** | Désactivé | Gestion des sous-titres pour les téléchargements : `Off`, `Embed` dans le fichier, enregistrer comme `File` séparé (sidecar), ou `Both`. L'intégration nécessite `ffmpeg`. |
| **Langues** | `en,pt,es` | Liste de codes de langue de sous-titres à récupérer, séparés par des virgules (par ex. `en,pt,es`). |
| **Inclure les auto-générés** | Activé | Récupère aussi les sous-titres générés automatiquement (par machine), pas seulement les manuels. |

### Convertisseur multimédia
| Paramètre | Par défaut | Explication |
|---------|---------|-------------|
| **Enregistrer dans le dossier source** | Désactivé | Lorsqu'il est activé, le fichier converti est enregistré à côté du fichier d'origine. |
| **Dossier de sortie par défaut** | `~/Downloads/BigTube/Converted/` | Définit le dossier utilisé par le convertisseur lorsque « enregistrer dans le dossier source » est désactivé. |
| **Enregistrer l'historique des conversions** | Activé | Conserve un enregistrement local des conversions dans `converter_history.json`. |
| **Supprimer une fois terminé** | Désactivé | Supprime automatiquement de la liste les conversions terminées. |
| **Supprimer si annulé** | Désactivé | Supprime automatiquement de la liste les conversions annulées. |
| **Nombre maximal d'entrées d'historique** | 50 | Combien d'entrées de conversion conserver dans la liste. Accepte des valeurs de 10 à 500. |

### Réseau et avancé
| Paramètre | Par défaut | Explication |
|---------|---------|-------------|
| **Fichier de cookies** | Vide | Utilise un fichier `cookies.txt` au format Netscape avec `yt-dlp --cookies`, utile pour le contenu nécessitant une session authentifiée. |
| **Cookies du navigateur** | Aucun | Importe les cookies directement depuis un navigateur détecté, tel que Firefox, Chrome, Chromium, Brave, Microsoft Edge, Vivaldi ou Opera, à l'aide de `yt-dlp --cookies-from-browser`. |
| **User-Agent** | Valeur BigTube par défaut | Remplace le User-Agent envoyé à `yt-dlp`. S'il est laissé vide, l'application utilise un User-Agent sûr basé sur Chrome. Inclut des préréglages pour les navigateurs détectés. |
| **Proxy** | Vide | Achemine les recherches, les métadonnées, le lecteur et les téléchargements via le proxy indiqué. Accepte les URL `http`, `https`, `socks4`, `socks4a`, `socks5` et `socks5h`, par exemple `socks5://127.0.0.1:1080`. |

### Système
| Paramètre | Par défaut | Explication |
|---------|---------|-------------|
| **Version actuelle / mise à jour des composants** | Automatique | Affiche la version locale de `yt-dlp` et permet de mettre à jour les composants téléchargés par l'application, tels que `yt-dlp` et `deno`, dans `~/.local/share/bigtube/bin/`. La mise à jour s'exécute dans une fenêtre de progression avec une barre de téléchargement en direct. |
| **Vérifier les mises à jour au démarrage** | Activé | Vérifie s'il existe des composants `yt-dlp`/`deno` plus récents au lancement de l'application. Lorsqu'une mise à jour est disponible, la notification comporte un bouton **Mettre à jour** qui ouvre immédiatement la fenêtre de progression. |
| **Surveillance du presse-papiers** | Désactivé | Détecte automatiquement les liens vidéo copiés dans le presse-papiers lorsque l'application est ouverte. |
| **Notifications système** | Activé | Contrôle les notifications système pour les événements de téléchargement et les erreurs. |

### Stockage et confidentialité
| Paramètre | Par défaut | Explication |
|---------|---------|-------------|
| **Effacer les données à la fermeture** | Désactivé | À la fermeture de l'application, efface les historiques de téléchargement, de recherche et de conversion. Les paramètres de l'application sont conservés. Lorsqu'il est activé, les options « enregistrer l'historique » sont désactivées dans l'interface. |
| **Exporter la sauvegarde** | Action manuelle | Enregistre une sauvegarde complète — les paramètres ainsi que les historiques de téléchargement, de recherche et de conversion, les téléchargements programmés, le cache des playlists et les favoris — dans un seul fichier JSON. |
| **Importer la sauvegarde** | Action manuelle | Restaure tous les paramètres et données à partir d'un fichier de sauvegarde valide. |
| **Effacer toutes les données de l'application** | Action manuelle | Supprime définitivement tous les fichiers de données (paramètres, historiques de téléchargement/recherche/conversion, téléchargements planifiés, favoris, cache des playlists et file d'attente en cours du convertisseur), recrée la configuration par défaut et redémarre l'application. |

### Clés de `config.json`
| Clé | Valeur par défaut | Utilisée par |
|-----|---------------|---------|
| `download_path` | `~/Downloads/BigTube/` | Dossier de téléchargement |
| `theme_mode` | `system` | Thème de l'interface |
| `theme_color` | `default` | Schéma de couleurs |
| `default_quality` | `ask` | Qualité préférée |
| `max_concurrent_downloads` | `3` | Téléchargements simultanés |
| `max_download_history` | `100` | Max d’éléments dans la liste des téléchargements |
| `max_converter_history` | `50` | Max d’éléments dans la liste du convertisseur |
| `add_metadata` | `false` | Métadonnées sur les téléchargements |
| `embed_subtitles` | `false` | Indicateur de sous-titres hérité (migré vers `subtitle_mode`) |
| `subtitle_mode` | `off` | Gestion des sous-titres : `off`, `embed`, `file`, `both` |
| `subtitle_langs` | `en,pt,es` | Langues de sous-titres à récupérer |
| `subtitle_auto` | `true` | Inclure les sous-titres auto-générés |
| `save_history` | `true` | Historique des téléchargements |
| `save_search_history` | `true` | Historique des recherches |
| `enable_suggestions` | `true` | Suggestions de recherche |
| `online_suggestions` | `true` | Suggestions d'autocomplétion en ligne |
| `max_suggestions` | `10` | Nombre de suggestions |
| `search_limit` | `15` | Nombre de résultats de recherche |
| `save_converter_history` | `true` | Historique du convertisseur |
| `auto_clear_finished` | `false` | Effacer les historiques à la fermeture |
| `converter_path` | `~/Downloads/BigTube/Converted/` | Dossier de sortie du convertisseur |
| `use_source_folder` | `false` | Le convertisseur enregistre dans le dossier source |
| `monitor_clipboard` | `false` | Surveillance du presse-papiers |
| `concurrent_fragments` | `16` | Fragments parallèles par téléchargement |
| `rate_limit` | `0` | Limite de vitesse en Ko/s |
| `use_aria2c` | `true` | Utilise `aria2c` comme téléchargeur externe lorsqu'il est installé |
| `system_notifications` | `true` | Notifications système |
| `post_process_cmd` | `""` | Commande après téléchargement |
| `cookies_file` | `""` | Fichier de cookies |
| `cookies_browser` | `""` | Cookies du navigateur |
| `user_agent` | `""` | User-Agent personnalisé |
| `proxy` | `""` | Proxy |
| `sponsorblock_mode` | `off` | SponsorBlock : `off`, `mark`, `remove` |
| `sponsorblock_cats` | `sponsor,selfpromo,interaction` | Catégories SponsorBlock à traiter |
| `remove_on_complete` | `false` | Supprimer de la liste les téléchargements terminés |
| `remove_on_cancel` | `false` | Supprimer de la liste les téléchargements annulés |
| `converter_remove_on_complete` | `false` | Supprimer de la liste les conversions terminées |
| `converter_remove_on_cancel` | `false` | Supprimer de la liste les conversions annulées |
| `check_updates_on_startup` | `true` | Vérifier les mises à jour `yt-dlp`/`deno` au démarrage |
| `player_volume` | `1.0` | Volume du lecteur (0.0–1.0), persisté depuis la barre du lecteur |
| `player_repeat` | `off` | Mode de répétition du lecteur : `off`, `all`, `one` |

> Compatibilité : les anciennes configurations comportant la clé `download_subtitles` sont automatiquement migrées vers `embed_subtitles`.

### Variables d’environnement
| Variable | Effet |
|----------|-------|
| `BIGTUBE_FULL_REDRAW=1` | Force GSK à redessiner toute la fenêtre à chaque image. Par défaut, BigTube utilise un redessin ciblé et léger au défilement pour éviter les « fantômes » (texte/vignettes figés) sur certaines combinaisons GTK4/Mesa/KWin ; n’activez cette variable que si des artefacts de défilement persistent, au prix d’une consommation CPU/batterie accrue. |
| `GSK_RENDERER` | Variable GTK standard pour choisir le moteur de rendu (`gl`, `vulkan`, `cairo`, …) ; respectée telle quelle. |

---

## 📋 Dépendances système

Exécution (requises pour lancer le binaire) :

```bash
# Arch Linux
sudo pacman -S gtk4 libadwaita gstreamer gst-plugins-base gst-plugins-good \
               gst-plugins-bad gst-plugin-gtk4 gst-plugin-va yt-dlp
# optional: ffmpeg (audio extraction and media conversion),
#           aria2 (faster multi-connection downloads)
sudo pacman -S ffmpeg aria2

# Ubuntu/Debian (22.04+)
sudo apt install libgtk-4-1 libadwaita-1-0 \
                 gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
                 gstreamer1.0-plugins-bad gstreamer1.0-gtk4 \
                 gstreamer1.0-vaapi va-driver-all yt-dlp ffmpeg aria2

# Fedora
sudo dnf install gtk4 libadwaita gstreamer1-plugins-base \
                 gstreamer1-plugins-good gstreamer1-plugins-bad-free \
                 gstreamer1-vaapi yt-dlp ffmpeg aria2
```

> Le lecteur intégré est un **aperçu léger en 360p** (flux progressif, très
> stable) pour vérifier une vidéo avant de la télécharger — pour la pleine
> qualité (jusqu'à 4K), utilisez **Télécharger**, qui récupère et fusionne les
> flux haute résolution dans un fichier propre. Le **décodage vidéo matériel**
> (`gst-plugin-va` / `gstreamer1.0-vaapi` + un pilote VA-API comme
> `intel-media-driver`) garde la lecture légère pour le CPU ; les paquets des
> distributions ci-dessus l'installent automatiquement.

Pour **compiler depuis les sources**, ajoutez la chaîne d'outils Rust et les en-têtes de développement :

```bash
# Arch Linux
sudo pacman -S rustup gtk4 libadwaita gstreamer base-devel
rustup default stable
```

---

## 🤝 Contribuer

Les contributions sont les bienvenues ! N'hésitez pas à :

1. Ouvrir une **Issue** pour signaler des bugs ou suggérer des fonctionnalités
2. Soumettre une **Pull Request** avec des améliorations
3. Aider aux traductions

---

## 💖 Soutenir le projet

Si **BigTube** vous est utile, envisagez de soutenir son développement. Toute aide est la bienvenue ! ❤️

[![GitHub Sponsors](https://img.shields.io/badge/GitHub-Sponsors-EA4AAA?logo=githubsponsors&logoColor=white)](https://github.com/sponsors/eltonfabricio10)

**PIX** (clé aléatoire, pour les dons depuis le Brésil) :

```
a30c24f3-490f-424b-93d3-f1181380bc30
```

> Astuce : vous pouvez aussi retrouver ces options dans l'application, sous **Menu → Dons** (avec un QR code PIX et « Copier-Coller »).

---

## 📄 Licence

Ce projet est sous licence **MIT**. Consultez le fichier [LICENSE](LICENSE) pour plus de détails.

---

<p align="center">
  Réalisé avec ❤️ par <a href="https://github.com/eltonfabricio10">eltonff</a>
</p>
