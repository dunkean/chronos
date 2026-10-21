# Chronos — roadmap du générateur de chronologie

Date : 21 octobre 2026. Statut : prototype local intégré livré. La roadmap a été revue et corrigée par Opus 5.5 en effort high, puis relue pendant l’intégration. Les jalons restent le plan d’évolution ; l’état réellement livré est décrit ci-dessous.

## État livré

- Moteur Rust déterministe, CLI et atelier local : réglages, carte, chronologie, fiches liées, navigation par année, import/export.
- Toute la stack est utilisée dans une fixture autonome : contexte initial Neural Earth/Orogen, placement et routes Magna Urbis, noms Namebench, institutions Pecking Order et personnes/familles Lineage Lab.
- Démo : petite planète de 1 600 km de diamètre, zone de 200 km découpée en quatre tuiles, 52 lieux et 71 liaisons ; 200 ans, 325 événements et 1 846 personnes importées dans la génération de référence.
- Deux modes explicites : scénario de l’étude (`pivot`, faits de 1356/1398 imposés) et histoire libre (`free`, événements issus des paramètres). Les offices collectifs ne reçoivent pas de titulaire individuel.
- Vérification : 10 tests Rust, formatage, Clippy, compilation WASM du noyau, validation de la fixture et parcours réels dans Chrome sur bureau et écran étroit.

Cette livraison réalise la tranche locale de J0–J4. Le graphe stratégique couvrant la planète entière, le raffinement paresseux, le présent imposé, la réconciliation démographique complète et les routes historiques évolutives restent à développer. Les ressources sont des potentiels simplifiés ; la démo ne fournit ni rivières ni lacs détaillés.

Lancement : [README.md](README.md). Méthode et limites : [note du moteur](docs/methods/chronology.md). Provenance : [données](docs/data-sources.md). Preuves de vérification et corrections de revue : [intégration du prototype](docs/reviews/prototype-integration.md).

Référence : [étude multiscalaire v3](conception_generateur_histoire_multiechelle_v3.md), notamment les sections 4A, 9–12 et 14–18. Cette roadmap adapte son démarrage : contexte de carte initial de Neural Earth, réutilisation des mécanismes Rust de Magna Urbis, graphe virtuel conservé comme repli et pour les essais autonomes. Le plan de l'étude (section 18) part du contrat d'invariance et du raffinement ; ici, la première livraison est une démo intégrée générée intégralement à la racine sur un petit monde, et le raffinement à la demande vient ensuite.

## 1. Résultat recherché et première livraison

Chronos produit une histoire datée d'un monde : lieux, communautés, organisations, pouvoirs, personnes importantes, ressources, relations et événements. À terme, le détail se développe à la demande sans changer les faits quand on visite les lieux dans un autre ordre.

La **première livraison** (fin de J4) est une application locale de prototype : moteur Rust, interface web pour configurer les paramètres et explorer une génération, carte liée à une chronologie. Elle permet de sélectionner une ville, une organisation ou une personne nommée, de changer d'année, de consulter les événements et leurs effets, d'exporter puis de rouvrir une génération.

Elle relie les cinq sources voisines : contexte cartographique de Neural Earth, établissements et routes de Magna Urbis, noms de Namebench, institutions de Pecking Order, personnes et familles de Lineage Lab. Chaque source porte un statut visible — **importée**, **approximée** ou **repli virtuel** — avec sa provenance. Une source indisponible ne bloque pas la démo : son repli est explicite, jamais présenté comme la donnée réelle.

Le prototype privilégie une tranche complète, rapide et lisible. Aucun moteur planétaire exhaustif, langage de règles, plateforme de validation ou système de plugins ne précède la première démo. Aucun LLM ne décide des faits canoniques.

## 2. Décisions de départ

- **Rust pour les calculs**, dans une bibliothèque sans E/S ni dépendance au navigateur. Un exécutable local sert l'interface et lit les fichiers. Le noyau reste compatible WASM par construction ; la vérification `wasm32` est une tâche non bloquante, obligatoire seulement pour l'intégration aux hôtes (J8). L'interface ne réimplémente pas les règles historiques.
- **Carte réelle dès la première tranche connectée** : importer un instantané du contexte initial de Neural Earth (champs Orogen et atlas). Ne pas déclencher l'inférence neuronale fine.
- **Deux fournisseurs interchangeables** : instantané cartographique importé et monde virtuel déterministe. Ils alimentent le même format ; les essais ne nécessitent ni serveur GPU ni projet voisin.
- **Villes et routes** : appeler `magna-urbis-core` comme dépendance Rust locale via un adaptateur, sans recopier son moteur ni construire de bâtiments. Ses sorties sont figées dans le bootstrap ; le canon ne dépend pas d'une réexécution ultérieure.
- **Mobilité historique en distances et temps** : Chronos calcule un graphe grossier de coûts entre régions ; Magna Urbis fournit la géométrie dans les régions détaillées (§3.3). Les tracés ne sont pas recalculés au fil des siècles dans cette phase.
- **Génération intégrale à la racine d'abord** : pour le petit monde de démo, tous les faits sont produits par la génération racine dans un ordre canonique ; la navigation ne fait que lire. Le raffinement paresseux (J5) arrive quand la taille l'exige.
- **Mode prospectif d'abord** : établissements et populations à une date initiale, puis histoire résultante. Le présent imposé vient plus tard (J7).
- **Tests proportionnés** : quelques régressions sur les invariants difficiles, compilation et essai navigateur ; pas de couverture arbitraire.

