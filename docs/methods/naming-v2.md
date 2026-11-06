# Organization naming v2 — L5

## Implemented

`v2::naming` evaluates the frozen Namebench `naming-1` pack at commit
`2820b509169039bbf66c73e96061766c7edf8e06`. The three original files and their
SHA-256 fingerprints are in [fixtures/naming](../../fixtures/naming/README.md).
Chronos never reads the live Namebench repository. V1 remains unchanged.

`expand(pack, kind, culture, context, choose)` returns the existing canonical
`model::NameTree`. Choices are injected integer indices; production uses
`uniform_below(world_seed, ["chronos-namebench-1", entity_id, path], 0, n)`.
The paths are exactly `pattern` and `slot/<name>`. Candidate concepts and
patterns sort by ID; slots keep pack order. Expansion supports the seven shapes,
requirements, tones, palette/activity filters, exclusions, distinct emblems,
stem disjointness, `fits` and `pairs_with`. The most precise kind is retained in
the tree; lookup walks to the nearest parent with patterns. It does not climb
again when that level has no eligible pattern. No retry or provider fallback
occurs when expansion returns no tree.

Regional organizations receive their tree at creation in `region.rs`, before
people exist. Packet organizations receive it once in `social.rs`. Their source
`mu_culture_id` is drawn from occupation weights at founding. For groups older
than the observed timeline, the earliest observed occupation supplies weights.
The founding axes supply `ConductClass` tone. Site, region and controlling polity
are references by ID; activity is drawn from local weights intersected with the
archetype's allowed activities. Later memberships, class changes, moves and
render options never redraw a name. Homonyms are allowed.

`SocialPacket.person_group_names` adds names to existing provider household and
simulated-core lineage views, without creating organizations or changing the
population. The earliest observed episode and a stable member fix culture,
activity and the `surname` reference (provider family key). These views have no
organizational conduct axes, so their naming tone is `ordinary`. The host resolves
family keys to display surnames. `initialize_organization_name` also accepts a
surname reference for future person-derived organizations.

`GroupRecord.name` remains as a legacy descriptor for API compatibility; with a
tree its provenance is `derived`, citing the Namebench generation fingerprint.
`name_fallback` is present only for absent packs or failed kind expansion and is
explicitly `synthetic`. Unknown cultures and invalid choices produce errors.
An optional frozen pack in `SocialInputs` makes pack absence explicit; the
application's normal build embeds the supplied pack.

The world manifest includes `packs.naming_generation` and
`naming_algorithm: chronos-namebench-1`. Full frozen pack bytes are archived with
the world, but neither their whole-file hash nor the rendering fingerprint enters
world identity. Changes to generation rules require a new naming rule version;
changes to draws' inputs require rebuilding the generation fingerprint.

`render(pack, tree, ref_texts, options)` has no seed or chooser. It returns text,
status and fallbacks, with English as the Chronos default and French supported.
It implements all target/source combinations, glosses, Latin/native scripts,
`templates_native`, source elision, French agreement and contractions, and stable
English premodification. Reference text is normalized to NFC with frozen Unicode
15.1 tables, outside canonical generation. Views should report
`rendering_fingerprint()` alongside rendered names. No rendered text is stored in
canonical records.

## Measured and reproducible

The original conformance file contains **306 trees and 3,366 render results**.
All match Rust exactly, including text, status, ordered fallbacks and consumed
scripted choices. No supplied case is unmatched. Additional small checks cover
parent-kind preservation, invalid draws, NFC/Hangul, immutable naming after a
conduct change, missing-pack provenance, and manifest fingerprint separation.

Run `cargo test --workspace` with `CHRONOS_PYTHON` and `CHRONOS_LINEAGE_SRC` set
to use the real worker. Core purity is checked with
`cargo check -p chronos-core --target wasm32-unknown-unknown`.

For ten names from the real two-domain demo (seed 11, 1300–1400), run:

```powershell
$env:CHRONOS_PYTHON='D:/workspace/self/chronos/.venv/Scripts/python.exe'
$env:CHRONOS_LINEAGE_SRC='D:/workspace/self/lineage-lab/src'
cargo run --release -p chronos-app --example naming_demo
```

Measured on the supplied runtime (`d74e53c2960b3a08bdcd7aa750fd26106c796c51953f78a7947f08118bd6d5a5`),
the demo contains **23,404 group records and zero synthetic name fallbacks**.
Its canon is `canon-2740f6b4e8bdb8be9ee1539abc164549`.

The example writes `output/naming-l5-evaluation/names.json`, with canon/runtime
identity, counts and two samples from each founding conduct class. Samples are
the first organized-group names in deterministic domain/group order that need no provisional
geographic label. They are not a quality or diversity benchmark.

## Limits and proposed extensions

Source-language coverage and historical accuracy are those of the frozen pack;
artistic/constructed status and source/script fallbacks remain visible. Its
blocklist deliberately excludes three-concept patterns. No claim of universal
name uniqueness or full language grammar is made. Canonical name changes,
exonyms, popular names and separate covert/public names remain proposed work.
The renderer accepts this validated pack format, not arbitrary user-authored
grammars. Display surnames and geographic labels belong to the host; surname
references alone do not invent personal records. The API and web integration is
a separate parallel lot, consuming the public Rust renderer and added fields.

# Nommage des organisations v2 — L5

## Implémenté

`v2::naming` évalue le pack figé Namebench `naming-1` au commit
`2820b509169039bbf66c73e96061766c7edf8e06`. Les trois fichiers d'origine et leurs
empreintes SHA-256 figurent dans [fixtures/naming](../../fixtures/naming/README.md).
Chronos ne lit jamais le dépôt Namebench vivant. La v1 reste inchangée.

