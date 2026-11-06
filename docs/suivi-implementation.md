# Suivi d'implémentation — entités sociales, multi-échelle, ressources

Document de reprise. Il permet de reprendre le développement dans une nouvelle conversation sans autre contexte. **Mets-le à jour à chaque session** : statut des tâches (§7), journal (§9), décisions (§8).

Dernière mise à jour : 06/11/2026. Statut global : **design social v2 détaillé, implémentation non commencée.** Rien des paliers P0–P5 ni des nouveaux lots S0–S8 n'est implémenté. Le statut du chantier ressources décrit ci-dessous est celui consigné le 26/10 et n’a pas été réaudité dans cette session.

**Point d’entrée actuel pour Chronos : [design social v2](design/chronos-social-v2/README.md).** Il intègre les remarques des deux tours de discussion, fixe le pilotage interne de Lineage Lab, les proportions statistiques par défaut et la première livraison S0–S5. Les paliers historiques P0–P5 restent des références ; la réécriture Rust de la généalogie et l’exactitude systématique des budgets ne sont plus des prérequis. Aucun moteur n’a changé dans cette session.

## 0. Reprendre en une minute

Message à coller au début d'une nouvelle conversation, depuis `D:\workspace\self\chronos` :

```text
Lis le design docs/design/chronos-social-v2/README.md et les contrats de son dossier, puis ce suivi.
Vérifie git status et les chemins actuels ; le tableau de worktrees du §5 est historique.
Commence par S0 du §7.4, puis la première tranche non terminée S0–S5 (ou la tâche indiquée).
Ne reprends pas P0.1 et ne remplace pas le moteur v1 : P0–P5 sont l'ancien plan.
Mets à jour §7 et §9 à la fin de la session.
```

Règles de travail (rappel de `CLAUDE.md`, qui fait foi) :
- documents, interface et diagnostics en français ; séparer **proposé / implémenté / mesuré** ;
- déterminisme : noyau Rust à clés sémantiques, `BTreeMap`/`BTreeSet` ; worker v2 à plan immuable et runtime épinglé, sans flux partagé entre requêtes ;
- `chronos-core` sans E/S ni horloge (`cargo check -p chronos-core --target wasm32-unknown-unknown`) ;
- changement de sémantique : versions et canon changent ; méthode v1 dans `docs/methods/chronology.md`, contrats et méthode v2 dans son dossier, sans réécriture de la méthode v1 ;
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
| 1b | [Design social v2](design/chronos-social-v2/README.md) et ses contrats JSON | Spécification courante d’implémentation issue des commentaires du 06/11 ; arbitrages prioritaires sur les anciennes propositions |
| 2 | [docs/reviews/vision-entites-sociales.md](reviews/vision-entites-sociales.md) | Vision, état actuel, autres projets, manques, faisabilité, design macro (§7), paliers |
| 3 | [conception_generateur_histoire_multiechelle_v3.md](../conception_generateur_histoire_multiechelle_v3.md) §4A, §4.5, §10, §14–18 | Conception de référence : entité réifiée, propriété des événements, raffinement et budgets, architecture Rust, déterminisme, validation, phases |
| 4 | [docs/methods/chronology.md](methods/chronology.md) | Algorithme implémenté aujourd'hui et ses limites |
| 5 | `../infinite_map/docs/architecture.md` (table des LOD) et `../infinite_map/docs/design/multiscale-hydrology.md` | Modèle multi-échelle de Neural Earth, à imiter (autorité globale, raffinement, propriété des frontières) |
| 6 | `NeuralEarth-Resources-Starter.md`, `NeuralEarth-Resources.md`, `NeuralEarth-Resources-Fantasy.md` (racine de ce dépôt) | Spécification des 35 couches de ressources |
| 7 | `../city_generator/docs/GROUND_MATERIAL_MAPPING.md` §2–5 et `docs/GROUND_TEXTURE_MAPPING_V1.md` | Lithologie et sols de tuile, choix des matériaux, extensions O1–O10 |

## 3. Architecture de l'écosystème

**Ce cadrage est décidé par Gregory. Ne pas le remettre en cause.**