## 3. Réutilisation des projets disponibles

Les observations ci-dessous proviennent des sources locales, lues le 21 octobre 2026 ; elles ne constituent pas une validation d'intégration.

| Projet | Apport retenu | Travail à prévoir |
|---|---|---|
| `../infinite_map` — Neural Earth | Contexte initial : points Orogen sur la sphère et cinq champs physiques (altitude signée en m, BIO1 °C, BIO4 °C×100, BIO12 mm, BIO15 %), diamètre de planète, seed et empreinte du monde. L'atlas Rust est un raster équirectangulaire 1 024 × 512 de ces champs. | Outil d'export numérique versionné de ces entrées ; procédure exacte (backend Python ou worker Orogen) à confirmer en J1. Pas de rivières, lacs ni sols dans ces champs. Ne pas utiliser une capture du rendu comme donnée. |
| `../city_generator/rust` — Magna Urbis | `settlements::Terrain::new(Raster)`, `place`, `roads` ; établissements, segments, jonctions, franchissements, sorties de bord et liaisons impossibles. | Raster régional projeté, sites-passerelles vers le graphe grossier, IDs stables. Vérifier qu'une dépendance `path` hors de son workspace compile. Pas d'urbanisation mondiale. |
| `../pecking-order` | API Rust 2.0 en cours d’évolution : système, thème, taille, structure, autorité ; rôles, fonctions, titres et pouvoirs. | Figer les réponses et l’empreinte du catalogue effectivement utilisé. Mapper les autorités connues à des opérations simples explicitement définies par Chronos ; ne pas exécuter une description textuelle. |
| `../namebench` | Noms de personnes par pays et genre, toponymes, styles fantasy. `all_names_pool` / `all_places_pool` chargent un pool complet sans tirage SQL. | Exporter des pools triés avec empreinte, puis tirer par clé stable en Rust. Les requêtes `ORDER BY RANDOM()` ne garantissent pas le replay. |
| `../lineage-lab` | Archive SQLite versionnée : naissances, décès, unions, filiations, résidences et rôles (maisons nobles, lignées marchandes, héros roturiers…) du noyau simulé ; identités stables par configuration et version d'algorithme. | Exporter un petit échantillon du noyau avec IDs externes ; table explicite de correspondance des établissements ; décalage de calendrier. Aucun champ de nom personnel n'a été repéré dans ses sources : Namebench nomme les personnes importées. La promotion rétroactive d'un résident d'arrière-plan n'existe pas. |
| `../genealogy`, `../ttrpg_content_generator` | Modèles démographiques antérieurs, scénarios et usages RPG. | Références ; Lineage Lab reste l'unique source généalogique pour éviter deux moteurs concurrents. |
| `../_deprecated/Fantasy-Map-Generator` | Archive d'Azgaar : coalitions, conflits, contagion sur les routes, zones de crise. | Reprendre des idées sous forme d'événements datés ; pas son état mutable global ni son hasard lié à l'ordre d'exécution. |

Le répertoire `../_deprecated/FantasyMapGenerator` sans tirets est un autre générateur C++ ; l'archive d'Azgaar pertinente est celle avec les tirets. `../world-builder-rs` possède un prototype de peuplement, mais ne devient pas une dépendance supplémentaire.

### 3.1 Autorité des données

Chaque fait a un seul propriétaire. Un adaptateur peut refuser une donnée incompatible et le signaler ; il ne la corrige jamais silencieusement.

