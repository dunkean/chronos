# Méthode sociale v2 — socle S0

## État implémenté

Le module public `chronos_core::v2` livre les contrats de données du lot S0.
Il est indépendant de `chronos-history-1` : le seul ajout au code v1 est
`pub mod v2;`. `chronos-social-2` désigne la famille v2, sans annoncer un
générateur social complet. Aucun fournisseur, processus Python, accès disque,
horloge ou nouvelle dépendance n'est ajouté au noyau.

### Adresses et décisions indexées

`ids::SemanticAddress` porte le type et les champs de l'adresse. `encode` écrit
`chr2\0`, puis, pour chaque champ, sa longueur u32 little-endian et son UTF-8,
en base64url sans bourrage, derrière le préfixe `type.` (`event.Y2hy…`). Le
point est absent de l'alphabet base64url. `decode` vérifie aussi l'encodage
canonique, les longueurs, le type, l'ASCII normalisé et les entiers décimaux.
Un ID dépassant 4 096 caractères est refusé.

**Chaque objet a une seule forme d'adresse.** `from_fields`, seule porte
d'entrée y compris pour `decode` et serde, fixe par type le nombre et le rôle
des champs :

| Type | Champs |
|---|---|
| `domain`, `plan` | monde, région, profil social, ordinal |
| `person` | domaine à plat, algorithme fournisseur, ID local |
| `household` | domaine à plat, `household`, origine, année de création |
| `entity` | propriétaire, type, `origin_block`, slot |
| `event` | propriétaire, type, `origin_block`, ordinal |
| `effect` | champs de l'événement parent, contrat, cible typée |
| `group` | propriétaire, `group_slot`, archétype, `origin_block`, ordinal |
| `transfer` | propriétaire, `transfer`, origine, destination, année, ordinal |
| `tenure` | propriétaire, `office`, `origin_block`, slot, ordinal |
| `relation` | propriétaire, `relation`, prédicat, types des extrémités, trois segments comptés, année, ordinal |

`DomainKey` et `Owner` sont les types structurés de domaine et de propriétaire.
Un `Owner` est une cellule d'atlas ou un domaine ; il entre dans l'adresse à
plat, précédé de son type (`cell` ou `domain`), sans jointure de chaînes. Une
cellule dont le nom imiterait un domaine reste donc une autre adresse. Un
mandat prend l'adresse de son office, qui doit être une entité de type `office`.

Les champs littéraux (`household`, `transfer`, `relation`, `group_slot`, `office`)
sont vérifiés à la construction comme au décodage. `relation` refuse un prédicat
hors de l'énumération fermée et un préfixe commun non maximal. `origin_block`
est l'année de début du bloc : il doit être aligné, et `ids::decade_block` le
calcule par `div_euclid`, y compris avant l'an zéro (`-1` donne `-10`). Années
et blocs sont bornés à ±100 000.

Un champ ne contient jamais d'ID encodé : la signature base64url de `chr2`
(`Y2hyMg`) est refusée à toute position, payload nu ou occurrence en milieu de
champ compris. Une chaîne qui contiendrait ces six caractères par hasard serait
donc refusée aussi. La migration, un changement de nom et une variation de
poids ne participent pas à une nouvelle adresse.

`effect` identifie l'effet d'un contrat d'impact sur une cible. Une cible
lisible de catalogue y entre comme libellé ; une cible déjà adressée y entre à
plat, sans son monde.

`ids::random_semantics_v2` est la façade des tirages. `domain_seed` appelle
exactement le mélangeur v1 avec les trois parties du §4.1. Les autres tirages
préfixent `random_semantics_v2`, le nombre de parties, leurs encodages
base64url, le sous-compteur et l'essai de rejet, tous explicites. Le mélangeur
`keys.rs` reste intact. `uniform_below` est uniforme sur `[0, upper)` par
rejet du préfixe de taille `2^64 mod upper`, sans état partagé. Les tests
épinglent ces dérivations par des valeurs littérales, obtenues d'un port Python
indépendant de `keys.rs` : changer une clé fait échouer un test.

### Octets canoniques

`canonical::canonical_bytes(&serde_json::Value)` produit du JSON UTF-8 compact,
sans BOM, avec clés triées par octets UTF-8, entiers décimaux et échappements
JSON minimaux. Tout nombre flottant est refusé, y compris `1.0`. L'API reçoit
une valeur JSON déjà construite : elle ne convertit pas des objets Rust
flottants, ce qui évite de transformer implicitement NaN en `null`.

