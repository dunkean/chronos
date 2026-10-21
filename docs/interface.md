# Interface web de Chronos — atelier d’histoire

État : implémentée dans `web/` (HTML, CSS et modules JavaScript, sans framework ni étape de construction). Servie en même origine par `chronos-app` ; la commande de lancement et le port (8785 par défaut) sont dans le [README](../README.md). Aucune ressource externe (police, script, image) n’est chargée.

L’interface n’invente aucun fait : elle affiche le `World` renvoyé par l’API et ne fait que le lire. Changer d’année, filtrer, ouvrir une fiche ou zoomer ne relance aucun tirage.

## Fichiers

| Fichier | Rôle |
|---|---|
| `web/index.html` | Structure : bandeau, réglages, carte, rail des années, panneau latéral, tiroir Sources. |
| `web/style.css` | Palette marine/ardoise, mise en page bureau/moyen/étroit, focus visibles. |
| `web/app.js` | Chargement, état (année, fiche, historique), câblage des vues, import/export. |
| `web/api.js` | Appels `fetch` en même origine ; lit `{"error": …}` des réponses non 2xx. |
| `web/model.js` | Index en lecture seule sur le monde : relevés par année, relations, statut de vie, titulaire d’office. |
| `web/config-form.js` | Formulaire de tous les champs `Config`, bornes, préréglages. |
| `web/map-view.js` | Carte canvas : relief calculé depuis `elevation_m`, routes, sites, halos, zoom, survol. |
| `web/timeline.js` | Rail des années : histogramme par type, bandes pluriannuelles, population totale. |
| `web/chronicle.js` | Onglets Chronique (filtres d’événements) et Index (toutes les entités). |
| `web/inspector.js` | Fiches de lieu, événement, organisation, personne, puissance. |
| `web/util.js` | Formats français, libellés, construction DOM sûre (`textContent` uniquement). |

Le serveur doit renvoyer les `.js` en `text/javascript` (modules ES) ; c’est le cas de `crates/chronos-app/src/server.rs`.

## API utilisée

| Appel | Usage |
|---|---|
| `GET /api/defaults` | Valeurs par défaut du formulaire, sources disponibles, capacités. Si l’appel échoue, le formulaire reprend les valeurs du contrat et le signale. |
| `GET /api/world` | Monde courant au chargement de la page. |
| `POST /api/generate` | Corps = `Config` directement. Le monde renvoyé remplace l’affichage ; en cas d’erreur, l’ancien reste affiché et le message du serveur apparaît. |
| `POST /api/import` | Corps = texte exact du fichier choisi, après une vérification locale (JSON valide, champs `schema_version` et `canon_id`). Le monde validé renvoyé par le serveur est affiché. |

**Export** : le bouton « Exporter le JSON » télécharge le texte JSON exact de la dernière réponse du serveur (génération, import ou monde courant), sans resérialisation par le navigateur.

## Parcours

- **Réglages** : tous les champs `Config`, regroupés en « Monde et durée » et « Peuplement » (toujours visibles), puis « Réglages avancés » (crises, ressources et échanges, distances, institutions). Bornes identiques à `Config::validate` ; le serveur reste l’autorité. Le champ « Scénario » choisit `pivot` (démo de l’étude : épidémie de 1356 et guerre avec sauvetage de 1398 imposées, quelles que soient les intensités) ou `free` (histoire libre : seuls les réglages décident des crises) ; un serveur qui ne renvoie pas ce champ est traité comme `pivot`. Quatre préréglages remplissent le formulaire sans générer et conservent la graine : Démo 1300 (`pivot`), Siècles calmes, Temps de crises et Monde virtuel (`free`). La même graine avec les mêmes réglages et données versionnées redonne les mêmes faits canoniques ; seule la durée de génération (`generation_ms`, non canonique) varie. Une pastille et une phrase signalent des réglages modifiés depuis le monde affiché.
- **Carte** : relief ombré calculé dans le navigateur à partir des altitudes numériques (rampe hypsométrique, ombrage normalisé sur les pentes de la carte), graticule, échelle en km, légende d’altitude et des types de routes. Sites : surface ∝ population du relevé de l’année, couleur de la puissance du relevé, anneau pour une capitale, cercle vide barré pour un lieu à zéro habitant, halos colorés pour les événements en cours. Molette ou boutons pour zoomer, glisser pour déplacer une fois zoomé, « Cellules brutes » pour voir la grille sans lissage. Le cartouche affiche l’année et les puissances avec leur nombre de lieux. La légende est repliée au départ ; si l’utilisateur l’ouvre ou la ferme, ce choix est gardé (`localStorage`).
- **Rail des années** : champ numérique, ±1, curseur, événement précédent/suivant (selon les filtres), lecture année par année. Clic ou glissé sur le rail pour changer d’année ; les bandes du haut (événements pluriannuels) ouvrent leur fiche.
- **Chronique** : période (année, décennie, jusqu’à l’année, tout), lieu, recherche plein texte, types d’événements. Les filtres de type s’appliquent aussi au rail et aux halos de la carte.
- **Fiches** : liens entre toutes les entités, historique précédent/suivant.
  - Lieu : état à l’année (population, variation depuis le relevé précédent et l’origine, vivres, richesse, stabilité), courbe de population, effets en cours, ressources initiales, routes, organisations, résidents, événements.
  - Événement : description, lieux, organisations, personnes (âge à l’événement), effets chiffrés avec population avant/après, causes et conséquences.
  - Organisation : office, règle de succession, titulaire en début de chronique et à l’année affichée, lieux, événements.
  - Personne : naissance, décès, statut à l’année, résidence, père, mère, enfants, fratrie, offices, événements, source.
  - Puissance : lieux tenus et habitants à l’année, organisations, événements.
