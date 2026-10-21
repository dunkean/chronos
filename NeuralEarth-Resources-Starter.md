# Neural Earth — 35 couches pour démarrer

Objectif : fournir des potentiels régionaux lisibles pour les villes et la géopolitique d'un monde médiéval fantastique. **Ces 35 couches constituent le scope initial.** Le [catalogue détaillé](NeuralEarth-Resources.md) reste une référence pour les raffinements ultérieurs.

On commence à une résolution régionale fixe, avec des scores de potentiel entre 0 et 1. Deux exceptions : la géologie est catégorielle et la saisonnalité de l'eau utilise un petit profil partagé. Le détail des espèces, les réserves physiques et le raffinement multiéchelle viendront ensuite.

## Entrées déjà disponibles

Réutiliser relief, pentes, climat, biomes et informations hydrologiques existantes. Ces entrées ne sont pas des couches à recréer. Les données de sol disponibles peuvent aussi servir ; leurs couleurs seules ne sont pas des mesures de fertilité.

Construction des couches : **C** = combinaison des données existantes ou des couches ci-dessous ; **P** = champ procédural régional ; **M** = combinaison avec une variation procédurale. Les numéros servent à lire les dépendances, sans imposer une architecture logicielle.

## 1. Socle partagé — 5 couches

| Nº | Couche | Construction |
| --- | --- | --- |
| 1 | **Contexte géologique grossier** | **P/M** : grandes provinces cohérentes, guidées par les informations géologiques disponibles et un bruit régional partagé. |
| 2 | **Fertilité des sols** | **M** : sols disponibles, contexte géologique, climat, relief et variation régionale. |
| 3 | **Disponibilité de l'eau** | **C** : ressources hydriques existantes, accès et climat. |
| 4 | **Saisonnalité de l'eau** | **C** : alternance humide/sèche, températures et comportement des ressources hydriques. |
| 5 | **Terres exploitables** | **C** : pente, profondeur des sols si disponible, roche affleurante et engorgement. |

Pour la saisonnalité, quatre coefficients par région suffisent au départ : abondance et manque d'eau dans l'année. Ce profil est partagé par les consommateurs ; les autres couches peuvent exposer un score annuel tenant compte de la saison défavorable.

## 2. Productions végétales, animales et alimentaires — 15 couches

Toutes décrivent un **potentiel d'usage**, sans présumer que cultures, troupeaux ou installations existent déjà.

| Nº | Couche | Construction |
| --- | --- | --- |
| 6 | **Céréales** | **C** : fertilité, eau, saisonnalité, terres exploitables et climat. |
| 7 | **Légumineuses** | **C** : même socle, avec un profil de besoins distinct des céréales. |
| 8 | **Racines, tubercules et maraîchage** | **C** : sols exploitables, fertilité, eau et climat. |
| 9 | **Vergers et fruits** | **C** : climat, gel, eau, sols et exposition. |
| 10 | **Cultures oléagineuses** | **C** : climat, sols et régime hydrique ; huiles alimentaires et autres usages. |
| 11 | **Plantes à fibres textiles** | **C** : climat, eau et sols ; profil global lin/chanvre/coton, sans espèces détaillées. |
| 12 | **Cultures de spécialité** | **M** : climat, biomes, sols et provinces ; aromatiques, épices, plantes tinctoriales et médicinales regroupées. |
| 13 | **Pâturages et bétail** | **C** : couvert herbacé, eau, relief et saison défavorable ; potentiel pastoral global. |
| 14 | **Volailles** | **C** : eau, climat, ressources alimentaires et abris naturels ; potentiel d'élevage. |
| 15 | **Apiculture** | **C** : végétation mellifère estimée depuis les biomes, climat et eau. |
| 16 | **Sylviculture et bois d'œuvre** | **C** : biomes forestiers, climat, sols, eau et relief. |
| 17 | **Bois de feu et biomasse combustible** | **C** : végétation ligneuse et potentiel forestier ; usage du même patrimoine végétal. |
| 18 | **Cueillette et chasse** | **M** : habitats, végétation, eau et climat ; potentiel alimentaire sauvage regroupé. |
| 19 | **Pêche continentale** | **C** : rivières, lacs, zones humides, connexions et climat. |
| 20 | **Pêche maritime** | **M** : contexte côtier, climat marin et profondeur si disponible ; variation régionale estimée. |

