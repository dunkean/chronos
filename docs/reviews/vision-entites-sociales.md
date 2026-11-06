# Chronos — vision « entités sociales », état des lieux et design macro

Date : 26 octobre 2026 ; mise à jour le même jour avec les apports d'Azgaar (§7.2–7.4, §7.7). Statut : **analyse et proposition**. Rien de ce qui est décrit en §6–7 n'est implémenté. Les chiffres « mesurés » proviennent des documents cités et n'ont pas été remesurés ici.

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
  règles        institutionnelles : admission, succession, décision (les coutumes et tabous sont des idées)
  traits        belliqueuse, secrète, pieuse… (modulent les probabilités)
  idées         distribution d'adhésion par dimension : croyance, langue, coutume/droit, savoir,
                style. Pour un agrégat, c'est la somme de ses membres, jamais un champ propre (§7.8) ← ajout
  appuis        relations datées avec cause vers des idées : ENDORSES (religion d'État, langue
                officielle), ADMINISTERS (Église, école), SUPPRESSES, PATRONS (§7.8)           ← ajout
  étiquettes    lectures dérivées, jamais stockées : religion majoritaire, religion d'État,
                monarchie, empire, peuple naval ou nomade, culte, hérésie… (§7.8)                ← ajout
  légitimité / contrôle effectif   distincts (usurpation, prétendants)                         ← ajout
  cohésion      unité interne ; factions matérialisables au zoom                                ← ajout
  capacités     dérivées : militaire, fiscale, administrative, portée logistique                ← ajout
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