| Domaine | Propriétaire | Chronos en fait |
|---|---|---|
| Relief, climat, terre/mer initiaux | Neural Earth (instantané figé) | Lecture seule ; l'épuisement historique modifie les stocks Chronos, pas le relief. |
| Position initiale des établissements, géométrie des routes régionales | Magna Urbis (sorties figées) | Attribue les `SiteId`/`EdgeId` ; possède ensuite l'état historique des sites (population, phase, contrôle). |
| Graphe grossier inter-régional, temps de trajet | Chronos | Coûts et durées, pas de tracé routier détaillé. |
| Modèles d'institutions : rôles, titres, pouvoirs, règles | Pecking Order (réponse et empreinte de catalogue figées) | Crée les organisations et offices concrets, interprète les méthodes prises en charge, possède les mandats datés et leurs causes. |
| Chaînes de noms | Namebench (pool figé) | Attribue un nom par clé stable ; le nom est un libellé, jamais une identité. |
| Vies, dates de naissance/décès, unions, filiations, résidence | Lineage Lab (archive figée) | Lecture seule ; choisit parmi ces personnes les titulaires et acteurs d'événements. |
| Effectifs agrégés, crises, guerres, successions effectives | Chronos | Source des bilans ; les personnes importées sont un échantillon inclus dans ces effectifs, jamais ajouté. |

Conséquences pour la généalogie et les institutions :

- Une crise Chronos ne tue pas une personne importée. Elle peut citer comme victime une personne dont le décès enregistré tombe dans sa fenêtre et son lieu ; ailleurs, la personne reste vivante et son absence de mention n'est pas une contradiction.
- Le décès enregistré d'un titulaire ouvre une vacance Chronos à cette date ; la règle de succession explicitement associée au profil institutionnel choisit parmi les éligibles vivants que Lineage Lab expose. Être enfant du roi ne fait pas l'héritier effectif (étude §12.7). Une méthode non interprétable laisse l'office vacant ou demande une règle de scénario, avec diagnostic.
- Le fournisseur institutionnel ne remplace pas le moteur généalogique. Les modes de désignation opérationnels sont des adaptations Chronos déclarées ; ni une autorité « héréditaire », ni le texte d’une sélection ne suffisent à fournir une généalogie ou un successeur.
- Les mortalités de Lineage Lab (`Event.extra_mortality`) sont probabilistes : ses comptes ne reproduisent pas exactement un bilan Chronos. Les écarts entre totaux Lineage Lab et effectifs Chronos sont affichés comme diagnostic, pas réconciliés. Générer un scénario Lineage Lab à partir des crises Chronos est une boucle différée (J6).

### 3.2 Frontière entre carte et histoire ; unités

L'instantané devient un `GeographySnapshot` figé pour un canon. Son identité inclut les données exportées, l'empreinte du monde Neural Earth, les paramètres, versions et étapes retenues. Une nouvelle carte ou un changement de couches produit un nouveau bootstrap.

- **Taille** : Neural Earth est paramétré par `world_diameter_km` et travaille avec la circonférence `C = π × diamètre`. Chronos stocke la valeur source et dérive le rayon en mètres `R = C / 2π` ; aucune autre conversion implicite.
- **Résolution réelle** : l'atlas a 1 024 colonnes, soit `C / 1 024` par cellule à l'équateur (≈ 39 km pour une planète de 40 000 km de circonférence), et une largeur divisée par `cos(latitude)` ailleurs. L'échantillonnage de conditionnement à 7 680 m interpole l'atlas ; il n'ajoute pas d'information. Les surfaces de cellules se pondèrent par `cos(latitude)`.
- **Projection** : les coordonnées métriques de l'atlas sont équirectangulaires et ne sont pas des distances hors de l'équateur. Chaque région détaillée utilise une projection locale documentée (plan tangent ou azimutale équidistante centrée sur la région).
- **Donnée absente** : interpoler l'altitude ne révèle ni col précis ni petite rivière. Les passages stratégiques peuvent être fournis explicitement ; toute dérivation est versionnée et marquée comme approximation.

### 3.3 Régions Magna Urbis et graphe grossier

`Terrain::new` accepte des rasters carrés de 16 à 1 024 cellules par côté et une largeur de 250 à 100 000 m. `place` limite la composition à 1 000 établissements par appel et la population d'un établissement spécifié à 5 000 000 (`u32`). Une région Magna Urbis couvre donc au plus 100 km de côté, soit environ 2,5 cellules d'atlas pour une planète de taille terrestre. Ne jamais lui passer la planète comme carte plane.

Conséquences assumées pour la première démo :