La fonction préserve l'ordre des listes. L'appelant doit fournir la projection
contractuelle : listes de faits triées par ID, épisodes et registres dans leur
ordre explicite, champs descriptifs et durées exclus. Elle ne devine ni les
colonnes d'identité ni les champs à exclure dans un JSON arbitraire. S0 ne
calcule et n'authentifie aucun SHA-256 : `plan_canonical_bytes` fournit les
octets, l'hôte calcule l'empreinte. Les empreintes `example` de la fixture
restent des valeurs de démonstration de forme.

Les vecteurs [canonical-json.v1.json](../design/chronos-social-v2/vectors/canonical-json.v1.json)
contiennent entrée, texte UTF-8 attendu et octets hexadécimaux. Ils couvrent
Unicode, contrôles, ordre des clés, ordre conservé des listes et les deux bornes
entières. `integer_bounds` documente ces bornes : de `-9223372036854775808` à
`18446744073709551615`, incluses. `rejections` liste les textes à refuser :
flottants, exposants, `-0`, entiers hors bornes, `NaN`, `Infinity`, surrogate
isolée et BOM. Rust les refuse au décodage du texte ou à la canonisation. Le
module `json` de Python accepte plusieurs de ces textes par défaut : le futur
worker devra les refuser explicitement. Ces vecteurs ne valident pas encore un
worker. Une clé d'objet dupliquée dans un texte JSON n'est pas détectée : le
décodage garde la dernière valeur avant que `canonical_bytes` ne la reçoive.

### Atlas, modèle social et temps

`atlas::HistoryAtlas::validate` vérifie références et unicité, hiérarchie
acyclique, bornes et référentiels, voisins réciproques, catalogues complets
de sites par sous-arbre et sommes des populations attendues maximales.
Fondations futures et abandon imposé sont refusés. Les occupations couvrent
exactement l'horizon, sans lacune ni chevauchement. Les routes portent des
jours entiers, une validité et un sens ; une liste vide reste une liste vide.
Les IDs d'atlas sont en ASCII imprimable sans espace, les coordonnées bornées à
±9e15 mm. Toute erreur porte l'atlas comme propriétaire et sa date d'ouverture.

`ExternalContact` a un propriétaire unique et deux extrémités triées. Une
extrémité est un site d'attache, avec le contexte, le profil et la population
qu'il voit au-delà et sa voie d'accès éventuelle, ou un contexte purement
extérieur. Il faut au moins un site. Face à un contexte extérieur, le site
déclare ce même contexte. Deux sites peuvent partager un contact : chacun ouvre
alors son port sous le même `contact_id`.

Le JSON d'atlas n'étant pas figé par un schéma dans l'étude, S0 emploie des
surfaces `area_m2` entières et des bornes métriques `*_mm`, sans conversion
géographique ni interprétation des LOD d'affichage. Les résumés d'une cellule
incluent ses descendants. La partition démographique n'est pas implémentée.

`model` sépare provenance, précision et couverture des comptes, entités,
états, relations et personnes. Les prédicats du §5.1 sont une énumération
fermée. Les phases d'événement sont sérialisées avec leurs numéros du §7.3.
`TimeSpan` décrit `[start_year, end_year)` ; seules les vies peuvent avoir
un intervalle vide. `HistoryEvent.end_year` donne la fin exclusive d'un
événement-cadre et vaut `null` pour un événement daté.

`CountMeasure.value`, `PersonRecord.death_year`, `HistoryEvent.end_year` et les
causes de `LegalStanding` et `AlignmentEpisode` exigent la présence explicite
de `null` : un champ absent est refusé, car `null` dit « inconnu » ou « aucun ».

`PersonRecord.observed_span` donne les années observées par le fournisseur ; le
nom évite la confusion avec l'axe `coverage` du §4.3, et le §5 de la conception
écrit encore `coverage`. `is_alive_and_covered` exclut l'année du décès et toute
date hors de cet intervalle, même si le décès est inconnu. `parent_refs` vise une
ligne (`slot`) ou une référence extérieure typée (`coverage_limit`,
`external_arrival`). `PersonRecord::validate` borne les dates, les références
et leur nombre à deux, seule la reproduction biparentale étant livrée.

`SocialEntity.name_tree` réserve l'arbre de nom que produira la grammaire
Namebench. Chronos stockera l'arbre dans le canon ; le texte sera rendu à
l'affichage. S0 le type comme `NameTree`, valeur JSON opaque qui passe le
contrôle canonique à la construction comme à la lecture : un flottant y est
refusé. Le champ est absent tant qu'aucun arbre n'existe, `null` n'est pas
accepté à sa place, et S0 ne contient aucune logique de génération.