| Projet | Rôle | Échelle |
|---|---|---|
| **Neural Earth** (`../infinite_map`) | **Seul système multi-échelle.** Relief, climat, hydrologie, sols, matériaux ; à terme, ressources et cités | De la planète au cm : LOD 11 = 61,44 km … LOD 7 = 3,84 km, LOD 3 = 240 m, LOD 0 = 30 m, LOD -6 = 47 cm |
| **Magna Urbis** (`../city_generator`) | Cités et établissements (`Terrain::place`, `Terrain::roads`, cultures). Travaille sur les **tuiles Neural Earth de 40 × 40 km** et utilise quelques échelles propres pour ses villes. **Sera intégré dans Neural Earth.** Son compilateur de sol (`godot-render/tools/surface_context.py`) est du traitement de tuile Neural Earth, hébergé ici provisoirement | Tuile 40 km, environ 30 m, puis échelles urbaines |
| **Chronos** (ce dépôt) | **Dimension temporelle à l'échelle monde.** Entités sociales, chronique canonique, raffinement temporel. Lit les agrégats de Neural Earth et porte le **delta historique** (extraction, déforestation, défrichement, fondations) | Régions (LOD grossiers) × blocs de temps |
| **Pecking Order** (`../pecking-order`) | Catalogue des structures internes : rôles, `level`, `reports_to`, `powers`, `collective`. Crate Rust | — |
| **Lineage Lab** (`../lineage-lab`) | Moteur démographique Python à piloter comme outil interne de Chronos v2, avec noyau + cohortes ; adaptateur historique à développer. L’import figé reste le comportement v1 | Personnes |
| **Namebench** (`../namebench`) | Noms et toponymes. Non déterministe : utiliser des pools figés | — |
| **ttrpg_content_generator** | Modèle d'origine (`doc/generation_schema.pptx`), catalogues de rôles, liens, cultures et races | — |

Principe commun : identités spatiales fournies par Neural Earth et hasard adressé pour les faits Chronos. Le worker Lineage Lab emploie ses flux isolés de plan et son runtime épinglé ; on ne prétend pas partager son RNG NumPy avec les autres moteurs. L’état naturel et le delta historique restent séparés.

## 4. Concepts clés

| Concept | Définition courte | Référence |
|---|---|---|
| `SocialEntity` | Type commun v2, avec données personnelles et institutions typées séparément ; le modèle v1 reste conservé | design v2 §5 |
| Nature | V2 : personne, groupe organisé, communauté, catégorie ou population ; capacité d’action séparée | design v2 §5 |
| Résolution | `canonique`, `esquisse` (comptes et agrégats) ou `virtuelle` (slot non résolu) | vision §7.2 |
| Relations | Similarités calculables dérivées ; faits particuliers datés stockés, y compris confiance, amitié et entraide. Parentés sous l’autorité du fournisseur | design v2 §5 et §9 |
| Cohortes | La population est comptée une seule fois, dans des cohortes disjointes. Une adhésion ne crée pas de personne | vision §7.2 |
| Canon racine | Simulation par **régions et entités saillantes**, en **blocs** (décennie ; année pour les crises), jamais par site | vision §5, §7.4 |
| Propriété | Propriétaire et adresse sémantique uniques ; v2 utilise l’adresse sans perte pour l’identité, SHA-256 pour les empreintes et des clés distinctes pour l’aléa | design v2 §4 |
| Précision des mesures | V2 : proportions attendues par défaut, réalisations locales pouvant diverger. Quotas et transferts exacts seulement quand le contrat le demande ; source, précision et couverture séparées | design v2 §4, §7 |
| Pas de remontée | Un enfant ne modifie jamais son parent. Tout effet de portée supérieure est réservé en amont | v3 §4.5 |
| Maturité `m` | Paramètre unique du bootstrap : colonisation ≈ 0,1–0,2, jeune ≈ 0,4–0,6, vieille ≈ 0,9 | vision §7.3 |
| Potentiel / stock / flux | Ressources : aptitude intrinsèque, quantité épuisable, rendement renouvelable (saisonnier si utile) | §7.2 |
| Provenance | `imported`, `derived`, `synthetic` ou `unavailable`. Un repli n'est jamais présenté comme importé | `CLAUDE.md` |

