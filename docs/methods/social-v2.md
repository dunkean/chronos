# Méthode sociale v2 — socle S0

## État implémenté

Le module public `chronos_core::v2` livre les contrats de données du lot S0.
Il est indépendant de `chronos-history-1` : le seul ajout au code v1 est
`pub mod v2;`. `chronos-social-2` désigne la famille v2, sans annoncer un
générateur social complet. Aucun fournisseur, processus Python, accès disque,
horloge ou nouvelle dépendance n'est ajouté au noyau.

### Adresses et décisions indexées

`ids::SemanticAddress` porte le type et les champs de l'adresse. Les
constructeurs couvrent entités, personnes, mandats, événements, groupes,
ménages, transferts, domaines et plans ; `relation` factorise le préfixe
commun des extrémités. `encode` écrit `chr2\0`, puis, pour chaque champ,
sa longueur u32 little-endian et son UTF-8, en base64url sans bourrage.
`decode` vérifie aussi l'encodage canonique, les longueurs, le type, l'ASCII
normalisé et les entiers décimaux. Un ID dépassant 4 096 caractères est refusé.

Le séparateur de préfixe retenu est `:` (`event:…`, `person:…`). Les champs
de portée simples sont des libellés sémantiques normalisés de la fixture,
pas des IDs déjà encodés. Pour un domaine ou un office lui-même adressé,
`person_in_domain`, `plan_for_domain` et `tenure_for_office` reprennent ses
champs à plat. Le manifeste doit fixer la représentation de chaque portée ;
les formes de portée simple et composée ne sont pas des alias interchangeables.
Un champ contenant un ID v2 encodé est refusé. La migration, un changement
de nom et une variation de poids ne participent pas à une nouvelle adresse.

`ids::random_semantics_v2` est la façade des tirages. `domain_seed` appelle
exactement le mélangeur v1 avec les trois parties du §4.1. Les autres tirages
préfixent `random_semantics_v2`, le nombre de parties, leurs encodages
base64url, le sous-compteur et l'essai de rejet, tous explicites. Le mélangeur
`keys.rs` reste intact. `uniform_below` est uniforme sur `[0, upper)` par
rejet du préfixe de taille `2^64 mod upper`, sans état partagé.

### Octets canoniques

`canonical::canonical_bytes(&serde_json::Value)` produit du JSON UTF-8 compact,
sans BOM, avec clés triées par octets UTF-8, entiers décimaux et échappements
JSON minimaux. Tout nombre flottant est refusé, y compris `1.0`. L'API reçoit
une valeur JSON déjà construite : elle ne convertit pas des objets Rust
flottants, ce qui évite de transformer implicitement NaN en `null`.

La fonction préserve l'ordre des listes. L'appelant doit fournir la projection
contractuelle : listes de faits triées par ID, épisodes et registres dans leur
ordre explicite, champs descriptifs et durées exclus. Elle ne devine ni les
colonnes d'identité ni les champs à exclure dans un JSON arbitraire. Pour le
calcul de `plan_fingerprint`, l'hôte retire `plan_id` et `plan_fingerprint` ;
S0 ne calcule et n'authentifie aucun SHA-256. Les empreintes et IDs lisibles
`example` de la fixture restent des valeurs de démonstration de forme.

Les vecteurs [canonical-json.v1.json](../design/chronos-social-v2/vectors/canonical-json.v1.json)
contiennent entrée, texte UTF-8 attendu et octets hexadécimaux. Ils couvrent
Unicode, contrôles, ordre des clés, ordre conservé des listes et entier u64
maximal. Ils ont été produits avec le JSON standard Python et sont comparés
aux octets Rust par les tests S0 ; cela ne valide pas encore un worker.

### Atlas, modèle social et temps

`atlas::HistoryAtlas::validate` vérifie références et unicité, hiérarchie
acyclique, bornes et référentiels, voisins réciproques, catalogues complets
de sites par sous-arbre et sommes des populations attendues maximales.
Fondations futures et abandon imposé sont refusés. Les occupations couvrent
exactement l'horizon, sans lacune ni chevauchement. Les routes portent des
jours entiers, une validité et un sens ; une liste vide reste une liste vide.
Les contacts désignent une entrée locale, un contexte extérieur, un profil
et, éventuellement, une route d'accès existante.

Le JSON d'atlas n'étant pas figé par un schéma dans l'étude, S0 emploie des
surfaces `area_m2` entières et des bornes métriques `*_mm`, sans conversion
géographique ni interprétation des LOD d'affichage. Les résumés d'une cellule
incluent ses descendants. La partition démographique n'est pas implémentée.

`model` sépare provenance, précision et couverture des comptes, entités,
états, relations et personnes. Les prédicats du §5.1 sont une énumération
fermée. Les phases d'événement sont sérialisées avec leurs numéros du §7.3.
`TimeSpan` décrit `[start_year, end_year)` ; seules les vies peuvent avoir
un intervalle vide. `PersonRecord::is_alive_and_covered` exclut l'année du
décès et toute date hors couverture, même si le décès est inconnu.

