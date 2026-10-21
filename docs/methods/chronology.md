# Méthode — chronologie racine de Chronos (v1 implémentée)

Statut : décrit le code de `crates/chronos-core` (`ALGORITHM_VERSION = "chronos-history-1"`). Les chiffres sont des paramètres de worldbuilding, pas des lois historiques ; aucune performance n'est revendiquée ici.

## Entrées et identité

- `Config` (voir `model.rs`) et, en mode `stack`, la fixture `fixtures/stack.json` préparée par `tools/`. En mode `virtual`, tout est synthétique (aucune donnée de la fixture n'est utilisée).
- Si la fixture manque ou est invalide en mode `stack`, la génération bascule sur le monde virtuel avec un avertissement et une source `stack-fixture` au statut `unavailable`.
- `canon_id` = empreinte FNV-1a 64 bits de `ALGORITHM_VERSION`, de la configuration **appliquée** et des empreintes de toutes les sources. `generation_ms` n'entre ni dans le canon ni dans le hachage de rejeu (`replay_hash`).
- Toute décision aléatoire est `hash(seed, clé sémantique)` (`keys.rs`) : pas de RNG partagé, pas de `HashMap`, pas de `DefaultHasher`. Les mêmes entrées donnent un export identique octet pour octet (`chronos-app generate`).

## Géographie

- **Stack** : carte, sites et routes de la fixture, sans modification géométrique. Les populations relatives sont importées et le total est remis à l'échelle de `population` (plus grand reste, total exact). `site_count` appliqué = nombre de sites importés. `map.radius_km` reste le rayon source ; si `world_radius_km` diffère, distances et temps sont multipliés par le rapport, avec avertissement.
- **Virtuel** : relief par bruit de valeur seedé, sites sur les meilleures cellules terrestres avec espacement, 3 plus proches voisins sans traversée maritime (aucune liaison inventée), distances orthodromiques.
- Temps de trajet canonique : `ceil(distance_km / travel_km_per_day)` jours, au moins 1, avant toute décision.

## Société

- Polities : `clamp(sites / 30, 2, 6)` capitales (le site le plus peuplé, puis échantillonnage du point le plus éloigné pondéré par la taille) ; territoire = capitale la plus proche en jours de route.
- Profils : monarchie, conseil, ligue marchande, communauté villageoise, collectif informel. Un modèle de la fixture est retenu par identifiant `kind` exact, puis par mots-clés ; sinon un profil générique Chronos étiqueté `synthetic`.
- Office collectif : si le rôle retenu par le modèle (`role_id`) est collectif dans Pecking Order (`roles[].collective`, ou `collective_office` explicite) — par exemple l'assemblée de `po-generic-city_league` —, l'organisation est exportée avec `collective_office: true` et `holder_id: null`. Ce `null` désigne l'instance collective abstraite, pas une vacance : aucune personne n'est installée et aucune succession individuelle n'est simulée. Les offices personnels (roi, délégué, chef de village…) restent attribués à des personnes.
- Succession : liste fermée d'identifiants (`hereditary`, `primogeniture`, `dynastic_selection`, `election`, `vote`, `member_vote`, `appointment`, `seniority`…). Toute autre règle laisse l'office vacant avec diagnostic.
- Personnes importées : dates, parents et IDs externes conservés tels quels ; nom attribué par clé stable (pool Namebench de la culture du site de l'ancêtre, prénom selon le sexe enregistré). Les personnes importées ne tiennent aucun rôle (titulaire, héros, victime citée, participant) hors de la couverture de l'archive (`genealogy_start_year`/`genealogy_end_year` ou `people_window` ; à défaut, fin estimée au dernier fait daté) : avant le début, rôle et résidence ne sont pas observés ; après la fin, une personne sans décès enregistré n'est plus suivie. Le monde exporte cette couverture (`World.genealogy_start_year`, `World.genealogy_end_year`) ; dates de vie et filiations restent inchangées. Chronos ne cause jamais la mort d'une personne importée : une crise peut seulement citer comme victime une personne dont le décès enregistré tombe cette année-là et dans ce lieu.
- Personnes virtuelles (`source: "virtual"`) : créées seulement quand aucune personne importée n'est éligible (titulaire, enfants d'un titulaire virtuel héréditaire, héros de repli).

## Boucle annuelle

Pour chaque année Y : instantané (état **au début** de Y), successions, guerres, épidémies, récoltes, commerce, migrations, reconstructions, troubles/réformes, découvertes, puis croissance naturelle (logistique vers la capacité, sans événement).

- Pertes : `floor(P × num / den)` avec P = population du site au début de Y, plafonnées par les présents ; l'effet enregistré est exactement la variation appliquée.
- Capacité alimentaire (personnes-années) : `max(P₀, 50) × (1,15 + 0,5 q) × resource_abundance`, q = rang du potentiel nourriture/eau du site. Les ressources du site sont des indices de potentiel, pas des gisements arpentés.
- Transferts (migration, refuge, reconstruction) : un seul événement, deux effets de somme nulle.
- Événements pluriannuels (guerre, vague d'épidémie) : événement-cadre sans effet ; les batailles et frappes datées le citent dans `causes`. Les causes sont toujours des événements antérieurs.
- Une bataille n'a lieu qu'à partir de l'année `début + floor(jours de marche / 365)`.

## Scénarios : `pivot` (démo) et `free`

`Config.scenario` (défaut `pivot`) fait partie du canon. Avec `free`, seuls les taux produisent guerres, épidémies et héros : avec `war_rate = 0` et `epidemic_rate = 0`, aucune guerre ni épidémie n'apparaît. Avec `pivot`, les éléments suivants sont imposés quand la fenêtre les couvre, en plus de l'histoire produite par les taux ; ils sont signalés comme « scénario pivot » dans les descriptions et la source `chronos-history`.

- Épidémie continentale `[1354, 1360]` : la ville la plus peuplée en 1354 perd exactement `floor(P₁₃₅₆ × 3 / 10)` en 1356.
- Guerre pivot autour de l'année du héros : exactement 3 villages détruits (top-K sur l'index complet des villages candidats, par jours de marche puis clé stable), reconstructions datées ensuite.
- Héros : membre adulte vivant d'une famille Lineage Lab (parent présent dans l'archive) en 1398, ou l'année faisable la plus proche (avertissement) ; sinon personne virtuelle étiquetée.

## Limites connues

- Génération intégrale à la racine ; le raffinement paresseux (J5) n'existe pas.
- Une seule personne par office personnel ; instances collectives non détaillées ; pas de mandats électifs à durée fixe ni de réformes institutionnelles structurelles.
- Les frontières ne changent que par les traités de fin de guerre ; les routes ne changent jamais.
- Les écarts entre la démographie Lineage Lab et les effectifs Chronos ne sont pas réconciliés.
