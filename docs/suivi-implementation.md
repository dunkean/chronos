# Suivi d'implémentation — entités sociales, multi-échelle, ressources

Document de reprise. Il permet de reprendre le développement dans une nouvelle conversation sans autre contexte. **Mets-le à jour à chaque session** : statut des tâches (§7), journal (§9), décisions (§8).

Dernière mise à jour : 26/10/2026, en fin de journée. Statut global : **conception faite, implémentation Chronos non commencée.** Rien des paliers P0–P5 n'est implémenté. Les apports d'Azgaar sont intégrés au design (vision §7.7) et au plan (§7.0, §7.3). Le chantier ressources est en cours dans Neural Earth (§7.2, R3c : refonte multi-échelle et performance).

## 0. Reprendre en une minute

Message à coller au début d'une nouvelle conversation, depuis `D:\Workspace\Self\RPG\chronos` :

```text
Lis docs/suivi-implementation.md en entier, puis les documents de son §2 « Lecture obligatoire ».
Vérifie l'état réel (git status, branches et worktrees du §5) et signale tout écart avec le §7.
Ensuite, reprends la prochaine tâche « à faire » du §7 (ou celle que je te donne).
Mets à jour §7 et §9 à la fin de la session.
```

Règles de travail (rappel de `CLAUDE.md`, qui fait foi) :
- documents, interface et diagnostics en français ; séparer **proposé / implémenté / mesuré** ;
- déterminisme strict : tout aléa passe par `keys::key/unit/range_i(seed, parties sémantiques)`, `BTreeMap`/`BTreeSet` seulement ;
- `chronos-core` sans E/S ni horloge (`cargo check -p chronos-core --target wasm32-unknown-unknown`) ;
- tout changement de sémantique change `ALGORITHM_VERSION` et `canon_id`, et `docs/methods/chronology.md` doit suivre ;
- avant de toucher au moteur, lire les §14, 15, 17 et 18 de l'étude v3.

## 1. Vision (résumé)

Le détail est dans [vision-entites-sociales.md §1](reviews/vision-entites-sociales.md).

- **Générer 100 à 2 500 ans d'histoire** médiévale-fantastique, sans progrès technique, à partir de deux entrées : une carte (relief, eau, climat, ressources) et un scénario de départ (colonisation, jeune civilisation ou vieille civilisation).
- **Un seul concept : l'entité sociale.** Elle est récursive (des membres qui sont eux-mêmes des entités) et reliée aux autres par la composition `IsPartOfMe[role]` et par des relations latérales.
- **Trois capacités.**
  1. *Bootstrap* d'un monde selon le scénario.
  2. *Progression* de la chronologie, avec rétroaction du climat, de la biologie et du peuplement.
  3. *Raffinement* vers les échelles fines, cohérent dans le temps, l'espace et la causalité. **Une décision n'est prise qu'une fois** : le zoom détaille, il ne refait jamais.

## 2. Lecture obligatoire, dans l'ordre

| # | Document | Pourquoi |
|---|---|---|
| 1 | `CLAUDE.md` (ce dépôt) | Commandes et invariants qui traversent les fichiers |
| 2 | [docs/reviews/vision-entites-sociales.md](reviews/vision-entites-sociales.md) | Vision, état actuel, autres projets, manques, faisabilité, design macro (§7), paliers |
| 3 | [conception_generateur_histoire_multiechelle_v3.md](../conception_generateur_histoire_multiechelle_v3.md) §4A, §4.5, §10, §14–18 | Conception de référence : entité réifiée, propriété des événements, raffinement et budgets, architecture Rust, déterminisme, validation, phases |
| 4 | [docs/methods/chronology.md](methods/chronology.md) | Algorithme implémenté aujourd'hui et ses limites |
| 5 | `../infinite_map/docs/architecture.md` (table des LOD) et `../infinite_map/docs/design/multiscale-hydrology.md` | Modèle multi-échelle de Neural Earth, à imiter (autorité globale, raffinement, propriété des frontières) |
| 6 | `NeuralEarth-Resources-Starter.md`, `NeuralEarth-Resources.md`, `NeuralEarth-Resources-Fantasy.md` (racine de ce dépôt) | Spécification des 35 couches de ressources |
| 7 | `../city_generator/docs/GROUND_MATERIAL_MAPPING.md` §2–5 et `docs/GROUND_TEXTURE_MAPPING_V1.md` | Lithologie et sols de tuile, choix des matériaux, extensions O1–O10 |
| 8 | [docs/integration-azgaar.md](integration-azgaar.md) (référence), puis [docs/reviews/azgaar-fmg-inspiration.md](reviews/azgaar-fmg-inspiration.md), puis le code de `../Fantasy-Map-Generator/src/model/` pour le lot ou la tâche en cours | Ce qu'on reprend d'Azgaar et comment l'intégrer (lots A0–A5, invariants, validation) ; règles de génération (tâches AZ) ; ce qu'il ne faut pas reprendre |
| 9 | `../neural-earth-resources/docs/RESOURCE_LAYERS.md` (branche `resources-layers`) | Layers de ressources, adressage des cellules, contrat d'export vers Chronos (R5) |

## 3. Architecture de l'écosystème

**Ce cadrage est décidé par Gregory. Ne pas le remettre en cause.**

| Projet | Rôle | Échelle |
|---|---|---|
| **Neural Earth** (`../infinite_map`) | **Seul système multi-échelle.** Relief, climat, hydrologie, sols, matériaux ; à terme, ressources et cités | De la planète au cm : LOD 11 = 61,44 km … LOD 7 = 3,84 km, LOD 3 = 240 m, LOD 0 = 30 m, LOD -6 = 47 cm |
| **Magna Urbis** (`../city_generator`) | Cités et établissements (`Terrain::place`, `Terrain::roads`, cultures). Travaille sur les **tuiles Neural Earth de 40 × 40 km** et utilise quelques échelles propres pour ses villes. **Sera intégré dans Neural Earth.** Son compilateur de sol (`godot-render/tools/surface_context.py`) est du traitement de tuile Neural Earth, hébergé ici provisoirement | Tuile 40 km, environ 30 m, puis échelles urbaines |
| **Chronos** (ce dépôt) | **Dimension temporelle à l'échelle monde.** Entités sociales, chronique canonique, raffinement temporel. Lit les agrégats de Neural Earth et porte le **delta historique** (extraction, déforestation, défrichement, fondations) | Régions (LOD grossiers) × blocs de temps |
| **Pecking Order** (`../pecking-order`) | Catalogue des structures internes : rôles, `level`, `reports_to`, `powers`, `collective`. Crate Rust | — |
| **Lineage Lab** (`../lineage-lab`) | Référence démographique ; mode hybride noyau + cohortes, IDs stables par permutation inversible. Python | Personnes |
| **Namebench** (`../namebench`) | Noms et toponymes. Non déterministe : utiliser des pools figés | — |
| **ttrpg_content_generator** | Modèle d'origine (`doc/generation_schema.pptx`), catalogues de rôles, liens, cultures et races | — |