`validate::causal_ranks` refuse les références absentes, doublons, causes
futures et cycles. Tout cycle causal reçoit `CanonicalDependencyCycle`,
y compris s'il traverse des phases ou des années. Les rangs sont construits
par tri topologique dans chaque phase ; l'ID départage seulement les nœuds
déjà disponibles. Une cause de même année en phase antérieure est acceptée.
`validate_composition` refuse les cycles stricts `component_of` sur l'ensemble
des épisodes, tout en autorisant les affiliations superposées. Ces contrôles
ne constituent pas encore une validation de généalogie fournisseur.

### Impacts

`region::resolve_impact` consomme un `CompleteCandidateCatalogue` du
propriétaire. Pour les sites, `from_atlas_sites` extrait ce catalogue de
l'atlas validé. Pour les autres types, `from_owner_catalogue` exige que
l'autorité appelante fournisse son catalogue complet ; S0 ne peut pas prouver
la complétude d'un ensemble arbitraire externe. Une projection ne produit
jamais un catalogue et ne recalcule jamais de quota.

En mode probabiliste, chaque cible éligible tire une seule exposition en bps,
sans dimension annuelle. Le mot aléatoire d'intensité a sa propre clé. Il est
livré comme `intensity_draw` ; l'interprétation du profil de gravité et les
épisodes locaux relèvent de S2. Les projections conservent l'ID de l'événement
parent ; S0 n'invente pas un nouvel événement local à chaque consultation.

En mode exact, K est un compte déclaré ou une fraction du catalogue éligible.
L'arrondi est obligatoire : seuls `floor` et `ceil` sont pris en charge,
sans arrondi implicite. Le classement utilise priorité puis ID, sur l'ensemble
complet, et refuse plus de 100 000 candidats avant filtrage. Un K impossible
échoue. Les modes exacts, y compris cible fixe, sur personnes et cohortes sont
refusés avec `UnsupportedCapability` jusqu'à S6.

### Plans de population

`population::PopulationPlan` reprend le schéma 1, avec types fermés,
`deny_unknown_fields`, champs nullable obligatoires et effets optionnels
omettables mais non nuls. Les variantes futures restent lisibles.
`validate_plan(plan, references)` vérifie le schéma embarqué à la compilation,
les bornes, unités, dates, références, poids positifs, rangs sociaux contigus,
graine de domaine, couvertures d'épisodes et budgets du prototype.
`PlanReferences` fournit les événements, offices, contacts, raccords culturels,
activités, empreintes des packs et maxima attendus de l'atlas sur tout l'horizon.
Les marqueurs et fondateurs doivent respecter ces maxima ; leur somme par
domaine ne dépasse pas 20 000. Ces références sont préparées par l'autorité
du canon et des catalogues, jamais par la fenêtre de consultation.

Le validateur accepte les données ordinaires S0–S5 comme contrats à préparer ;
il n'exécute aucune capacité S3. Il refuse capacités inconnues, champs S6–S8,
personnes fixées, contraintes démographiques exactes, opérations personnelles,
rôles requis, noyau requis, ponts nominatifs, reproduction personnalisée et
fournisseur entièrement procédural. La politique libre requiert les quatre
tentatives déclarées du §25.4, sans les exécuter.

Les paramètres reconnus sont ceux de l'exemple normatif, tous explicites.
Les `_bps`, `_ppm`, `_mm` et âges sont contrôlés sans flottants. Le dictionnaire
exécutable S1 n'étant pas livré, seule la valeur de résidence `either`, attestée
par l'exemple, est acceptée ; les autres sont refusées explicitement plutôt
que d'inventer une liste. Les conversions d'effets du §26.4 restent à effectuer
dans l'adaptateur S3, séparément des noms de paramètres de profils.

## Proposé, non implémenté

S1–S8 conservent leur périmètre dans l'étude : catalogues exécutables et raccords,
canon général, partition des domaines, compilation des gravités, fournisseur
démographique, validation des résultats et bilans, recrutement, API et vues,
contraintes personnelles exactes, ponts et optimisations. Les signatures de
fonctions proposées au §14.1 ne sont pas des stubs exécutables dans S0.

## Validation et mesures

`tests/v2_s0.rs` contient la fixture pure (seed 42, horizon 1300–1400,
`chronos-social-2`) et les régressions contractuelles, dont deux résolutions
indépendantes pour comparer accès direct et projection d'événement, sans
fournisseur. Les tests de probabilité vérifient les décisions par cible,
jamais un quota sur leur somme. Les tests unitaires vérifient aussi l'encodage
binaire et le chemin de rejet de l'uniforme entier. L'exemple de plan est
désérialisé, validé avec ses références de fixture, puis comparé en canonique.
Aucune mesure de performance ni exécution démographique n'est revendiquée.