## 5. Dépôts, branches et fichiers

Le tableau suivant est un relevé **historique du 26/10**. Dans ce workspace, les chemins actifs sont `D:\workspace\self\chronos`, `../magna-urbis`, `../infinite_map`, `../lineage-lab`, `../namebench` et `../pecking-order`. Les anciennes mentions `city_generator` et `D:\Workspace\Self\RPG` ne sont pas des chemins de reprise courants. Le design et les discussions du 06/11 sont la documentation nouvelle ; leur état git se vérifie à la reprise.

| Où | Quoi | État au 26/10/2026 |
|---|---|---|
| `chronos` `main` | Prototype J0–J4 (`crates/chronos-core`, `crates/chronos-app`, `web/`, `fixtures/stack.json`) | Non commités : `CLAUDE.md`, `AGENTS.md` (modifié), `docs/reviews/vision-entites-sociales.md`, ce document |
| `city_generator` (Magna Urbis), `origin/main` | Contient le moteur Godot (`godot-render/`), `surface_context.py`, la doc des matériaux | Checkout local sur `codex/render-v2-snapshot-2026-10-23` : **ne pas y toucher** |
| `D:\Workspace\Self\RPG\magna-urbis-resources` | Worktree Magna Urbis, branche `resources-layers` partie de `origin/main` | Chantier ressources, en pause |
| `infinite_map` (Neural Earth), `origin/main` | Orogen (`plates`, `crust`, `boundaries`, `convergence`, `uplift`, `superPlates`, `hotspot`), Köppen, biomes, pédologie, hydrologie multi-échelle | Render v2 (`ce7f8cb`, branche `codex/render-v2-snapshot-2026-10-23`) **en cours de fusion dans main par Gregory** |
| `D:\Workspace\Self\RPG\neural-earth-resources` | Worktree Neural Earth, branche `resources-layers` partie de `origin/main` `c0e068b`, **sans Render v2** | Chantier ressources, en pause ; à rebaser |

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

### 7.1 Paliers Chronos (vision §7.6)

Les paliers ci-dessous décrivent le plan antérieur. Pour démarrer l’implémentation sociale, suivre les lots S0–S8 du §7.4 et les arbitrages D6–D13.

**P0 — Modèle `SocialEntity`** : `à faire`. Preuve de sortie : même démo et mêmes invariants, sur le nouveau modèle.

