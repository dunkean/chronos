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
| `site`, `activity` | monde, ID de catalogue |
| `entity` | propriétaire, type, `origin_block`, slot |
| `asset` | propriétaire, `asset`, type, `origin_block`, slot |
| `event` | propriétaire, type, `origin_block`, ordinal |
| `impact` | champs de l'événement parent, slot du contrat |
| `effect` | champs du contrat, type de la cible, champs de la cible sans le monde |
| `group` | propriétaire, `group_slot`, archétype, `origin_block`, ordinal |
| `transfer` | propriétaire, `transfer`, origine, destination, année, ordinal |
| `tenure` | propriétaire, `office`, `origin_block`, slot, ordinal |
| `relation` | propriétaire, `relation`, prédicat, types des extrémités, trois segments comptés, année, ordinal |
| `standing` | propriétaire, `standing`, types des extrémités, trois segments comptés, année, ordinal |

`DomainKey` est le domaine structuré. `OwnerScope` est la portée d'un propriétaire,
en quatre variantes : `world`, `cell`, `site`, `domain`. `Owner` lui ajoute le
monde et entre dans l'adresse à plat, précédé de son discriminant, sans jointure
de chaînes. Une cellule dont le nom imiterait un domaine reste donc une autre
adresse. Les faits portent `owner_scope: OwnerScope`, sans le monde : l'atlas
porte aussi ce type, et il est une entrée du canon, donc antérieur à tout
`canon_id`. `SemanticAddress::owner` relit le propriétaire d'une adresse. Un
mandat prend l'adresse de son office, qui doit être une entité de type `office`.

Les champs littéraux (`household`, `transfer`, `relation`, `standing`, `asset`,
`group_slot`, `office`) sont vérifiés à la construction comme au décodage. `relation` refuse un prédicat
hors de l'énumération fermée et un préfixe commun non maximal. `origin_block`
est l'année de début du bloc : il doit être aligné, et `ids::decade_block` le
calcule par `div_euclid`, y compris avant l'an zéro (`-1` donne `-10`). Années
et blocs sont bornés à ±100 000.

Un champ ne contient jamais d'ID encodé. Une sous-chaîne n'est refusée que si
elle décode en vraie adresse : payload `chr2\0`, champs complets et structure d'un
type connu, à toute position, avec ou sans préfixe de type, suivie ou non d'autres
caractères. La seule présence de la signature base64url `Y2hyMg` ne suffit pas.
Une adresse dont la structure est valide mais qui contient elle-même un ID reste
reconnue, ce qui attrape une double imbrication. La migration, un changement de
nom et une variation de poids ne participent pas à une nouvelle adresse.

Un `ObjectRef` a une seule forme canonique, qu'`ObjectRef::address` vérifie : une
personne est un `PersonSlot`, une entité de type `office` un `Office`, une autre
entité, un groupe ou un ménage une `Entity`. Le constructeur de relation accepte
tout `ObjectRef` adressable, sites, actifs et activités compris, et `link` relit
propriétaire, prédicat, extrémités et début.

`impact` adresse un contrat, fils de son événement. `effect` adresse l'exposition
d'une cible à ce contrat, fille du contrat et unique par cible ; il n'a pas de
dimension annuelle. Les épisodes du profil temporel (§7.2) seront ses sous-slots,
ajoutés en S2.

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

Le §4.2 vaut pour tout le socle. `plan_canonical_bytes`, `HistoryAtlas::canonical_bytes`
et `canonical_list_bytes` appliquent l'ordre canonique sans exiger qu'il soit celui
reçu. `canonical_list_bytes` traite toute liste de `CanonicalRecord` : entités,
états, relations, événements, personnes et statuts légaux. Elle trie par ID, ou par
entité puis date pour les états, et trie les listes internes d'IDs. Épisodes et
registres gardent leur ordre, qui est un fait.

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

`ExternalContact` a un propriétaire unique, le monde ou une cellule qui enclôt ses
sites, et deux extrémités triées. Une extrémité est un site d'attache, avec le
contexte, le profil et la population qu'il voit au-delà et sa voie d'accès
éventuelle, ou un contexte purement extérieur. Il faut au moins un site. Face à un
contexte extérieur, le site déclare ce même contexte. Deux sites peuvent partager
un contact : chacun ouvre alors son port sous le même `contact_id`.

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
nom évite la confusion avec l'axe `coverage` du §4.3. `is_alive_and_covered` exclut
l'année du décès et toute date hors de cet intervalle, même si le décès est inconnu.
`parent_refs` vise une ligne (`slot`) ou une référence extérieure typée
(`coverage_limit`, `external_arrival`). `PersonRecord::validate` borne les dates,
refuse un décès avant la naissance, valide `observed_span`, et limite les parents
à deux, seule la reproduction biparentale étant livrée.

**Identité des enregistrements.** `validate` prouve qu'un fait est bien celui que
son ID adresse. Erreur : `InvalidRecordIdentity`.

| Fait | Contrôle |
|---|---|
| `SocialEntity` | `id` égale `semantic_address` ; propriétaire égal à `owner_scope` ; entité : `kind` dans l'adresse ; slot de groupe : `archetype_id` dans l'adresse ; ménage : `kind = household` ; personne : `kind = person` et `nature = person`, seule nature admise pour cette adresse |
| `HistoryEvent` | adresse d'événement de ce type et de ce propriétaire |
| `SocialRelation` | adresse de relation de mêmes prédicat, extrémités et début |
| `LegalStanding` | adresse de statut de mêmes sujet, juridiction et début |
| `PersonRecord` | adresse de personne dont l'ID local est `provider_ref` ; parents de table adressés |
| `ImpactContract` | adresse d'impact, fille de `parent_event_id` et portée par `owner_scope` |