`validate::causal_ranks` refuse les références absentes, doublons, causes
futures, dates d'événement-cadre invalides et cycles. Tout cycle causal reçoit
`CanonicalDependencyCycle`, y compris s'il traverse des phases ou des années.
Les rangs sont construits par tri topologique dans chaque phase ; l'ID départage
seulement les nœuds déjà disponibles. Une cause de même année en phase
antérieure est acceptée. `validate_composition` refuse les cycles stricts
`component_of` sur l'ensemble des épisodes, tout en autorisant les affiliations
superposées. `CompositionCycle` désigne un vrai cycle ; un ID de relation
dupliqué reçoit `InvalidCanonicalValue`. Ces contrôles ne constituent pas encore
une validation de généalogie fournisseur.

### Alignement et légalité

`AlignmentAxes` porte `morality_bps` et `lawfulness_bps`, bornés à ±10 000 à la
construction comme à la lecture. `conduct_class` est une fonction pure des axes
et de `ConductThresholds` : le noyau ne fige aucun seuil, leurs valeurs viennent
d'`engine_defaults.alignment` en S1. La classe n'est jamais stockée.
`entity_conduct_class` rend `None` pour une catégorie ou une population.

`EntityState` porte `alignment`, `conduct_tags` et `visibility`. `ConductTag`
est fermé sur les six tags que nomme le §27.4 ; S1 y ajoutera les siens. Ses
variantes suivent l'ordre alphabétique pour que `BTreeSet` sérialise dans
l'ordre canonique. `PersonRecord.alignment_episodes` est une liste ordonnée :
tout épisode après le premier cite l'événement qui change les axes.

`LegalStanding` est relatif à une juridiction et daté. Son sujet est une entité,
une activité ou un slot de personne ; sa juridiction est une entité.
`legal_status_at` rend `licit` sans fiche applicable, et une autre juridiction
n'a aucun effet. `validate_legal_standings` regroupe les fiches par couple
sujet/juridiction, refuse les chevauchements et exige de chaque changement un
événement `edict`, `charter`, `heresy_trial` ou `banishment` qui ne lui soit pas
postérieur. La première fiche d'une chaîne vaut état initial et peut ne citer
aucun événement. S0 ne sait pas distinguer un état initial tardif d'un
changement depuis le `licit` implicite : ce contrôle attend les statuts de S2.

### Impacts

`region::resolve_impact` consomme un `CompleteCandidateCatalogue` du
propriétaire. `from_owner_catalogue` exige un `CatalogueProof` lié au
propriétaire et au type de cible, avec le nombre de candidats et une empreinte
du catalogue trié. Une liste tronquée, substituée ou portant d'autres dates est
refusée avec `IncompleteCatalogue`. Pour les sites, `CatalogueProof::for_atlas_sites`
et `from_atlas_sites` tirent la preuve de l'atlas validé. L'émission d'une preuve
pour un autre type reste interne au noyau, autorité future de ces catalogues :
une page ou une fenêtre de consultation n'a pas de quoi en produire une. Les champs
de la preuve sont publics pour qu'elle soit publiée avec le canon : S0 détecte un
catalogue incomplet, il n'authentifie pas une preuve forgée. L'empreinte FNV-1a
est un contrôle d'intégrité, jamais une identité. Une projection ne produit
jamais un catalogue et ne recalcule jamais de quota.

En mode probabiliste, chaque cible éligible tire une seule exposition en bps,
sans dimension annuelle. Le mot aléatoire d'intensité a sa propre clé. Il est
livré comme `intensity_draw` ; l'interprétation du profil de gravité et les
épisodes locaux relèvent de S2. Chaque `TargetImpact` porte l'ID de l'événement
parent et un `effect_id` propre à sa cible. Ces identités sont dérivées du
contrat et de la cible : S0 n'invente pas de fait à la consultation.

En mode exact, **K se calcule sur les candidats éligibles** à `eligibility_time`
et retenus par le sélecteur, jamais sur le catalogue brut : un compte déclaré
doit y tenir, une fraction porte sur leur nombre. L'arrondi est obligatoire :
seuls `floor` et `ceil` sont pris en charge, sans arrondi implicite. Le classement
utilise priorité puis ID, sur l'ensemble complet. Le budget de 100 000 candidats
se mesure en revanche sur le catalogue entier, avant filtrage : 100 000 est
accepté, 100 001 refusé. Un K impossible échoue. Les modes exacts, y compris cible
fixe, sur personnes et cohortes sont refusés avec `UnsupportedCapability` jusqu'à
S6. `ExactImpactInfeasible` ne sert qu'aux modes non probabilistes qui ne peuvent
atteindre leur cible ; un contrat mal formé reçoit `InvalidImpactContract`.

### Plans de population

