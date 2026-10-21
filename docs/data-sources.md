# Sources de données de la fixture Chronos

Date : 21 octobre 2026. Ce document décrit `fixtures/stack.json`, le bootstrap (schema_version 1) lu par Chronos au lancement, et la préparation qui l'a produit. **Chronos n'exécute aucun de ces outils** : une fois la fixture présente, l'application n'a besoin ni de Python, ni de Node, ni de GPU, ni des projets voisins.

Les cinq sources voisines sont réellement appelées. Aucune n'est remplacée par un repli virtuel dans cette fixture. Les approximations sont énumérées au §5.

| Source | Statut affiché | Ce qui est réellement utilisé |
|---|---|---|
| Neural Earth (Orogen) | importée | Pipeline Orogen d'origine (`native/orogen/pipeline.mjs`), exécuté sur CPU par Node, seed 42 |
| Rasters régionaux | dérivée | Rééchantillonnage Chronos des champs Orogen vers 4 tuiles Magna Urbis |
| Magna Urbis | importée | `magna_urbis_core::settlements::Terrain::new(Raster)`, `place`, `roads` (crate Rust appelée par chemin) |
| Ressources des sites | dérivée | Formules de l'outil Chronos (§4) |
| Namebench | importée | Base mondiale `data/names.db` en lecture seule, plus son `FantasyGenerator` |
| Pecking Order | importée | Moteur Rust `pecking_order::Engine` (thème `generic`), sur un instantané figé |
| Lineage Lab | importée | `lineage_lab.engine.generate(Scenario, None)` en mémoire, modèle `full` |

Empreintes : chaque entrée `sources[]` de la fixture porte une empreinte SHA-256 ; `provenance` détaille les empreintes d'entrée (champs Orogen, sources Magna Urbis, base Namebench, `kb_digest`, scénario Lineage Lab, outils). L'empreinte de la fixture elle-même est dans `fixtures/stack.json.sha256`.

## 1. Relancer la préparation

