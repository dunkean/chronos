# Chronos — vision « entités sociales », état des lieux et design macro

Date : 26 octobre 2026. Statut : **analyse et proposition**. Rien de ce qui est décrit en §6–7 n'est implémenté. Les chiffres « mesurés » proviennent des documents cités et n'ont pas été remesurés ici.

Sources lues : ce dépôt (code, [étude v3](../../conception_generateur_histoire_multiechelle_v3.md), [roadmap](../../Roadmap.md), [méthode](../methods/chronology.md), fixture) ; `../ttrpg_content_generator` (`doc/generation_schema.pptx`, `archive/template.py`, `worldbuilder_social_engine_study.md`, `src/genealogy/`) ; `../city_generator` (Magna Urbis) ; `../infinite_map` (Neural Earth) ; `../lineage-lab` ; `../pecking-order` ; `../namebench`.

## 1. Ce que tu veux

**Un générateur de 100 à 2 500 ans d'histoire** pour un monde médiéval fantastique sans progrès technique, à partir d'une carte (relief, eau, climat, ressources) et d'une situation de départ : **colonisation**, **jeune civilisation** ou **vieille civilisation**.

**Un seul concept décrit tout le monde : l'entité sociale.** Une entité peut être un individu, une famille, un village, une guilde, une religion, un peuple ou un pays. Elle est réifiable (groupe de groupes) et contient des membres, qui sont eux-mêmes des entités. Elle porte une structure (rôles des membres), des activités, des ressources (stocks, flux, dettes), des zones d'influence décomposées en sites, une chronologie, des objectifs, des coutumes, des traits et une culture, qui prolonge celle de Magna Urbis. Elle est reliée aux autres entités par l'inclusion `IsPartOfMe[role]` et par des relations latérales : sang, communauté, proximité, idéologie, économie, activité, alliance et amitié. Un individu est une feuille qui appartient à plusieurs entités à la fois.

**Le multi-échelle est central.** Un fait macro (« la nation 1 reçoit 50 000 immigrants et une ville majeure ») se décompose de façon procédurale et déterministe en faits micro : 30 villages, des croissances, des routes. Les personnes importantes (rois, héros, bourgmestres) ont une vraie généalogie ; les secondaires sont instanciées à la volée.

**Trois capacités attendues :**
1. *Bootstrap* d'un monde complet selon les trois scénarios : demander les établissements aux autres bibliothèques en fonction de la population, puis créer les entités initiales.
2. *Progression* : faire évoluer les entités et le peuplement, avec les aléas climatiques et biologiques en rétroaction.
3. *Raffinement* : à une date et une échelle données, générer les niveaux inférieurs de façon cohérente dans le temps, l'espace et la causalité. **Chaque décision n'est prise qu'une fois.** Un zoom la détaille sans jamais la reprendre.

## 2. État actuel de Chronos (implémenté)

| Domaine | Ce qui existe | Écart avec la vision |
|---|---|---|
| Monde | Fixture figée : 52 sites, 71 routes, zone de 200 km, planète de 1 600 km ; repli virtuel | Pas de bootstrap : les sites sont imposés, et il n'existe aucun scénario de départ |
| Modèle social | Types plats `Polity`, `Organization` (un office, un titulaire), `Person` ; 2 à 6 polities par plus proche capitale ; 5 profils | Aucune entité générique, aucune imbrication, aucun graphe de relations, ni objectifs, ni coutumes, ni traits |
| Histoire | Boucle annuelle **par site** sur tout le monde, en ordre fixe ; 14 types d'événements ; effets exacts ; causes antérieures | Processus pilotés par des taux, pas par les objectifs des entités ; scénario `pivot` codé en dur |
| Peuplement | Routes et sites figés ; croissance logistique sans événement ; migrations entre sites existants | Aucun nouveau village, aucune nouvelle route, aucun changement de rang urbain |
| Ressources | 6 indices de potentiel par site (nourriture, eau, bois, fer, pierre, sel) | Ni stocks, ni dettes, ni droits (possession, exploitation, taxe) |
| Multi-échelle | Inexistant : tout est généré à la racine (J5 non commencé) | Le cœur de la vision |
| Qualité | Déterminisme strict (`keys::key`, `BTreeMap`), `canon_id`, rejeu et import vérifiés, provenance honnête, 10 tests | **Socle solide, à conserver** |