Principe commun : **une seule identité de cellule** (LOD, coordonnées) et **un seul schéma de hasard adressé** (seed, couche, LOD, cellule ou propriétaire, slot), partagés par les trois systèmes. L'état naturel est immuable et produit par Neural Earth. Le delta historique est produit par Chronos. Une échelle fine lit « naturel + delta » à une date donnée.

## 4. Concepts clés

| Concept | Définition courte | Référence |
|---|---|---|
| `SocialEntity` | Abstraction unique qui remplace `Polity`, `Organization` et `Person`. Elle porte `kind` et un archétype de catalogue | vision §7.2 ; v3 §4A |
| Nature | `acteur` (peut décider), `agrégat` (population) ou `catégorie` (identité partagée) | vision §7.2 |
| Résolution | `canonique`, `esquisse` (comptes et agrégats) ou `virtuelle` (slot non résolu) | vision §7.2 |
| Relations | Trois familles. **Structurelles**, stockées et datées (composition, gouvernement, propriété). **D'affinité**, dérivées des attributs et jamais stockées (sang, culture, proximité, idéologie, activité). **Contractuelles et historiques**, stockées avec un événement cause (alliance, commerce, dette, rivalité, amitié, grief) | vision §7.2 |
| Cohortes | La population est comptée une seule fois, dans des cohortes disjointes. Une adhésion ne crée pas de personne | vision §7.2 |
| Idée, adhésion, appui | Un seul type `Idea` pour croyance, langue, coutume, savoir, style. Les cohortes portent des distributions d'adhésion ; les agrégats en sont la somme. Les appuis (`ENDORSES`, `ADMINISTERS`, `SUPPRESSES`, `PATRONS`) sont des décisions datées | vision §7.8 |
| Étiquette dérivée | Religion d'État, forme de gouvernement, rang, type culturel, culte ou hérésie, et même culture : des lectures par classifieurs purs, jamais stockées. **Règle : générique d'abord ; Azgaar fournit du contenu, pas de la structure** | vision §7.8 ; `integration-azgaar.md` §1.1 |
| Canon racine | Simulation par **régions et entités saillantes**, en **blocs** (décennie ; année pour les crises), jamais par site | vision §5, §7.4 |
| Propriété | Chaque fait a un propriétaire unique (scope × bloc). Son ID est `(propriétaire, bloc, slot, version)` et son aléa `key(seed, propriétaire, bloc, slot)` | v3 §4.5, §15 ; vision §7.5 |
| Budgets exacts | Un fait macro (« +30 villages, +50 000 habitants ») est alloué exactement aux sous-cellules et aux années, au prorata de masses canoniques, quel que soit l'ordre de requête | v3 §10.6–10.7 |
| Pas de remontée | Un enfant ne modifie jamais son parent. Tout effet de portée supérieure est réservé en amont | v3 §4.5 |
| Maturité `m` | Paramètre unique du bootstrap : colonisation ≈ 0,1–0,2, jeune ≈ 0,4–0,6, vieille ≈ 0,9 | vision §7.3 |
| Potentiel / stock / flux | Ressources : aptitude intrinsèque, quantité épuisable, rendement renouvelable (saisonnier si utile) | §7.2 |
| Provenance | `imported`, `derived`, `synthetic` ou `unavailable`. Un repli n'est jamais présenté comme importé | `CLAUDE.md` |

## 5. Dépôts, branches et fichiers

| Où | Quoi | État au 26/10/2026 |
|---|---|---|
| `chronos` `main` | Prototype J0–J4 (`crates/chronos-core`, `crates/chronos-app`, `web/`, `fixtures/stack.json`) | Docs de vision et de suivi commités (`2cdb2ac`). Mises à jour du 26/10 au soir non commitées (Azgaar, `CLAUDE.md`, `AGENTS.md`) |
| `city_generator` (Magna Urbis), `origin/main` | Contient le moteur Godot (`godot-render/`), `surface_context.py`, la doc des matériaux | Checkout local sur `codex/render-v2-snapshot-2026-10-23` : **ne pas y toucher** |
| `D:\Workspace\Self\RPG\magna-urbis-resources` | Worktree Magna Urbis, branche `resources-layers` | Note `docs/RESOURCE_TILE_REFINEMENT.md` (`bb7dcdf`), poussée sur `main` |
| `infinite_map` (Neural Earth), `origin/main` | Orogen (`plates`, `crust`, `boundaries`, `convergence`, `uplift`, `superPlates`, `hotspot`), Köppen, biomes, pédologie, hydrologie multi-échelle, **Render v2** (fusionné, `d6cbb78`) | Checkout local : ne pas y toucher |
| `D:\Workspace\Self\RPG\neural-earth-resources` | Worktree Neural Earth, branche `resources-layers` = `main` `de2fec6` | Layers de ressources (§7.2), poussées sur `main`. `terrain-diffusion` y est une jonction vers le sous-module du checkout principal |
| `D:\Workspace\Self\RPG\Fantasy-Map-Generator` | Azgaar FMG v1.156.0 (MIT), clone de référence. Code dans `src/model/<domaine>/` | Lecture seule, source d'inspiration (§7.3) |

Fichiers Chronos à connaître : `model.rs` (tous les types sérialisés, `SCHEMA_VERSION`, `ALGORITHM_VERSION`, `MAX_SITE_YEARS`), `social.rs` (`build_society`, `import_people`, `select_profiles`), `generate.rs` (`run_history`), `keys.rs`, `validate.rs`, `tests/invariants.rs`.

Commandes : voir `CLAUDE.md`. Les essentielles :

```powershell
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p chronos-core --target wasm32-unknown-unknown
.\demo.ps1
```

## 6. Point de départ (implémenté et mesuré)

- **Implémenté** : monde figé (52 sites, 71 routes) ; boucle annuelle par site, avec 14 types d'événements et des effets exacts ; `Polity`, `Organization` et `Person` plats ; 6 indices de ressources par site ; scénarios `pivot` et `free` ; `canon_id`, rejeu et import vérifiés ; 10 tests d'invariants.
- **Mesuré** (documents existants, non remesuré) : génération de référence d'environ 17 ms, `canon-3fa7f6a56d5ce200` (seed 42, 1300–1500, `pivot`).
- **Absent** : entités génériques, support spatial hiérarchique, bootstrap, évolution du peuplement, raffinement, généalogie longue, ressources réelles, processus pilotés par les entités.

## 7. Plan et suivi

Statuts : `à faire`, `en cours`, `fait`, `bloqué (raison)`. Chaque tâche finie note sa preuve : test, commit ou mesure.

### 7.0 Ordre de bataille (prochaines étapes, dans l'ordre)

Les tâches AZ et les lots A (§7.3) sont rattachés aux paliers : on les traite dans le palier, pas à part. Seuls A0–A2 peuvent avancer avant P0.