Un profil agricole générique de départ suffit. Les familles restent larges : on choisira plus tard quelles céréales, essences, espèces ou spécialités elles représentent localement.

## 3. Matières premières stratégiques — 10 couches

Les scores indiquent une présence ou une abondance relative. Ils ne représentent pas encore des réserves en tonnes. Les métaux partagent des provinces et foyers de minéralisation ; éviter dix bruits indépendants uniformément dispersés.

| Nº | Couche | Construction |
| --- | --- | --- |
| 21 | **Fer** | **M** : contexte géologique, affleurements et concentrations régionales. |
| 22 | **Cuivre** | **M** : contexte géologique et foyers de minéralisation. |
| 23 | **Étain** | **M** : contexte géologique et concentrations plus localisées. |
| 24 | **Plomb** | **M** : contexte géologique et foyers métalliques partagés. |
| 25 | **Métaux précieux** | **M** : or et argent regroupés ; géologie, foyers minéralisés et dépôts remaniés. |
| 26 | **Sel** | **M** : côtes favorables, bassins fermés et provinces salifères. |
| 27 | **Pierre de construction** | **C/M** : géologie, affleurements et facilité d'accès. |
| 28 | **Argiles** | **M** : sols, géologie, bas-reliefs et contexte sédimentaire. |
| 29 | **Sables utilisables** | **M** : géologie, contexte sédimentaire, rivières et littoral. |
| 30 | **Combustibles minéraux** | **M** : provinces favorables et concentrations ; charbon principalement à ce stade. |

La géologie est une approximation procédurale assumée. Le détail des minerais, les métaux modernes, les gemmes et les chaînes industrielles restent dans le catalogue de référence.

## 4. Synthèses spatiales et capacités — 5 couches

| Nº | Couche | Construction |
| --- | --- | --- |
| 31 | **Force hydraulique** | **C** : débit disponible, pente/chute et saisonnalité de l'eau. |
| 32 | **Potentiel portuaire** | **C** : forme des côtes, abri, accès et profondeur si disponible. |
| 33 | **Passages et corridors stratégiques** | **C** : relief, cols, vallées, franchissements et détroits ; intérêt physique du passage. |
| 34 | **Exposition aux inondations** | **C** : proximité et régime des eaux, relief et plaines basses. |
| 35 | **Soutien alimentaire local** | **C** : potentiels alimentaires, terres exploitables, eau et saisons, sous une convention simple d'occupation des sols. |

La dernière couche est un indicateur comparatif, pas encore un nombre d'habitants soutenables. Elle doit respecter une répartition de référence entre cultures, pâturages et milieux sauvages : ne pas additionner leurs maxima sur une même surface. Les populations existantes ne servent pas à calculer ce potentiel.

## Mise en œuvre minimale

1. Réutiliser les données existantes ; ajouter le socle partagé et la saisonnalité de l'eau.
2. Calculer les grandes familles par combinaisons simples et paramétrables. Employer le procédural pour les contrastes régionaux manquants, avec seed et coordonnées stables.
3. Produire les cartes à la résolution régionale choisie. Les dépendances intermédiaires peuvent rester en mémoire ; calcul et stockage permanents des 35 couches ne sont pas obligatoires.
4. Dans l'interface, afficher une couche sélectionnée, avec sa légende et la valeur sous le pointeur. Quelques vues de comparaison suffisent.

Les scores restent des estimations de prototype. Une forte aptitude n'implique ni une exploitation effective ni une possession politique. Les usages concurrents partagent le même terrain, la même eau ou la même forêt ; Magna Urbis et Chronos détermineront ensuite les usages réels.