Point clé : l'[étude v3](../../conception_generateur_histoire_multiechelle_v3.md) décrit déjà ta vision presque mot pour mot. Le §4A couvre l'entité sociale réifiée, les arêtes typées, les empreintes, les institutions, les objectifs et le cycle de vie ; les §10 et §15 couvrent le raffinement et la règle « une décision, une fois ». **L'écart est donc d'implémentation, pas de conception.** La livraison J0–J4 a privilégié une démo intégrée sur un petit monde figé, au détriment du modèle d'entités et du zoom.

## 3. Ce que fournissent les autres projets

| Projet | Fournit (implémenté) | Ne fournit pas | Accès depuis Chronos |
|---|---|---|---|
| **ttrpg_content_generator** | Le modèle d'origine dans `generation_schema.pptx` : échelle monde → individu, `Entity`/`Group`/`Person`, `Parents/Children<type, role>`, relations typées. Un moteur généalogique déterministe (`src/genealogy/`) : `PoliticalTimeline`, `Settlement`, statuts, `PromotionRule` | Code social plat et dépendant du LLM ; `archive/template.py` ne s'importe pas ; aucun événement structuré | Concepts et catalogues (rôles, liens, cultures et races) à porter |
| **Magna Urbis** (Rust, `magna-urbis-core`) | `Terrain::place` : scores de site, espacement à la Christaller, modes `list` / `counts` / `auto` (Zipf), positions épinglées. **Ajouter des places en fin de liste ne déplace pas les précédentes.** `Terrain::roads` : itinéraires à moindre coût, classes, franchissements, 4 niveaux de développement, 9 cultures routières. 41 à 53 cultures (enceintes, phases de croissance, préférences de site, familles de noms) | Pas d'entrée « population régionale = X » ; pas de temps de trajet ; pas d'ajout incrémental de route (tout est re-routé) ; une ville régénérée plus grande ne contient pas la plus petite ; région ≤ 100 km | Bibliothèque Rust pure, déterministe. **Licence GPL-3.0-only**, alors que Chronos est MIT/Apache |
| **Neural Earth** | Graphe Orogen (~100 k sommets : relief, climat sur deux saisons, Köppen, biomes), atlas 2048 × 1024, hydrologie (bassins, lacs, débits mensuels, navigabilité par type d'embarcation), sols approximatifs | **Aucune couche de ressources minérales ni biologiques** : la spécification des 35 couches existe, pas l'implémentation. Export encore « Now » sur sa roadmap | Python/Node : export figé uniquement |
| **Pecking Order** | 242 systèmes, 1 731 rôles, `level`, `importance`, `reports_to`, `powers`, `collective`, thèmes alignés sur les cultures de Magna Urbis. Pur, compatible WASM | Ni succession, ni personnes, ni économie (les verbes `allocate`, `tax`… disent *qui* gère, pas *comment*) | Crate Rust, dépendance directe possible. Chronos n'en utilise aujourd'hui que 6 systèmes et ignore niveaux, pouvoirs et thèmes |
| **Lineage Lab** | Démographie annuelle complète, statuts, nations datées. **Mode hybride** : noyau simulé d'environ 5 % plus cohortes comptées, personnes d'arrière-plan reconstruites à la demande avec des IDs stables (permutations inversibles) | Pas de lignée isolée à la demande (tout le monde est simulé d'abord) ; ni office, ni héritage de biens ; Python seulement | Échantillon figé de 1 846 personnes sur **120 ans** (1300–1420) |
| **Namebench** | 8,6 M noms, 10 M toponymes, 731 générateurs fantasy | Non déterministe (`ORDER BY RANDOM()`) | Pools figés (3 cultures aujourd'hui) |

Couches de roches et ressources (Magna Urbis `origin/main` au 26/10/2026, commit `9843e35`, travail Godot `godot-render/`). Le compilateur de contexte de sol `godot-render/tools/surface_context.py` produit, sur des tuiles de 40 km à environ 30 m :
- **une lithologie synthétique** : 5 fractions continues de roche mère (granite, calcaire, grès, argilo-marneux, basalte), avec strates plissées, intrusions et coulées ; `hardness`, `weatherability`, `structureAzimuth`, `bedrockVariation`. Elle est indépendante du relief et du climat, et tirée de la seed ;
- **des sols** : `sand`/`silt`/`clay`, `organic`, `stoniness`, `soilDepth`, `depositDepth`, `moisture`, `drainage`, `permeability`, `storage` ;
- **des processus et une provenance** : potentiels alluvial, colluvial, éolien, littoral et de gel, plus la composition lithologique transportée vers l'aval (`upstream*`, `sedimentProvenance`).

Le rendu consomme ces champs pour choisir 90 types visuels de sol (`docs/GROUND_TEXTURE_MAPPING_V1.md`, dont 22 aspects de roche). **Il n'y a aucune ressource exploitable** (minerais, métaux, sel, gypse, argile à potier, tourbe), ni gibier, ni essences de bois. L'étude `docs/GROUND_MATERIAL_MAPPING.md` §4.2 renvoie explicitement sels, gypse, oxydes et minéralisations à une extension optionnelle (O1, O5) non implémentée. Ce sont des champs de rendu calculés en Python sur des tuiles, sans contrat d'export vers un monde entier.

Mesures publiées : Lineage Lab hybride, 1 M fondateurs × 60 ans en 0,83 s ; Chronos, génération de référence en ~17 ms (52 sites × 200 ans, hors préparation des données).

## 4. Ce qu'il manque

1. **Le modèle d'entité générique** : identité unique, nature, composition récursive, arêtes typées datées, empreintes spatiales, registre de ressources, objectifs, règles, culture.
2. **Un support spatial hiérarchique stable** (monde → région → terroir → site) qui porte des agrégats : population par culture, capacité de charge, nombre de sites potentiels. Aujourd'hui, l'unité de calcul est le site, ce qui empêche à la fois l'échelle planétaire et le zoom.
3. **Le bootstrap par scénario** : distribution de la population selon la capacité de charge et l'accessibilité, appel de `place` en mode `list`, puis génération des entités initiales (cultures, polities, religions, organisations).
4. **L'évolution du peuplement** : fondation et abandon de sites, changements de rang, routes par époque. Côté Magna Urbis, il manque des routes incrémentales et un temps de trajet.
5. **Le raffinement** : enveloppes héritées, allocation exacte des budgets (« +30 villages », « −30 % ») sur les sous-cellules et les années, réservation de rôles, événements locaux natifs.
6. **Une généalogie sur 2 500 ans** pour les notables. L'échantillon figé de 120 ans ne peut pas couvrir la durée cible.
7. **Des ressources réelles** : couches de potentiel (lithologie et sols de Magna Urbis, gisements à concevoir), stocks, flux et droits.
8. **Des processus pilotés par les entités** : pressions → objectifs → actions possibles → événements, au lieu de taux globaux.

## 5. Ce que je ferais différemment

- **Une seule abstraction d'entité au lieu de `Polity`, `Organization` et `Person`.** Le type (`kind`) et un archétype de catalogue spécialisent l'entité. Les polities, guildes et familles deviennent des données, pas des structures Rust distinctes.
- **Ne stocker que les relations contingentes.** Dans ta liste, `hasTheSameBlood`, `isSameAsMe`, `isCloseToMe`, `thinkLikeMe` et `doStuffWithMe` se **dérivent** des attributs : parenté, culture, empreinte, idéologie, activité. Les calculer à la demande, via un index, évite des millions d'arêtes et toute incohérence. On ne stocke, datées et avec leur événement cause, que les relations qui sont des faits historiques : inclusion avec rôle, gouvernement, propriété, alliance, contrat commercial, dette, rivalité, amitié, grief.
- **Mettre le canon racine au niveau des agrégats, pas des sites.** La boucle actuelle par site et par année ne passe pas à l'échelle d'une planète sur 2 500 ans et rend le zoom impossible. La racine simule régions et entités saillantes par blocs (décennie, puis année pour les crises). Les sites et villages sont une *projection* raffinée.
- **Faire du bootstrap une étape de génération, pas une fixture.** Chronos doit appeler Magna Urbis à l'exécution (crate Rust pure), ce qui impose de régler la licence : relicencier `magna-urbis-core` ou passer Chronos en GPL. La fixture reste un cache et un repli.
- **Rapatrier en Rust la généalogie des notables**, sur le modèle du mode hybride de Lineage Lab : maisons et dynasties simulées, cohortes comptées, personnes secondaires reconstruites par permutation inversible. Lineage Lab devient la référence statistique pour calibrer et tester, pas une source figée de 120 ans.
- **Sortir les scénarios du moteur.** `pivot` devient un jeu de *contraintes d'auteur* (étude §11, J7) appliqué par le même mécanisme que les ancres, pas des constantes dans `generate.rs`.
- **Exploiter Pecking Order en entier** : niveaux, `reports_to`, pouvoirs et thèmes culturels donnent la structure interne et les capacités des entités. Chronos garde la succession et l'économie.
- **Garder** le déterminisme par clés sémantiques, les effets exacts, la provenance honnête, le `canon_id` et le rejeu. C'est la bonne fondation.

## 6. Étude de faisabilité

**Conception : faisable, et déjà largement instruite.** L'étude v3 (§2, réf. R1–R31) recense les précédents. *Caves of Qud* montre l'approche « événement d'abord, justification ensuite » ; Dwarf Fortress, la séparation entre figures suivies et population abstraite ; HTN, la décomposition hiérarchique ; la PCG par contraintes, la résolution de slots ; Mathieu et al., les patrons multiniveaux ; Salmon et al., l'aléa à compteur ; Hannan et Freeman, le cycle de vie des organisations. La revue de ttrpg cite en plus Sugarscape, la diffusion culturelle d'Axelrod, l'homophilie et les petits mondes, utiles pour les relations dérivées. Aucun de ces travaux ne livre l'ensemble ; la combinaison est nouvelle, mais chaque brique est connue.

**Calcul : faisable, sous condition de granularité.** Ordre de grandeur indicatif, non mesuré : 2 000 régions × 250 décennies = 500 k couples région-bloc à la racine. C'est comparable aux 20 M de couples cellule-période estimés en §16.1, donc compatible avec une génération en quelques dizaines de secondes si la boucle racine ne parcourt jamais les sites ni les personnes. Le raffinement local (une vallée, un siècle) ne touche que quelques centaines de sites. Le mode hybride de Lineage Lab montre qu'une démographie de notables plus cohortes est rapide.

**Risques réels, par ordre de gravité :**

1. **Budgets exacts à travers les échelles** (étude §10.6–10.7). « +30 villages, +50 000 habitants » doit se répartir dans l'espace et dans le temps sans dépendre de l'ordre de consultation. La solution connue, une allocation récursive sur cellules à partir de masses canoniques, exige que chaque cellule connaisse à l'avance son nombre de sites candidats. C'est le point dur, à prototyper en premier.
2. **Continuité géométrique du peuplement.** Magna Urbis garde les places existantes quand on en ajoute, ce qui va dans le bon sens. Mais il re-route tout le réseau et ne garantit pas qu'une ville de 1450 contienne celle de 1350. Il faut des routes « par époque », gelées puis complétées, et un contrat de croissance urbaine (étude §10.10).
3. **Pas de remontée causale.** Un fait local ne peut pas modifier le macro. Tout effet de portée supérieure doit être réservé par le parent : c'est la règle de propriété (§4.5) qui garantit « une décision, une fois ».
4. **Données manquantes.** Les ressources minérales et biologiques n'existent ni dans Neural Earth ni dans Magna Urbis. La lithologie et les sols du travail Godot donnent une bonne base : pierre de taille (dureté), argile, terres agricoles (texture, profondeur, drainage) et sables. Les gisements (fer, cuivre, étain, or, argent, sel, gypse), le gibier et les essences demandent une couche dédiée, conçue à partir de la spécification des 35 couches. Elle serait conditionnée par la lithologie et marquée `derived`.
5. **Licence GPL** de Magna Urbis si elle est liée à l'exécution.
6. **Densité narrative.** Sans calibrage, chaque village devient « le plus important du monde ». Il faut une saillance par entité et des périodes calmes.

**Verdict.** Faisable par paliers. Le bootstrap et la progression macro sont un travail d'ingénierie classique. Le raffinement avec budgets exacts est la partie de recherche : à prouver sur un petit monde avant de grossir.

## 7. Design macro proposé

### 7.1 Couches

```text
Physique (Neural Earth, immuable) ──► Support spatial hiérarchique (cellules stables : monde/région/terroir/site)
                                              │ agrégats : capacité, population par culture, sites potentiels
Catalogues (Pecking Order, cultures MU, ─► Graphe d'entités sociales (identités, composition, relations datées)
archétypes, coutumes, noms)                   │
                                     Chronique canonique (événements à propriétaire scope×bloc, budgets, slots)
                                              │
                      Raffineur pur  Refine(scope, bloc, LOD) = f(canon parent, contexte local, clés)
                                              │
            Projections : Magna Urbis (sites, routes, épisodes urbains) · généalogie · noms · récits
```

### 7.2 L'entité sociale (ta liste, plus les ajouts proposés)

```text
SocialEntity
  id, kind (famille, village, guilde, culte, peuple, royaume…), archetype (catalogue + version)
  nature        acteur (peut décider) | agrégat (population) | catégorie (identité partagée)   ← ajout
  résolution    canonique | esquisse (agrégats, comptes) | virtuelle (slot non résolu)         ← ajout
  existence     fondation → transformations → dissolution ; SUCCESSOR_OF / SPLIT_FROM / MERGED_INTO ← ajout
  structure     rôles (Pecking Order : niveau, reports_to, pouvoirs) + membres IsPartOfMe[role]
  activités     métiers et fonctions économiques, pondérés
  ressources    registre : stocks, flux entrants/sortants, dettes/créances, droits (posséder/exploiter/taxer/revendiquer)
  empreintes    présence, contrôle, juridiction, économie, culture, recrutement → cellules → sites ; siège ← ajout
  objectifs     bornés, pondérés, parfois contradictoires
  règles        coutumes, admission, succession, décision, tabous
  traits        belliqueuse, secrète, pieuse… (modulent les probabilités)
  culture       culture MU + langue/famille de noms + religion + valeurs
  légitimité / contrôle effectif   distincts (usurpation, prétendants)                         ← ajout
  cohésion      unité interne ; factions matérialisables au zoom                                ← ajout
  capacités     dérivées : militaire, fiscale, administrative, portée logistique                ← ajout
  savoirs       techniques, magie, secrets (technologie stable mais savoirs inégaux)            ← ajout
  prestige / réputation   non conservés, distincts de la richesse                               ← ajout
  mémoire       griefs, dettes d'honneur, mythes fondateurs (relations historisées à cause)     ← ajout
  saillance     importance narrative → densité d'événements et niveau de détail                 ← ajout
  chronique     références aux événements (jamais une copie)
```

Relations en trois familles :

- **structurelles**, stockées et datées : `IsPartOfMe[role]` (composition stricte, acyclique), adhésion, gouvernement, représentation, propriété ;
- **d'affinité**, dérivées et jamais stockées : sang, culture, proximité, idéologie, activité, calculées depuis les attributs ;
- **contractuelles et historiques**, stockées avec un poids, une direction et un événement cause : alliance (`hasTheSameChallenge`), commerce (`makeMoneyWithMe`), dette, rivalité, amitié (`isMyFriend`), grief.

La population se compte une seule fois, dans des cohortes disjointes ; les adhésions ne créent pas de personnes.

### 7.3 Bootstrap par scénario (capacité 1)

Un seul algorithme, paramétré par une **maturité** `m` : colonisation ≈ 0,1–0,2, jeune ≈ 0,4–0,6, vieille ≈ 0,9.

1. **Capacité de charge** par cellule, tirée des potentiels (eau, sols, climat, pâturage) et de l'accessibilité (coût de trajet).
2. **Foyers d'origine** : K points d'entrée (côtes, estuaires, cols) pour une colonisation, des noyaux anciens sinon. La population initiale diffuse depuis ces foyers selon la capacité et la distance de trajet, jusqu'à remplir la fraction `m` de la capacité. La part urbaine et la profondeur de la hiérarchie (Zipf) croissent avec `m`.
3. **Sites** : par région, Chronos calcule la liste des établissements (rang × population × culture) et appelle `Terrain::place` en mode `list` ; les grandes villes sont des ancres canoniques. Puis `Terrain::roads`, avec un niveau de développement lié à `m`.
4. **Entités** : blocs culturels issus des foyers ; polities par regroupement sur le graphe de trajets (chefferies ou cités à faible `m`, royaumes et empires à fort `m`) ; religions ; organisations par site selon le rang et les activités (archétypes Pecking Order) ; maisons nobles et notables, avec leur généalogie.

Une variante permet de valider la cohérence : obtenir « vieille civilisation » en lançant la progression depuis « colonisation » sur plusieurs siècles (*spin-up*). C'est plus lent, mais cela garantit qu'un départ « vieux » ressemble à un passé possible.

### 7.4 Progression (capacité 2)

La boucle racine tourne par blocs canoniques (décennie ; année pour les crises) sur les **régions et les entités saillantes**, jamais sur les sites. Chaque bloc enchaîne :

1. *pressions* : démographie, nourriture, ressources, dettes, griefs ;
2. *intentions*, tirées des objectifs et des traits ;
3. *actions possibles*, filtrées par les règles, les capacités et les ressources ;
4. *chocs exogènes* : climat, séisme, épidémie ;
5. *événements proposés*, puis validés et appliqués dans un ordre canonique.

Le peuplement produit des **faits agrégés à budget** : « région R, [1320, 1330) : +12 000 habitants, +7 villages, la ville X passe au rang de bourg, ouverture du corridor R–S ». Les entités naissent, se scindent, fusionnent et meurent par événements, sans réécriture du passé.

### 7.5 Raffinement (capacité 3) : « une décision, une fois »

- **Propriété** : chaque fait a un propriétaire unique (scope × bloc) et un ID dérivé de `(propriétaire, bloc, slot sémantique, version)`. Toute décision aléatoire est `key(seed, propriétaire, bloc, slot)`. Elle ne dépend ni de l'ordre ni de la fenêtre consultée.
- **Descente** : `Refine(scope, bloc, LOD)` hérite des événements et budgets du parent et les **alloue exactement** aux sous-cellules et aux années, au prorata de masses canoniques. Pour les villages : ancres existantes épinglées et nouveaux `SettlementSpec` à clés stables pour Magna Urbis ; routes ajoutées par époque. Le raffineur ajoute ensuite des événements natifs locaux bornés par l'enveloppe héritée.
- **Interdits** : un enfant ne modifie jamais un parent ; tout effet de portée supérieure est réservé en amont (slots de héros, déclencheurs).
- **Personnes** : les notables sont canoniques. Les secondaires sont reconstruits depuis des cohortes avec des IDs stables, selon le schéma de Lineage Lab hybride.
- **Matérialisation** : le détail est une fonction pure, qu'on peut mettre en cache ou recalculer. Le canon ne contient que la chronique macro, les budgets et les clés.

### 7.6 Organisation Rust et paliers

`chronos-core` sans E/S, découpé en modules :

- `space` : cellules hiérarchiques et agrégats ;
- `entity` : graphe, registre de ressources, empreintes ;
- `catalog` : adaptateurs Pecking Order et cultures ;
- `bootstrap` ;
- `process/*` : démographie, économie, politique, guerre, religion, peuplement, aléas ;
- `chronicle` : événements, propriétaires, budgets ;
- `refine` ;
- `lineage` : notables et cohortes ;
- `adapters/mu`.

| Palier | Contenu | Preuve de sortie |
|---|---|---|
| P0 | Modèle `SocialEntity` et relations ; migration des polities, organisations et personnes actuelles | Même démo, mêmes invariants, sur le nouveau modèle |
| P1 | Support hiérarchique et bootstrap à 3 scénarios avec appel de Magna Urbis à l'exécution | Trois mondes distincts depuis une même carte, rejouables à l'octet près |
| P2 | Progression macro par blocs et entités saillantes, peuplement à budgets | 500 ans sur plusieurs régions ; villages fondés et abandonnés |
| P3 | Raffinement d'une région : allocation exacte, villages, routes par époque, événements natifs | Tests métamorphiques A→B / B→A, accès direct contre zoom progressif (§17.2) |
| P4 | Généalogie des notables en Rust et personnes secondaires à la demande | Dynastie cohérente sur 1 000 ans ; héros lié à une origine unique |
| P5 | Montée en échelle : planète, 2 500 ans | Temps mesurés, publiés avec le matériel utilisé |

Décisions à prendre avant P1 : la **licence** de `magna-urbis-core` ; la **taille des cellules** du support spatial (réutiliser les sommets Orogen ou une grille propre) ; le **pas de bloc** racine (décennie proposée).