0. **Piste rapide Azgaar, possible tout de suite et en parallèle.** A0 : outil de préparation dans `tools/` qui extrait un petit pack épinglé avec manifeste et notices. Puis A1 : emblèmes attribués aux puissances et organisations actuelles, et noms. Puis A2 : ruine d'un village détruit, auberge, foire rattachée à une ligue, comme projections des faits existants. Suivre le « premier parcours recommandé » de `integration-azgaar.md` §6. Prévoir la migration vers `SocialEntity` dès la conception, sans créer d'identité concurrente.
1. **P0 — modèle d'entités.** P0.1 et P0.2, puis P0.7 : le méta-modèle d'idées (vision §7.8), avec idées, adhésions, appuis et classifieurs d'étiquettes. Le type culturel (AZ3), la forme de gouvernement (AZ11) et le statut religieux (AZ7) y sont des **étiquettes lues**, pas des champs. On y ajoute les relations diplomatiques et la vassalité datées (AZ5, partie modèle) et les embargos (AZ18, partie modèle). Puis P0.3 → P0.4 → P0.5, avec le test d'isolement des tirages (AZ17), et P0.6. Sortie : démo et invariants inchangés sur le nouveau modèle.
2. **Spike P3 (recherche, en parallèle de la fin de P0)** : allocation exacte « +N villages, +X habitants » sur une région, indépendante de l'ordre. C'est le risque n° 1 de la faisabilité ; ne pas le repousser après P2.
3. **P1 — support spatial et bootstrap.** Il faut d'abord trancher D2 (LOD des régions) et D4 (transport des agrégats Neural Earth). Ensuite :
   - P1.1, avec les coûts de trajet en heures (AZ2) et la capacité témoin (AZ4) ;
   - P1.2 : adaptateur Neural Earth, branché sur le contrat de ressources (R5) quand il est stable ;
   - P1.3 : moteur d'expansion unique (AZ1), puis les rangs de sites par règles (AZ8), les noms Markov (AZ10), les provinces et colonies (AZ12), et la religion folk (AZ7, partie bootstrap) ;
   - P1.4 : spin-up.
4. **P2 — progression par blocs**, avec AZ5 (cascade de guerre), AZ6 (corridors par époque), AZ7 (hérésies et conversion), AZ9 (zones), AZ13 (capacité militaire), AZ14 (vestiges), AZ15 et AZ18 (économie et fiscalité), AZ16 (verrous et ancres).
5. **P3 (raffinement complet), P4 (généalogie), P5 (échelle).**

### 7.1 Paliers Chronos (vision §7.6)

**P0 — Modèle `SocialEntity`** : `à faire`. Preuve de sortie : même démo et mêmes invariants, sur le nouveau modèle.