- Le raster régional rééchantillonne l'atlas ; `water` vient du signe de l'altitude (mer = 1), `river_width = 0`, `river_ids = -1`, aucun lac. Les poids rivière/pont de Magna Urbis restent sans effet et la position fine des villes reflète surtout ses propres champs dérivés. Statut affiché : **approximé**. Pour plus de structure, la démo peut choisir une planète plus petite, où chaque cellule couvre moins de terrain.
- Les régions sont peu nombreuses (deux ou trois), sans recouvrement ni urbanisation globale. Les populations Chronos sont en `u64` ; l'adaptateur rejette une valeur hors des bornes de Magna Urbis au lieu de la tronquer.
- **Graphe grossier** : Chronos calcule un plus court chemin sur les cellules d'atlas de la zone de démo (coût de pente, interdiction terre/mer selon le mode, liaisons maritimes entre cellules côtières ou ports). Il produit des corridors avec longueur, temps et qualité « grossier », pas de tracé routier.
- **Raccord** : pour chaque corridor entrant dans une région, l'adaptateur ajoute un site-passerelle (`RoadSite` à faible population, clé stable) au point d'entrée projeté. Magna Urbis y route les chemins locaux ; Chronos relie la jonction-passerelle au corridor. Les sorties de bord automatiques (`exits`) ne servent pas de raccord canonique. Ce mécanisme reste à valider en J1.
- Les indices de jonctions (`a`, `b`) sont locaux à une sortie ; ils ne deviennent pas des IDs globaux. Les sites et jonctions reçoivent des IDs dérivés de la région et d'une clé stable.

Le **repli virtuel** suit le même format : sites déterministes sur une portion de sphère, graphe des k plus proches voisins, distances orthodromiques et temps de trajet.

## 4. Contrat de bootstrap

On suppose que les données nécessaires peuvent être fournies. Le bootstrap ne doit pas devenir un nouveau générateur de planète ou un catalogue encyclopédique. Une fixture virtuelle et un export de carte partagent les mêmes objets. Les champs marqués *(plus tard)* ne sont pas requis pour la première livraison.

| Objet | Données nécessaires à Chronos |
|---|---|
| Monde | Identité, seed, diamètre source et rayon dérivé en mètres, calendrier, année initiale, durée (démo ≈ 200 ans ; 2 000 ans au plus, plus tard), versions et sources. |
| Support spatial stable | Identifiants, voisinages, position sphérique, surface réelle ; hiérarchie parent/enfants *(plus tard, J5)* ; domaines indépendants des frontières politiques. |
| Contexte physique | Terre/mer, relief agrégé, champs climatiques ; eau douce, potentiels agricoles et risques dérivés et marqués comme approximations. |
| Frontières naturelles | Mers, chaînes et cols connus ou dérivés ; perméabilité, saisonnalité *(plus tard)*. Une rivière peut être obstacle et corridor navigable distincts, si une source la fournit. |
| Sites et occupations | `SiteId`, position, région, phase d'occupation initiale, population, activité, culture. Sites potentiels futurs réservables *(plus tard)*. |
| Réseau de mobilité | Nœuds de sites, ports, passerelles et jonctions ; arêtes orientées, mode terrestre/maritime (fluvial si fourni), longueur en mètres, temps en minutes par mode, disponibilité datée. Saison et capacité *(plus tard)*. |
| Population | Effectifs entiers (`u64`) par cohortes disjointes, culture et catégorie utile ; natalité/mortalité et capacité de support. Les affiliations ne créent pas de population supplémentaire. |
| Actifs et ressources | Localisation, quantité ou rendement, unité, renouvellement/extraction, accès ; distinguer possession, exploitation, taxation et revendication. |
| Contexte social | Communautés, pouvoirs initiaux, institutions, objectifs, offices et relations datées ; références aux résolutions Pecking Order. |
| Personnes et noms | Pools de noms avec empreinte ; personnes importées avec ID externe, dates, parentés, résidence et correspondance de lieux. Les identifiants ne dépendent jamais des libellés. |

Conventions :

- Positions globales en latitude/longitude ; cartes régionales en mètres dans leur projection déclarée.
- Temps de trajet en minutes entières, `ceil(longueur / vitesse)` avec vitesses de mode paramétrées en m/min.
- Années entières ; plages semi-ouvertes `[début, fin)`. Lineage Lab utilise des intervalles d'événements inclusifs (`end_year ≥ start_year`) : l'import convertit `fin = end_year + 1` et applique le décalage de calendrier déclaré.
- Stocks en unités entières ou quantifiées. Une perte en fraction s'applique à l'effectif au début du bloc concerné, arrondie par une règle unique documentée (par défaut `floor(P × num / den)`, reste conservé) ; « exactement 30 % » signifie exactement cette valeur entière.
- Les valeurs manquantes sont explicites ; « inconnu » ne devient ni zéro ni passage praticable.

### Ressources minimales

Première livraison : **alimentation** (production, réserves, consommation, pertes) et **richesse** (réserves, recettes, coûts, péages dans une unité conventionnelle ; prestige et légitimité restent des scores distincts), plus un matériau stratégique pour motiver un conflit.

Plus tard : eau douce saisonnière, bois/combustible renouvelable, fer/pierre/sel en gisements bornés, capacités de fret, de mobilisation et d'entretien (des capacités, pas des gisements).