`population::PopulationPlan` reprend le schéma 1, avec types fermés,
`deny_unknown_fields`, champs nullable obligatoires et effets optionnels
omettables mais non nuls. Les variantes futures restent lisibles.
`validate_plan(plan, references)` vérifie le schéma embarqué à la compilation et
lu une seule fois, les bornes, unités, dates, références, poids positifs, rangs
sociaux contigus, graine de domaine, couvertures d'épisodes et budgets du prototype.
`PlanReferences` fournit le canon, l'horizon du monde, `world_seed`, les
événements, offices, contacts, raccords culturels, activités, empreintes des
packs et maxima attendus de l'atlas sur tout l'horizon. Ces références sont
préparées par l'autorité du canon et des catalogues, jamais par la fenêtre de
consultation.

Le plan est lié à son canon : `canon_id`, `world_seed` et `timeline` égalent ceux
des références. `domain_id` est une adresse de domaine de ce monde et de ce profil
social, `plan_id` l'adresse de plan du même domaine. L'exemple de plan porte de
vraies adresses, et sa graine de domaine en découle.

`culture_variants` résout chaque `mu_culture_id` vers sa famille et son profil
social. La variante principale, toutes les variantes de poids non nul de tous les
épisodes et tous les sites du domaine doivent résoudre le profil du plan ; sinon
`UnsupportedCapability`, jusqu'à S8. Une variante de poids nul peut avoir un autre
profil mais doit exister dans le raccord. Une référence absente ou une famille
erronée reste une erreur de plan, pas de capacité.

Un profil de parenté doit reprendre le `dependent_age` du profil social, ne pas
permettre un parent encore dépendant et utiliser un modèle de ménage admis pour
le mode de résidence. Un seul modèle est livré aujourd'hui, compatible avec les
trois modes : ce dernier contrôle n'a donc d'effet qu'à l'arrivée d'un second
modèle. Un port reprend le contexte, le profil et la population de l'extrémité du
contact ancrée à son site.

`plan_canonical_bytes` exclut `plan_id` et `plan_fingerprint` et applique l'ordre
canonique sans exiger qu'il soit déjà celui du fichier. Sont triés par ID :
capacités, profils, sites, opérations, demandes de rôles, personnes fixées,
contraintes, ports, routes et leurs listes d'IDs ; les marqueurs le sont par site
puis année, les chocs par événement, période et sites. Les épisodes d'occupation,
épisodes de taux et périodes protégées gardent leur ordre. Deux plans qui ne
diffèrent que par l'ordre de leurs listes ont les mêmes octets.

Le validateur accepte les données ordinaires S0–S5 comme contrats à préparer ;
il n'exécute aucune capacité S3. Il refuse capacités inconnues, champs S6–S8,
personnes fixées, contraintes démographiques exactes, opérations personnelles,
rôles requis, noyau requis, ponts nominatifs, reproduction personnalisée et
fournisseur entièrement procédural. La politique libre requiert les quatre
tentatives déclarées du §25.4, sans les exécuter.

Les paramètres reconnus sont ceux de l'exemple normatif, tous explicites.
Les `_bps`, `_ppm`, `_mm` et âges sont contrôlés sans flottants. `residence`
accepte `patrilocal`, `matrilocal` et `either`, comme `config.py` de Lineage Lab ;
toute autre valeur est une erreur de plan. Les conversions d'effets du §26.4
restent à effectuer dans l'adaptateur S3, séparément des noms de paramètres de
profils.

## Proposé, non implémenté

S1–S8 conservent leur périmètre dans l'étude : catalogues exécutables et raccords,
canon général, partition des domaines, compilation des gravités, fournisseur
démographique, validation des résultats et bilans, recrutement, API et vues,
contraintes personnelles exactes, ponts et optimisations. Les signatures de
fonctions proposées au §14.1 ne sont pas des stubs exécutables dans S0. Génération
des axes, `alignment_fit`, inéligibilité, statuts initiaux et rendu des arbres de
nom appartiennent aux lots suivants.

## Validation et mesures

`tests/v2_s0.rs` contient la fixture pure (seed 42, horizon 1300–1400,
`chronos-social-2`) et les régressions contractuelles. La preuve de sortie S0
compare, sans fournisseur, l'accès direct par cible et la projection paginée d'une
résolution indépendante : un événement-cadre, deux batailles filles sur deux
cellules, une épidémie, des `effect_id` distincts par cible et les `intensity_draw`.
Ce test a été vérifié par mutation : `for_target` qui ignore l'ID et `project` qui
ignore son filtre le font échouer. Les tests de probabilité vérifient les décisions
par cible, jamais un quota sur leur somme. L'exemple de plan est désérialisé,
validé avec ses références de fixture, puis comparé en canonique. Aucune mesure de
performance ni exécution démographique n'est revendiquée.
