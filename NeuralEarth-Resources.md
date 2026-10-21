# Neural Earth — catalogue des ressources et potentiels multiéchelles

Version de travail : 21 octobre 2026. Destinataires : Neural Earth, Magna Urbis et Chronos.

## 1. Objectif et périmètre

Générer le **patrimoine naturel d'une planète entière**, puis en révéler les détails à la demande : ressources présentes, réserves, renouvellement, qualités, contraintes et potentiels d'exploitation. Ces données doivent permettre de placer les établissements, comprendre leurs spécialisations et simuler plusieurs siècles de transformations territoriales.

Ce document fixe un **catalogue cible v1**, avec les classes et leurs instances. Il couvre les usages préagricoles, antiques, médiévaux et industriels, avec un complément moderne et une extension fantasy facultative. Il ne cherche pas à nommer chaque espèce biologique, chaque variété cultivée ou chaque minéral connu : les instances sont des ressources ou profils utiles au jeu, extensibles sans changer le modèle. Les exemples d'essences, espèces et minéraux indiquent comment décliner ces profils.

**Lecture :** §3 pour les ressources naturelles ; §4 pour les cultures et élevages possibles ; §5 pour l'énergie, les services et les sites ; §6 pour les capacités intégrées. Les §8–9 constituent le contrat de génération et d'export. Pour démarrer, suivre la tranche A du §10 ; le catalogue détaillé ne demande pas de tout implémenter immédiatement.

**La présence physique ne dépend pas de l'époque.** Le cuivre profond existe avant les techniques permettant son exploitation. Les profils de scénario déterminent les ressources connues, accessibles et utiles ; ils ne retirent pas les autres de la planète.

### 1.1 Répartition des responsabilités

| Projet | Responsabilité |
| --- | --- |
| Neural Earth | Géographie, sous-sol, sols, eau, habitats, patrimoine biologique initial, stocks naturels, renouvellement et aptitudes physiques. |
| Magna Urbis | Implantation et organisation spatiale des établissements, accès et réseaux, emprises d'exploitation proposées, ouvrages et formes urbaines. |
| Chronos | Découverte, appropriation, droits, exploitation, échanges, investissements, dégradation, épuisement, populations et événements datés. |

Neural Earth fournit le potentiel d'une rizière, d'un port, d'une mine ou d'un pâturage. La rizière aménagée, le port construit, la concession minière et le troupeau effectivement présent relèvent de l'état historique. Les besoins du présent imposé peuvent contraindre le bootstrap, mais doivent être explicites.

### 1.2 Quatre natures de données

| Nature | Définition | Exemples |
| --- | --- | --- |
| **Stock** | Quantité présente à une date de référence. | Minerai, volume de bois sur pied, eau d'une nappe, biomasse de poissons. |
| **Flux** | Apport ou renouvellement par durée, avec saisonnalité. | Recharge d'une nappe, accroissement forestier, débit, production primaire. |
| **Aptitude** | Possibilité d'un usage sous des hypothèses nommées. | Blé pluvial, pâturage ovin, implantation portuaire, irrigation gravitaire. |
| **Service / contrainte** | Effet du milieu sur d'autres usages. | Pollinisation, protection contre l'érosion, exposition aux crues, franchissabilité. |

Un indice d'abondance n'est ni une réserve en tonnes ni une production annuelle. Une aptitude agricole ne signifie pas que le terrain est déjà cultivé.

### 1.3 Priorités de construction

- **P0 — socle** : eau, sols fonctionnels, bois, alimentation sauvage, pâturages, pêche, matériaux courants, fer/cuivre/étain/plomb/or/argent, sel, potentiels intégrés et accès. Toutes les grandes familles utiles doivent exister dès cette tranche, même avec une représentation simplifiée.
- **P1 — diversification** : espèces et cultures détaillées, ressources artisanales, pierres précieuses, dépôts mieux différenciés, écologie et contraintes plus fines.
- **P2 — complément industriel et moderne** : métaux technologiques, hydrocarbures complexes, nucléaire et potentiels énergétiques détaillés.
- **PX — fantasy** : extension désactivée par défaut.

Ces priorités sont un ordre de réalisation. Une ressource moderne peut être générée physiquement dès P0 si son ajout coûte peu ; sa chaîne d'exploitation reste différée.

## 2. Données fondatrices à fournir avant les ressources

### 2.1 Géographie et continuité spatiale

- Coordonnées sur la planète, rayon, superficies réelles, altitude et bathymétrie, pente, exposition, rugosité, terre/mer et profondeur des eaux.
- Unités géographiques stables : îles, plaines, plateaux, vallées, massifs, bassins et sous-bassins. Fournir géométrie, voisinages, rattachement hiérarchique et identifiants. Les bassins et les habitats peuvent aussi constituer des couches transversales au découpage de référence.
- Limites et passages : crêtes, falaises, grands cours d'eau, déserts, cols, gués potentiels, détroits, estuaires, accès côtiers. Qualifier les obstacles et les coûts de passage, y compris les directions et saisons pertinentes.
- Hydrographie connectée : amont/aval, confluences, sources, lacs, exutoires, zones humides, plaines inondables, nappes et échanges surface/sous-sol.
- Provinces géologiques et biogéographiques : elles donnent des corrélations spatiales aux dépôts et au vivant. Le climat seul ne détermine ni les gisements de cuivre ni la présence native du riz ou des chevaux.

### 2.2 Climat et habitats réellement multiéchelles

Les classes de biomes et de Köppen servent à décrire le contexte ; les rendements doivent aussi dépendre de variables continues : température, précipitations, rayonnement, humidité, évapotranspiration estimée, vent, durée de végétation, gel, neige et disponibilité saisonnière de l'eau. Pour l'océan : température, profondeur, salinité, courants et productivité estimée.

Prévoir un cycle annuel échantillonnable. Un profil mensuel terrestre est une première représentation pratique ; le calendrier et la durée de l'année doivent être nommés. Conserver les moyennes, la variabilité interannuelle et la corrélation spatiale des anomalies : une sécheresse touche un ensemble de régions, pas des pixels indépendants.

Au zoom, les effets de vallée, d'altitude, d'exposition et de proximité de l'eau précisent les champs sans réinventer le climat régional. Distinguer biome potentiel et couvert effectivement présent à une date, pour permettre ensuite le déboisement, les cultures et les successions écologiques.

### 2.3 Sols et sous-sol fonctionnels

| Famille de propriétés | Champs utiles |
| --- | --- |
| Texture et structure | Fractions sable/limon/argile de la terre fine, pierrosité, densité apparente, profondeur du sol et profondeur exploitable par les racines. |
| Eau | Rétention utile, infiltration, perméabilité, drainage, engorgement, profondeur de nappe, caractère saisonnier. |
| Chimie | pH, matière organique, carbonates, salinité ; indices de disponibilité de l'azote, du phosphore et du potassium ; contraintes toxiques naturelles si modélisées. |
| Dynamique | Érodibilité, sensibilité au tassement, renouvellement lent de la fertilité, risque de salinisation et de perte de matière organique. |
| Géologie | Lithologie, âge géologique simplifié, structures, altération, affleurements, couverture et profondeur du substrat ; provinces de minéralisation. |

La matière organique est distincte de la répartition granulométrique : **sable + limon + argile = 1 dans la terre fine** ; l'humus n'est pas une quatrième taille de grain. La profondeur et l'eau doivent limiter un sol visuellement fertile.

Les propriétés retenues s'inspirent de la description fonctionnelle des sols de la [FAO, HWSD v2](https://www.fao.org/soils-portal/data-hub/soil-maps-and-databases/harmonized-world-soil-database-v20/en/). Il s'agit d'un modèle procédural simplifié, pas de la reproduction de cette base.

**État du voisin vérifié à la date du document :** `backend/terrain_soil.py` et `backend/terrain_pedology.py` existent déjà dans Neural Earth. Ils décrivent leur résultat comme une approximation visuelle et régionale. Leurs mélanges et couleurs constituent un point de départ ; ils ne fournissent pas à eux seuls les propriétés et stocks demandés ici.

### 2.4 Sources géologiques : générer des ensembles cohérents

Une province tectonique actuelle est un indice de contexte ; elle ne suffit pas à reconstituer toute l'histoire géologique. Pour le prototype, un ensemble de provinces paramétrées, déterminées par la seed et le contexte physique, suffit à produire une histoire simplifiée du sous-sol, avec ses incertitudes. Les familles de sources à représenter progressivement sont :

- Corps magmatiques et intrusions minéralisées ; pegmatites ; systèmes hydrothermaux en filons, porphyres et contacts ; concentrations liées au volcanisme.
- Dépôts sédimentaires stratifiés ; concentrations chimiques ou biologiques ; évaporites et saumures de bassins fermés.
- Profils d'altération et concentrations résiduelles, dont latérites et bauxites ; enrichissements superficiels.
- Placers fluviaux, anciens placers et concentrations littorales de minéraux lourds ; sources amont et transport associés.
- Roches métamorphiques et transformations de sources antérieures ; affleurements de matériaux de construction.
- Bassins à matières organiques fossiles : charbons et systèmes pétroliers ; roche source, réservoir et piège simplifiés si modélisés.
- Sources biologiques accumulées ou fossilisées : tourbe, guano, ambre et dépôts coquilliers.