Les reliefs et climats de Neural Earth conditionnent ces potentiels. Sa pédologie est une approximation d'apparence et de composition ; elle ne fournit pas d'inventaire de minerais. Les gisements manquants viennent d'un générateur Rust déterministe conditionné par la carte ou d'une entrée explicite, avec version et provenance.

Les routes initiales restent géométriquement fixes. Guerre, blocus ou saison peuvent changer leur disponibilité et leur coût historique. Une nouvelle route ou un pont reste un épisode/projet tant que le fournisseur cartographique n'a pas réalisé sa géométrie ; aucune accessibilité n'est inventée au zoom.

## 5. Architecture Rust compacte

Commencer avec une bibliothèque `chronos-core` et un petit exécutable `chronos-app` pour les commandes et l'interface locale. Garder des modules internes plutôt qu'une crate par concept :

```text
crates/chronos-core/src/
  model.rs       # identités, temps, événements et contrats
  world.rs       # graphe spatial, mobilité, ressources et agrégats
  social.rs      # entités, relations, institutions et offices
  generate.rs    # chronologie racine, propositions et transitions
  adapters/      # snapshot carte, Magna Urbis, noms, institutions, généalogie
crates/chronos-app/   # CLI, serveur local de l'interface, fichiers
web/                 # configuration, carte et chronologie
fixtures/            # petit monde virtuel et extraits figés redistribuables
tools/               # export ponctuel depuis les projets voisins
```

Ces chemins sont prévus. `refine.rs`, les extractions de crates et les bindings WASM viennent lorsqu'un usage réel les justifie (J5, J8). Le noyau ne dépend ni de Python, ni de CUDA, ni d'un serveur cartographique. Les outils d'export peuvent être des scripts Python lancés dans l'environnement du projet voisin, en lecture seule ; ils produisent des fichiers JSON versionnés avec empreinte. Les calculs historiques restent en Rust. Les entrées sont des structures Rust simples, pas un DSL.

### Règles de calcul indispensables

- Une clé de décision repose sur seed, propriétaire canonique, bloc temporel, rôle sémantique et versions ; pas sur l'ordre des requêtes, l'index d'une liste, un RNG partagé, `DefaultHasher` ou l'itération d'une `HashMap`.
- Les blocs canoniques sont indépendants de la fenêtre affichée ; commencer par décennies/années.
- Un événement transfrontalier a un seul propriétaire et un seul `EventId`. Composition (`PART_OF`) et causalité (`CAUSES`, `ENABLES`, `MOTIVATES`) sont distinctes.
- Un événement est proposé, vérifié contre les capacités et états, puis appliqué dans un ordre stable. Les explications littéraires ne constituent pas des preuves causales.
- Les populations, stocks et flux se comptent une seule fois. Un transfert sort d'une région et entre dans une autre ; un décès ne se répète pas dans plusieurs crises.
- Une édition d'auteur crée un nouveau canon et rejoue tout en v1. Les caches éventuels ne changent pas les faits.

## 6. Périmètre de la première implémentation

| MAINTENANT (J0–J4) | PLUS TARD (J5+) |
|---|---|
| Workspace `chronos-core` + `chronos-app`, interface web locale | Bindings WASM et worker, intégration dans Neural Earth / Magna Urbis (J8) |
| Monde virtuel seedé (repli) et instantané Neural Earth figé | Planète entière, 2 000 ans, centaines de millions d'habitants (J8) |
| Deux ou trois régions Magna Urbis, sites-passerelles, graphe grossier | Rivières, cols et lacs réels ; routes recalculées dans l'histoire (J9) |
| Pools Namebench figés, tirage par clé stable | Traduction et styles multiples des noms |
| Quatre profils d'organisations, méthodes de règles prises en charge, mandats | Réformes, scissions, fusions ; catalogue social enrichi (J7, J9) |
| Échantillon Lineage Lab figé, sélection parmi les personnes existantes | Réservation conditionnelle de rôles, scénario Lineage Lab généré depuis Chronos (J6) |
| Génération intégrale à la racine, ordre canonique, `top-K` sur index complet | Raffinement paresseux, enveloppes, allocation croisée hiérarchique (J5) |
| Alimentation, richesse, un matériau ; six types d'événements | Ressources complètes, religion, culture, écologie, magie… (J9) |
| Export/import JSON stable, panneau de provenance et statuts | Stockage indexé, éditions d'auteur, présent imposé (J7) |
| Tests : replay, budgets, unicité, temps de trajet, import | Tests métamorphiques de zoom et statistiques multi-seeds (étude §17) |

## 7. Jalons et critères de sortie

### J0 — Squelette et première génération visible (repli virtuel)