| # | Tâche | Statut |
|---|---|---|
| P0.1 | Spécifier `SocialEntity` (champs vision §7.2), `EntityKind`, `Nature`, `Resolution`, et le cycle de vie (`SUCCESSOR_OF`, `SPLIT_FROM`, `MERGED_INTO`) dans un doc de méthode avant le code | à faire |
| P0.2 | Arêtes stockées datées : `MemberOf{role}`, `Governs`, `Owns`, `AlliedWith`, `TradesWith`, `Debt`, `Rivalry`, `Friendship`, `Grievance`, avec `cause` (ID d'événement) et intervalle de validité | à faire |
| P0.3 | Relations d'affinité calculées à la demande : sang, culture, proximité, idéologie, activité. Index, aucun stockage | à faire |
| P0.4 | Migrer `Polity`, `Organization` et `Person` vers `SocialEntity`. Le JSON exporté change : nouvelle `SCHEMA_VERSION` et `ALGORITHM_VERSION`, nouvelle génération de référence | à faire |
| P0.5 | Adapter `validate.rs`, l'inspecteur web et les tests d'invariants. Ajouter des tests d'acyclicité de la composition et d'unicité des cohortes | à faire |
| P0.6 | Exploiter tout Pecking Order (niveaux, `reports_to`, pouvoirs, thèmes) pour la structure interne | à faire |

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

Statut : **en pause**, en attente de la fusion de Render v2 dans `infinite_map/main`.

Objectif : des layers de ressources du monde, raisonnées en **potentiel, stock et flux**, saisonnières si nécessaire, dans la chaîne des LOD de Neural Earth.
- **Autorité globale** : Orogen. Le contexte tectonique donne les provinces métallogéniques et lithologiques : arcs de subduction (Cu-Au porphyres, Au-Ag épithermaux), granites de collision (Sn-W), ophiolites et VMS (Cu, Cr), cratons (fer rubané, Au), rifts et bassins arides (évaporites), plus le charbon et la tourbe.
- **Raffinement** de chaque LOD par son parent, avec agrégats conservés et budgets hérités. Les gisements discrets reçoivent un tonnage vers 480–240 m ; les affleurements et chapeaux de fer sont placés à 30 m ou plus fin.
- **Réinjection dans le choix des textures** aux LOD fins, via les extensions O1, O3 et O5 de `GROUND_MATERIAL_MAPPING.md`.
- **Lien avec Chronos** : Chronos lit les agrégats régionaux et écrit le delta historique (épuisement, déforestation).
- **Côté Magna Urbis** : une note courte sur la migration de `geology()` et `surface_context.py` vers un raffinement Neural Earth, et sur ce que consomment les cités.

| # | Tâche | Statut |
|---|---|---|
| R0 | Prototype monde `neural-earth-resources/backend/terrain_resources.py`, commit WIP `56d78b7`, non poussé. Contenu : adressage `(LOD, i, j)` sur 30·2^LOD m avec ID 64 bits, parent et enfants ; aires sphériques exactes ; portage vérifié de `keys.rs` ; budgets entiers exacts (plus forts restes). Contextes tectoniques Orogen → 8 classes de roche. Climat mensuel ; NPP Miami ; céréales de type GAEZ ; pâture mensuelle et transhumance ; bois ; gibier (Damuth) ; pêche ; sel marin ; tourbe ; fer des marais ; matériaux. 16 familles de gisements discrets (Poisson, tonnage et teneur log-normaux, placés au LOD 3). Fantasy désactivée par défaut. Export régional et delta historique (extraction, défrichement, repousse). **Mesuré** : monde au LOD 10 (652 × 1304) en 28,6 s, 8 000 gisements (seed 7985403534069300490, « continents »). Worktree Magna Urbis non modifié | fait (WIP) |
| R0b | Faire passer `tests/python/test_terrain_resources.py`. Cause du blocage : au LOD 14, placer un gisement au LOD 3 évalue environ 4 M de cellules enfants par gisement. Remède : descente par étapes ou profondeur limitée | à faire |
| R1 | Rebaser `neural-earth-resources` sur le nouveau `origin/main`. Render v2 a été fusionné le 26/10/2026 (merge `d6cbb78`, tête `051dd1f`). Puis inventorier toutes les layers : Render v2 (`docs/design/render-v2*.fr.md`, `backend/terrain_render_v2.py`, `render-v2-soil-literature.fr.md`, `docs/layers.md`, `docs/materials.md`) et les textures côté Godot dans Magna Urbis. Enfin, réaligner le prototype | en cours (agent relancé) |
| R2 | Étude `neural-earth-resources/docs/RESOURCE_LAYERS.md` : ontologie, provinces, LOD de décision, saisonnalité, règles multi-échelles, contrat d'interface neutre, section « Intégration Neural Earth – Chronos – Magna Urbis », réinjection dans les textures, fantasy optionnelle | à faire |
| R3 | Prototype de layers monde + tests (déterminisme, conservation des agrégats, ordre des requêtes, cohérence des gisements avec la lithologie) dans Neural Earth | à faire |
| R3b | Toutes les layers de ressources sont visibles comme **layers informatives** dans le viewer Neural Earth (groupe « Ressources », même mécanisme que les layers Orogen et Pedology). Sélecteur saisonnier, provenance dans la légende, gisements avec tonnage, teneur et accessibilité médiévale, et `docs/layers.md` mis à jour | à faire (demandé par Gregory) |
| R4 | Note Magna Urbis : migration de `geology()` et de `surface_context.py`, consommation par les cités | à faire |
| R5 | Côté Chronos : remplacer les 6 indices par site par les agrégats régionaux et un registre stock / flux / delta | à faire (après P1.1) |

Si l'agent n'est plus joignable (nouvelle conversation), relance-le avec le brief de l'annexe A, en précisant de reprendre le WIP des branches `resources-layers`.

### 7.3 Enrichissement depuis Azgaar

**Statut : proposition documentée, aucune intégration implémentée.** Voir [integration-azgaar.md](integration-azgaar.md) : capacités présentes dans Fantasy Map Generator, écarts avec Chronos, pièces à extraire, contrats et critères de validation. Les reprises passent par des packs de contenu figés, des fonctions Rust pures et des projections cartographiques ; les responsabilités de §3 restent applicables.

| Lot proposé | Portée | Statut |
|---|---|---|
| A0–A2 | Extraction sourcée, emblèmes, noms et lieux d'intérêt dérivés des faits existants | à faire |
| A3 | Croyances, territoires et organisations, articulés avec P0–P3 | à faire |
| A4 | Biens, recettes et registre économique, après contrat des ressources et R5 | à faire |
| A5 | Voyages structurés, couches et exports datés | à faire |

Ces lots complètent P0–P5 ; ils n'en changent pas le statut. Le document n'ajoute pas de dépendance FMG à l'exécution.

### 7.4 Livraison sociale v2

La spécification est rédigée ; les statuts ci-dessous concernent uniquement le code à développer. Le dossier contient le design, un raccord de 79 styles Magna Urbis vers 21 familles de paramètres, un schéma de plan démographique et un exemple de contrat.

| Lot | Travail | Statut |
|---|---|---|
| S0 | Types, clés, atlas, contrats d’impact et validation v2 | à faire |
| S1 | Profils culturels et archétypes exécutables ; intégrer le raccord préparé | à faire |
| S2 | Canon général, régions, domaines fixes et contexte extérieur | à faire |
| S3 | Worker Lineage Lab et programme historique noyau/cohortes | à faire |
| S4 | Complément procédural, recrutement, mandats et groupes informels | à faire |
| S5 | API, stockage, carte, graphes interactifs, chronologie et bilans | à faire |
| S6 | Scénarios exacts, personnes requises et branches | à faire |
| S7 | Liaisons exactes d’acteurs nommés entre domaines | à faire |
| S8 | Mesures, checkpoints, sélection paresseuse, grands sites, fournisseur démographique procédural, profils mixtes, nouveaux sites dynamiques et formats binaires éventuels | à faire |

Après S5, à planifier : portage de Lineage Lab en Rust (D12), comme nouveau fournisseur démographique sous le même contrat.

Première livraison utilisable : S0–S5. Les contraintes exactes avancées et les liaisons nominatives entre domaines sont des extensions identifiées. La source de vérité des contrats et critères de sortie est [le design](design/chronos-social-v2/README.md).

## 8. Décisions

| # | Décision | Statut | Notes |
|---|---|---|---|
| D0 | Neural Earth est le seul système multi-échelle ; Magna Urbis sera intégré dedans ; Chronos est la dimension temporelle à l'échelle monde | **décidé** (Gregory, 26/10/2026) | §3 |
| D1 | Licence : `magna-urbis-core` est en GPL-3.0-only, Chronos en MIT/Apache. Relicencier, passer Chronos en GPL, ou n'utiliser Magna Urbis qu'en processus séparé | ouvert | Bloque l'appel à l'exécution (P1.3) |
| D2 | Taille des cellules régionales de Chronos : quel LOD Neural Earth (proposé : LOD 9–10 pour la région, ~15–30 km) | ouvert | Avant P1.1 |
| D3 | Pas de bloc racine : décennie proposée, année pour les crises | proposé | Avant P2 |
| D4 | Format et transport des agrégats Neural Earth vers Chronos : export figé d'abord, puis API | ouvert | Lié à R2 (contrat neutre) |
| D5 | Dépôt d'accueil final des modules communs (adressage des cellules, hasard adressé, contrat des ressources) | ouvert | Après l'intégration Neural Earth / Magna Urbis |
| D6 | Causalité descendante et cohérence statistique ; émergence ascendante facultative | **décidé** par les commentaires utilisateur du 06/11 | design v2 §1 |
| D7 | Chronos pilote Lineage Lab comme outil interne ; complément procédural pour monter en échelle | **décidé** par les commentaires utilisateur du 06/11 | Protocole et adaptations proposés en §8 et §13 |
| D8 | Proportions attendues par défaut ; exactitude explicite pour scénarios et engagements | **décidé** par les commentaires utilisateur du 06/11 | Révise la généralisation des budgets exacts |
| D9 | Domaine local fixe, horizon fixe, worker Python depuis l’hôte et paquets canoniques | choix technique proposé pour implémentation | Préserve le noyau pur et le rejeu |
| D10 | Styles Magna Urbis regroupés en familles historiques avec variantes ; graphes interactifs et bilans dès le prototype | orientation validée par les commentaires utilisateur du 06/11 | Raccord et choix graphiques proposés dans le dossier |
| D11 | Frontières statistiques dans S0–S5 ; suivi nominatif exact par contrat de liaison dans S7 | périmètre technique proposé | Limite déclarée, aucune fonctionnalité existante prétendue |
| D12 | Lineage Lab reste en Python pour S0–S5 ; il devra passer en Rust à terme | **décidé** (Gregory, 06/11/2026) | Migration non planifiée, à placer après S5. Elle n'est pas un prérequis : la démographie reste derrière le contrat de fournisseur. Un moteur Rust sera un nouveau fournisseur versionné, donc de nouveaux canons, sans rejeu à l'identique des mondes Python |
| D13 | Alignement et légalité dès S0–S5 : personnes et organisations typées de « sans foi ni loi » à « saint », mafia / lambda / vertueuse ; axes gradués et valeurs culturelles ; légalité par juridiction et datée | **décidé** (Gregory, 06/11/2026) | Design v2 §27. Valeurs initiales proposées, non calibrées |

Pour S0–S5, D1/D2/D4 ne bloquent pas le développement : l’atlas et les géométries sont des exports figés, la fixture définit explicitement cellules/routes/sites, et aucun lien d’exécution nouveau avec Magna Urbis n’est requis. Ces décisions restent à résoudre pour les intégrations d’hôte ultérieures, sans inventer une seconde grille dans Chronos.

## 9. Journal

- **06/11/2026, design social v2** : prise en compte des deux tours de commentaires ; préparation de `docs/design/chronos-social-v2/` avec modèle social, hasard indexé, plans démographiques, pilotage interne de Lineage Lab, raccord culturel, recrutement, mandats, impacts statistiques ou exacts, monde ouvert, visualisations, API et lots S0–S8. Discussion et annotations utilisateur conservées. Relectures indépendantes par Claude Opus 5.5 et intégration des corrections sur les cycles, registres démographiques, mobilité, noyau, profils et contrats autonomes. Documentation et contrats seulement : aucun changement de moteur, aucune mesure de performance nouvelle.
- **26/10/2026, reprise Azgaar** : audit du checkout Fantasy Map Generator `67d555b`, rédaction de `docs/integration-azgaar.md`, lien depuis le README et lots proposés A0–A5 en §7.3. Comparaison des capacités avec le code Chronos actuel ; limites explicites du cycle économique, du hasard global et des récits sans causes. Documentation seulement : aucun moteur, schéma, canon ou statut P0–P5 modifié ; aucun benchmark exécuté.
- **26/10/2026** : analyse de la vision et rédaction de `docs/reviews/vision-entites-sociales.md`. Recherche des couches de roches : la lithologie et les sols sont trouvés dans `city_generator/origin/main` (`godot-render/tools/surface_context.py`), sans aucune ressource exploitable. Agent ressources lancé, puis recadré deux fois : (1) niveau monde dans Neural Earth avec Orogen ; (2) Neural Earth comme seule chaîne multi-échelle. Mis en pause en attendant la fusion de Render v2. Création de ce document. Rien n'est commité dans Chronos.
- **26/10/2026, pause de l'agent ressources** : commit WIP `56d78b7` dans `neural-earth-resources` (prototype monde, voir R0). Tests non passants (R0b). `RESOURCE_LAYERS.md` non écrit. À réaligner sur Render v2 : 8 classes de roche, score céréalier par pédologie, tourbe et toute la réinjection dans les textures. Littérature non vérifiée : Campbell (boisseaux par acre), Rackham (rotations), sel de Lüneburg.

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