Depuis la racine de Chronos, avec Node.js, Rust/cargo et les projets voisins `../infinite_map`, `../city_generator`, `../namebench` (avec `.venv` et `data/names.db`), `../pecking-order`, `../lineage-lab` (avec `.venv`) :

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/prepare.ps1
```

Ce script enchaîne exactement les commandes suivantes (toutes relatives à la racine Chronos) :

```powershell
node tools/orogen/export_orogen.mjs ../infinite_map output/orogen-seed42 42
cargo build --release --manifest-path tools/export-map/Cargo.toml --target-dir output/target-tools
output/target-tools/release/chronos-export-map.exe output/orogen-seed42 output/map/map-export.json
../namebench/.venv/Scripts/python.exe tools/namebench/export_names.py ../namebench output/names/names.json
python tools/pecking_order/snapshot.py ../pecking-order output/pecking-order-snapshot
cargo build --release --manifest-path tools/export-blueprints/Cargo.toml --target-dir output/target-tools
output/target-tools/release/chronos-export-blueprints.exe output/blueprints/blueprints.json
../lineage-lab/.venv/Scripts/python.exe tools/lineage/export_people.py ../lineage-lab output/map/map-export.json output/people/people.json
python tools/assemble_fixture.py
python tools/validate_fixture.py
```

`python tools/assemble_fixture.py --check` vérifie que la fixture est identique, octet pour octet, à un réassemblage des fichiers de `output/`. Durée mesurée sur la machine de préparation : Orogen ≈ 4 s, région et Magna Urbis ≈ 6 s, Namebench ≈ 4 s, Lineage Lab ≈ 1 s, hors compilation Rust.

Les projets voisins sont seulement lus. Les sorties intermédiaires vont dans `output/` (ignoré), les crates d'outillage se compilent dans `output/target-tools`. Les deux crates de `tools/` déclarent leur propre `[workspace]` et ne font pas partie du workspace Chronos.

**Reproductibilité mesurée.** Deux exécutions complètes successives produisent des fichiers identiques pour Orogen (`fields.f32`), la carte et Magna Urbis (`map-export.json`), Namebench et Lineage Lab. La sortie Pecking Order dépend de l'état du dépôt voisin au moment de l'instantané (§3.4).

## 2. Structure de `fixtures/stack.json`

JSON compact, clés triées, UTF-8 ; environ 1 Mo. Champs du contrat (`.agent-work/implementation-contract.md`) : `schema_version`, `seed`, `map`, `sites`, `routes`, `names`, `blueprints`, `people`, `sources`. Les champs supplémentaires sont facultatifs pour le cœur :

- `map` : `width` = `height` = 128, `elevation_m` (lignes du nord au sud), `bounds` `[lon_min, lat_min, lon_max, lat_max]`, `radius_km` = 800, `source`. En plus : `projection` (`equirectangular`), `temperature_c` et `precipitation_mm` (même disposition que l'altitude), `center_lon_deg`, `center_lat_deg`, `planet_diameter_km`, `source_vertices_in_bounds`.
- `sites` (52), triés par `id` (`site-<tuile>-<clé Magna Urbis>`, par exemple `site-nw-main`, `site-se-village-4`). `region_id` = `region-nw|ne|sw|se` (une tuile Magna Urbis chacune). `culture_id` = `french` pour les tuiles ouest, `british` pour les tuiles est : ce sont des clés de réservoirs de noms, choisies par la préparation. Pas de `name` : Chronos nomme les sites. En plus : `lon_deg`, `lat_deg`, `elevation_m`, `temperature_c`, `precipitation_mm`, `magna_urbis` (tuile, clé, classe, archétype, rayon, distance à l'eau).
- `routes` (71) : `from`/`to` sont des `id` de sites ; `path` en coordonnées normalisées ; `distance_km` mesurée le long du tracé Magna Urbis dans le plan régional ; `travel_days` = `distance_km / 25` (vitesse par défaut, arrondi au centième, à requantifier par le cœur) ; `kind` = classe de route Magna Urbis (`main`, `local`, `track`) ; `source` = `magna-urbis-roads`. En plus : `tiles`, `gateways` (non vide quand la liaison traverse une frontière de tuile).
- `names` : `french`, `british`, `fantasy`, chacun avec `forenames` (300), `surnames` (250), `places` (250), plus `forenames_male` et `forenames_female` (150 chacun). Listes dédoublonnées, normalisées NFC, triées par point de code.
- `blueprints` (6) : champs du contrat ; `succession` ∈ `hereditary`, `vote`, `appointment`. En plus : `label_en`, `office_title_en`, `system_id`, `theme`, `authority`, `structure`, `selection_fr`/`selection_en` (texte source), `office_powers`, `roles` (rôles complets de la résolution).
- `people` (1 846) : champs du contrat. `site_index` indexe `sites` (ordre trié) et donne la **dernière résidence enregistrée** (lieu du décès, ou résidence en 1420). `death_year: null` signifie **vivant à la fin de la fenêtre Lineage Lab (1420)**, décès ultérieur inconnu — pas une immortalité. En plus : `status` (0–3, 3 = classe la plus rare), `activity`, `founder`, `birth_site_index`, `residences` (`[année_de_début, site_index]`, la première entrée à `max(naissance, 1300)`), `selection` (`elite`, `commoner-1398`, `ancestor`).
- Autres champs : `people_window` `[1300, 1420]`, `regions`, `gateways`, `resource_units`, `warnings`, `provenance`.

`python tools/validate_fixture.py` vérifie ce contrat : types, unicité des IDs, extrémités de routes, parents présents dans le tableau et cohérents (sexe, âge), indices de site, tri des noms, statuts de source et absence de chemins absolus.

## 3. Préparation par source

### 3.1 Neural Earth / Orogen

`tools/orogen/export_orogen.mjs` reproduit la requête du backend maintenu (`backend/terrain_orogen_stages.py`, `_request` et `_native`) pour le profil de base `orogen` : 80 plaques, 3 continents, 30 % de terres, 100 000 sommets, paramètres climatiques par défaut de `native/orogen/climate-parameters.json`. Il lance trois fois le `pipeline.mjs` voisin, sans le modifier : relief (`skipPost`, `skipClimate`, instantané retenu), érosion (reprise), climat (reprise). L'interpolation CUDA du backend n'est pas appelée ; aucune inférence neuronale n'intervient. `manifest.json` conserve les empreintes de chaque étape et de chaque fichier de `native/orogen`.

`tools/export-map` lit `climate/fields.f32` et échantillonne les champs avec un portage CPU de `terrain_orogen_cuda.rasterize` : quatre plus proches sommets, éventails de triangles incidents, poids barycentriques sphériques `M⁻¹q`, repli au plus proche sommet si aucun triangle ne contient le point (0 cas ici). L'altitude est interpolée en unités Orogen puis convertie par la loi `coastal-slope-v1` (`terrain_orogen._metres`) ; BIO1 (°C) et BIO12 (mm) suivent `terrain_conditioning.orogen_bioclim`.

**Petite planète, choix explicite.** Le graphe Orogen vit sur la sphère unité. Neural Earth accepte un `world_diameter_km` de 10 à 100 000 km ; la fixture retient **1 600 km de diamètre** (rayon 800 km). À taille terrestre, une région de 100 km ne couvrirait qu'environ deux sommets source ; ici l'espacement est d'environ 9 km. Le climat Orogen ne dépend pas du diamètre : ce sont des bandes climatiques de type terrestre appliquées à un petit monde.

**Choix de la région.** Balayage déterministe au pas de 1°, |lat| de 15 à 50°, |lon| ≤ 160°. Critères : chaque tuile ≥ 60 % de terres ; 72 à 97 % de terres au total ; terres de 6 à 18 °C et de 450 à 2 000 mm ; aucune terre échantillonnée au-dessus de 2 600 m. Score : `-4|terres-0,88| - |T-12|/5 - |ln(P/900)| - |σ-350|/350`, premier maximum. Sur 173 candidats, le choix est centré à **lon 154°, lat −43°** : 87 % de terres, 12,0 °C, 704 mm, écart-type d'altitude 367 m. Le plafond d'altitude a été ajouté après un premier essai où Magna Urbis plaçait des villages à 3 000 m et 5 900 m sur des plateaux lissés par l'interpolation.

**Carte d'ensemble.** 128 × 128 pixels équirectangulaires couvrant les 200 km détaillés plus 14 km de marge. Emprise : lon 141,1° à 166,9°, lat −52,2° à −33,4° ; 854 sommets source dans l'emprise. L'emprise est carrée en mètres à la latitude centrale ; l'échelle est-ouest varie d'environ ±13 % entre le haut et le bas de la carte.

### 3.2 Magna Urbis

`tools/export-map` dépend de `../city_generator/rust/crates/core` par chemin. La compilation hors du workspace de Magna Urbis fonctionne : c'était un risque ouvert de la roadmap.

- **Tuiles** : quatre carrés adjacents de 100 km (`nw`, `ne`, `sw`, `se`) dans une projection azimutale équidistante centrée sur la région. Erreur d'échelle inférieure à 0,5 % à 141 km du centre. Chaque raster fait 256² cellules (390,6 m) ; `water` = 1 si altitude < 0, `water_level` = 0, `river_width` = 0, `river_ids` = −1.
- **Placement** : `place(seed = "chronos-export-map-v1:tile:<t>:place", mode = counts)` ; un établissement principal (11 000, 7 500, 6 000, 9 000 habitants selon la tuile), 3 bourgs, 9 villages ; `relief` déduit de l'écart-type local (`plains` < 120 m ≤ `hills` < 450 m ≤ `mountains`) ; préférence `elevation = -0,4` (basses terres). Résultat : 52 sites (16 bourgs, 36 villages), 84 417 habitants au total. Ces effectifs sont des paramètres d'entrée, pas un recensement.
- **Raccords** : sur chaque frontière commune, un site-passerelle de 20 habitants est placé sur la terre la plus proche du milieu de la paire de sites la plus proche de part et d'autre. Le choix pénalise la pente et se fait par pas de 1 km entre 5 et 95 km. Il est ajouté aux deux appels `roads`. Les sorties de bord automatiques sont désactivées (`exits = 0`) ; aucune liaison maritime n'est créée.
- **Routes** : `roads(seed = "…:roads", niveau rural, densité 0,5)`. Les segments des quatre tuiles forment un seul réseau. Les passerelles y sont des jonctions de passage. Les liaisons site à site sont les plus courts chemins qui ne traversent aucun autre site (117 candidats). Comme les tronçons partagés relient beaucoup de sites par une même jonction, la fixture garde un sous-graphe **1,15-spanner** glouton. Les candidats sont parcourus du plus court au plus long, et une liaison est gardée seulement si les liaisons déjà gardées ne relient pas ses extrémités en moins de 1,15 fois sa longueur. Il en reste 71, et tous les sites restent reliés. Les tracés sont simplifiés (Douglas–Peucker, 120 m), puis projetés dans la carte.
- **Déterminisme** : deux exécutions donnent un `map-export.json` identique octet pour octet. Aucun avertissement Magna Urbis.

### 3.3 Namebench

`tools/namebench/export_names.py` ouvre `../namebench/data/names.db` en SQLite `mode=ro&immutable=1`. La classe `NameDB` de Namebench n'est pas utilisée pour se connecter, car elle passe le fichier en journal WAL. Aucun `ORDER BY RANDOM()`.

- `french` → style Namebench `style.french` (FR) ; `british` → `style.english_uk` (GB), qui est l'alias `english` de Namebench (il n'existe pas d'alias `british`).
- Prénoms : les 150 premiers de chaque genre selon le rang de popularité (`philipperemy`), longueur ≥ 3, égalités départagées par le nom. Noms de famille : les 250 premiers par rang.
- Lieux : 250 lieux GeoNames `PPL` de 1 000 à 30 000 habitants, échantillonnés par clé SHA-256 de leur identifiant. Ce sont des petites villes plutôt que des capitales actuelles.
- `fantasy` : prénoms tirés des tableaux réels `fantasy.medievals` (`nm2` masculins, `nm4` féminins ; `nm1`/`nm3` sont des titres), par clé SHA-256. Noms de famille : `FantasyGenerator("dungeon_and_dragons.humans")`. Lieux : `FantasyGenerator("towns_and_cities.fantasy_towns")`. Les deux générateurs tournent après `random.seed` sur une chaîne fixe.
- Les formes d'origine sont conservées, y compris le bruit de la source (par exemple « Amg », « Bah » parmi les noms français) ; aucun filtrage éditorial.

### 3.4 Pecking Order

Le dépôt voisin était en cours de réécriture pendant la préparation : son README décrit encore l'API Python, mais `src/pecking_order` ne contient plus de sources Python, et `data/` et `src/*.rs` changeaient d'une minute à l'autre. Une compilation a même échoué sur un état intermédiaire (« Invalid role: monastery.abbot »). Pour obtenir une entrée cohérente :

1. `tools/pecking_order/snapshot.py` copie `Cargo.toml`, `build.rs`, `src/`, `data/` et `web/` dans `output/pecking-order-snapshot` et enregistre leurs empreintes (`SNAPSHOT.json`).
2. `tools/export-blueprints` compile le crate `pecking-order` depuis cet instantané. Le `build.rs` voisin s'exécute dans la copie et n'écrit que dans le répertoire cible Chronos. L'outil appelle `Engine::embedded().resolve({"system": …, "theme": "generic", "locale": "fr"})`.
3. `kb_digest` = SHA-256 de `serde_json::to_vec(&engine.content)`, la base effectivement compilée.

Profils : `kingdom` (monarchie), `free_city` (conseil), `village` (communauté villageoise), `city_league` (ligue marchande, office collectif, `max_holders` null), `merchant_guild` (guilde), `mutual_aid_cell` (collectif informel). L'office principal suit la règle du moteur : niveau 0, non collectif, importance maximale ; à défaut, le rôle collectif le plus important. Pour la guilde marchande, la source place « Maître artisan » au niveau 0 et « Maître de guilde » au niveau 7 ; la fixture reprend la source telle quelle. `succession` est une **interprétation Chronos** de l'autorité Pecking Order : `hereditary` → `hereditary` ; `democratic`/`consensual` → `vote` ; les autres → `appointment` ; un office uniquement collectif → `vote`. Le texte `selection` de la source est conservé à côté.

Relancer la préparation plus tard reprend l'état courant du dépôt voisin : le `kb_digest` changera si son contenu a changé.

### 3.5 Lineage Lab

`tools/lineage/export_people.py` part du préréglage livré `presets/medieval.yaml` et simule en mémoire, modèle `full`, seed 42, de 1300 à 1420 (120 ans) :

- Établissements explicites = les 52 sites de la fixture. L'identifiant Lineage Lab est l'indice du site dans `sites` trié. Les positions viennent du plan régional (km) et la capacité de la population Magna Urbis, mise à l'échelle par Lineage Lab (`capacity_mode: scale`). C'est une adaptation de la fixture : la correspondance des lieux est exacte par construction, pas un appariement par rang.
- 2 000 fondateurs. Les événements du préréglage (guerre 1060, épidémie 1100) et sa période 1120 sont retirés, pas décalés : Chronos possède les crises. Sans eux, le préréglage seul décroît d'environ moitié en 120 ans (mesuré : 3 000 → 1 454). La fixture active donc la régulation bornée native de Lineage Lab (`target_population` 2 800, plancher = 2 000 fondateurs) : 2 000 → 2 020 habitants en 1420.
- 10 083 personnes simulées. Personnes exportées (1 846) : statut ≥ 2 vivant à un moment de 1300–1420 (610), 120 roturiers adultes (≥ 16 ans) vivants en 1398, choisis par clé SHA-256, et la fermeture de tous leurs ancêtres jusqu'aux fondateurs. Tout `father_id`/`mother_id` est présent dans le tableau. 364 personnes exportées sont adultes et vivantes en 1398, dont 86 de statut élevé.
- Les dates de naissance et de décès, les parentés et les résidences ne sont ni décalées ni réattribuées. Calendrier : année Lineage Lab = année Chronos, sans décalage. `external_id` = `lineage-lab:<12 premiers caractères du SHA-256 du scénario>:<id>`. Lineage Lab ne fournit pas de nom propre : Chronos nomme les personnes avec Namebench.

## 4. Ressources des sites (dérivées)

Calculées dans `tools/export-map/src/main.rs` (`resources`) à partir du climat et de l'altitude Orogen au site, de l'activité tectonique Orogen et des scores Magna Urbis (pente, part constructible, distance à l'eau). Ce sont des potentiels conventionnels, pas des gisements relevés.

- `food` (personnes-années/an) = `P·(0,7 + 0,8·S) + 300·S`, avec `S` = climat × (0,4 + 0,6 × terre arable).
- `water` (personnes-années/an) = `P·(0,6 + 1,4·clamp((BIO12−150)/850)) + 200·clamp(BIO12/1000)`.
- `timber`, `iron`, `stone`, `salt` : indices 0–100. Le bois dépend de BIO12 et BIO1, le fer de l'activité tectonique et de l'altitude, la pierre de la pente et de l'altitude, le sel de la proximité de la mer (réduit sous 8 °C).

## 5. Limites et approximations

- Relief et climat interpolés depuis un graphe d'environ 9 km d'espacement : pas de cols, de rivières, de lacs ni de sols. Magna Urbis ne voit que la mer ; ses archétypes « pont » et « confluence » ne peuvent pas apparaître.
- Petite planète choisie pour la démonstration ; le climat Orogen ne dépend pas de la taille.
- Carte d'ensemble équirectangulaire : déformation est-ouest d'environ ±13 % ; les distances de route sont métriques. À 1,8 km par pixel, quelques tronçons côtiers et un bourg portuaire (`site-nw-town-2`, +4,8 m) tombent sur des pixels marins de la carte d'ensemble ; dans le raster Magna Urbis à 390 m, ils sont à terre.
- Seule la région de 200 km est détaillée ; aucun graphe planétaire ni corridor grossier n'est fourni.
- Noms : fréquences contemporaines et lieux GeoNames actuels, sans authenticité historique. Les générateurs fantasy dépendent de l'implémentation `random` de Python.
- Pecking Order : instantané d'un dépôt en réécriture ; la règle de succession est une interprétation Chronos.
- Lineage Lab : échantillon de 2 000 habitants simulés, sans lien avec les crises Chronos. Ses totaux ne réconcilient pas les effectifs Chronos (84 417 habitants Magna Urbis). Ses personnes sont un échantillon inclus dans ces effectifs, jamais ajouté. Hors de 1300–1420, il n'y a pas de personne importée.