| # | Tâche | Statut |
|---|---|---|
| P0.1 | Spécifier `SocialEntity` (champs vision §7.2), `EntityKind`, `Nature`, `Resolution`, et le cycle de vie (`SUCCESSOR_OF`, `SPLIT_FROM`, `MERGED_INTO`) dans un doc de méthode avant le code. Prévoir les liens vers les spécifications de contenu d'`integration-azgaar.md` §5 (`EmblemSpec`, `BeliefSpec`, `PointOfInterest`…). Séparer croyance, communauté de fidèles et organisation ; les affiliations sont datées et peuvent coexister | à faire |
| P0.2 | Arêtes stockées datées : `MemberOf{role}`, `Governs`, `Owns`, `AlliedWith`, `TradesWith`, `Debt`, `Rivalry`, `Friendship`, `Grievance`, avec `cause` (ID d'événement) et intervalle de validité | à faire |
| P0.3 | Relations d'affinité calculées à la demande : sang, culture, proximité, idéologie, activité. Index, aucun stockage | à faire |
| P0.4 | Migrer `Polity`, `Organization` et `Person` vers `SocialEntity`. Le JSON exporté change : nouvelle `SCHEMA_VERSION` et `ALGORITHM_VERSION`, nouvelle génération de référence | à faire |
| P0.5 | Adapter `validate.rs`, l'inspecteur web et les tests d'invariants. Ajouter des tests d'acyclicité de la composition et d'unicité des cohortes | à faire |
| P0.6 | Exploiter tout Pecking Order (niveaux, `reports_to`, pouvoirs, thèmes) pour la structure interne | à faire |
| P0.7 | Méta-modèle d'idées (vision §7.8). (a) `Idea` avec dimension, propriétés génériques et lignée `DERIVED_FROM` / `MERGED_FROM`. (b) Adhésions en distributions entières par cohorte ; les agrégats sont des sommes, jamais stockées. (c) Appuis `ENDORSES`, `ADMINISTERS`, `SUPPRESSES`, `PATRONS`, datés avec cause. (d) Classifieurs purs et versionnés : religion majoritaire ou d'État, folk / organisée / culte / hérésie, forme de gouvernement, rang, type culturel, culture. Doc de méthode avant le code. Tests : somme des adhésions = population de la cohorte ; étiquette identique quel que soit l'ordre de lecture | à faire |

**P1 — Support hiérarchique et bootstrap à 3 scénarios** : `à faire`. Il dépend des décisions D1, D2 et D4 (§8). Preuve de sortie : trois mondes distincts depuis une même carte, rejouables à l'octet près.

| # | Tâche | Statut |
|---|---|---|
| P1.1 | Cellules spatiales alignées sur les LOD Neural Earth : identité, parent, enfants, agrégats (capacité, population par culture, sites potentiels) | à faire |
| P1.2 | Lecture des agrégats Neural Earth (relief, climat, hydrologie, sols, puis ressources) par un adaptateur côté app. Le cœur reste pur | à faire |
| P1.3 | Bootstrap paramétré par `m` : capacité de charge → foyers → diffusion → liste d'établissements → `Terrain::place` (mode `list`) + `Terrain::roads` → entités | à faire |
| P1.4 | Variante *spin-up* (vieille civilisation = colonisation + progression) pour la validation | à faire |

**P2 — Progression macro par blocs** : `à faire`. Pressions → intentions → actions possibles → chocs → événements validés. Peuplement à budgets agrégés. Scénarios sortis du moteur sous forme de contraintes d'auteur. Preuve de sortie : 500 ans sur plusieurs régions, avec des villages fondés et abandonnés.

**P3 — Raffinement d'une région** : `à faire`. C'est la partie de recherche, **à prototyper tôt sur un petit monde**. Allocation exacte, villages, routes par époque, événements natifs. Preuve de sortie : tests métamorphiques A→B / B→A, et accès direct contre zoom progressif (v3 §17.2).

**P4 — Généalogie des notables en Rust** : `à faire`. Modèle hybride de Lineage Lab ; personnes secondaires reconstruites à la demande. Preuve de sortie : dynastie cohérente sur 1 000 ans.

**P5 — Montée en échelle** : `à faire`. Planète × 2 500 ans, avec des temps mesurés publiés et le matériel utilisé.

### 7.2 Chantier ressources (Neural Earth, consommé par Chronos)

Statut : **fusionné sur `main`** le 27/10/2026 : Neural Earth `de2fec6`, Magna Urbis `bb7dcdf`. Layers évalués sur GPU, compilation en arrière-plan, monde Orogen de 4 000 km, potentiels par métal et districts. Restes en R3e et R7.

Objectif : des layers de ressources du monde, raisonnées en **potentiel, stock et flux**, saisonnières si nécessaire, dans la chaîne des LOD de Neural Earth.
- **Autorité globale** : Orogen. Le contexte tectonique donne les provinces métallogéniques et lithologiques : arcs de subduction (Cu-Au porphyres, Au-Ag épithermaux), granites de collision (Sn-W), ophiolites et VMS (Cu, Cr), cratons (fer rubané, Au), rifts et bassins arides (évaporites), plus le charbon et la tourbe.
- **Raffinement** de chaque LOD par son parent, avec agrégats conservés et budgets hérités. Les gisements discrets reçoivent un tonnage vers 480–240 m ; les affleurements et chapeaux de fer sont placés à 30 m ou plus fin.
- **Réinjection dans le choix des textures** aux LOD fins, via les extensions O1, O3 et O5 de `GROUND_MATERIAL_MAPPING.md`.
- **Lien avec Chronos** : Chronos lit les agrégats régionaux et écrit le delta historique (épuisement, déforestation).
- **Côté Magna Urbis** : une note courte sur la migration de `geology()` et `surface_context.py` vers un raffinement Neural Earth, et sur ce que consomment les cités.

| # | Tâche | Statut |
|---|---|---|
| R0 | Prototype monde `neural-earth-resources/backend/terrain_resources.py`, commit WIP `56d78b7`, non poussé. Contenu : adressage `(LOD, i, j)` sur 30·2^LOD m avec ID 64 bits, parent et enfants ; aires sphériques exactes ; portage vérifié de `keys.rs` ; budgets entiers exacts (plus forts restes). Contextes tectoniques Orogen → 8 classes de roche. Climat mensuel ; NPP Miami ; céréales de type GAEZ ; pâture mensuelle et transhumance ; bois ; gibier (Damuth) ; pêche ; sel marin ; tourbe ; fer des marais ; matériaux. 16 familles de gisements discrets (Poisson, tonnage et teneur log-normaux, placés au LOD 3). Fantasy désactivée par défaut. Export régional et delta historique (extraction, défrichement, repousse). **Mesuré** : monde au LOD 10 (652 × 1304) en 28,6 s, 8 000 gisements (seed 7985403534069300490, « continents »). Worktree Magna Urbis non modifié | fait (WIP) |
| R0b | Faire passer `tests/python/test_terrain_resources.py`. Cause du blocage : au LOD 14, placer un gisement au LOD 3 évalue environ 4 M de cellules enfants par gisement. Remède : descente par étapes ou profondeur limitée | fait (16 tests OK en 42 s) |
| R1 | Rebaser `neural-earth-resources` sur le nouveau `origin/main`. Render v2 a été fusionné le 26/10/2026 (merge `d6cbb78`, tête `051dd1f`). Puis inventorier toutes les layers : Render v2 (`docs/design/render-v2*.fr.md`, `backend/terrain_render_v2.py`, `render-v2-soil-literature.fr.md`, `docs/layers.md`, `docs/materials.md`) et les textures côté Godot dans Magna Urbis. Enfin, réaligner le prototype | fait : rebasé sur `051dd1f`. Render v2 utilisé : `woody`, `height`, `needle`, `leaf`/`grass`/`snow` mensuels, `ice`. Non utilisé : `mineral`, `sand`/`clay`, `moisture` (apparence ou proxys de pente) |
| R2 | Étude `neural-earth-resources/docs/RESOURCE_LAYERS.md` : ontologie, provinces, LOD de décision, saisonnalité, règles multi-échelles, contrat d'interface neutre, section « Intégration Neural Earth – Chronos – Magna Urbis », réinjection dans les textures, fantasy optionnelle | fait (17 sections, commit `a583658`) ; réinjection dans les textures proposée seulement |
| R3 | Prototype de layers monde + tests (déterminisme, conservation des agrégats, ordre des requêtes, cohérence des gisements avec la lithologie) dans Neural Earth | fait : `backend/terrain_resources.py`, commit `847abc3`. Autorité au LOD 10 (30,72 km), répartition jusqu'au LOD 3 (240 m). **Mesuré** : 63–65 s pour le monde (dont environ 34 s pour Render v2), rechargement du cache en 1,0 s, 7 989 gisements |
| R3b | Toutes les layers de ressources sont visibles comme **layers informatives** dans le viewer Neural Earth (groupe « Ressources », même mécanisme que les layers Orogen et Pedology). Sélecteur saisonnier, provenance dans la légende, gisements avec tonnage, teneur et accessibilité médiévale, et `docs/layers.md` mis à jour | fait : 121 vues, commit `61ce0c3`. Contrôlées par Edge headless (tuiles servies, légendes affichées, inspecteur) ; **appréciation visuelle à faire**. Le serveur, `index.html` et `terrain_toolbar.js` sont modifiés, ce qui invalide le cache des tuiles neuronales |
| R4 | Note Magna Urbis : migration de `geology()` et de `surface_context.py`, consommation par les cités | fait : `magna-urbis-resources/docs/RESOURCE_TILE_REFINEMENT.md`, commit `bb7dcdf`. Aucun code |
| R3c | **Retour de test de Gregory : refusé en l'état.** (1) Ce n'est pas multi-échelle à l'affichage : un simple suréchantillonnage bilinéaire de la grille LOD 10, donc de gros pixels et aucun bruit à fort zoom. À refaire : valeur = parent × modulation fine (DEM, hydrologie et Render v2 au LOD affiché) × bruit déterministe multi-octave, puis correction conservative (pycnophylactique) ; contacts catégoriels déformés ; gisements en vraies formes. (2) Lenteur : environ 0,1 s par tuile, mais plusieurs secondes par changement de vue. Objectif : la même fluidité que les autres layers, mesurée avant/après | **livré, à valider** : commits `be8eb4f` et `8c7d13e`, `backend/terrain_resource_detail.py`, 20 tests OK. Chaque LOD est calculé depuis son parent avec modulation par le relief et les rivières, bruit adressé, conservation exacte (écart d'environ 10⁻¹⁵) et pas de couture entre tuiles. Gisements dessinés en formes (filons, amas, lentilles, placers). **Mesuré :** tuile en cache 15–30 ms ; changement de vue en première visite, médiane 4,5 s (1 à 44 s) ; revisite, médiane 2,4 s ; tout sur CPU. **Revue des captures (Claude) :** (1) les roches sont un bruit en confettis sans unités géologiques cohérentes (pas de corps kilométriques, de ceintures ni de plis lisibles), avec un motif régulier au LOD 7 ; (2) la neige montre encore des bords rectilignes de cellules de 30 km au LOD 4 et des blocs au LOD 7 ; (3) couplage au relief faible (\|ρ\| ≤ 0,2), mesuré seulement pour la pierre et la neige ; (4) fluidité « comme les autres layers » non démontrée, pas de GPU |
| R3d | **Verdict de Gregory sur R3c : inutilisable.** À refaire : (1) GPU obligatoire, sur l'architecture de Render v2 : évaluateur WGSL dans le navigateur, miroir NumPy de référence, tests de parité CPU/WGSL, tests WebGPU, benchmark en timestamps GPU comparé à Render v2 ; (2) des zones cohérentes (massifs, ceintures, bassins) aux frontières organiques, pas des confettis ; bruit fin limité aux contacts ; aucun artefact de grille ; (3) une qualité visuelle et d'intégration au niveau de Render v2. Budgets canoniques exacts dans la référence NumPy ; affichage conservatif à une tolérance documentée. **Livré** (`e191185` code, `9552428` docs et captures, non poussé) : évaluateur WGSL `web/terrain_resources_gpu.js` et miroir `backend/terrain_resource_gpu.py` ; parité de 114 cas sans écart (erreur relative max 2,4e-7). Mesures sur RTX 3090 : 0,11–0,26 ms GPU par tuile contre 0,86–0,97 ms pour Render v2 ; blocs grossiers des roches à 3,0 ms. Roches : provinces tirées des contextes Orogen, mêmes parts canoniques à tous les LOD ; 77 % des cellules n'ont qu'une roche ; contacts entre roches différentes au LOD 4 : 0,37 %, contre 5,2 % avant. Réserves : grain en blocs de la NPP au LOD 4, de cause inconnue ; vue monde des roches encore chargée ; évaporites à 0,1 % contre 0,7 % attendus ; symboles de gisements petits ; compilation d'un monde en ~200 s contre 90 s ; erreurs 500 quand la mémoire est saturée ; suite Python complète non terminée. Détails dans `RESOURCE_LAYERS.md` §16.7 et §17. **Verdict de Gregory : inutilisable, trop lent.** Mesure : la première couche ressource attend 221 s, le temps que `compile_world` tourne de façon synchrone dans la requête HTTP. Le cache disque (767 Mo) est invalidé à chaque modification du compilateur. Le serveur a aussi planté une fois (segfault, code 139). Correctifs demandés : premier pixel en moins de 2 s sur un monde chargé et moins de 10 s sur un monde neuf ; état de chargement visible ; cache d'environ 0,3 Go avec une clé versionnée ; cause du crash ; benchmark à froid. **Correctif livré** (`39e4e87`, `0bbb6f3`, non poussé) : calcul dans un processus séparé, publié en 4 étapes, avec progression affichée dans la légende ; compilation complète en 27–41 s au lieu de 206 s ; cache de 0,17 Go avec une clé `COMPILER_VERSION`. Ma mesure côté serveur (monde 42, à recompiler) : roches disponibles en 8 s, les 7 couches principales en 22 s, puis 0,04–0,6 s par couche. Échec de l'objectif de 10 s sur un monde neuf (dépôts jusqu'à 41 s). Échantillonnage des roches passé de 4×4 à 2×2 (écart de parts p90 de 0,067). Cause du crash 139 non établie : l'isolation en processus et faulthandler sont ajoutés | livré, en attente du verdict de Gregory |
| R3e | **Remarque de Gregory : cuivre, fer, etc. apparaissent comme des sites, pas des zones.** À faire : (1) une couche « Potentiel · métal » par métal, qui combine les prospectivités des familles de gisements, pondérées grade-tonnage (provinces métallogéniques en zones) ; (2) un placement hiérarchique province → district → gisement, avec budgets exacts et clés adressées ; (3) un stock affiché selon le zoom : densité à l'échelle du district au zoom large, grappes au zoom moyen, emprises au zoom fort, masse conservée ; (4) une légende qui distingue potentiel et stock. Le budget de temps du premier pixel ne doit pas augmenter. Confié à Codex `gpt-6.1-sol` high (session `01a17a9b`), qui reprend les modifications non commitées de l’agent Opus arrêté. Précédé de R3f **Livré par Codex** (`1bb329c` code, `de2fec6` docs) : potentiels par métal, districts déterministes, noyaux de stock adaptés au zoom, légendes ; parité de 216 cas sans écart ; premier pixel à chaud en 0,29–0,82 s. Ma revue des captures : les stocks forment bien des grappes en districts ; le potentiel cuivre au LOD 10 reste presque uniforme sur les terres, sans ceintures nettes ; les ellipses de stock au LOD 4 ont l'air plaquées. À reprendre si Gregory le juge utile. | fait (poussé sur `main`, `de2fec6`) |
| R3f | **Demande de Gregory : monde Orogen initial de 4 000 km de diamètre** au lieu d'environ 12 732 km, pour réduire la taille du monde et les calculs. La stack Neural Earth gère déjà n'importe quel diamètre (`world_diameter_km`). Il suffit de changer la valeur par défaut et d'adapter les modules ressources, qui figeaient les limites du monde. Commit séparé : le changement touche Neural Earth au-delà des ressources, à pousser ou séparer au choix de Gregory **Livré** : `world_diameter_km = 4000` pour le profil Orogen, dans `terrain_generation.py` et `terrain_generation_controls.js` ; ressources calées sur les limites de l'atlas. Compilation du monde : 13,6 s contre 41 s. Couture visible du détail de terrain Render v2 au raccord en longitude (§19 de RESOURCE_LAYERS). | fait (poussé sur `main`, `4ec7d53`) |
| R3g | Suite de la revue R3e, confiée à Codex `gpt-6.1-sol` high : (1) potentiels par métal en vraies ceintures et provinces liées au contexte tectonique, avec mesure du contraste (part du potentiel portée par 10 % et 20 % des terres) ; (2) emprises de stock organiques modulées par le relief, masse conservée ; (3) couture du détail Render v2 au raccord en longitude, avec test de régression. Livraison par bundle, puis push sur `main` après ma revue **Poussé sur `main` (`ad8c6ba`) à la demande de Gregory, avant la fin de la validation par Codex** : seule une compilation des fichiers a été vérifiée. Restent à faire : `RESOURCE_LAYERS.md` (pas encore mis à jour), parité, benchmark, mesures de contraste. Les nouvelles captures sont dans `docs/images`. `terrain_longitude.py` entre dans l'identité du manifeste, ce qui invalide une nouvelle fois le cache des tuiles neuronales | poussé, validation à finir |
| R3h | **Retour de Gregory : cartes de gisements vides ; « je veux savoir où est quoi » ; les techniques d'extraction ne progressent pas.** Confié à Codex : (1) une couche par ressource, avec des symboles proportionnels au tonnage récupérable, visibles à tous les zooms, et l'emprise réelle en zoom fort ; (2) potentiels et prospectivités relégués dans un groupe « Diagnostic » en bas de liste, car ce ne sont que l'intensité du tirage ; (3) stock contenu retiré de l'affichage ; (4) densité médiévale des gisements calibrée sur des sources historiques (198 gisements seulement pour la graine 42) et correction de la rareté des évaporites ; (5) mise à jour de la doc R3g manquante **Livré et poussé** (`784ca9f`, `e2046f9`, rebasés sur la localisation `44681b3` ; conflit de `web/terrain_resources.js` résolu à la main) : 198 → 11 817 sites ; évaporites à 0,683 % ; compilation en 13,4 s ; premier pixel à chaud en 0,17–0,40 s ; parité de 252 cas sans écart. Restes : densités historiques provisoires, districts denses qui se chevauchent, couture Render v2 toujours là | fait |
| R3i | Corrections, confiées à Codex dans le worktree `../ne-resources-fix` (branche `resources-fix`, port 8775) : locales complètes (fr, en, zh, es, pt) avec un test qui détecte les traductions manquantes ; regroupement à l'écran des sites qui se chevauchent ; vraie cause de la couture au raccord en longitude, avec test de régression **Livré** (`ef3c7a4`, `3aae67b`, `ee20ec6`) : 5 langues, test qui énumère `legend_specs()` (110 spécifications) ; regroupement des sites (gypse au LOD 4 : 1 658 → 464 symboles) ; la couture venait de `/coarse/natural-v1` non raccordé (écart synthétique 628 → 1,6 m), sans vérification visuelle en conditions réelles. | fait (`72919d9`) |
| R3j | **Optimisation : « 13 s c'est infernal, il faut beaucoup moins ».** Confiée à un Codex dédié dans le worktree `../ne-resources-perf` (branche `resources-perf`, port 8776). Cibles : compilation complète ≤ 1,5 s (objectif ambitieux < 0,5 s), première couche < 1 s, changement de couche ≤ 0,3 s. Profilage d'abord. Sorties identiques au bit près à `e2046f9` (budgets, gisements, classes) ; champs flottants dans une tolérance documentée **Passe 1** (`fb3c07f`, `d7a9404`, en bundle) : compilation 13,9–14,8 s → 5,7–6,0 s, sortie identique au bit près, premier NPP à 0,72 s. Reste : gisements 3 s, tectonique 1,6 s, géologie 1 s. **Passe 2 lancée** (même session Codex `01a17e6e`) : l'identité au bit près avec `e2046f9` est levée, au profit d'un test statistique (±10 %) et d'un test de déterminisme d'un run à l'autre. Cible ≤ 1,5 s **Passe 2** (`a862d1c`, `2659f2e`) : gisements vectorisés, géologie sur CUDA, worker préchauffé, cache mappé en mémoire. Compilation 1,01–1,03 s (1,51 s sous charge du viewer), premier NPP en 0,61 s, chargement du cache en 0,04 s. Tests statistiques et de déterminisme OK. Reste : < 0,5 s non atteint, démarrage de l'exécutable environ 4,8 s. | fait (`72919d9`) |
| R3k | **Question de Gregory : « le sel, c'est pas sur toutes les côtes ? »** Confié à Codex (`../ne-resources-fix`) : tout le sel dans la couche « Sel ». On y ajoute deux types de sites côtiers, avec production annuelle et saison : marais salants solaires (côtes basses, chaudes et sèches) et salines à sel bouilli (autres côtes basses, production limitée par le bois ou la tourbe). Le sel gemme et les sources salées restent. Le flux de sel marin continu passe dans « Diagnostic ». Calibrage sourcé, 5 langues, compilation à +10 % au plus. Push sur `main` dès que c'est prêt **Livré** (`16971da`, `20bae6c`) : 1 365 marais salants solaires (54 kt/an) et 3 503 salines à sel bouilli (100 kt/an), soit 88 % des côtes basses à moins de 25 km d'une saline ; 386 sources salées. COMPILER_VERSION passe à 14, compilation en 1,08 s. Coefficients historiques provisoires | fait (`20bae6c`) |
| R6 | Décisions de Gregory avant de pousser : (a) accepter l'invalidation du cache des tuiles neuronales ; (b) LOD d'autorité 10 ou 9 (environ 4 fois le coût) ; (c) géologie de tuile, 5 ou 8 classes de roche ; (d) relire les calibrations provisoires (densités de gisements, tonnages, semence, assolement, pêche, sel, tourbe) **Tranché par la consigne de Gregory « commit, merge, push sur main »** : invalidation du cache neuronal acceptée ; autorité au LOD 10 ; 8 classes de roches ; calibrations provisoires, documentées. | tranché (27/10) |
| R7 | Restes : réinjection dans les textures (O1, O3, O5, nouveaux types sel, gypse et tourbe dans `ground_surface.py`), placers transportés par l'hydrologie, relief à 240 m dans la répartition, sérialisation des deltas Chronos, mondes de diamètre autre que celui par défaut | à faire |
| R5 | Côté Chronos : remplacer les 6 indices par site par les agrégats régionaux et un registre stock / flux / delta. Ce registre est le même que celui du lot A4 (§7.3) : une seule économie, pas deux formules | à faire (après P1.1) |

