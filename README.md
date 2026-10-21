# Chronos

Prototype local d'un générateur de chronologie déterministe pour un petit monde : lieux, routes, royaumes, organisations, personnes nommées et événements datés avec leurs effets, explorables dans un atelier web (carte + frise + inspecteur).

- Conception : [étude multiscalaire](conception_generateur_histoire_multiechelle_v3.md), [roadmap](Roadmap.md).
- Méthode implémentée : [docs/methods/chronology.md](docs/methods/chronology.md).
- Données importées : [docs/data-sources.md](docs/data-sources.md) (préparation dans `tools/`, fixture `fixtures/stack.json`).
- Ressources à fournir par Neural Earth : [socle initial de 35 couches](NeuralEarth-Resources-Starter.md) ; [catalogue détaillé](NeuralEarth-Resources.md) conservé pour les raffinements ultérieurs. Spécifications distinctes du prototype livré.
- Univers : [catalogue fantasy et patrons steampunk, cyberpunk et Waterworld](NeuralEarth-Resources-Fantasy.md), extension du catalogue détaillé.
- Interface : [docs/interface.md](docs/interface.md).

## Lancer la démo

Prérequis : Rust stable (`cargo`). Aucun service externe, GPU ni projet voisin n'est nécessaire à l'exécution.

```powershell
.\demo.ps1              # compile en release, ouvre le navigateur, sert http://127.0.0.1:8785/
.\demo.ps1 -Port 8790 -NoBrowser
```

```bat
demo.cmd                rem même chose depuis cmd.exe ; port optionnel : demo.cmd 8790
```

Ou directement :

```powershell
cargo run --release -p chronos-app -- serve --port 8785
```

Le serveur écoute uniquement sur `127.0.0.1`. Il trouve `web/` et `fixtures/` depuis le répertoire courant ou, à défaut, en remontant depuis l'exécutable (`target/release/`). Options : `--root DIR`, `--fixture FICHIER`, `--web DIR`. Ctrl+C pour arrêter.

## Ligne de commande

```powershell
cargo run --release -p chronos-app -- generate --out world.json            # configuration par défaut
cargo run --release -p chronos-app -- generate --config config.json --out world.json
cargo run --release -p chronos-app -- check world.json                      # validation + comparaison au rejeu
```

`generate` écrit `generation_ms = 0` : deux exécutions avec les mêmes entrées produisent des fichiers identiques octet pour octet. Un champ absent de `config.json` prend la valeur par défaut du contrat (sans `--config`, les valeurs adaptées à la fixture sont utilisées, comme `/api/defaults`).

## Scénario de démonstration ou histoire libre

- `scenario: "pivot"` (défaut) : démonstration 1300–1500 ; épidémie continentale 1354–1360 (perte exacte `floor(P₁₃₅₆ × 3/10)` dans la plus grande ville en 1356), guerre avec exactement trois villages détruits et héros Lineage Lab en 1398, si la fenêtre les couvre. Ces événements sont imposés et étiquetés « scénario pivot ».
- `scenario: "free"` : aucune crise imposée ; guerres, épidémies et héros ne viennent que des taux. `{"scenario":"free","war_rate":0,"epidemic_rate":0}` donne une histoire sans guerre ni épidémie.

## Offices collectifs

Une organisation dont l'office retenu par Pecking Order est collectif (par exemple l'assemblée d'une ligue de cités) est exportée avec `collective_office: true` et `holder_id: null` : c'est une instance collective abstraite, pas un office vacant. Aucune personne n'y est nommée et aucune succession individuelle n'est générée.

## API HTTP

| Méthode | Chemin | Réponse |
|---|---|---|
| GET | `/api/health` | `{"ok":true}` |
| GET | `/api/defaults` | `{"config", "sources", "capabilities"}` — configuration par défaut adaptée à la fixture (nombre de sites, population, rayon) |
| GET | `/api/world` | monde courant (génère le monde par défaut au premier appel) |
| POST | `/api/generate` | corps = `Config` (champs manquants = défauts) → `World` |
| POST | `/api/import` | corps = `World` exporté → `World` validé et conservé |
| GET | `/`, `/app.js`, … | fichiers statiques de `web/` |

Erreurs : statut non-2xx et `{"error":"message"}`. Un import est refusé si sa structure est invalide, si son `canon_id` ne correspond pas à sa configuration et à ses sources, ou si — les mêmes sources étant disponibles — ses faits diffèrent du rejeu.

## Sources et statuts

Chaque monde liste ses sources avec un statut `imported`, `derived`, `synthetic` ou `unavailable`. En mode `stack`, la carte (Neural Earth), les sites et routes (Magna Urbis), les noms (Namebench), les modèles d'institutions (Pecking Order) et les personnes (Lineage Lab) viennent de la fixture ; ce qui manque est remplacé par un repli explicitement `synthetic`. Sans fixture, l'application fonctionne avec un monde entièrement virtuel et l'indique dans les avertissements ; rien n'y est présenté comme importé.

## Développement

```powershell
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p chronos-core --target wasm32-unknown-unknown   # le noyau n'a ni E/S ni horloge
```

Structure : `crates/chronos-core` (modèle, géographie, société, boucle annuelle, validation ; dépend seulement de `serde`/`serde_json`), `crates/chronos-app` (CLI et serveur HTTP en bibliothèque standard), `web/` (interface sans build), `tools/` (préparation ponctuelle des données, hors workspace).

## Limites du prototype

Génération intégrale à la racine d'un petit monde (pas de raffinement à la demande) ; instantanés annuels bornés à `years × sites ≤ 150 000` ; offices personnels simples et instances collectives abstraites ; routes figées ; écarts Lineage Lab/Chronos non réconciliés. Détails dans la [note de méthode](docs/methods/chronology.md#limites-connues).