- **Index** : puissances, lieux, organisations, personnes, avec recherche. C’est aussi l’accès clavier à tous les lieux de la carte.
- **Bandeau** : identité courte du canon, graine, période, carte réellement utilisée (d’après la provenance : source `stack-fixture` importée, sinon « repli virtuel (carte importée indisponible) » quand `map_mode = stack` n’a pas pu être servi) et scénario.
- **Sources** : identité du canon, carte utilisée, scénario, couverture généalogique, métadonnées de la carte (source, grille, emprise, taille de cellule, rayon), chaque source avec statut (importée, dérivée, synthétique, indisponible), détail et empreinte, avertissements du monde, capacités du serveur, limites du prototype. Le bandeau résume les statuts.
- **Adresse** : l’année et la fiche ouverte sont gardées dans le fragment d’URL (`#an=1356&fiche=site:…`) ; la configuration ne l’est pas.

## Lectures des données

Conventions du noyau reprises par l’interface (voir `crates/chronos-core/src/model.rs`) :

- Un relevé `Snapshot.year = Y` est l’état au **début** de l’année Y. La fiche d’un événement compare le relevé de `year` à celui de `end_year + 1` ; l’écart inclut aussi la croissance et les autres événements, la colonne « Habitants » donne l’effet propre.
- `end_year` est inclus. Un événement est « en cours » en Y si `year ≤ Y ≤ end_year`.
- Les événements englobants (guerre, vague d’épidémie) n’ont pas d’effet direct ; leur fiche totalise les effets des événements qui les citent comme cause.
- `holder_id` est le titulaire au début de la chronique. Le titulaire à l’année affichée est déduit des événements `succession` de l’organisation, avec `person_ids = [nouveau, précédent]` (l’un peut manquer en cas de vacance). Le titulaire affiché pour une année doit y être actif : en vie et avant son année de décès ; sinon la fiche indique « pas de titulaire actif » avec la raison (décès, fin de trace). Une organisation avec `collective_office = true` (champ optionnel, faux par défaut) est une instance collective : la fiche affiche « instance collective (titre) », jamais « vacant » ni le nom d’une personne.
- Une personne vit jusqu’à son année de décès incluse (« année du décès ») ; elle ne participe plus aux événements à partir de cette année selon le contrat. Une personne importée sans décès enregistré est **censurée** à la fin de l’échantillon : au-delà, son statut est « inconnu ». La fin d’échantillon vient des champs exportés quand ils existent (`Person.known_until_year`, puis `World.genealogy_end_year` ; `World.genealogy_start_year` est affiché dans Sources), sinon de la même estimation que le noyau (dernière naissance ou décès enregistré parmi les personnes importées), affichée comme estimation. Les dates de vie importées ne sont jamais modifiées par l’affichage.
- En carte importée, `site_count` est désactivé (la fixture fixe les lieux) et le rayon ne multiplie que distances et durées ; la note sous chaque champ le dit, et le serveur ajoute un avertissement.
- Un identifiant cité mais absent de l’export s’affiche « absent de l’export (ID …) », jamais comme un lien vide. Les couleurs de puissance invalides sont remplacées par un gris neutre.

## Accessibilité et formats

- Bureau (≥ 1280 px) : réglages, carte et rail, panneau latéral. Moyen (900–1279 px) : réglages en tiroir au-dessus de la carte. Étroit (< 900 px) : une colonne qui défile. Sous 520 px, le cartouche ne garde que l’année.
- Libellés réels sur tous les champs, onglets au clavier (flèches, Début, Fin), focus visible, tiroir Sources en `<dialog>` modal (Échap pour fermer), zones `role="status"` et `role="alert"` pour l’état et les erreurs.
- La carte canvas a un libellé texte ; touches + − 0 et flèches quand elle a le focus. Les lieux se parcourent au clavier par l’Index.
- Aucun mouvement automatique : seule la lecture, déclenchée par l’utilisateur, fait défiler les années. `prefers-reduced-motion` désactive le défilement animé.

## Vérifications faites et limites

Vérifié par l’agent interface : `node --check` sur chaque module, cohérence des imports/exports entre modules, et un essai Node avec un DOM et un canvas simulés qui construit toutes les fiches, la chronique, l’index, la carte et le rail sur un monde produit par `chronos-app generate` (repli virtuel, 50 lieux, 307 événements) et sur un petit monde de contrat. Cet essai ne prouve ni la mise en page ni le rendu réel.

Vérifié ensuite lors de l’intégration dans Chrome : chargement de la vraie fixture, génération depuis le formulaire, mode libre puis retour à la démo, navigation 1356/1398, effets avant/après, assemblée collective, famille du héros, couverture généalogique à 1450, dialogue Sources, téléchargement et réimportation JSON. Aucun message d’erreur de page ou de console. À 390 px, les réglages commencent repliés, le millésime reste lisible, les outils ne le recouvrent pas et il n’y a pas de débordement horizontal. Le fragment d’URL est maintenant suivi aussi lors d’une navigation dans l’onglet ouvert.

Non vérifié : lecteurs d’écran, gestes tactiles sur un appareil réel et campagne de performances. Détails : [revue d’intégration](reviews/prototype-integration.md).

Limites connues :

- La carte n’est pas navigable au lecteur d’écran ; l’Index est l’alternative.
- Pas de zoom à deux doigts sur écran tactile (boutons + et −).
- Les listes longues sont tronquées (300 événements, 150 entrées d’index, puis « Afficher plus ») plutôt que virtualisées.
- Le titulaire à l’année affichée repose sur la convention `person_ids` des successions ; si le noyau la change, cette ligne devient fausse.
- La fin d’échantillon généalogique est estimée tant que l’export ne la fournit pas.