Pour reprendre le chantier ressources, lance Codex (`codex exec -C ../neural-earth-resources -m gpt-6.1-sol -c model_reasoning_effort=high -s workspace-write`) avec un brief autonome inspiré de l'annexe A. Repartir de `main` (`de2fec6`). Codex ne peut pas écrire dans le `.git` d'un worktree : il livre un bundle git, à récupérer par `git fetch <bundle>`.

### 7.3 Enrichissement depuis Azgaar Fantasy Map Generator

**Statut : proposition documentée, aucune intégration implémentée.** Aucune dépendance FMG à l'exécution.

Deux documents complémentaires :
- **[docs/integration-azgaar.md](integration-azgaar.md) — document de référence.** Il dit *quoi* reprendre et *comment l'intégrer* : contenus (emblèmes, lieux d'intérêt, religions, biens et recettes, voyages, noms, cartographie), trois voies d'intégration, invariants (§5) et critères de validation (§7). Les lots A0–A5 en viennent.
- **[docs/reviews/azgaar-fmg-inspiration.md](reviews/azgaar-fmg-inspiration.md) — étude des règles de génération.** Elle couvre l'expansion, la diplomatie, les routes, la fiscalité et le hasard. Les tâches AZ en viennent ; chacune est rattachée à un palier et à un lot.