Le choix de conception est de générer d'abord des **sources hôtes partagées**, puis leur composition, leur géométrie et leur accessibilité. Les typologies détaillées pourront s'appuyer sur les [modèles de gisements de l'USGS](https://pubs.usgs.gov/publication/sir20105070). Au prototype, quelques familles bien corrélées sont préférables à un générateur spécialisé par métal ; garder la méthode et les simplifications explicites.

## 3. Catalogue des ressources naturelles

Les identifiants proposés sont stables, en anglais ASCII, avec libellés français séparés. Les noms de minéraux ou d'espèces dans les descriptions sont des variantes de source, pas systématiquement des couches supplémentaires. Les classifications ci-dessous sont des choix de conception pour le jeu.

### 3.1 Eau et réservoirs hydriques — P0

| Identifiant | Instance | Données particulières |
| --- | --- | --- |
| `water.river` | Eau de rivière ou de fleuve | Débit saisonnier, étiage, variabilité, connexions et qualité naturelle. |
| `water.lake` | Eau de lac ou d'étang naturel | Volume, apports, sorties, variation du niveau et salinité. |
| `water.spring` | Source et résurgence | Débit, pérennité, alimentation et altitude de captage. |
| `water.renewable_groundwater` | Nappe à recharge significative | Réservoir, recharge, profondeur, transmissivité estimée et connexions. |
| `water.fossil_groundwater` | Nappe très lentement renouvelée | Stock et renouvellement faible à l'échelle historique. |
| `water.wetland` | Eau de marais et de tourbière | Stock variable, engorgement, échanges et rôle tampon. |
| `water.snowpack` | Réserve saisonnière de neige | Équivalent eau et calendrier de fonte ; pas un apport indépendant de la pluie/neige. |
| `water.glacier` | Glace et réserve glaciaire | Stock, bilan et contribution à l'aval ; extraction non assimilée à un renouvellement illimité. |
| `water.brackish` | Eau saumâtre | Salinité, volumes, variation et usages compatibles. |
| `water.seawater` | Eau de mer | Salinité et accès ; usages marins, salines et dessalement éventuel. |
| `water.mineral_spring` | Eau naturellement minéralisée | Composition simplifiée et compatibilité des usages. |
| `water.thermal_spring` | Eau chaude naturelle | Débit, température et recharge ; lien avec le potentiel géothermique. |

La précipitation est un **forçage climatique**, non une treizième réserve à additionner aux rivières et nappes. Un même volume qui traverse plusieurs régions n'est consommable qu'une fois. Les échanges entre eaux souterraines et superficielles justifient un budget connecté : [USGS, interactions eau de surface / eau souterraine](https://www.usgs.gov/mission-areas/water-resources/science/groundwatersurface-water-interaction?qt-science_center_objects=0).

### 3.2 Bois, tiges et matériaux végétaux structuraux — P0, détail P1

| Identifiant | Instance fonctionnelle | Déclinaisons du profil |
| --- | --- | --- |
| `wood.structural_softwood` | Résineux de construction | Pin, sapin, épicéa, mélèze, cèdre. |
| `wood.structural_hardwood` | Feuillus de construction | Chêne, châtaignier, orme, hêtre. |
| `wood.durable_timber` | Bois naturellement durable | Chêne, robinier, teck, cèdre ; durabilité selon le contexte d'emploi. |
| `wood.long_straight_timber` | Grands fûts droits | Profils pour mâts, poutres longues et grands ouvrages. |
| `wood.elastic_timber` | Bois souple et élastique | Frêne, if, orme ; usages d'outillage et d'arc. |
| `wood.light_timber` | Bois léger et facile à travailler | Peuplier, saule, tilleul, balsa. |
| `wood.fine_timber` | Bois d'ébénisterie | Noyer, érable, acajou, palissandre, ébène. |
| `wood.aromatic_timber` | Bois aromatique | Santal, cèdre, bois d'agar ; compatibilité biogéographique requise. |
| `wood.fuelwood` | Biomasse ligneuse utilisable en combustible | Usage possible de peuplements partagés avec les profils précédents. |
| `stem.bamboo` | Bambou | Diamètre, longueur, densité et renouvellement des tiges. |
| `stem.rattan` | Rotin | Longueur, flexibilité et production récoltable. |
| `stem.reed` | Roseaux et cannes | Couverture, vannerie et construction légère. |
| `stem.thatch` | Graminées de couverture | Herbes hautes utilisables en chaume. |
| `bark.cork` | Liège | Arbres porteurs et intervalle de récolte. |

Pour un peuplement : surface, composition, volume sur pied, classes d'âge simplifiées, dimensions et qualités des tiges, accroissement, régénération et sensibilité aux perturbations. **Les usages du même arbre partagent son stock.** Le charbon de bois, les planches, la pâte à papier et le goudron de bois sont des transformations ultérieures.

### 3.3 Ressources végétales et produits sauvages non ligneux — P0/P1

| Identifiant | Instance | Déclinaisons |
| --- | --- | --- |
| `wild.fruit` | Fruits sauvages | Baies, fruits charnus et fruits de palmiers compatibles avec la province biologique. |
| `wild.nuts` | Fruits secs et graines comestibles | Noisettes, noix, châtaignes, glands utilisables, pignons. |
| `wild.roots` | Racines, tubercules et rhizomes alimentaires | Profils terrestres ou humides ; fraction comestible et saison. |
| `wild.greens` | Feuilles, pousses et légumes sauvages | Plantes herbacées, jeunes pousses et fougères comestibles sélectionnées par profil. |
| `wild.seeds` | Graines alimentaires sauvages | Graminées et autres plantes à graines récoltables. |
| `wild.mushrooms` | Champignons comestibles | Production liée à l'habitat, au substrat et à l'humidité. |
| `wild.truffles` | Champignons souterrains recherchés | Compatibilité avec arbres hôtes et sols. |
| `wild.sweet_sap` | Sève sucrée | Érable, bouleau, palmiers appropriés. |
| `wild.resin` | Résines | Conifères et autres arbres résinifères. |
| `wild.aromatic_resin` | Résines aromatiques | Encens, myrrhe, benjoin, copal. |
| `wild.gum` | Gommes végétales | Acacias et autres espèces productrices. |
| `wild.latex` | Latex naturel | Hévéa, gutta-percha et profils analogues. |
| `wild.tannin` | Matières tannantes | Écorces, galles, feuilles et fruits riches en tanins. |
| `wild.fibre` | Fibres sauvages | Ortie, liber, feuilles de palmiers, alfa et autres graminées fibreuses. |
| `wild.dye` | Végétaux, bois et lichens tinctoriaux | Profils de colorants rouges, jaunes, bleus et bruns ; ressource brute. |
| `wild.medicinal` | Plantes à usage médicinal potentiel | Profils de racines, feuilles, écorces et résines ; sans efficacité générique implicite. |
| `wild.aromatic` | Plantes aromatiques et odorantes | Herbes, fleurs, écorces et bois. |
| `wild.spice` | Épices sauvages | Graines, fruits, racines, écorces et boutons floraux. |
| `wild.saponin` | Plantes à saponines | Matières végétales utilisables pour le lavage après préparation. |
| `wild.fodder` | Fourrage herbacé naturel | Biomasse consommable par les animaux, qualité et saisonnalité. |
| `wild.browse` | Fourrage ligneux | Feuilles, rameaux et fruits accessibles aux herbivores. |
| `wild.lichen` | Lichens utiles | Fourrage, colorants et autres usages selon le profil. |

La séparation entre bois et produits biologiques non ligneux suit le vocabulaire général des [produits forestiers non ligneux de la FAO](https://www.fao.org/forestry/nwfp/en). Les profils et identifiants de ce catalogue sont propres au projet. Pour chaque profil : habitat, abondance, partie récoltée, cycle saisonnier, renouvellement et compétition avec les autres prélèvements.

### 3.4 Faune terrestre et produits animaux sauvages — P0/P1

| Identifiant | Instance fonctionnelle | Déclinaisons |
| --- | --- | --- |
| `fauna.large_grazer` | Grands herbivores de milieux ouverts | Bisons, antilopes, équidés sauvages et profils analogues. |
| `fauna.woodland_game` | Grand gibier forestier | Cervidés et suidés sauvages. |
| `fauna.mountain_game` | Gibier de relief | Caprinés sauvages et autres herbivores montagnards. |
| `fauna.small_game` | Petit gibier terrestre | Lièvres, lapins et rongeurs adaptés. |
| `fauna.land_bird` | Oiseaux terrestres | Galliformes, pigeons et profils analogues. |
| `fauna.waterfowl` | Oiseaux d'eau | Canards, oies et oiseaux de zones humides. |
| `fauna.egg_colony` | Colonies productrices d'œufs récoltables | Oiseaux côtiers ou terrestres ; prélèvements liés à la reproduction. |
| `fauna.fur_bearer` | Animaux à fourrure | Castor, martre, renard et profils biologiquement compatibles. |
| `fauna.large_tusked` | Grande faune à défenses | Éléphant, mammouth ou profils de scénario. |
| `fauna.edible_insect` | Insectes alimentaires | Orthoptères, larves et insectes sociaux selon les provinces. |
| `fauna.wild_bee` | Abeilles sauvages | Colonies, miel récoltable, cire et pollinisation. |
| `fauna.dye_insect` | Insectes tinctoriaux | Cochenilles, kermès ; plantes hôtes nécessaires. |
| `fauna.lac_insect` | Insectes producteurs de laque | Hôtes et climats compatibles. |
| `fauna.wild_silkworm` | Vers à soie sauvages | Espèces et plantes hôtes compatibles. |

Viande, graisse, peaux, fourrures, os, cornes, tendons, plumes, duvet et défenses sont des **sorties possibles d'une même population**, pas des troupeaux de ressources indépendants. Miel et cire partagent les colonies. La présence de prédateurs et les capacités de dispersion peuvent être représentées comme contraintes écologiques, sans détailler toute la chaîne alimentaire au prototype.

### 3.5 Ressources halieutiques et biologiques aquatiques — P0, détail P1

| Identifiant | Instance fonctionnelle | Habitat et contraintes principales |
| --- | --- | --- |
| `aquatic.small_pelagic` | Petits poissons pélagiques | Bancs côtiers ou hauturiers ; sardines, harengs, anchois. |
| `aquatic.large_pelagic` | Grands poissons pélagiques | Domaines marins et migrations ; thons et profils analogues. |
| `aquatic.demersal_fish` | Poissons de fond | Plateaux continentaux et fonds ; morues, merlus, poissons plats. |
| `aquatic.reef_fish` | Poissons récifaux | Habitat récifal, température et renouvellement. |
| `aquatic.estuary_fish` | Poissons d'estuaire et de lagune | Salinité variable et connexion aux nurseries. |
| `aquatic.cold_freshwater_fish` | Poissons d'eau douce froide | Eau oxygénée, température et réseau connecté. |
| `aquatic.warm_freshwater_fish` | Poissons d'eau douce tempérée à chaude | Lacs, rivières et plaines inondables ; carpes, silures, tilapias selon la province. |
| `aquatic.migratory_fish` | Poissons migrateurs entre mer et rivière | Saumons, aloses, esturgeons et anguilles ; obstacles et itinéraires. |
| `aquatic.marine_crustacean` | Crustacés marins | Crevettes, crabes, homards et langoustes. |
| `aquatic.freshwater_crustacean` | Crustacés d'eau douce | Écrevisses et crevettes d'eau douce. |
| `aquatic.bivalve` | Bivalves alimentaires | Huîtres, moules, palourdes et coquilles ; habitat et qualité de l'eau. |
| `aquatic.gastropod` | Gastéropodes aquatiques | Ormeaux, conques et autres escargots aquatiques. |
| `aquatic.cephalopod` | Céphalopodes | Poulpes, seiches et calmars. |
| `aquatic.echinoderm` | Échinodermes récoltables | Oursins et holothuries. |
| `aquatic.marine_mammal` | Mammifères marins | Cétacés et pinnipèdes selon les provinces ; migrations et reproduction lente. |
| `aquatic.sponge` | Éponges naturelles utilisables | Substrat, profondeur et renouvellement. |
| `aquatic.ornamental_coral` | Coraux ornementaux | Habitat spécialisé et croissance lente. |
| `aquatic.pearl_mollusc` | Mollusques perliers | Probabilité de perle et population hôte partagée avec les autres usages. |
| `aquatic.dye_mollusc` | Mollusques tinctoriaux | Ressource biologique brute ; transformation ultérieure en colorant. |
| `aquatic.brown_algae` | Algues brunes | Laminaires et autres algues adaptées. |
| `aquatic.red_algae` | Algues rouges | Profils alimentaires et gélifiants. |
| `aquatic.green_algae` | Algues vertes | Profils récoltables selon les habitats. |
| `aquatic.freshwater_plant` | Végétaux aquatiques alimentaires | Macrophytes comestibles et graines selon le profil. |

Chaque population a une aire, une biomasse initiale estimée, des habitats critiques, un renouvellement, une saison et éventuellement une migration. Un banc mobile ne doit pas être dupliqué dans chaque cellule traversée. Les espèces explicitement nommées sont des déclinaisons optionnelles des guildes, à restreindre par province biologique.

La productivité halieutique doit tenir compte des habitats et connexions, pas seulement du biome côtier. Voir les [habitats estuariens de la NOAA](https://www.fisheries.noaa.gov/west-coast/habitat-conservation/estuary-habitat-west-coast) et l'[approche écosystémique des pêches de la FAO](https://www.fao.org/4/y4470e/y4470e06.htm). Une première estimation procédurale de prélèvement renouvelable doit être étiquetée comme telle ; elle ne constitue pas une estimation scientifique du rendement maximal durable.

### 3.6 Minerais métalliques et éléments associés — P0/P1, complément P2

Séparer le **gisement physique**, son minerai et les éléments récupérables. Un même gisement peut contenir plusieurs éléments. Les sources minérales données ci-dessous sont des exemples de représentation, pas une règle suffisante de placement. Les familles de dépôts et les usages doivent être calibrés avec les [profils de matières minérales du BGS](https://www.bgs.ac.uk/mineralsuk/minerals/commodity-profiles/) ; le [catalogue USGS des matières minérales](https://www.usgs.gov/centers/national-minerals-information-center/mineral-commodity-summaries/) constitue un autre référentiel de vocabulaire.

| Identifiant | Élément ou ressource cible | Priorité et formes à distinguer |
| --- | --- | --- |
| `mineral.iron` | Fer | P0 : oxydes, fer des marais, formations massives et concentrations superficielles. |
| `mineral.copper` | Cuivre | P0 : cuivre natif, oxydes superficiels, minerais sulfurés. |
| `mineral.tin` | Étain | P0 : gisements primaires et alluvionnaires. |
| `mineral.lead` | Plomb | P0 ; argent éventuellement associé. |
| `mineral.zinc` | Zinc | P1 ; sulfures, carbonates et associations polymétalliques. |
| `mineral.gold` | Or | P0 : filons, disséminations et placers ; finesse et profondeur. |
| `mineral.silver` | Argent | P0 : gisement propre ou coproduit. |
| `mineral.mercury` | Mercure | P1 ; minerais dont le cinabre, sans mercure métallique automatiquement disponible. |
| `mineral.antimony` | Antimoine | P1. |
| `mineral.arsenic` | Arsenic | P1 ; également contrainte naturelle de qualité de certaines eaux et minerais. |
| `mineral.bismuth` | Bismuth | P1. |
| `mineral.nickel` | Nickel | P2 ; sources sulfurées et latéritiques. |
| `mineral.cobalt` | Cobalt | P2 ; préciser le lien avec le gisement hôte. |
| `mineral.manganese` | Manganèse | P1/P2. |
| `mineral.chromium` | Chrome | P2. |
| `mineral.aluminium` | Aluminium | P2 ; bauxite et qualité de la matière première. |
| `mineral.magnesium` | Magnésium | P2 ; roches et saumures hôtes, avec référence à la source partagée. |
| `mineral.titanium` | Titane | P2 ; dépôts rocheux et sables minéralisés. |
| `mineral.vanadium` | Vanadium | P2. |
| `mineral.molybdenum` | Molybdène | P2. |
| `mineral.tungsten` | Tungstène | P2. |
| `mineral.niobium` | Niobium | P2. |
| `mineral.tantalum` | Tantale | P2. |
| `mineral.lithium` | Lithium | P2 ; roches, argiles ou saumures selon le modèle choisi. |
| `mineral.beryllium` | Béryllium | P2. |
| `mineral.zirconium` | Zirconium | P2 ; zircon et dépôts hôtes. |
| `mineral.hafnium` | Hafnium | P2 ; associé aux sources de zirconium. |
| `mineral.gallium` | Gallium | P2 ; récupération liée à une matière hôte. |
| `mineral.germanium` | Germanium | P2 ; récupération liée à une matière hôte. |
| `mineral.indium` | Indium | P2 ; récupération liée à une matière hôte. |
| `mineral.cadmium` | Cadmium | P2 ; coproduit et contrainte de qualité. |
| `mineral.thallium` | Thallium | P2 ; coproduit et contrainte de qualité. |
| `mineral.rhenium` | Rhénium | P2 ; lié aux minerais hôtes. |
| `mineral.selenium` | Sélénium | P2 ; élément associé, pas nécessairement une mine autonome. |
| `mineral.tellurium` | Tellure | P2 ; élément associé. |
| `mineral.caesium` | Césium | P2. |
| `mineral.rubidium` | Rubidium | P2. |
| `mineral.platinum` | Platine | P1/P2 ; métal natif, placer ou association. |
| `mineral.palladium` | Palladium | P2. |
| `mineral.rhodium` | Rhodium | P2. |
| `mineral.iridium` | Iridium | P2. |
| `mineral.osmium` | Osmium | P2. |
| `mineral.ruthenium` | Ruthénium | P2. |
| `mineral.scandium` | Scandium | P2. |
| `mineral.yttrium` | Yttrium | P2. |

**Terres rares — P2 :** le profil simplifié peut stocker un assemblage, avec composition explicitée lorsqu'il est raffiné. Les instances de base sont `mineral.lanthanum` — lanthane ; `mineral.cerium` — cérium ; `mineral.praseodymium` — praséodyme ; `mineral.neodymium` — néodyme ; `mineral.samarium` — samarium ; `mineral.europium` — europium ; `mineral.gadolinium` — gadolinium ; `mineral.terbium` — terbium ; `mineral.dysprosium` — dysprosium ; `mineral.holmium` — holmium ; `mineral.erbium` — erbium ; `mineral.thulium` — thulium ; `mineral.ytterbium` — ytterbium ; `mineral.lutetium` — lutécium. L'yttrium et le scandium sont déjà déclarés ci-dessus. Le prométhium n'est pas une ressource minière ordinaire de ce catalogue.

L'uranium et le thorium sont déclarés avec les combustibles minéraux (§3.10), sans seconde réserve sous un autre identifiant.

### 3.7 Sels, minéraux industriels, amendements et pigments — P0/P1, complément P2

| Identifiant | Instance | Formes et usages possibles |
| --- | --- | --- |
| `industrial.halite` | Sel gemme | Alimentation, conservation et chimie après extraction et préparation. |
| `industrial.natural_brine` | Saumure naturelle | Composition : sodium, potassium, magnésium, lithium, brome ou iode selon la province ; une source physique partagée. |
| `industrial.bromine` | Brome contenu dans une source naturelle | Saumure ou autre source hôte ; produit récupérable conditionnel au procédé. |
| `industrial.iodine` | Iode contenu dans une source naturelle | Saumures, dépôts ou biomasse hôtes ; référencer les stocks et flux concernés. |
| `industrial.potash` | Sels de potassium | Sylvinite, carnallite et assemblages ; amendement ou chimie selon la technique. |
| `industrial.nitrate` | Dépôts naturels de nitrates | Dépôts minéraux, distincts de la production biologique ou industrielle de salpêtre. |
| `industrial.phosphate` | Roches phosphatées | Phosphorite et autres matières phosphatées. |
| `industrial.guano` | Guano accumulé | Dépôt biologique, âge, stock et renouvellement par les colonies ; usage partagé avec celles-ci. |
| `industrial.sulfur` | Soufre | Natif ou contenu dans une matière hôte ; accès selon le procédé. |
| `industrial.borate` | Borates | Borax, ulexite et assemblages. |
| `industrial.natron` | Carbonates naturels de sodium | Natron et trona, avec composition. |
| `industrial.alum` | Matières premières d'alun | Alunite et dépôts appropriés ; transformation nécessaire selon la source. |
| `industrial.fluorite` | Fluorine | Qualité chimique ou ornementale du même matériau. |
| `industrial.barite` | Barytine | Qualité et pureté. |
| `industrial.celestine` | Célestine | Matière première de strontium. |
| `industrial.gypsum` | Gypse | Plâtre et amendement après préparation. |
| `industrial.anhydrite` | Anhydrite | Liant et autres usages après préparation. |
| `industrial.common_clay` | Argile commune | Terre crue, brique et céramique selon ses propriétés. |
| `industrial.kaolin` | Kaolin | Céramique fine et autres usages. |
| `industrial.ball_clay` | Argile plastique fine | Mélanges céramiques. |
| `industrial.fireclay` | Argile réfractaire | Température d'usage estimée et impuretés. |
| `industrial.bentonite` | Bentonite | Argile à propriétés spécifiques. |
| `industrial.silica` | Silice de qualité technique | Quartz, quartzite ou sable suffisamment pur ; référence au gisement hôte. |
| `industrial.feldspar` | Feldspath | Céramique et verre. |
| `industrial.graphite` | Graphite | Lubrifiant, réfractaire et usages modernes. |
| `industrial.mica` | Mica | Feuillets et qualités techniques. |
| `industrial.talc` | Talc | Pureté et propriétés. |
| `industrial.magnesite` | Magnésite | Magnésie après transformation. |
| `industrial.diatomite` | Diatomite | Matière siliceuse légère. |
| `industrial.perlite` | Perlite | Roche utilisable après traitement thermique. |
| `industrial.vermiculite` | Vermiculite | Qualité du gisement et potentiel après traitement. |
| `industrial.zeolite` | Zéolites naturelles | Propriétés et composition simplifiées. |
| `industrial.natural_abrasive` | Abrasifs naturels | Émeri, corindon non gemme, grenat industriel ; composition précisée. |
| `industrial.fibrous_mineral` | Minéraux fibreux | Profils industriels facultatifs ; renseigner aussi la propriété de danger du matériau. |
| `pigment.red_ochre` | Ocre rouge | Matière ferrugineuse ; couleur et qualité du dépôt. |
| `pigment.yellow_ochre` | Ocre jaune | Matière ferrugineuse ; couleur et qualité. |
| `pigment.umber` | Terre d'ombre | Terres colorantes avec composition simplifiée. |
| `pigment.green_earth` | Terre verte | Matières minérales naturellement colorantes. |

Le cinabre, la malachite, l'azurite, le lapis-lazuli, la craie et le graphite peuvent aussi fournir des pigments ; utiliser leurs sources déjà présentes. Les colorants fabriqués, engrais composés, acides, explosifs, ciments et verres ne sont pas des ressources naturelles supplémentaires.

Le calcium, le sodium, le potassium, le phosphore, le bore, le baryum, le strontium, le fluor et le silicium peuvent être décrits dans la composition des matières hôtes de ce catalogue. Leur extraction sous forme d'élément ou de composé est une transformation ; ne pas ajouter une seconde réserve indépendante à la roche, au sel ou à la saumure concernés.

### 3.8 Roches, granulats et pierres d'outillage — P0/P1

| Identifiant | Instance | Propriétés utiles |
| --- | --- | --- |
| `stone.sand` | Sable | Granulométrie, salinité, forme des grains et pureté ; les usages ne sont pas interchangeables. |
| `stone.gravel` | Gravier et galets | Granulométrie et accessibilité. |
| `stone.limestone` | Calcaire | Pierre, chaux ou ciment après transformation ; un même stock. |
| `stone.chalk` | Craie | Construction, amendement et pigment selon qualité. |
| `stone.dolostone` | Dolomie | Construction, amendement et matière première industrielle. |
| `stone.sandstone` | Grès | Grain, tenue, taille et altérabilité. |
| `stone.granite` | Granite | Affleurement, fracturation et blocs extractibles. |
| `stone.basalt` | Basalte | Construction, pavage et granulats. |
| `stone.gneiss` | Gneiss | Structure et possibilités de taille. |
| `stone.schist` | Schiste | Débit et tenue des matériaux. |
| `stone.slate` | Ardoise | Fissilité et dimensions des plaques. |
| `stone.marble` | Marbre | Couleur, qualité, blocs et accès. |
| `stone.travertine` | Travertin et tuf calcaire | Porosité et propriétés de construction. |
| `stone.volcanic_tuff` | Tuf volcanique | Propriétés de construction et caractère éventuellement pouzzolanique. |
| `stone.pumice` | Pierre ponce | Matériau léger et abrasif. |
| `stone.pozzolan` | Matière pouzzolanique naturelle | Qualité réactive ; peut être un usage d'une source volcanique déjà présente. |
| `stone.soapstone` | Stéatite | Taille, tenue thermique et composition. |
| `stone.serpentinite` | Serpentinite | Pierre décorative ou de construction. |
| `stone.flint` | Silex et chert | Qualité de taille et nodules disponibles. |
| `stone.obsidian` | Obsidienne | Qualité de taille et affleurements. |
| `stone.quartzite` | Quartzite | Outillage, construction ou source siliceuse suivant la qualité. |
| `stone.shell_deposit` | Accumulations coquillières | Stock ancien ou apport récent ; usages de construction ou de chaux. |

Ajouter dureté, résistance, porosité, facilité de taille et taille maximale des blocs lorsque cela sert Magna Urbis. Un profil régional de roche dominante suffit d'abord ; les carrières exploitables se précisent au zoom.

### 3.9 Gemmes, matières ornementales et raretés naturelles — P1

| Identifiant | Instance | Variantes de base |
| --- | --- | --- |
| `gem.diamond` | Diamant | Qualité gemme ou industrielle, dépôt primaire ou secondaire. |
| `gem.corundum` | Corindon gemme | Rubis et saphir, couleurs et qualité. |
| `gem.beryl` | Béryl gemme | Émeraude, aigue-marine, morganite et héliodore. |
| `gem.chrysoberyl` | Chrysobéryl | Alexandrite et œil-de-chat. |
| `gem.spinel` | Spinelle | Couleurs et qualité. |
| `gem.garnet` | Grenat gemme | Familles de grenats ; qualité ornementale ou abrasive. |
| `gem.topaz` | Topaze | Couleurs et qualité. |
| `gem.tourmaline` | Tourmaline | Couleurs et qualité. |
| `gem.zircon` | Zircon gemme | Fraction gemme d'une source de zircon. |
| `gem.quartz` | Quartz ornemental | Cristal de roche, améthyste, citrine, quartz rose et fumé. |
| `gem.chalcedony` | Calcédoine | Agate, cornaline, onyx et chrysoprase. |
| `gem.jasper` | Jaspe | Variantes de couleur et de motif. |
| `gem.opal` | Opale | Qualité, hydratation et fragilité. |
| `gem.turquoise` | Turquoise | Qualité et couleur. |
| `gem.jade` | Jade | Jadéite et néphrite, sources distinctes. |
| `gem.lapis_lazuli` | Lapis-lazuli | Qualité et composition de la roche. |
| `gem.malachite` | Malachite ornementale | Usage d'une source cuprifère. |
| `gem.azurite` | Azurite ornementale | Usage d'une source cuprifère. |
| `gem.fluorite` | Fluorine ornementale | Fraction de qualité ornementale d'une source existante. |
| `ornament.amber` | Ambre | Résine fossile, dépôt primaire ou remanié. |
| `ornament.jet` | Jais | Matière organique fossile de qualité ornementale. |
| `ornament.fossil` | Fossiles remarquables | Type, rareté et conservation ; aucun prix culturel imposé. |

Perles, coraux, nacre, coquillages, défenses et plumes proviennent des populations biologiques du catalogue. L'or et l'argent restent les ressources minérales déjà déclarées. Une qualité gemme est une fraction du matériau hôte, pas une réserve ajoutée à celui-ci.

### 3.10 Combustibles, matières fossiles et fluides du sous-sol — P0/P1, complément P2

| Identifiant | Instance | Précisions |
| --- | --- | --- |
| `fuel.peat` | Tourbe | Stock, humidité et accumulation lente ; stock quasi fini à l'échelle d'une exploitation historique. |
| `fuel.lignite` | Lignite | Épaisseur, profondeur et pouvoir calorifique. |
| `fuel.coal` | Houille et charbon de rang intermédiaire | Rang, teneur en soufre, propriétés et aptitude à la cokéfaction si utile. |
| `fuel.anthracite` | Anthracite | Qualité et accessibilité. |
| `fuel.crude_oil` | Pétrole naturel | Réservoir, profondeur, qualité et conditions de pression. |
| `fuel.natural_gas` | Gaz naturel | Composition et réservoir ; distinguer gaz associé ou non au pétrole. |
| `fuel.natural_bitumen` | Bitume naturel | Suintements et dépôts ; usages historiques possibles. |
| `fuel.oil_sand` | Sables bitumineux | Source contenant le bitume, sans double réserve d'hydrocarbures. |
| `fuel.oil_shale` | Roches riches en kérogène | Potentiel sous un procédé adapté ; pas de pétrole liquide déjà disponible. |
| `fuel.gas_hydrate` | Hydrates de gaz | P2 facultatif ; conditions de stabilité et accessibilité. |
| `fuel.uranium` | Minerais d'uranium | Stock contenu et caractéristiques du gisement. |
| `fuel.thorium` | Minerais de thorium | Stock contenu, souvent associé à une source hôte. |
| `fluid.helium` | Hélium naturel | Composition et réservoir partagé éventuel. |
| `fluid.natural_carbon_dioxide` | Dioxyde de carbone naturel concentré | Réservoirs et émissions naturelles ; usages de scénario. |
| `fluid.native_hydrogen` | Hydrogène naturel | P2 expérimental, seulement avec un modèle explicitement assumé. |

Bois combustible et résidus agricoles relèvent des stocks biologiques déjà décrits. Charbon de bois, coke, gaz manufacturé, carburants raffinés et biocarburants sont des produits de transformation. Soleil, vent et courants sont des flux énergétiques, décrits au §5.

### 3.11 Constituants atmosphériques — P2 facultatif

Préfixe `atmosphere.` : `nitrogen` — diazote ; `oxygen` — dioxygène ; `argon` — argon ; `carbon_dioxide` — dioxyde de carbone ; `neon` — néon ; `helium` — hélium ; `krypton` — krypton ; `xenon` — xénon.

La composition et la pression atmosphériques sont des paramètres de planète. Ces constituants sont déclarés pour les scénarios industriels ; ils ne nécessitent pas des gisements locaux ni une couche haute résolution chacun. Une extraction technique dépend des concentrations, de l'énergie et des installations. L'hélium atmosphérique et celui d'un réservoir souterrain sont des sources différentes ; la vapeur d'eau reste dans le cycle hydrique. Le profil terrestre peut prendre pour référence la [description de l'atmosphère par la NOAA](https://gml.noaa.gov/outreach/carbon_toolkit/basics.html), sans imposer sa composition à toutes les planètes.

## 4. Aptitudes agricoles, pastorales et aquacoles

Ces instances sont des **profils de production possibles**, pas des champs cultivés ni des animaux domestiques générés automatiquement. Chaque profil combine besoins écologiques, origine biogéographique et conditions d'exploitation. La diffusion des plantes, la domestication et les introductions sont historiques ou définies dans le bootstrap.

Pour commencer, calculer des profils fonctionnels de familles et activer les espèces détaillées à la demande. Le catalogue ci-dessous déclare toutes les instances proposées pour v1 ; il ne demande pas un raster planétaire permanent pour chacune.

### 4.1 Profils de cultures — P0 par familles, détail P1

Les codes de cette table prennent le préfixe `crop.`. Chaque association « code — libellé » déclare une instance distincte ; un végétal à plusieurs usages garde le même identifiant.

| Famille | Instances de base |
| --- | --- |
| Céréales | `wheat` — blé ; `barley` — orge ; `rye` — seigle ; `oat` — avoine ; `rice` — riz ; `maize` — maïs ; `sorghum` — sorgho ; `pearl_millet` — mil perlé ; `foxtail_millet` — millet des oiseaux ; `finger_millet` — éleusine ; `teff` — teff. |
| Pseudo-céréales | `buckwheat` — sarrasin ; `quinoa` — quinoa ; `amaranth` — amarante. |
| Légumineuses alimentaires | `pea` — pois ; `lentil` — lentille ; `chickpea` — pois chiche ; `faba_bean` — fève ; `common_bean` — haricot commun ; `cowpea` — niébé ; `mung_bean` — haricot mungo ; `pigeon_pea` — pois d'Angole ; `soybean` — soja ; `lupin` — lupin. |
| Racines et tubercules | `potato` — pomme de terre ; `sweet_potato` — patate douce ; `cassava` — manioc ; `yam` — igname ; `taro` — taro ; `turnip` — navet ; `carrot` — carotte ; `parsnip` — panais ; `beetroot` — betterave potagère. |
| Légumes et maraîchage | `cabbage` — choux ; `leafy_greens` — légumes feuilles ; `onion` — oignon ; `garlic` — ail ; `leek` — poireau ; `cucumber` — concombre ; `squash` — courges ; `melon` — melon ; `watermelon` — pastèque ; `tomato` — tomate ; `aubergine` — aubergine ; `sweet_pepper` — poivron. |
| Oléagineux | `olive` — olivier ; `sesame` — sésame ; `sunflower` — tournesol ; `rapeseed` — colza ; `peanut` — arachide ; `safflower` — carthame ; `castor` — ricin ; `oil_palm` — palmier à huile ; `coconut` — cocotier. Lin, soja et chanvre ont aussi un usage oléagineux. |
| Fibres | `flax` — lin ; `hemp` — chanvre ; `cotton` — coton ; `jute` — jute ; `ramie` — ramie ; `sisal` — sisal ; `abaca` — abaca ; `kenaf` — kénaf ; `nettle` — ortie textile. |
| Sucres et amidons arborés | `sugarcane` — canne à sucre ; `sugar_beet` — betterave sucrière ; `sugar_maple` — érable à sucre ; `sugar_palm` — palmier à sucre ; `sago_palm` — palmier à sagou ; `breadfruit` — arbre à pain. |
| Fruits tempérés | `apple` — pommier ; `pear` — poirier ; `plum` — prunier ; `cherry` — cerisier ; `peach` — pêcher ; `apricot` — abricotier ; `grape` — vigne ; `strawberry` — fraisier ; `raspberry` — framboisier. |
| Fruits secs | `almond` — amandier ; `walnut` — noyer ; `hazelnut` — noisetier ; `chestnut` — châtaignier ; `pistachio` — pistachier ; `cashew` — anacardier. |
| Fruits chauds et tropicaux | `fig` — figuier ; `date_palm` — palmier dattier ; `pomegranate` — grenadier ; `citrus` — agrumes ; `banana` — bananier à dessert ; `plantain` — bananier plantain ; `mango` — manguier ; `papaya` — papayer ; `guava` — goyavier ; `avocado` — avocatier ; `pineapple` — ananas. |
| Boissons et stimulants | `tea` — théier ; `coffee` — caféier ; `cacao` — cacaoyer ; `mate` — maté ; `kola` — kolatier ; `hops` — houblon ; `tobacco` — tabac. Vigne et céréales conservent leurs identifiants précédents. |
| Épices | `black_pepper` — poivrier noir ; `long_pepper` — poivre long ; `chilli` — piments ; `ginger` — gingembre ; `turmeric` — curcuma ; `cinnamon` — cannelle ; `clove` — girofle ; `nutmeg` — muscade et macis ; `cardamom` — cardamome ; `vanilla` — vanille ; `saffron` — safran ; `cumin` — cumin ; `coriander` — coriandre ; `anise` — anis ; `caraway` — carvi ; `mustard` — moutarde. |
| Plantes tinctoriales | `woad` — pastel ; `indigo` — indigotiers ; `madder` — garance ; `weld` — gaude ; `henna` — henné ; `logwood` — campêche. Les cultures déjà citées à usage tinctorial gardent leur identifiant. |
| Aromatiques et parfumerie | `lavender` — lavande ; `mint` — menthes ; `thyme` — thym ; `rosemary` — romarin ; `rose` — rosiers à parfum ; `jasmine` — jasmin. |
| Plantes à usage médicinal ou spécialisé | `cinchona` — quinquina ; `artemisia` — armoises de profil adapté ; `aloe` — aloès ; `liquorice` — réglisse ; `ginseng` — ginseng ; `opium_poppy` — pavot à opium. La présence de ces profils ne constitue pas une simulation pharmacologique. |
| Fourrages cultivés | `alfalfa` — luzerne ; `clover` — trèfles ; `vetch` — vesces ; `forage_grass` — graminées fourragères. Céréales et résidus peuvent participer aux rations sans nouvelle surface fictive. |
| Arbres et plantations utiles | `rubber_tree` — hévéa ; `mulberry` — mûrier pour feuilles et fruits ; `tannin_tree` — arbres à tanins ; `resin_tree` — arbres à résines ; `timber_tree` — profil de plantation forestière, lié aux essences du §3.2. |

Modes de conduite à distinguer quand pertinents : pluvial, irrigué, inondé, décrue, verger, agroforesterie, terrasse et culture protégée. Un mode inadapté à une espèce est absent. Le riz pluvial et le riz inondé partagent `crop.rice`, mais ont des besoins et rendements différents.

Sorties par profil et mode : fraction de terrain apte, rendement potentiel avec unité et produit récolté, calendrier, besoin en eau, contraintes majeures, variabilité et temps avant première récolte pour les pérennes. Les profils de conduite sont nommés ; ne pas appliquer par défaut les rendements d'une agriculture moderne à un scénario médiéval.

La distinction aptitude/rendement, pluvial/irrigué et niveau de conduite s'appuie sur la logique de [FAO–IIASA GAEZ](https://www.fao.org/gaez/gaezv4/en). Les espèces sélectionnées et la future méthode procédurale sont propres au projet ; il n'est pas demandé de réimplémenter GAEZ.

### 4.2 Profils d'élevage — P0 par systèmes, détail P1

Préfixe `livestock.`. Ces profils décrivent la compatibilité écologique et les besoins ; ils ne créent pas la domestication, le cheptel ou ses produits.

| Système | Instances de base |
| --- | --- |
| Bovins | `taurine_cattle` — bovins taurins ; `zebu` — zébus ; `water_buffalo` — buffles d'eau ; `yak` — yaks. |
| Petits ruminants | `sheep` — ovins ; `goat` — caprins. |
| Équidés | `horse` — chevaux ; `donkey` — ânes. La mule est un hybride produit par l'élevage, pas une population naturelle indépendante. |
| Camélidés | `dromedary` — dromadaires ; `bactrian_camel` — chameaux de Bactriane ; `llama` — lamas ; `alpaca` — alpagas. |
| Élevage de milieux froids | `reindeer` — rennes. Le yak est déjà déclaré. |
| Omnivores | `pig` — porcins ; `rabbit` — lapins. |
| Oiseaux domestiques | `chicken` — poules ; `duck` — canards ; `goose` — oies ; `turkey` — dindes ; `guineafowl` — pintades ; `pigeon` — pigeons. |
| Invertébrés élevés | `honeybee` — abeilles ; `silkworm` — vers à soie ; `edible_insect` — insectes alimentaires adaptés ; `lac_insect` — insectes à laque. |

Conserver fourrage de saison, eau, abri climatique, contraintes de déplacement et concurrence avec les cultures. Les sorties possibles sont viande, lait, œufs, laine, poils, soie, miel, cire, peaux, fumure, traction et transport. Elles dépendent de la même allocation de cheptel, de nourriture et de travail ; elles ne sont pas toutes maximisables simultanément.

### 4.3 Profils d'aquaculture — P1

Préfixe `aquaculture.` : `carp` — carpes ; `catfish` — silures et poissons-chats compatibles ; `tilapia` — tilapias ; `trout` — truites ; `salmon` — saumons ; `shrimp` — crevettes ; `crayfish` — écrevisses ; `oyster` — huîtres ; `mussel` — moules ; `clam` — palourdes ; `pearl_mollusc` — mollusques perliers ; `seaweed` — algues cultivables.

Fournir température, salinité, oxygénation et renouvellement de l'eau, espace compatible, habitat sensible et besoins alimentaires externes. Une aptitude aquacole n'est pas une production gratuite ajoutée à la pêche sauvage. Les installations, intrants, rejets, introductions et droits sont historiques.

## 5. Flux énergétiques, services écologiques et opportunités spatiales

### 5.1 Potentiels énergétiques — P0 pour eau/vent, détail P2

| Identifiant | Potentiel | Sorties utiles |
| --- | --- | --- |
| `energy.water_power` | Force hydraulique | Débit exploitable et hauteur de chute disponible ; énergie saisonnière sous hypothèses d'ouvrage. |
| `energy.wind` | Vent | Distribution de vitesse, directions, saisonnalité et exposition ; référence de hauteur. |
| `energy.solar` | Rayonnement solaire | Énergie reçue par surface, saisonnalité et masques ; usage thermique ou photovoltaïque ultérieur. |
| `energy.geothermal` | Chaleur du sous-sol | Température/profondeur, flux estimé, fluide et renouvellement ; contrainte de prélèvement. |
| `energy.tidal` | Marées | Amplitude, courants et géométrie des sites ; dépend du modèle astronomique de la planète. |
| `energy.wave` | Houle | Flux énergétique, exposition et saisonnalité. |
| `energy.current` | Courants marins | Vitesse et stabilité, sans double compte avec les courants de marée. |

Ces flux ne sont ni du combustible en réserve ni une production électrique déjà construite. Conversions techniques et puissance installée relèvent des usages. L'énergie hydraulique partage l'eau et le réseau avec irrigation, navigation, habitats et approvisionnement.

### 5.2 Services et capacités de soutien des écosystèmes — P1, simplification P0

| Identifiant | Service | Effet à représenter |
| --- | --- | --- |
| `ecosystem.pollination` | Pollinisation | Compatibilité des pollinisateurs, habitats et portée autour des cultures concernées. |
| `ecosystem.pest_regulation` | Régulation biologique | Effet potentiel des habitats et prédateurs sur certains ravageurs. |
| `ecosystem.soil_renewal` | Entretien des sols | Recyclage de matière organique, structure et renouvellement lent. |
| `ecosystem.nutrient_cycling` | Recyclage des nutriments | Effet sur la fertilité ; ne crée pas de matière sans source. |
| `ecosystem.erosion_control` | Protection contre l'érosion | Couverture végétale, pente et fragilité du sol. |
| `ecosystem.water_retention` | Rétention et soutien d'étiage | Sols, zones humides, végétation et connexions hydriques. |
| `ecosystem.flood_attenuation` | Atténuation des crues | Espaces d'expansion et stockage temporaire, avec limites. |
| `ecosystem.water_purification` | Épuration naturelle | Capacité limitée, type de charge et temps de résidence. |
| `ecosystem.coastal_protection` | Protection du littoral | Dunes, mangroves, marais côtiers et récifs. |
| `ecosystem.aquatic_nursery` | Reproduction et nurseries aquatiques | Habitats critiques et connexions qui soutiennent les populations. |
| `ecosystem.habitat_connectivity` | Continuité écologique | Dispersion, recolonisation et itinéraires migratoires. |
| `ecosystem.carbon_storage` | Stockage de carbone | Stock dans sols/végétation et évolution potentielle ; facultatif pour l'histoire ancienne. |

Ces services sont des modificateurs ou des fonctions du milieu, pas des marchandises à sommer avec les récoltes. La [FAO décrit notamment le rôle du cycle des nutriments, de la pollinisation et de la régulation des ravageurs dans la productivité agricole](https://www.fao.org/4/ai553e/ai553e00.htm). Le niveau de détail ci-dessus est un choix de conception.

### 5.3 Opportunités géographiques pour les villes et territoires — P0

| Identifiant | Instance | Information attendue |
| --- | --- | --- |
| `site.harbour` | Abri portuaire naturel | Profondeur, accès, protection, exposition, glace et tendance à l'ensablement. |
| `site.river_landing` | Débarcadère fluvial potentiel | Berge, profondeur, stabilité et accès terrestre. |
| `site.ford` | Gué potentiel | Profondeur, largeur, débit, substrat et saison de franchissement. |
| `site.bridge_crossing` | Site propice à un pont | Largeur, appuis, berges et régime hydrique ; aucun pont préconstruit. |
| `site.ferry_crossing` | Traversée par bac possible | Distance, courant, accostage et saisonnalité. |
| `site.mountain_pass` | Col et passage de relief | Altitude, pente, largeur, accès et fermeture saisonnière. |
| `site.land_corridor` | Corridor terrestre | Continuité, largeur et coût de déplacement. |
| `site.strait` | Détroit | Largeur, profondeur, courants, exposition et connexions maritimes. |
| `site.defensible_position` | Position naturellement défendable | Relief relatif, accès, surface utilisable et approvisionnement en eau. |
| `site.buildable_land` | Terrain constructible potentiel | Pente, portance, drainage et exposition aux aléas. |
| `site.cave` | Cavité ou abri naturel | Géométrie simplifiée, accès, eau et stabilité. |
| `site.salt_pan` | Site de saline possible | Eau salée, évaporation, terrain et contraintes saisonnières. |
| `site.irrigation_intake` | Prise d'eau et zone irrigable potentielles | Niveau relatif, débit mobilisable et aire desservable. |
| `site.watermill` | Site de moulin hydraulique | Chute, débit et possibilités d'accès ; référence au potentiel hydraulique. |

Les coûts de déplacement terrestre, fluvial, maritime et de franchissement sont également indispensables. Au départ, distances et temps suffisent ; le tracé détaillé des routes peut venir ensuite. Centralité commerciale, péage rentable, capitale et frontière politique sont calculés par les usages et l'histoire ; la géographie fournit leurs conditions physiques.

## 6. Ressources intégratives à exposer aux consommateurs

Ces couches résument plusieurs entrées sans les remplacer. **Chaque résultat indique ses hypothèses** : usages du sol, technique/conduite, régime hydrique, calendrier, régime alimentaire et état écologique de référence. Les valeurs sont des estimations du modèle procédural, accompagnées d'une plage d'incertitude ou d'un statut qualitatif.

| Identifiant | Capacité intégrative | Sortie et dépendances principales |
| --- | --- | --- |
| `capacity.rainfed_agriculture` | Agriculture pluviale | Surface apte et rendements par profil ; climat, sols, pente et calendrier. |
| `capacity.irrigated_agriculture` | Agriculture irriguée | Surface potentiellement desservable, rendements et besoin en eau ; ressources mobilisables et accès. |
| `capacity.flood_recession_agriculture` | Agriculture de décrue | Surface et calendrier compatibles avec les crues et le retrait des eaux. |
| `capacity.horticulture` | Maraîchage | Surface apte, eau et potentiel par cycle sous une conduite nommée. |
| `capacity.orchard` | Arboriculture | Aptitude, rendement à maturité et délai d'installation des pérennes. |
| `capacity.agroforestry` | Agroforesterie | Combinaison de strates et productions compatibles, avec partage de surface/eau/lumière. |
| `capacity.pasture` | Pâturage | Matière sèche utilisable par saison, qualité et prélèvement compatible avec le renouvellement. |
| `capacity.hay` | Foin | Biomasse récoltable et calendrier ; usage alternatif du même couvert herbacé. |
| `capacity.winter_fodder` | Soutien hivernal ou de saison sèche | Fourrage disponible et conservation supposée ; goulot saisonnier explicite. |
| `capacity.livestock` | Charge pastorale possible | Animaux ou unités animales sous un profil documenté de ration, eau et mobilité. |
| `capacity.forestry` | Sylviculture et bois renouvelable | Stock sur pied et prélèvement annuel estimé par qualité, avec régénération. |
| `capacity.fuel_biomass` | Combustible biologique | Biomasse allouable et énergie brute ; concurrence avec bois d'œuvre, fourrage et restitution au sol. |
| `capacity.gathering` | Cueillette | Aliments et matières récoltables par saison, effort et renouvellement. |
| `capacity.hunting` | Chasse | Prélèvement estimé de populations animales, avec reproduction et habitat. |
| `capacity.inland_fishery` | Pêche continentale | Biomasse et prélèvement potentiel par réseau ou lac, habitat et saison. |
| `capacity.coastal_fishery` | Pêche côtière | Potentiels par zone marine et moyens d'accès ; habitats et renouvellement. |
| `capacity.offshore_fishery` | Pêche hauturière | Ressources mobiles accessibles sous un profil de navigation ; pas de duplication des stocks. |
| `capacity.aquaculture` | Aquaculture | Surface ou volume compatible et production conditionnelle, avec besoins et contraintes. |
| `capacity.water_supply` | Approvisionnement en eau | Volume mobilisable par saison, qualité, renouvellement et conditions de captage. |
| `capacity.mining` | Extraction minière possible | Réserves par matière, qualité, profondeur et difficulté sous un profil technique ; pas un indice universel. |
| `capacity.quarrying` | Carrières et matériaux | Quantités accessibles, qualité, blocs et granulométrie sous un profil technique. |
| `capacity.salt_production` | Production de sel | Matière ou saumure disponible, évaporation/énergie et conditions d'aménagement. |
| `capacity.mechanical_energy` | Énergie mécanique mobilisable | Sites hydrauliques et éoliens compatibles avec un équipement nommé. |
| `capacity.settlement` | Accueil physique d'un établissement | Surface, eau, aléas et accès ; plusieurs contraintes consultables. |
| `capacity.food_support` | Soutien alimentaire d'une population | Énergie/protéines alimentaires potentielles, saisonnalité et nombre de personnes sous un scénario explicite. |
| `capacity.local_surplus` | Surplus potentiel exportable | Production d'un scénario moins semences, pertes, stocks de sécurité, usages intermédiaires et besoins d'une population fournie par le consommateur. |

`capacity.local_surplus` nécessite des hypothèses sociales et une population : c'est une **évaluation de scénario**, pas une propriété intrinsèque du terrain. De même, un bassin peut accueillir une grande ville importatrice même si son soutien alimentaire local est faible. Chronos combine ressources, accès, échanges et institutions.

### 6.1 Règles de calcul à ne pas perdre dans les agrégats

1. **Concurrence foncière.** Un hectare n'offre pas simultanément son rendement maximal de blé, de forêt et de pâturage. Décrire les usages alternatifs et les mélanges compatibles ; les surfaces allouées doivent respecter le terrain disponible.
2. **Concurrence hydrique.** Irrigation, consommation, industrie, énergie, navigation et écologie partagent les apports et réservoirs. Référencer le bassin ou la nappe, pas seulement une valeur locale.
3. **Coproduits.** Grain/paille, viande/peau, miel/cire et cuivre/argent d'un même gisement sont liés par des rendements et une source physique commune.
4. **Saisons.** Une capacité annuelle ne masque pas le manque d'eau de l'été, le déficit fourrager hivernal ou une route fermée. Fournir le calendrier et le minimum saisonnier pertinent.
5. **Stock et renouvellement.** Couper le stock forestier permet temporairement de dépasser l'accroissement ; cela ne rend pas ce prélèvement permanent. Même distinction pour poissons, gibier, tourbe et nappes.
6. **Technique.** Donner des profils de conduite et d'accès plutôt qu'une valeur « utilisable » absolue : ramassage de surface, extraction peu profonde, extraction profonde, transport ou irrigation disponibles.
7. **Population.** Ne déduire aucune capacité agricole de la population initiale. La capacité alimentaire se calcule sous des hypothèses explicites de surfaces, productions, alimentation, eau, pertes et saisons.

Exemple conceptuel : pour un scénario alimentaire, `personnes_soutenues = min(énergie_alimentaire_disponible / besoin_énergétique, protéines_disponibles / besoin_protéique, eau_domestique_disponible / besoin_hydrique)`, puis vérifier le calendrier des stocks et apports. Les quantités disponibles viennent d'une allocation compatible des usages et de leurs intrants. Les besoins, conversions et pertes appartiennent au profil ; cette formule simple ne représente pas à elle seule une nutrition complète.

## 7. Contraintes naturelles à joindre aux ressources

Une ressource abondante peut être difficile à exploiter ou fragile. Fournir les couches pertinentes, avec intensité, saisonnalité, fréquence estimée et degré d'incertitude :

- **Eau et climat** : sécheresse, déficit saisonnier, crue, submersion, gel, canicule, neige, glace, tempête et cyclone si le modèle le permet.
- **Terrain et géologie** : érosion, glissement, avalanche, chute de blocs, instabilité karstique, séisme, volcanisme et tsunami lorsqu'ils sont représentés.
- **Sols et milieux** : salinité, engorgement, désertification potentielle, incendie, perte de fertilité, toxicité naturelle et acidité contraignante.
- **Biologie** : sensibilité aux ravageurs, pression écologique sur les récoltes et habitats favorables à des vecteurs de maladies. Les épidémies et leurs victimes sont générées par l'histoire, pas par cette couche.
- **Accès** : profondeur, couverture, difficulté de creusement, isolement, transport, glace marine et franchissements saisonniers.

Ne pas générer automatiquement une série d'événements historiques parce qu'une susceptibilité existe. Chronos utilise l'exposition et les mécanismes concernés pour tirer les événements et leurs conséquences.

## 8. Contrat de données minimal

### 8.1 Descripteur d'une classe ou instance

`resource_id`, libellé, famille, nature (`stock`, `flux`, `suitability`, `service`), unité, variantes, milieux compatibles, source physique, usages possibles et paramètres de renouvellement. Un profil cultivé ajoute ses exigences écologiques ; une capacité ajoute ses hypothèses de conduite.

Un premier registre de données suffit. Les familles biologiques peuvent rester abstraites jusqu'au raffinement, avec une distribution d'espèces compatible et stable. Éviter une hiérarchie logicielle complexe par minerai ou par plante.

### 8.2 Occurrence physique : gisement, réservoir ou population

| Champ | Contenu |
| --- | --- |
| `source_id` | Identifiant physique stable, partagé par les différents usages et coproduits. |
| `kind`, `resource_ids` | Type de source et matières ou profils qu'elle peut fournir. |
| `geometry`, `scope_id` | Aire, ligne, volume simplifié ou point ; unité de rattachement. |
| `quantity`, `unit`, `reference_date` | Stock initial, avec unité explicite, date et statut d'estimation. |
| `composition`, `quality` | Fractions/teneurs, impuretés et propriétés utiles. |
| `renewal`, `seasonality` | Flux, reproduction ou régénération et répartition dans l'année. |
| `depth`, `cover`, `access_constraints` | Difficultés physiques, séparées des droits et de la technique disponible. |
| `connections` | Bassin, nappe, réseau fluvial, habitat, migration ou source hôte partagée. |
| `provenance`, `model_version` | Entrées utilisées, méthode, seed et version de génération. |
| `uncertainty` | Estimé, qualitatif, intervalle ou distribution ; absence de données distincte de zéro. |

Pour les minerais, distinguer **tonnage de roche minéralisée**, **teneur** et **masse de matière contenue**. La récupération dépend du procédé ; une « réserve économiquement exploitable » n'est pas une constante géologique. Pour les matières énergétiques, préciser si l'unité est une masse, un volume dans des conditions de référence ou une énergie.

Pour l'eau, distinguer volume stocké (`m3`), débit (`m3/s`), recharge (`m3/an`) et précipitation (`mm/période`). Pour les plantes, préciser produit et matière sèche/fraîche ; pour les cultures, rendement (`kg/ha/récolte`) et production (`kg/période`). Pour la forêt, préciser bois sur pied (`m3`) ou biomasse sèche ; pour l'énergie, puissance (`W`) et énergie (`J` ou `Wh`) ne sont pas interchangeables.

Les indices normalisés `[0,1]` sont acceptables lorsque les unités physiques ne sont pas encore calibrées. Ils doivent conserver ce statut et ne pas être présentés comme des tonnes, rendements ou personnes.

### 8.3 Lecture et export attendus

- Résumé d'une région : surfaces, ressources, stocks et potentiels, contraintes, hypothèses et connexions.
- Échantillonnage d'une couche à une résolution demandée et lecture des occurrences qui intersectent une emprise.
- Consultation d'une source physique par identifiant, même lorsqu'elle traverse plusieurs régions.
- Évaluation d'un petit ensemble de scénarios de conduite : les couches physiques ne dépendent pas de la population du scénario.
- Export numérique versionné utilisable sans capture d'écran ni service GPU actif. Rust est la cible d'intégration ; un manifeste et des tableaux/objets numériques suffisent pour la première livraison.

## 9. Règles du multiéchelle procédural

1. **Engagement global.** Générer d'abord les provinces, champs directeurs, grands réservoirs et budgets nécessaires à l'histoire mondiale. Les grandes disponibilités d'une région ne dépendent pas de sa première visite.
2. **Raffinement contraint.** Le détail local répartit un stock ou un budget déjà engagé ; il peut révéler un gisement concentré et des zones vides. Il n'ajoute pas gratuitement du minerai, de la forêt ou de l'eau au total engagé.
3. **Incertitude explicite.** Une probabilité de présence ou un intervalle régional n'est pas une quantité certaine. Distinguer une prévision, un stock physique latent fixé par le canon et les informations découvertes par les sociétés. Raffiner une représentation ne crée pas une découverte historique automatique.
4. **Agrégation adaptée.** Stocks et surfaces disjoints s'additionnent ; intensités se pondèrent par les surfaces pertinentes. Un rendement régional se recalcule avec les allocations et contraintes : on ne somme pas des rendements par hectare. Débits de tronçons connectés et probabilités ne s'additionnent pas comme des stocks.
5. **Sources partagées.** Un gisement, un banc de poissons ou une nappe qui traverse une frontière garde une identité unique. Les allocations d'exploitation évitent les doubles comptes entre régions et entre niveaux de détail.
6. **Indépendance des requêtes.** Seed, versions et clés géographiques/sémantiques déterminent les résultats. Ordre d'ouverture, taille des fenêtres, parallélisme et contenu du cache ne changent pas le patrimoine naturel.
7. **Continuité des réseaux.** Cours d'eau, nappes, corridors, migrations et passages restent raccordés entre tuiles, niveaux, méridiens et pôles. Un passage stratégique connu au niveau global se retrouve dans le détail.
8. **Cohérence des corrélations.** Géologie et histoire des milieux pilotent les regroupements. Éviter une couche de bruit indépendante pour chaque métal ou chaque plante. Un raffinement précise les facteurs physiques et les redistributions autorisées.
9. **État historique séparé.** Les prélèvements, défrichements, pollutions, ouvrages et introductions modifient un état daté référencé aux sources naturelles. Recharger une tuile ne régénère pas un gisement épuisé ni une forêt coupée.
10. **Coût proportionnel au besoin.** Résumés agrégés pour le monde ; occurrences et champs locaux à la demande. Ne pas allouer un raster planétaire haute résolution par instance du catalogue.

Pour les quantités engagées, la conservation doit être contrôlable malgré les arrondis numériques. Pour les composantes encore probabilistes, définir les bornes et ce qui peut être précisé : ne pas promettre une conservation exacte de valeurs qui ne sont encore que des espérances.

## 10. Ordre de réalisation proposé

### Tranche A — socle utilisable par Magna Urbis et Chronos

1. Fixer unités, identifiants, coordonnées, provenance et règles d'agrégation ; exporter les unités géographiques et leurs accès.
2. Rendre les champs climatiques continus, habitats et sols fonctionnels cohérents entre niveaux ; ajouter le budget hydrique connecté.
3. Générer les familles P0 : eau, bois, fourrage, alimentation sauvage, poissons, matériaux, sel et minerais essentiels. Fournir une première géologie procédurale cohérente, même grossière et explicitement approximative.
4. Exposer les capacités agriculture/pâturage/forêt/pêche/eau et les contraintes ; deux ou trois profils de conduite suffisent au départ.
5. Afficher les ressources et sources physiques, et vérifier un parcours monde → région → site avec lecture des hypothèses et unités.

### Tranche B — variété économique et trajectoires locales

Déployer les profils agricoles, les ressources artisanales, les espèces et les dépôts P1. Ajouter les qualités, saisonnalités et dépendances qui changent réellement l'implantation ou la spécialisation d'un établissement. Raffiner la concurrence des usages et les services écologiques utiles.

### Tranche C — complément industriel, moderne ou fantasy

Activer les familles P2 ou PX selon le monde. Leur présence ne doit pas imposer au prototype une simulation industrielle complète. Les recettes, marchés, infrastructures et technologies restent dans les consommateurs.

### Vérifications suffisantes pour la première livraison

- Deux ordres d'exploration d'une même région donnent les mêmes sources et quantités.
- Le détail d'une région respecte ses budgets engagés ; une source frontalière n'est pas dupliquée.
- Un scénario agricole respecte les surfaces, l'eau et les saisons, avec unités explicites.
- Une forêt utilisée en bois d'œuvre et en combustible partage son stock ; une nappe ne se recharge pas lors du zoom.
- Un gisement polymétallique garde le même tonnage hôte pour ses différentes matières.
- Les couches donnent des contrastes lisibles : vallée agricole, plateau pastoral, massif minier, côte halieutique et zone pauvre ou contrainte. Les paramètres expliquent ces différences.

## 11. Extension fantasy facultative — PX

Préfixe `fantasy.` : `mana_source` — source de mana ; `ley_flow` — flux tellurique ; `enchanted_ore` — minerai aux propriétés configurées ; `arcane_crystal` — cristal réservoir ; `alchemical_plant` — flore alchimique ; `magical_fauna` — faune à ressources particulières ; `supernatural_water` — eau aux propriétés configurées ; `planar_site` — lieu d'accès ou de phénomène planaire.

Définir pour chaque instance stock ou flux, renouvellement, portée, contraintes, interactions et effets physiques autorisés. Les noms comme mithril ou adamantine appartiennent au profil de monde et à ses règles, pas au catalogue naturel commun. Conserver les mêmes exigences de stabilité, sources partagées et budgets que pour les ressources ordinaires.

## 12. Références et statut scientifique

Le catalogue, les priorités, identifiants, profils de jeu et règles d'intégration sont une proposition de conception. Les sources suivantes servent à vérifier le vocabulaire et les distinctions physiques ; elles ne valident pas un générateur qui n'est pas encore implémenté.

- [USGS — Mineral Commodity Summaries](https://www.usgs.gov/centers/national-minerals-information-center/mineral-commodity-summaries/) : référentiel de matières minérales ; aucune donnée de prix ou réserve terrestre à recopier comme vérité d'une planète fictive.
- [BGS — Mineral commodity profiles](https://www.bgs.ac.uk/mineralsuk/minerals/commodity-profiles/) : types de gisements, matières premières et usages ; point de départ pour les associations géologiques à documenter.
- [USGS — Mineral deposit models for resource assessment](https://pubs.usgs.gov/publication/sir20105070) : typologies de gisements et sources hôtes.
- [FAO — Harmonized World Soil Database v2](https://www.fao.org/soils-portal/data-hub/soil-maps-and-databases/harmonized-world-soil-database-v20/en/) : propriétés fonctionnelles des sols.
- [FAO–IIASA — GAEZ v4](https://www.fao.org/gaez/gaezv4/en) : distinction ressources, aptitude des terres, rendement, eau et conduite. La version est citée comme référence méthodologique, sans prétendre qu'il s'agit de la version la plus récente.
- [FAO — Non-wood forest products](https://www.fao.org/forestry/nwfp/en) : ressources biologiques non ligneuses.
- [FAO — Ecosystem approach to fisheries](https://www.fao.org/4/y4470e/y4470e06.htm) : habitats, populations, interactions et limites de prélèvement.
- [NOAA — Estuary Habitat on the West Coast](https://www.fisheries.noaa.gov/west-coast/habitat-conservation/estuary-habitat-west-coast) : rôle des connexions et habitats estuariens.
- [NOAA — Basics of the Carbon Cycle](https://gml.noaa.gov/outreach/carbon_toolkit/basics.html) : composition de l'atmosphère terrestre comme profil de référence facultatif.
- [USGS — Groundwater/Surface-Water Interaction](https://www.usgs.gov/mission-areas/water-resources/science/groundwatersurface-water-interaction?qt-science_center_objects=0) : réservoirs et flux hydriques connectés.
- [FAO — Ecosystem Services Sustain Agricultural Productivity and Resilience](https://www.fao.org/4/ai553e/ai553e00.htm) : fonctions écologiques soutenant la production.

Pour chaque méthode effectivement ajoutée, décrire les entrées, unités, hypothèses, approximation, déterminisme et comportement multiéchelle dans une note courte. Privilégier des mécanismes compréhensibles et des paramètres calibrables ; les indices procéduraux doivent rester identifiables comme des estimations.