Créer le workspace Rust, un scénario virtuel seedé et l'application locale. Une carte de sites/corridors, une liste d'événements datés et un sélecteur d'année communiquent déjà. Configurer seed, durée, nombre de sites, population et densité.

**Sortie :** lancer la démo avec une commande documentée ; générer puis exporter une histoire ; les mêmes entrées reproduisent un export identique octet pour octet. Pas de framework de plugins.

### J1 — Contexte Neural Earth et peuplement Magna Urbis

Écrire l'outil d'export du contexte initial et définir `GeographySnapshot`. Rasteriser l'atlas avec la même convention que Neural Earth (1 024 × 512, quatre plus proches voisins pondérés) ou importer l'atlas exporté ; ne conserver qu'une seule voie. Choisir deux ou trois régions, construire leurs rasters projetés (§3.3), appeler `place` puis `roads` avec des seeds dérivés de clés stables, ajouter les sites-passerelles et calculer le graphe grossier.

Les paramètres de placement et de routes de ces régions sont exposés dans l'interface ; les relancer produit un nouveau bootstrap.

**Sortie :** une génération s'affiche sur un fond issu de Neural Earth, avec établissements et routes Magna Urbis reliés par des corridors grossiers ; les liaisons impossibles apparaissent comme telles. Le même instantané réimporté donne le même bootstrap. Sans export disponible, le monde virtuel reste sélectionnable et étiqueté comme repli.

### J2 — Noms, institutions et personnes importés