Copie de référence : **`../Fantasy-Map-Generator`, révision `67d555b` (v1.156.0)**, épinglée par `integration-azgaar.md`. Le code est dans `src/model/<domaine>/`. L'étude des règles a été faite sur la v1.154.0, dont les chemins `src/generators/` correspondent à `src/model/`. **Avant chaque lot ou tâche, relire le fichier source concerné à cette révision.** Licence MIT : réimplémenter à partir des règles ; toute extraction garde la notice et un manifeste (`integration-azgaar.md` §8).

Les trois voies d'intégration (`integration-azgaar.md` §4) :
1. des **packs de contenu** extraits pendant la préparation (`tools/`), en JSON neutre avec `upstream_revision`, empreintes, licence et clés stables, statut `imported` ;
2. des **règles portées en fonctions Rust pures**, avec clés sémantiques et sans état global, statut `derived` ;
3. des **adaptateurs cartographiques côté client**, qui lisent `World` à une date sans rien tirer.

Un import de carte `.map` reste une option séparée (§4.4).

**Lots (référence : `integration-azgaar.md` §6)**

| Lot | Travail | Dépendance | Tâches AZ liées | Statut |
|---|---|---|---|---|
| A0 | Épingler les sources, extraire un petit pack et ses correspondances, conserver les notices | Aucune | — | à faire |
| A1 | Emblèmes (`EmblemSpec`, stables au fil des successions) et noms complémentaires ; renderer déterministe | A0 | AZ10 | à faire |
| A2 | Lieux d'intérêt dérivés des événements existants (ruine après destruction, auberge, foire liée à une ligue) | A0 ; contrat d'identité et de date | AZ14 | à faire |
| A3 | Méta-modèle d'idées (P0.7), puis semis de croyances, langues et coutumes depuis les catalogues Azgaar ; territoires, provinces, diplomatie datée | P0, P1, puis P2 et P3 | AZ1, AZ3, AZ5, AZ7, AZ11, AZ12 | à faire |
| A4 | Biens, recettes, registre daté de stocks, droits et échanges, avec contreparties ; fiscalité | R2, R5, P0, P2 | AZ15, AZ18 | à faire |
| A5 | Voyages structurés (segments, transports, durées), couches et exports datés | Routes canoniques, projections temporelles | AZ2, AZ6, AZ9 | à faire |