`causal_ranks`, `validate_composition`, `validate_legal_standings` et `resolve_impact`
appellent ces contrôles. Un slot de groupe ne contraint pas `kind`, que son adresse
ne porte pas.

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

`EntityState` porte `alignment`, `conduct_tags` et `visibility`. Un tag de conduite
est un ID, pas une énumération Rust : le catalogue évolue en donnée (§5.2), et
`EntityState::validate` et `PersonRecord::validate` reçoivent ce catalogue en
paramètre. `PersonRecord.alignment_episodes` est une liste ordonnée : tout épisode
après le premier cite l'événement qui change les axes.

`LegalStanding` est relatif à une juridiction et daté. Son sujet est une entité,
une activité ou un slot de personne ; sa juridiction est une entité.
`legal_status_at` rend `licit` sans fiche applicable, et une autre juridiction
n'a aucun effet. `validate_legal_standings` reçoit `observed_start` et la fin du
monde, regroupe les fiches par couple sujet/juridiction et refuse chevauchements
et sorties de période. Seul un segment commençant à `observed_start` peut être sans
cause ; tout autre début en cite une, y compris un premier segment tardif. Un
segment qui finit avant la fin du monde sans successeur immédiat, ou avant un
trou, cite son événement de fin dans `end_cause_event_id`. Toutes ces causes sont
de type `edict`, `charter`, `heresy_trial` ou `banishment`, sans être postérieures
à la date qu'elles expliquent. Un groupe fondé après `observed_start` dont le statut
n'est pas `licit` cite donc l'un de ces événements dès sa première fiche.

### Impacts

`region::resolve_impact` consomme un `CompleteCandidateCatalogue`. Ce type n'a ni
champ public ni `Deserialize` et ne se construit que depuis une source d'autorité
du noyau : `from_atlas(&atlas, &owner)`, qui énumère elle-même les sites de l'atlas
validé, pour le monde, une cellule ou un site. Aucune liste n'est reçue de
l'appelant. Un domaine est refusé avec `UnsupportedCapability` : sa partition
appartient à S2, qui ajoutera d'autres sources de la même façon. Une projection ne
produit jamais un catalogue et ne recalcule jamais de quota.

Le contrat, son événement parent, le catalogue et les effets partagent canon et
propriétaire : un contrat d'un autre monde ou d'une autre portée que le catalogue
est refusé avec `InvalidImpactContract`.

En mode probabiliste, chaque cible éligible tire une seule exposition en bps,
sans dimension annuelle. Le mot aléatoire d'intensité a sa propre clé. Il est
livré comme `intensity_draw` ; l'interprétation du profil de gravité et les
épisodes locaux relèvent de S2. Chaque `TargetImpact` porte l'ID de l'événement
parent et un `effect_id` propre à sa cible. Ces identités sont dérivées du
contrat et de la cible : S0 n'invente pas de fait à la consultation. Les clés de
tirage portent l'ID du contrat, donc son adresse.

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
social, `plan_id` l'adresse de plan du même domaine. `domain_sites` porte la
partition attendue : les sites du plan égalent exactement ceux de son domaine, et
les mêmes sites sous un autre ordinal sont refusés. L'exemple de plan porte de
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

`plan_canonical_bytes` exclut `plan_id` et `plan_fingerprint`. Sont triés par ID :
capacités, profils, sites, demandes de rôles, personnes fixées, contraintes, ports,
routes et leurs listes d'IDs ; les marqueurs le sont par site puis année, les chocs
par événement, période et sites. Les épisodes d'occupation, épisodes de taux et
périodes protégées gardent leur ordre.

`historical_program` est une liste explicitement ordonnée par `(année, phase, op_id)`,
clé d'application et de normalisation que rend `application_key`. La phase découle
du type d'opération : 10 pour `protect_person`, à sa date de début ; 30 pour
`force_death` ; 40 pour `move_households` et `admit_people`.

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
fonctions proposées au §14.1 ne sont pas des stubs exécutables dans S0. Sources de
catalogue autres que l'atlas, sous-slots d'effet, génération des axes, `alignment_fit`, inéligibilité, statuts initiaux et rendu des arbres de
nom appartiennent aux lots suivants.

## Validation et mesures

`tests/v2_s0.rs` contient la fixture pure (seed 42, horizon 1300–1400,
`chronos-social-2`) et les régressions contractuelles. La preuve de sortie S0
compare, sans fournisseur, l'accès direct par cible et la projection paginée d'une
résolution indépendante : un événement-cadre, deux batailles filles sur deux
cellules, une épidémie, des `effect_id` distincts par cible et les `intensity_draw`.
Les tests de normalisation permutent chaque liste triée du plan et de l'atlas, avec
au moins deux éléments. Leur pouvoir d'échec n'a été vérifié qu'en partie : une
campagne de retrait des comportements a été interrompue, et la plupart des tris
de normalisation n'y sont pas passés. Les tests de probabilité vérifient les décisions
par cible, jamais un quota sur leur somme. L'exemple de plan est désérialisé,
validé avec ses références de fixture, puis comparé en canonique. Aucune mesure de
performance ni exécution démographique n'est revendiquée.