Exporter des pools Namebench bornés, triés, avec empreinte ; nommer sites, organisations et personnes en Rust par clé stable (l'ID externe Lineage Lab pour une personne importée).

Importer des réponses Pecking Order figées (requête normalisée, version et empreinte du catalogue). Mettre en œuvre quatre profils : communauté villageoise, monarchie, ligue marchande, collectif informel. Utiliser les modèles existants quand ils conviennent ; un profil fantasy conçu pour Chronos est étiqueté comme tel, pas comme provenant de Pecking Order. Séparer titre, fonction, pouvoirs, règle de succession et titulaire. Interpréter une liste fermée de méthodes (nomination, vacance, vote simple, succession héréditaire simple, droits d'actifs), déclarée dans l’adaptateur du catalogue figé ; toute autre règle produit un diagnostic.

Exporter un échantillon du noyau Lineage Lab couvrant la fenêtre de démo (par exemple 1300–1420), avec décalage de calendrier et table de correspondance de ses établissements vers des `SiteId`. Nommer les titulaires initiaux parmi les personnes vivantes et éligibles. Sans archive, des personnes virtuelles nommées par Namebench sont générées et marquées « repli ».

**Sortie :** villes, organisations et personnes nommées ; deux royaumes et une ligue marchande transrégionale ; chaque office a au plus le nombre de titulaires autorisé ; les affiliations se chevauchent sans doubler les effectifs ; le panneau de provenance affiche version et empreinte de chaque jeu.

### J3 — Boucle historique et démo pivot

Ajouter production/consommation, démographie agrégée et transitions sociales datées. Proposer les événements à partir du contexte, des objectifs, capacités, institutions et connexions ; appliquer les mutations dans un ordre stable. Commencer sur le monde virtuel dès J0 si les imports ne sont pas prêts.

Six types d'événements : guerre, épidémie, succession, crise vivrière, foire, conflit local ; plus des années paisibles et des reconstructions. Les quotas exacts (trois villages détruits parmi cinquante) se tirent par `top-K` sur l'index complet des candidats, à la racine. Les taux sont des paramètres de worldbuilding, pas des lois historiques.

**Sortie :** l'histoire explique qui agit, où, quand, pourquoi et avec quels effets ; les scénarios de la démo pivot (§9) sont générés ; les bilans ne deviennent jamais négatifs ; une armée n'arrive pas avant son temps de trajet ; un décès Lineage Lab de titulaire ouvre une succession conforme à la règle.

### J4 — Atelier et première livraison intégrée

Réunir configuration, carte et navigation chronologique. Exposer seulement les paramètres réellement interprétés : monde, durée, régions et placement, mobilité, démographie, ressources, institutions, intensité des types d'événements. Préréglages, état de génération et erreurs compréhensibles.

Cliquer sur un lieu, une organisation, une personne ou un événement ; lire ses relations et causes ; changer d'année ; filtrer ; comparer avant/après. Exporter/importer scénario, données figées et canon sans chemins absolus. Un mode autonome n'utilise que la fixture virtuelle redistribuable.

**Sortie :** démonstration complète Neural Earth → Magna Urbis → ressources → Pecking Order → Namebench → Lineage Lab → chronologie Rust consultable, avec statut et provenance de chaque source et liste des approximations. **C'est la première livraison visée.** Les jalons suivants la prolongent ; ils ne la bloquent pas.

### J5 — Raffinement à la demande *(différé)*

Résoudre les ancêtres requis, leurs enveloppes et les propriétaires d'événements partagés. Ajouter des événements natifs locaux compatibles avec les états antérieurs et ultérieurs ; une cause locale de portée mondiale doit avoir été réservée en amont. Réserver sites, organisations et rôles tardifs sur un ensemble de candidats stable. Passer du `top-K` complet à des allocations hiérarchiques quand la taille l'exige ; fixer une seule décomposition croisée espace/temps : consulter les années avant les villages donne les mêmes marges que l'inverse.

**Sortie :** accès direct à un village, fenêtres recouvrantes et visites inversées restituent les mêmes faits au même détail ; les récits locaux enrichissent sans réécrire.

### J6 — Généalogie conditionnelle *(différé)*

Définir une réservation conditionnelle de rôle avec rejet explicite des contraintes impossibles (étude §12.4–12.5). Éventuellement générer un scénario Lineage Lab (établissements, événements de mortalité) depuis le bootstrap et les crises Chronos, puis réimporter : la cohérence statistique s'améliore, l'autorité reste celle de §3.1. Réconcilier quotas individuels et agrégés avant publication du canon.

### J7 — Présent imposé, causalité enrichie et traces urbaines *(différé)*

Ancres terminales dures et objectifs souples, recherche causale bornée, réformes/scissions/fusions, éditions d'auteur. Rejeter une histoire incompatible au lieu de téléporter les populations à la date finale.

Produire un `UrbanHistoryContract` : phases d'occupation, ruines, reconstructions, migrations, institutions et traces survivantes. Magna Urbis décide de la géométrie. Une stèle ou un cimetière ancien n'est pas obligé de survivre jusqu'au présent.

**Sortie :** villes et organisations imposées au présent restent explicables ; une édition rejoue dans un nouveau canon ; un exemple urbain consomme les épisodes datés.

### J8 — Échelle planétaire et intégration des hôtes *(différé)*

Mesurer avant d'optimiser : canon racine, graphe, préparation de données, raffinement froid/chaud, mémoire et export. Passer progressivement de quelques régions à une planète, 2 000 ans, populations agrégées de centaines de millions ; augmenter séparément sites, acteurs et relations. États contigus, index épars, voisinages pertinents, caches bornés ; pas de matrices tous-sites/tous-sites ni de boucles sur les habitants. Paralléliser seulement si le profilage le justifie, en appliquant les décisions partagées dans un ordre canonique.

Brancher la bibliothèque dans Neural Earth et les contrats historiques dans Magna Urbis ; opérations volumineuses par lots dans un worker WASM, mêmes règles qu'en natif. Les flottants natifs et WASM pouvant différer, les sorties Magna Urbis restent figées dans le bootstrap.

**Cibles à mesurer, non acquises :** canon racine en quelques dizaines de secondes ; vues contextualisées en dizaines/centaines de millisecondes. Publier matériel, paramètres et détail réellement généré. Les ~5 % de personnes détaillables (étude §12.1) sont un plafond de détail, pas une population à simuler au démarrage.

### J9 — Extensions guidées par l'usage *(différé)*

Religion, diffusion culturelle, écologie, savoirs, magie, criminalité, mémoire, catalogue social enrichi, ressources complètes. Chaque domaine propose, valide, applique et raffine ses événements selon la même logique.

La géométrie de routes historiques, de nouveaux ponts et de grands travaux est une décision d'intégration avec Neural Earth/Magna Urbis ; Chronos prépare épisodes et contrats, et garde seulement son graphe grossier de coûts.

## 8. Ordre de réalisation et premières tâches

Chemin principal : `J0 → J1 → J2 → J3 → J4`. Après J0, J1 (carte), J2 (imports sociaux) et J3 (boucle sur monde virtuel) peuvent avancer en parallèle par périmètres de fichiers. J5–J9 s'appuient sur la démo livrée et peuvent être réordonnés selon l'usage.

Premières tâches concrètes :

1. Définir les IDs, le scénario seedé, le format d'export JSON et la commande de lancement locale.
2. Produire une génération virtuelle visible avec lieux et événements datés.
3. Exporter le contexte initial d'un monde Neural Earth ; fixer diamètre, projection régionale et résolution réelle.
4. Brancher `magna-urbis-core` sur deux régions, ajouter les passerelles et le graphe grossier ; figer une fixture de carte.
5. Exporter pools Namebench, résolutions Pecking Order et échantillon Lineage Lab ; écrire les tables de correspondance et le panneau de provenance.
6. Livrer la boucle historique, les scénarios pivots et la navigation ; ajouter les invariants difficiles au fur et à mesure.

Stabiliser le contrat partagé (`model.rs`, format d'export) avant les modifications concurrentes. Une revue d'intégration vérifie ensuite compilation, lancement, parcours visible et quelques cas seedés.

## 9. Vérification minimale et démonstration pivot

Tests ciblés au départ : replay identique (export stable) ; budgets exacts selon la règle d'arrondi et stocks non négatifs ; unicité d'événement, d'office et d'origine de personne ; arrivée après le temps de trajet ; import/export et rejet d'une empreinte incompatible ; déterminisme d'un appel Magna Urbis à seed égal. Les mêmes requêtes dans un ordre différent renvoient les mêmes faits (trivial en génération intégrale, à durcir en J5). Ajouter un test seulement pour un invariant réel ou un défaut constaté.

À chaque tranche : `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, compilation et tests concernés ; essai navigateur pour une interaction modifiée. Comparer carte, chronologie, sélection et date affichées. Un serveur HTTP qui répond n'établit pas que l'atelier fonctionne.

**Démo pivot** (monde démarrant en 1300, environ 200 ans) : deux royaumes, cinquante villages, une ligue marchande traversant une frontière. Une épidémie continentale `[1354, 1361)` fait perdre à une ville exactement `floor(P₁₃₅₆ × 3 / 10)` habitants en 1356, `P₁₃₅₆` étant sa population au début de 1356. Trois villages exactement sont détruits pendant une guerre. Un héros issu d'une famille Lineage Lab, vivant et adulte en 1398 selon l'archive, sauve le souverain en 1398 ; s'il n'existe aucun éligible, le scénario le signale ou bascule vers une personne virtuelle étiquetée. Consulter un village, remonter le temps, ouvrir la famille du héros puis l'autre royaume conserve les mêmes faits. Noms, institutions, personnes et lieux affichent leur source et leur statut.

## 10. Points de vigilance et sources locales

- Atlas grossier : ≈ 2,5 cellules par région Magna Urbis à taille terrestre, ni rivières ni lacs ; importer les passages nécessaires et garder leur qualité de source.
- Régions : dédoublonner sites et passerelles ; ne pas extrapoler une grille locale à la sphère.
- Généalogie : fenêtre et lieux limités à l'échantillon importé ; hors fenêtre, les personnes sont virtuelles et étiquetées.
- Catalogue institutionnel : distinguer description, règle exécutable, prescription normative et transgression historique.
- Noms : pool déterministe, formes originales conservées, identités indépendantes d'une traduction.
- Prototype : priorité à la génération complète et à son exploration ; abstractions et optimisations seulement pour un besoin observé.

Sources inspectées :

- [Neural Earth : atlas Rust](../infinite_map/native/neural-earth/src/atlas.rs), [entrée des champs](../infinite_map/native/neural-earth/src/lib.rs), [worker d'exécution](../infinite_map/wasm/runtime-worker.mjs), [étapes géographiques](../infinite_map/backend/terrain_orogen_stages.py), [pédologie](../infinite_map/backend/terrain_pedology.py).
- [Magna Urbis : raster et placement](../city_generator/rust/crates/core/src/settlements/mod.rs), [routes](../city_generator/rust/crates/core/src/settlements/roads/mod.rs), [manifeste du noyau](../city_generator/rust/crates/core/Cargo.toml), [bindings existants](../city_generator/rust/crates/wasm/src/settlements.rs).
- [Pecking Order : modèles Rust](../pecking-order/src/model.rs), [moteur](../pecking-order/src/engine.rs), [systèmes](../pecking-order/data/systems.json), [contrat de contenu](../pecking-order/data/CONTENT_CONTRACT.md).
- [Namebench : limites et usages](../namebench/README.md), [accès aux pools](../namebench/namegen/db.py).
- [Lineage Lab : contrat hybride](../lineage-lab/docs/genealogy-hybrid.md), [configuration](../lineage-lab/src/lineage_lab/config.py).
- [Azgaar : diplomatie et campagnes](../_deprecated/Fantasy-Map-Generator/modules/burgs-and-states.js), [zones et propagation](../_deprecated/Fantasy-Map-Generator/main.js), [notice de l'archive](../_deprecated/Fantasy-Map-Generator/LICENSE). Conserver les notices si du code est effectivement repris.

Ces liens décrivent les capacités locales observées. Les objectifs non couverts par l’état livré restent des tâches de la roadmap.

Note de version : Pecking Order a migré de son ancien paquet Python vers une API Rust 2.0 pendant cette réalisation. Les références ci-dessus suivent les sources réellement présentes ; le rapport de revue conserve le constat antérieur. Les capacités importées sont celles de la fixture figée, et non celles supposées d’un dépôt voisin en évolution.