**Tâches de règles de génération (AZ)**

| # | Tâche | Statut |
|---|---|---|
| AZ0 | Étude [azgaar-fmg-inspiration.md](reviews/azgaar-fmg-inspiration.md) : 15 modules lus dans le code. Licence MIT avec autorisation explicite des dérivés ; il est recommandé de réimplémenter à partir des règles plutôt que de traduire le code. FMG est statique (aucune simulation historique), utilise un aléa séquentiel global et une seule échelle | fait (26/10/2026) |
| AZ1 | Moteur d'expansion concurrente à coût : Dijkstra multi-sources pour cultures, États, religions et marchés ; plafond `growthRate` → maturité `m` ; coûts entiers et départage total `(coût, id entité, id cellule)` | à faire (P1.3) |
| AZ2 | Table de transports médiévale et coût de trajet en heures | à faire (P1.1) |
| AZ3 | Type culturel (générique, chasseur, montagnard, fluvial, lacustre, naval, nomade) comme **étiquette lue** dans les activités et le milieu (P0.7). Les préférences de foyer d'Azgaar servent seulement à semer le bootstrap | à faire (P0.7, P1.3) |
| AZ4 | Capacité de charge de référence, indépendante de la résolution | à faire (P1.1) |
| AZ5 | Diplomatie et cascade de guerre (vassaux, alliés qui rompent par peur, neutralité si alliés des deux côtés), résolue une fois à la déclaration, relations datées avec leur cause | à faire (P0.2, P2) |
| AZ6 | Corridors par époque : graphe d'Urquhart, remise ×0,5 sur les tronçons existants, pas de re-routage | à faire (P2, P3) |
| AZ7 | Religions par le **moteur générique d'idées** : diffusion, conversion, divergence (hérésie = `DERIVED_FROM`), syncrétisme, adoption et révocation d'une religion d'État par événement. Folk, organisée, culte et hérésie sont des étiquettes. Poids fort de la dimension croyance (exclusivité, légitimité, cohésion, tabous) | à faire (P0.7, P2) |
| AZ8 | Rangs des sites par règles déclaratives | à faire (P1.3, P2) |
| AZ9 | Zones = vue datée des événements en cours (dont épidémie suivant les routes) | à faire (P2) |
| AZ10 | Noms par chaînes de Markov déterministes | à faire (P1) |
| AZ11 | Formes de gouvernement et rangs comme **étiquettes lues** dans la structure Pecking Order et la composition ; titres culturels tirés d'un catalogue | à faire (P0.6, P0.7, P2) |
| AZ12 | Provinces et colonies | à faire (P1.3, P2) |
| AZ13 | Capacité militaire calculée, jamais stockée | à faire (P2) |
| AZ14 | Vestiges datés : champs de bataille, ruines, ponts délaissés | à faire (P2, P3) |
| AZ15 | Vocabulaire économique | à faire (R5, P2) |
| AZ16 | Verrous et identifiants réservés | à faire (P2) |
| AZ17 | Test : ajouter une entité ne change pas les tirages des autres processus | à faire (P0.5) |
| AZ18 | Fiscalité et commerce politique : taux selon la forme de gouvernement, trésor comme stock, taxes comme flux par bloc, tarifs et embargos datés issus de la diplomatie, dépenses. Respecter les réserves comptables de `integration-azgaar.md` §3.5 (pas de création monétaire sans contrepartie déclarée) | à faire (A4, P0.2, P2) |

Les lots et les tâches AZ complètent P0–P5 sans en changer le statut. Les responsabilités du §3 restent applicables : Azgaar n'ajoute pas un second monde spatial.

## 8. Décisions

| # | Décision | Statut | Notes |
|---|---|---|---|
| D0 | Neural Earth est le seul système multi-échelle ; Magna Urbis sera intégré dedans ; Chronos est la dimension temporelle à l'échelle monde | **décidé** (Gregory, 26/10/2026) | §3 |
| D1 | Licence : `magna-urbis-core` est en GPL-3.0-only, Chronos en MIT/Apache. Relicencier, passer Chronos en GPL, ou n'utiliser Magna Urbis qu'en processus séparé | ouvert | Bloque l'appel à l'exécution (P1.3) |
| D2 | Taille des cellules régionales de Chronos : quel LOD Neural Earth (proposé : LOD 9–10 pour la région, ~15–30 km) | ouvert | Avant P1.1 |
| D3 | Pas de bloc racine : décennie proposée, année pour les crises | proposé | Avant P2 |
| D4 | Format et transport des agrégats Neural Earth vers Chronos : export figé d'abord, puis API | ouvert | Lié à R2 (contrat neutre) |
| D5 | Dépôt d'accueil final des modules communs (adressage des cellules, hasard adressé, contrat des ressources) | ouvert | Après l'intégration Neural Earth / Magna Urbis |

## 9. Journal