`expand(pack, kind, culture, context, choose)` retourne le `model::NameTree`
canonique existant. Les choix sont des indices entiers injectés ; en production,
ils utilisent `uniform_below(world_seed, ["chronos-namebench-1", entity_id, path],
0, n)`. Les chemins sont exactement `pattern` et `slot/<name>`. Concepts et motifs
sont triés par ID ; les slots gardent l'ordre du pack. L'expansion couvre les sept
formes, prérequis, tons, filtres palette/activité, exclusions, emblèmes distincts,
racines disjointes, `fits` et `pairs_with`. L'arbre conserve le kind précis ; la
recherche remonte au premier parent possédant des motifs. Elle ne remonte plus
si ce niveau n'a aucun motif admissible. Une absence d'arbre ne provoque ni
nouvelle tentative ni changement de fournisseur.

Les organisations régionales reçoivent leur arbre à la création dans `region.rs`,
avant les personnes ; les autres à la création dans `social.rs`. Le
`mu_culture_id` source est tiré selon les poids d'occupation à la fondation. Pour
les groupes antérieurs à la période observée, la première occupation observée
fournit les poids. Les axes de fondation donnent le ton `ConductClass`. Site,
région et pouvoir territorial sont référencés par ID ; l'activité est tirée parmi
les poids locaux compatibles avec l'archétype. Appartenances, changements de
classe, déplacements et options d'affichage ne retirent jamais un nom. Les
homonymes sont permis.

`SocialPacket.person_group_names` nomme les vues existantes de ménages du
fournisseur et de lignées du noyau simulé, sans créer d'organisations ni changer
la population. Le premier épisode observé et un membre stable fixent culture,
activité et référence `surname` (clé familiale du fournisseur). Ces vues n'ont
pas d'axes de conduite organisationnelle ; leur ton de nommage est `ordinary`.
L'hôte résout les clés familiales en patronymes d'affichage.
`initialize_organization_name` accepte aussi une référence de patronyme pour de
futures organisations fondées sur des personnes.

`GroupRecord.name` conserve le descripteur historique pour compatibilité API ;
avec un arbre, sa provenance est `derived`, avec l'empreinte de génération du
pack. `name_fallback` n'est présent que sans pack ou si le kind ne peut pas
produire d'arbre ; il est explicitement `synthetic`. Une culture inconnue ou un
choix invalide est une erreur. Le pack optionnel de `SocialInputs` représente
explicitement son absence ; l'application embarque normalement le pack fourni.

Le manifeste du monde inclut `packs.naming_generation` et
`naming_algorithm: chronos-namebench-1`. Le pack complet est archivé avec le monde,
mais ni son empreinte de fichier complet ni l'empreinte de rendu n'entrent dans
l'identité du monde. Une modification des règles de génération exige une nouvelle
version de règle ; une modification des entrées des tirages exige une nouvelle
empreinte de génération.

`render(pack, tree, ref_texts, options)` n'a ni seed ni fonction de choix. Il
retourne texte, statut et replis, en anglais par défaut dans Chronos ou en
français. Il couvre les combinaisons cible/source, gloses, écritures latine/native,
`templates_native`, élisions source, accords et contractions françaises, et
antéposition anglaise stable. Les références sont normalisées en NFC avec des
tables Unicode 15.1 figées, hors génération canonique. Les vues doivent exposer
`rendering_fingerprint()` avec les noms affichés. Aucun texte rendu n'est stocké
dans les faits canoniques.

## Mesuré et reproductible

Le fichier de conformité d'origine contient **306 arbres et 3 366 rendus**. Tous
concordent exactement avec Rust : texte, statut, ordre des replis et choix
scriptés consommés. Aucun cas fourni n'est incompatible. Des vérifications
ciblées couvrent le kind parent, les tirages invalides, NFC/Hangul, l'absence de
renommage après changement de conduite, la provenance sans pack et la séparation
des empreintes dans le manifeste.

Lancer `cargo test --workspace` avec `CHRONOS_PYTHON` et `CHRONOS_LINEAGE_SRC`
pour utiliser le véritable worker. Vérifier le noyau avec
`cargo check -p chronos-core --target wasm32-unknown-unknown`.

Sur le runtime fourni (`d74e53c2960b3a08bdcd7aa750fd26106c796c51953f78a7947f08118bd6d5a5`),
le démonstrateur contient **23 404 groupes et aucun nom de repli synthétique**.
Son canon est `canon-2740f6b4e8bdb8be9ee1539abc164549`.

La commande d'exemple ci-dessus extrait dix noms du vrai démonstrateur à deux
domaines (seed 11, 1300–1400). Elle écrit
`output/naming-l5-evaluation/names.json`, avec identités canon/runtime, comptes et
deux noms par classe de fondation. Ce sont les premiers noms d'organisations, dans l'ordre stable
des domaines/groupes, n'exigeant aucun libellé géographique provisoire. Cet
échantillon n'est pas une mesure de qualité ou de diversité.

## Limites et extensions proposées

Couverture linguistique et précision historique restent celles du pack figé ;
statuts artistiques/construits et replis source/écriture restent visibles. La
liste d'exclusion ne couvre volontairement pas les motifs à trois concepts.
L'unicité universelle et une grammaire linguistique complète ne sont pas garanties.
Renommages historiques, exonymes, noms populaires et noms publics/clandestins
distincts restent proposés. Le rendu accepte ce format de pack validé, pas des
grammaires arbitraires. Patronymes et libellés géographiques d'affichage relèvent
de l'hôte ; une référence de patronyme n'invente aucune personne. L'intégration
API/web est un lot parallèle utilisant le rendu Rust public et les champs ajoutés.