1. **Capacité de charge** par cellule, tirée des potentiels (eau, sols, climat, pâturage ; les layers de ressources de Neural Earth quand elles seront disponibles) et de l'accessibilité. **Coût de trajet en heures entières**, issu d'une table de transports médiévale (AZ2). En attendant les ressources, une capacité témoin lisible (eau courante, confluences, altitude, nature de la côte), invariante à la résolution (AZ4).
2. **Foyers d'origine** : K points d'entrée (côtes, estuaires, cols) pour une colonisation, des noyaux anciens sinon. Chaque foyer sème le **répertoire d'idées** de son peuple : langue, croyances, coutumes, savoirs. L'adaptation écologique (naval, nomade…) n'est pas assignée : elle se lit ensuite dans les activités et le milieu (AZ3, §7.8). La population initiale diffuse depuis ces foyers selon la capacité et la distance de trajet, jusqu'à remplir la fraction `m` de la capacité. La part urbaine et la profondeur de la hiérarchie (Zipf) croissent avec `m`.
3. **Un seul moteur d'expansion concurrente à coût** (AZ1), repris d'Azgaar. C'est un Dijkstra multi-sources, en coûts entiers, avec un départage total `(coût, id entité, id cellule)` et un **plafond fonction de `m`**. Il sert aux peuples, aux polities (avec affinité culturelle et lissage des enclaves), aux aires de marché et à la diffusion initiale des idées, avec des poids différents.
4. **Sites** : par région, Chronos calcule la liste des établissements (rang × population × culture) et appelle `Terrain::place` en mode `list` ; les grandes villes sont des ancres canoniques. Le rang et les fonctions (port, fort, monastère, caravansérail…) suivent des règles déclaratives (AZ8). Puis `Terrain::roads`, avec un niveau de développement lié à `m` ; la topologie initiale est un graphe d'Urquhart (AZ6).
5. **Entités** :
   - peuples (entités `catégorie`) issus des foyers, avec leurs distributions d'idées ;
   - polities issues de l'expansion, de la chefferie ou cité (faible `m`) au royaume ou à l'empire (fort `m`). Leur structure de pouvoir vient de Pecking Order ; « monarchie » ou « empire » en sont des lectures (AZ11). Elles se découpent en provinces à centre urbain, avec des colonies détachées (« Nouvelle-X ») pour le scénario colonisation (AZ12) ;
   - des organisations qui administrent des idées (Églises, écoles) et des appuis (religion d'État, langue officielle), plus nombreux quand `m` est élevé, et seulement là où les distributions les justifient (AZ7, §7.8) ;
   - des organisations par site selon le rang et les activités (archétypes Pecking Order) ;
   - des maisons nobles et des notables, avec leur généalogie ;
   - les relations diplomatiques initiales selon la distance de graphe, et la vassalité par rapport de puissance (AZ5).

Une variante permet de valider la cohérence : obtenir « vieille civilisation » en lançant la progression depuis « colonisation » sur plusieurs siècles (*spin-up*). C'est plus lent, mais cela garantit qu'un départ « vieux » ressemble à un passé possible.

### 7.4 Progression (capacité 2)

La boucle racine tourne par blocs canoniques (décennie ; année pour les crises) sur les **régions et les entités saillantes**, jamais sur les sites. Chaque bloc enchaîne :

1. *pressions* : démographie, nourriture, ressources, dettes, griefs ;
2. *intentions*, tirées des objectifs et des traits ;
3. *actions possibles*, filtrées par les règles, les capacités et les ressources ;
4. *chocs exogènes* : climat, séisme, épidémie ;
5. *événements proposés*, puis validés et appliqués dans un ordre canonique.

Mécanismes de bloc repris d'Azgaar, mais rendus temporels et causaux (AZ5–AZ18, voir §7.7) :
- **guerre** : la cascade d'entrée (vassaux, alliés qui rompent par peur, neutralité si alliés des deux côtés) est résolue une fois, à la déclaration ;
- **idées** (croyances, langues, coutumes, savoirs) : un seul moteur générique de diffusion, conversion, divergence (hérésie, dialecte) et syncrétisme (§7.8). L'adoption ou la révocation d'une religion d'État est un événement ;
- **routes** : corridors ouverts par époque, avec une remise ×0,5 sur l'existant, sans re-routage ;
- **économie** : trésor, taxes selon la structure de pouvoir, tarifs et embargos issus de la diplomatie, plus les dépenses ;
- **sites** : changements de rang, constructions de murailles et de temples comme événements datés.

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

Décisions à prendre avant P1 : la **licence** de `magna-urbis-core` ; la **taille des cellules** du support spatial, sous forme d'un LOD Neural Earth (voir D2 dans le [suivi](../suivi-implementation.md)), puisque Neural Earth est le seul système multi-échelle ; le **pas de bloc** racine (décennie proposée).

### 7.7 Apports retenus d'Azgaar Fantasy Map Generator

Documents :
- [docs/integration-azgaar.md](../integration-azgaar.md) (référence) : quoi reprendre et comment l'intégrer, lots A0–A5 ;
- [azgaar-fmg-inspiration.md](azgaar-fmg-inspiration.md) : règles de génération, tâches AZ.

Copie de référence : `../Fantasy-Map-Generator`, révision `67d555b` (v1.156.0). FMG est sous licence MIT avec autorisation des dérivés, mais on réimplémente à partir des règles, sans traduire le code.

**Contenus repris** (`integration-azgaar.md` §3), à intégrer au modèle :
- **emblèmes** (`EmblemSpec`) attribués une fois à une entité et stables au fil des successions ; un changement d'armes est un épisode daté ;
- **lieux d'intérêt**, en séparant toujours trois objets : le lieu physique, le fait attesté dans le canon, et le récit ou la rumeur qui le décrit ;
- **religions**, en séparant la croyance, la communauté de fidèles et l'organisation qui l'administre (structure Pecking Order) ; les affiliations sont datées et peuvent coexister, et ne s'additionnent pas comme des populations. Dans le méta-modèle (§7.8), cela donne : la croyance est une `Idea`, la communauté est la distribution d'adhésion, et l'organisation est une entité liée par `ADMINISTERS`. Le cas des religions devient celui de toutes les idées ;
- **biens et recettes**, dans un registre daté de stocks, droits et transferts avec contreparties. C'est le même registre que les ressources (R5) : une seule économie ;
- **voyages** structurés en segments ;
- **cartographie** : labels et exports à une date, sans tirage de faits.

Ces contenus arrivent par des packs figés, extraits pendant la préparation avec provenance `imported`, ou par des fonctions Rust pures, avec provenance `derived`.

**Règles de génération retenues :** FMG donne de bonnes **règles de départ**. Chronos les rend **temporelles et causales** :

| Règle FMG | Devient dans Chronos | Où |
|---|---|---|
| Expansion multi-sources à coût, plafond `growthRate` | Moteur unique d'expansion, en coûts entiers et départage total ; plafond = f(`m`) | §7.3, AZ1 |
| Types culturels (nomade, naval, fluvial…) | Étiquette lue dans les activités et le milieu, jamais assignée | §7.8, AZ3 |
| Score d'habitabilité (débit, confluences, côte) | Capacité témoin invariante à la résolution | §7.3, AZ4 |
| Vitesses de transport | Coût d'arête en heures, partagé | §7.3, AZ2 |
| Formes d'État et titres | Étiquette lue dans la structure de pouvoir (Pecking Order) ; une révolution change la structure, et l'étiquette suit | §7.8, AZ11 |
| Provinces, colonies | `IsPartOfMe[province]`, colonies du scénario colonisation | §7.3, AZ12 |
| Folk / organisée / culte / hérésie, religion d'État | Étiquettes de lecture sur le moteur générique d'idées : adhésions, appuis, lignée `DERIVED_FROM` datée | §7.8, AZ7 |
| Diplomatie par distance, vassalité, cascade de guerre | Relations datées avec cause ; cascade résolue une fois | §7.3–7.4, AZ5 |
| Urquhart + remise ×0,5 | Corridors par époque, sans re-routage | §7.3–7.4, AZ6 |
| Taxes, trésor, tarifs et embargos | Registre de ressources, flux par bloc, embargos datés, plus les dépenses | §7.4, AZ18 |
| Zones (épidémie sur les routes, rébellions) | Projection à une date des événements actifs | AZ9 |
| Marqueurs à légende causale | Vestiges datés d'événements clos | AZ14 |
| Verrous, IDs réservés | Ancres et contraintes d'auteur | AZ16 |

**Écarté :** la génération statique avec un passé plaqué après coup ; l'aléa séquentiel global et les réensemencements corrélés ; l'échelle unique dépendante de la résolution ; l'état global mutable ; la matrice diplomatique sans date ni cause ; l'économie à l'équilibre en une passe ; les anachronismes. FMG n'apporte rien à la généalogie (P4).

**Règle générale (§7.8) :** on ne copie pas la structure d'Azgaar quand un mécanisme générique donne le même résultat. Ses types (religion d'un État, type culturel, forme de gouvernement) deviennent des **étiquettes de lecture**. Ses contenus (doctrines, noms, emblèmes, biens) deviennent des **catalogues**.