- **27/10/2026, R3k poussé** (`20bae6c`) : sel côtier solaire et bouilli dans la couche « Sel ».
- **27/10/2026, R3i et R3j fusionnés et poussés** (`72919d9`) : deux Codex en parallèle dans `../ne-resources-fix` et `../ne-resources-perf` ; conflit de doc résolu (sections 21 à 23) ; 49 tests et 288 cas de parité OK après la fusion.
- **27/10/2026, R3h poussé** : une couche par ressource, gisements médiévaux, groupe Diagnostic. Rebase sur la localisation poussée entre-temps sur `main`.
- **27/10/2026, R3g poussé sans validation complète** (`ad8c6ba`), à la demande de Gregory. Magna Urbis poussé avec le token fourni.
- **27/10/2026, R3d bis, R3e, R3f, fusion** : compilation des ressources en arrière-plan (`39e4e87`, `0bbb6f3`) ; Codex (`gpt-6.1-sol` high) remplace les agents Opus pour coder ; monde Orogen à 4 000 km ; potentiels par métal et districts. Poussé sur `main` : Neural Earth `de2fec6`, Magna Urbis `bb7dcdf`, Chronos (ce commit). Tests relancés avant le push : 62 tests Python OK (ressources, monde serveur, génération, étapes Orogen).
- **26–27/10/2026, R3d livré** : rendu GPU des ressources sur le modèle de Render v2 (`e191185`, `9552428` dans `neural-earth-resources`, non poussé). Roches en provinces cohérentes ; revue des captures : nette amélioration aux LOD 4 et 2, vue monde encore chargée. Viewer sur le port 8775. Réserves en R3d.
- **26/10/2026, reprise Azgaar** : audit du checkout Fantasy Map Generator `67d555b`, rédaction de `docs/integration-azgaar.md`, lien depuis le README et lots proposés A0–A5 en §7.3. Comparaison des capacités avec le code Chronos actuel ; limites explicites du cycle économique, du hasard global et des récits sans causes. Documentation seulement : aucun moteur, schéma, canon ou statut P0–P5 modifié ; aucun benchmark exécuté.
- **26/10/2026** : analyse de la vision et rédaction de `docs/reviews/vision-entites-sociales.md`. Recherche des couches de roches : la lithologie et les sols sont trouvés dans `city_generator/origin/main` (`godot-render/tools/surface_context.py`), sans aucune ressource exploitable. Agent ressources lancé, puis recadré deux fois : (1) niveau monde dans Neural Earth avec Orogen ; (2) Neural Earth comme seule chaîne multi-échelle. Mis en pause en attendant la fusion de Render v2. Création de ce document. Rien n'est commité dans Chronos.
- **26/10/2026, pause de l'agent ressources** : commit WIP `56d78b7` dans `neural-earth-resources` (prototype monde, voir R0). Tests non passants (R0b). `RESOURCE_LAYERS.md` non écrit. À réaligner sur Render v2 : 8 classes de roche, score céréalier par pédologie, tourbe et toute la réinjection dans les textures. Littérature non vérifiée : Campbell (boisseaux par acre), Rackham (rotations), sel de Lüneburg.
- **26/10/2026, Render v2 fusionné** dans `infinite_map/main` (`d6cbb78`) ; agent ressources relancé (R1) ; ajout de R3b (layers informatives dans le viewer).
- **26/10/2026, chantier ressources terminé (non poussé)** : Neural Earth `847abc3`, `61ce0c3`, `a583658` ; Magna Urbis `bb7dcdf`. 121 layers dans le viewer, 16 tests OK. Cache écrit dans `E:\TerrainDiffusionRuntime\resources`. Décisions en R6, restes en R7.
- **26/10/2026 (nuit), R3c livré** : descente multi-échelle des ressources avec conservation (`be8eb4f`, `8c7d13e`), viewer relancé sur le port 8775. Réserves de la revue de captures et performance en R3c ; à valider visuellement par Gregory.
- **26/10/2026 (soir), méta-modèle** : à la demande de Gregory, on ne copie plus la structure d'Azgaar. Ajout de la vision §7.8 : idées, adhésions, appuis, moteur de diffusion générique, étiquettes dérivées. La religion reste un moteur majeur, avec un poids fort, sans code propre. Répercuté dans `integration-azgaar.md` (§1.1, §3.3, §3.4, §4.2, §5, lot A3), dans la vision §7.2–7.4 et §7.7, et dans le suivi (P0.7, AZ3, AZ7, AZ11, A3, concepts).
- **26/10/2026 (soir), réconciliation Azgaar** : pull de `a12fc50` (`docs/integration-azgaar.md`, lots A0–A5). Fusion avec l'étude des règles (AZ0–AZ18) en un seul §7.3, où chaque lot est relié à ses tâches AZ. Ajout de l'ordre de bataille §7.0, avec la piste rapide A0–A2 avant P0. Design mis à jour dans la vision : §7.2 (adaptation, forme), §7.3 (bootstrap), §7.4 (mécanismes de bloc), §7.7 (apports retenus). `CLAUDE.md` et `AGENTS.md` pointent vers ce document. Clone de référence FMG : `../Fantasy-Map-Generator` `67d555b` (v1.156.0).
- **26/10/2026, étude Azgaar FMG** : `docs/reviews/azgaar-fmg-inspiration.md` ; tâches AZ1–AZ17 reportées au §7.3. FMG n'apporte rien à P4 (aucune personne ni dynastie).

## Annexe A — Brief de relance de l'agent ressources

```text
Objectif : extension « ressources » de Neural Earth (D:\Workspace\Self\RPG\infinite_map), en français pour la doc.
Travailler dans le worktree D:\Workspace\Self\RPG\neural-earth-resources, branche resources-layers.
Rebaser d'abord sur origin/main, qui doit contenir Render v2. Commits locaux seulement, pas de push.
Ne pas toucher aux checkouts principaux de infinite_map ni de city_generator.

Cadrage :
- Neural Earth est le seul système multi-échelle (LOD 11 = 61 km … LOD -6 = 47 cm, docs/architecture.md).
  Modèle d'architecture : docs/design/multiscale-hydrology.md.
- Magna Urbis = cités sur les tuiles Neural Earth de 40 km, à intégrer dans Neural Earth.
  Son godot-render/tools/surface_context.py (géologie, sols, textures) est du traitement de tuile Neural Earth.
- Chronos = temps à l'échelle monde : lit les agrégats régionaux, écrit le delta historique.

À livrer :
- docs/RESOURCE_LAYERS.md :
  - ontologie potentiel / stock / flux, saisonnalité ;
  - provinces métallogéniques depuis Orogen (plates, crust, boundaries, convergence, uplift, superPlates, hotspot) ;
  - LOD de décision de chaque fait ;
  - règles multi-échelles : agrégats conservés, budgets hérités, hasard adressé (seed, couche, LOD, cellule),
    indépendance à l'ordre des requêtes ;
  - contrat d'interface neutre, avec provenance imported / derived / synthetic ;
  - état naturel immuable + delta Chronos ;
  - réinjection dans les textures (O1, O3, O5 de GROUND_MATERIAL_MAPPING.md) ;
  - fantasy optionnelle ;
  - section « Intégration Neural Earth – Chronos – Magna Urbis ».
- Prototype de layers et tests (déterminisme, conservation, ordre, cohérence lithologie-gisements).
- Note courte côté Magna Urbis (worktree magna-urbis-resources) : migration de geology() et surface_context.py,
  consommation par les cités.

Sources :
- NeuralEarth-Resources*.md (dépôt chronos) ;
- infinite_map/docs (layers, materials, render-v2*, soil literature), backend/terrain_orogen*.py,
  terrain_koppen.py, terrain_biomes.py, terrain_pedology.py ;
- city_generator/docs/GROUND_*.md ;
- littérature, vérifiée et citée : USGS Cox & Singer 1986, FAO GAEZ / Ecocrop, Lieth (Miami), Damuth,
  rendements médiévaux, Dwarf Fortress, Azgaar.

Rapport final : fichiers, layers (unité, nature, saisonnalité, LOD), règles, tests (commande + résultat),
mesures, commits, décisions ouvertes.
```