### 7.8 Méta-modèle : idées, adhésions, appuis et étiquettes dérivées

Statut : **proposition**.

**Principe.** Chronos ne stocke pas « la religion d'un pays », « la langue d'un peuple » ou « la forme d'un État » comme des champs. Il représente quatre mécanismes génériques, et ces notions sont des **lectures** de son état. Les doctrines, langues, coutumes, savoirs et styles viennent de catalogues (dont Azgaar) : du vocabulaire, pas de la structure. Un pays a des croyances, souvent une majoritaire ; une religion d'État *peut* en émerger, elle n'est pas imposée.

**1. Idées : un seul type pour tout ce qui se transmet.** `Idea` couvre croyance, langue ou dialecte, coutume ou loi, savoir ou technique (y compris la magie), idéologie, style (art, architecture, héraldique) et mythe. Elle porte :
- **une dimension**, qui fixe ses règles. Une croyance peut être exclusive ou cumulable (monothéisme, panthéon) ; on parle plusieurs langues ; un savoir s'accumule sans en chasser un autre ;
- **des propriétés génériques** :
  - attrait, c'est-à-dire la tendance à l'adoption spontanée ;
  - exclusivité dans sa dimension ;
  - coût d'adoption (apprentissage, initiation) ;
  - modes de transmission : verticale (parents), horizontale (pairs, commerce), oblique (élites, institutions) ;
  - prérequis (écriture, clergé, maître) ;
- **des effets sur son porteur** : cohésion, légitimité, tabous qui filtrent des actions, savoirs qui débloquent des activités ;
- **une lignée** : `DERIVED_FROM` (schisme, dialecte, réforme) et `MERGED_FROM` (syncrétisme, créole), créées par des événements datés.

**2. Adhésions : des distributions, pas des attributs.** Chaque cohorte (population comptée une seule fois) porte, par dimension, une distribution sur les idées : comptes entiers, les k premières plus « autres ». Une personne canonique porte ses propres idées. Pour un agrégat (région, pays, peuple), la distribution est la **somme de ses membres**. « La religion majoritaire du royaume » est donc un argmax calculé, jamais stocké. Les minorités et les doubles appartenances (quand l'exclusivité est faible) viennent naturellement.

**3. Appuis : les seules choses stockées, parce que ce sont des décisions.** Ce sont des relations datées avec cause, d'une entité-acteur vers une idée :
- `ENDORSES` : religion d'État, langue officielle, code de lois ;
- `ADMINISTERS` : une Église, une école, une guilde qui garde un savoir ;
- `SUPPRESSES` : interdit, persécution ;
- `PATRONS` : mécénat.

Une religion d'État naît d'un événement « adoption », quand les pressions s'y prêtent : part majoritaire, conversion du souverain, besoin de légitimité, poids d'une organisation. Elle peut être révoquée.

**4. Dynamique : un seul moteur pour toutes les dimensions.** À chaque bloc et dans chaque région, les parts évoluent par :
- transmission verticale, qui suit la démographie des cohortes ;
- diffusion par contact, pondérée par les trajets, le commerce et les migrations, sur le même graphe qu'AZ1 et AZ2 ;
- biais de conformité (majorité locale) et de prestige (élites, villes) ;
- coercition et incitation : appuis, missions des organisations ;
- divergence en isolement, qui produit une nouvelle idée (dialecte, hérésie) au-delà d'un seuil ;
- syncrétisme au contact prolongé, si l'exclusivité est faible.

Les conversions sont des **faits agrégés à budget**, par exemple « région R, [1320, 1330) : 4 000 adhérents passent de X à Y ». Elles sont réparties exactement au raffinement. Les seuils et paramètres dépendent de la dimension.

Pistes de littérature, à vérifier avant toute citation formelle :
- Cavalli-Sforza et Feldman (1981), Boyd et Richerson (1985) : transmission culturelle, biais de conformité et de prestige ;
- Axelrod (1997) : dissémination culturelle et risque d'homogénéisation ;
- Stark et Bainbridge : sectes, Églises, économie religieuse ;
- Weber : routinisation du charisme ;
- Turchin : frontières méta-ethniques ;
- Kuran : falsification des préférences sous coercition.

**5. Étiquettes dérivées : des classifieurs purs, versionnés, documentés dans `docs/methods`.**

| Étiquette | Exemple de règle de lecture |
|---|---|
| Folk, organisée, culte, hérésie | Folk : aucune organisation `ADMINISTERS`, liée à une seule culture. Organisée : une organisation l'administre. Culte : petite, exclusive, récente. Hérésie : `DERIVED_FROM` une idée administrée, avec griefs ou `SUPPRESSES` entre les deux |
| Religion majoritaire, religion d'État | Argmax des adhésions ; existence d'un `ENDORSES` actif |
| Monarchie, république, théocratie, oligarchie | Structure Pecking Order : qui détient les pouvoirs souverains, succession héréditaire ou élective, rôle du clergé |
| Chefferie, cité-État, royaume, empire | Taille, nombre de peuples et de vassaux, profondeur de `IsPartOfMe` |
| Peuple naval, nomade, montagnard… | Parts d'activités (pêche, pastoralisme mobile, commerce maritime) et milieu de l'empreinte |
| Culture, ethnie | Groupe d'idées partagées : langue, coutumes, croyances. Une culture est une `catégorie` lue dans les distributions, ce qui rend l'ethnogenèse possible (scission, fusion) |

Les titres (khanat, califat, doge) sont le nom de l'étiquette dans la langue ou la culture concernée, tiré d'un catalogue.

**6. Ce qui reste typé, et pourquoi.**
- **La dimension d'une idée** : une croyance et une langue n'ont ni les mêmes règles d'exclusivité ni la même transmission.
- **Ressources et biens** : grandeurs physiques soumises à la conservation.
- **Démographie et cohortes.**
- **Rôles et pouvoirs** (Pecking Order).
- **La religion reste un moteur majeur de civilisation.** Elle passe par le mécanisme générique, sans code à part, mais avec un poids à part : exclusivité forte, effets sur la légitimité et la cohésion, organisations puissantes, tabous qui filtrent les actions, griefs et `SUPPRESSES` qui alimentent des guerres de religion.

**7. Réalisme et risques.**
- **Coût**, ordre de grandeur non mesuré : 2 000 régions × 250 blocs × 4 dimensions × 8 idées, soit environ 16 M mises à jour entières de parts. C'est compatible avec l'objectif du §6.
- **Dynamique :** une diffusion pure homogénéise tout (Axelrod) ; trop de divergence donne le chaos. Il faut des forces de rappel (identité, isolement, coercition), des tests de diversité sur plusieurs siècles, et des contraintes d'auteur pour fixer une religion ou une langue voulue.
- **Explicabilité :** chaque changement de majorité ou d'appui est un événement cité ; chaque étiquette s'explique par sa règle.
- **Déterminisme :** flux de parts entiers, ordre canonique, aléa `key(seed, région, bloc, dimension, slot)`.
