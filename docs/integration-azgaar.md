# Réutiliser Fantasy Map Generator dans Chronos

26 octobre 2026. **Statut : audit des sources et proposition d'intégration.** Les fonctions d'Azgaar décrites ci-dessous sont présentes dans le checkout examiné. Leur intégration à Chronos n'est pas réalisée par ce document ; aucun moteur, contrat JSON ou résultat de référence n'est modifié, et aucun benchmark n'a été lancé.

Repères : Chronos `2cdb2ac`, méthode `chronos-history-1` ; Azgaar/Fantasy-Map-Generator [`67d555b83f0b2f40ae50b03c9bbfd018175acc6d`][F0]. Les liens amont sont figés sur cette révision. Pour l'état de Chronos, voir la [méthode implémentée][C1], le [contrat de la fixture][C2] et le [suivi des paliers][C3].

## 1. Recommandation

**Reprendre les catalogues, certaines règles de génération et les outils cartographiques d'Azgaar pour enrichir Chronos, en conservant le canon historique de Chronos et les responsabilités de l'écosystème.** Le gain principal concerne les religions, les territoires, les marchandises, les lieux d'intérêt, les emblèmes et les récits locaux. Ce contenu donnerait une traduction concrète aux événements et aux entités sociales déjà prévus dans la conception.

La cible est plus exigeante qu'un atlas à une date : un village détruit devient une ruine après sa destruction ; une hérésie apparaît après un schisme ; une ligue possède des marchés et organise des foires ; une maison conserve ses armes au fil des successions. Ces faits doivent être identifiables, datés, explicables et identiques quel que soit l'ordre de consultation.

Le cadrage existant reste applicable : **Neural Earth porte l'espace et le milieu naturel ; Magna Urbis les établissements et leur géométrie ; Chronos le temps et les transformations historiques ; Pecking Order les structures institutionnelles ; Lineage Lab les vies et filiations ; Namebench les noms.** Azgaar apporte des sources de contenu et des méthodes à adapter à ces contrats. Il n'ajoute pas un second monde spatial à maintenir en parallèle. [Conception sociale][C4]

### 1.1. Principe : le modèle générique d'abord, le contenu Azgaar ensuite

*Ajout du 26/10/2026, après revue.* On ne reprend pas la **structure** d'Azgaar quand un mécanisme générique de Chronos produit le même résultat. Chez Azgaar, un État *a* une religion, une culture *a* un type (naval, nomade), un État *a* une forme. Chez Chronos, ces notions sont des **étiquettes lues** dans un état plus général :
- **idées** (croyances, langues, coutumes, savoirs, styles) ;
- **adhésions**, des distributions portées par les cohortes ;
- **appuis**, des relations datées comme `ENDORSES` ou `ADMINISTERS` ;
- **structure de pouvoir** (Pecking Order) et **activités**.

Ce méta-modèle est décrit dans la [conception sociale §7.8][C4]. Une religion d'État n'est donc pas imposée : elle émerge d'un événement d'adoption quand la majorité, le souverain ou une Église y poussent. La religion reste un moteur majeur, avec un poids fort dans le mécanisme générique, mais sans code propre.

Ce qu'on reprend d'Azgaar se range alors en trois catégories :
- des **catalogues de contenu** : doctrines, divinités, noms, emblèmes, biens, recettes, lieux d'intérêt, vocabulaire narratif ;
- des **règles de lecture** : sa taxonomie folk / organisée / culte / hérésie, ses formes d'État, ses types culturels, réécrits comme classifieurs ;
- des **heuristiques de bootstrap** : préférences de foyer, coûts d'expansion.

Les sections 3.3 et 3.4 sont à lire avec ce principe.

## 2. Ce qu'Azgaar fournit et ce qui manque à Chronos

Fantasy Map Generator construit une carte finie sur un graphe de cellules, puis génère successivement climat, biomes, cultures, établissements, États, routes, religions, provinces, marchés, production et couches narratives. Les données sont conservées principalement dans `grid` et `pack`. Le pipeline est une séquence ordonnée ; il ne s'agit pas d'un ordonnanceur multiéchelle à dépendances déclarées. [Pipeline][F1]

Chronos dispose déjà d'événements annuels avec causes et effets, de populations, de puissances, d'organisations, de personnes, de trajets et d'une frise. Il importe une fixture de 52 sites, 71 routes et 1 846 personnes, avec six modèles institutionnels. En revanche, il ne possède pas encore les entités sociales récursives, le bootstrap hiérarchique et le raffinement paresseux prévus dans les paliers P0–P5. Sa boucle économique utilise surtout vivres, richesse conventionnelle, stabilité et événements de foire ; elle n'est pas un système de marchandises. [Méthode][C1], [code de la chronique][C6]

| Domaine | Ce que fait Azgaar à la révision examinée | Écart dans Chronos actuel | À récupérer |
| --- | --- | --- | --- |
| Emblèmes | Compose blasons, formes, couleurs, partitions et charges ; affiche et exporte les emblèmes | Couleurs de puissances, sans identité héraldique structurée | Catalogues, recette déclarative et règles de composition |
| Lieux d'intérêt | Génère marqueurs et légendes : ruines, auberges, grottes, sources, nécropoles, rencontres et autres particularités | Pas de modèle générique de lieu d'intérêt relié à un événement, un établissement et une organisation | Tables de contenu, critères de contexte, noms et descriptions |
| Religions | Cultes populaires, religions organisées, cultes et hérésies ; divinités, origines et expansion spatiale | Pas de croyances, d'appartenances religieuses ou de schismes simulés ; les modèles Pecking Order ne suffisent pas à les créer | Vocabulaire, doctrines, divinités, règles de nommage, heuristiques de diffusion. La taxonomie devient une lecture du méta-modèle d'idées (§1.1) |
| Cultures et territoires | Préférences géographiques, types culturels, expansion, États, provinces et relations diplomatiques | Culture surtout utilisée comme clé de noms ; puissances simples, sans provinces ni graphe social complet | Contraintes d'implantation et heuristiques de bootstrap politique. Types culturels et formes d'État deviennent des étiquettes dérivées (§1.1) |
| Économie | Biens bruts et manufacturés, recettes, besoins, marchés, prix, production, échanges, taxes, tarifs et embargos | Six indices de ressources par site ; aucun catalogue de biens ni de recettes, aucun registre de transactions | Ontologie des biens, recettes et mécanismes économiques à redéfinir dans le temps |
| Voyages | Itinéraires avec étapes, transports, vitesses, séjours, durées et habillage narratif | Distances et temps de trajet existent, sans voyage nommé et structuré avec étapes | Modèle d'étapes, calcul des durées et tables narratives |
| Noms | Bases éditables et génération Markov de noms inventés | Pools Namebench figés et listes de repli, sans petit générateur productif intégré | Bases utiles et principe de génération légère, en complément de Namebench |
| Carte et édition | Labels territoriaux, couches, frontières, symboles, notes, styles et exports SVG/PNG/GeoJSON | Carte canvas et inspecteur de lecture ; pas d'atelier cartographique comparable | Algorithmes de placement, règles d'affichage, parcours d'auteur et formats d'export |

Les paragraphes suivants précisent ce que ces reprises impliquent. La présence d'un système chez Azgaar ne prouve pas qu'il soit directement portable, historiquement calibré ou adapté à une simulation de plusieurs siècles.

## 3. Les reprises par domaine

### 3.1. Emblèmes pour les entités et les lignées

Le [générateur d'emblèmes][F2] s'appuie sur des données séparées : charges, partitions, pièces, positions, formes et couleurs. Les recettes et données constituent une reprise plus isolable que les contrôleurs de l'application entière. Le générateur et son rendu utilisent néanmoins des services et types FMG : leur extraction demande un adaptateur.

**Proposition Chronos :** une spécification `EmblemSpec` porte un identifiant, une recette versionnée et une attribution à une entité. La composition est décidée par une clé sémantique stable ; le navigateur dessine la recette déjà choisie. Un changement d'armes, une union de maisons ou l'adoption d'un sceau peuvent devenir des épisodes datés. Une succession ordinaire ne régénère pas les armes de la maison.

Première tranche possible sans attendre P0 : un pack figé d'emblèmes est attribué aux puissances et organisations existantes. Le rattachement à des maisons et à des offices sera ensuite migré vers `SocialEntity`. Les données et SVG nécessaires sont embarqués pour préserver le fonctionnement hors ligne.

### 3.2. Lieux d'intérêt et descriptions liées aux faits

Le [générateur de marqueurs][F3] et les [tables de vocabulaire][F4] couvrent déjà de nombreux lieux et incidents. Ils prennent du contexte cartographique ; certains détails d'auberges utilisent aussi les marchandises et la production locales. C'est une source de contenu plus riche qu'une liste d'icônes. Toutefois, certaines descriptions tirent une date au hasard : cette méthode n'est pas un passé causal.

**À reprendre :** archétypes, conditions d'accueil, vocabulaire et variantes descriptives. **À remplacer :** les dates, protagonistes, destructions et causes inventés indépendamment de la chronique.

Une ruine issue d'une guerre référence le site et l'événement de destruction ; son état à une année donnée vient du canon. Une auberge est liée à un établissement et éventuellement à une organisation et un bâtiment Magna Urbis. Elle ne crée pas une population supplémentaire : ses employés et clients sont des membres ou résidents déjà comptés. Une grotte naturelle peut provenir du milieu ; son occupation, sa découverte ou sa réputation sont des faits historiques distincts.

Séparer trois objets : **le lieu physique**, **le fait attesté dans le canon**, **le récit ou la rumeur qui le décrit**. Une légende peut être incertaine ; sa présence ne rend pas vrai son contenu. Un récit de découverte ne change pas rétroactivement la date de création de ce qu'il décrit.

### 3.3. Religions et institutions religieuses

Le [module des religions][F5] produit des formes de croyance, des noms, des divinités, des centres, une expansion et des origines. Il distingue notamment `Folk`, `Organized`, `Cult` et `Heresy`. Ses règles utilisent les cultures, le terrain et les routes pour répartir une religion sur la carte. Cette construction fournit un état spatial ; elle n'est pas une histoire complète des conversions, des dogmes et des schismes.

Chronos peut reprendre cette taxonomie et ces heuristiques pour initialiser un monde. Il doit ensuite distinguer **la croyance**, **la communauté des fidèles** et **l'organisation qui l'administre** : une même croyance peut avoir plusieurs clergés ou ordres. Pecking Order fournit les structures de ces organisations ; Chronos choisit leurs membres, possessions, mandats et événements.

Les affiliations sont datées et peuvent coexister. Un schisme crée une nouvelle identité et une relation d'origine avec sa cause ; il ne renomme pas tous les anciens fidèles. Le détail local hérite des effectifs ou parts réservés par la région. Les adhésions religieuses et politiques ne sont pas des populations disjointes que l'on pourrait additionner.

Pour un univers fantastique, préciser séparément si une divinité est un objet de croyance ou un acteur réel. Le vocabulaire d'Azgaar ne décide pas cette règle d'univers.

**Lecture méta-modèle (§1.1).** Cette séparation est un cas particulier d'un mécanisme commun :
- la croyance est une `Idea` de dimension *croyance* ;
- la communauté est la distribution d'adhésion des cohortes ;
- l'organisation est une entité liée à l'idée par `ADMINISTERS` ;
- la religion d'État est un `ENDORSES` daté.

La diffusion, les conversions, les schismes (`DERIVED_FROM`) et les syncrétismes passent par le même moteur que les langues, les coutumes et les savoirs. Les catégories `Folk`, `Organized`, `Cult` et `Heresy` ne sont pas des types stockés : ce sont des classifieurs qui lisent cet état.

### 3.4. Cultures États provinces et diplomatie

Les [cultures][F6] ont des préférences environnementales et des types comme fluvial, lacustre, maritime, nomade ou montagnard. Les [États][F7] et [provinces][F8] apportent des mécanismes de regroupement territorial et de subdivision. La diplomatie possède un vocabulaire de relations, dont alliance, rivalité, dépendance et hostilité.

Ces règles sont utiles au **bootstrap P1** : définir des foyers et une répartition initiale cohérente avec le milieu et les trajets, en fonction du scénario colonisation, jeune civilisation ou vieille civilisation. Leurs champs d'expansion sont des heuristiques de génération, pas une chronologie à réutiliser comme si chaque conquête avait été simulée.

Adapter les méthodes aux cellules stables de Neural Earth, avec une correspondance explicite des cultures entre Magna Urbis, Namebench et Pecking Order. Conserver des catégories distinctes pour culture, langue, croyance, citoyenneté et pouvoir politique. Le contrôle d'un territoire ne transforme pas automatiquement la culture de tous ses habitants.

**Lecture méta-modèle (§1.1).**
- Le **type culturel** (fluvial, nomade, naval…) n'est pas assigné. Il se lit dans les parts d'activités et le milieu de l'empreinte ; les préférences de foyer d'Azgaar ne servent qu'à semer le bootstrap.
- La **forme d'État** (monarchie, théocratie, république) se lit dans la structure de pouvoir Pecking Order. Le **rang** (royaume, empire) se lit dans la taille, la diversité des peuples et la profondeur des vassalités. Une révolution change la structure, et l'étiquette suit.
- La **culture** elle-même est une catégorie lue dans les distributions d'idées partagées (langue, coutumes, croyances), ce qui permet l'ethnogenèse.

Les relations qui ont des effets deviennent des relations datées de Chronos : traité d'alliance, vassalité, embargo ou rupture. Un différend historique peut être relié à un événement ; une proximité culturelle peut rester une affinité calculée, conformément à la conception sociale. Un récit de campagne chez Azgaar n'est pas importé comme une preuve des pertes et changements de contrôle correspondants.

### 3.5. Marchandises production marchés et fiscalité

La reprise économique est substantielle : le [catalogue des biens][F9] porte des unités, catégories de besoins, productions par biome, modificateurs et recettes alternatives. La [production][F10] transforme les ressources en produits ; les [marchés][F11] regroupent des territoires, gèrent stocks et prix et répartissent des surplus. La [fiscalité][F12] comprend ventes, prélèvements, tarifs et embargos.

Chronos ne possède aujourd'hui ni ces biens ni ces échanges : sa fonction `trade` actualise une richesse conventionnelle et peut produire une foire avec des gains. Importer un catalogue et construire un registre économique serait donc une évolution du moteur, pas une simple amélioration des descriptions. [Code actuel][C6]

**Commencer par le catalogue et les recettes.** Traduire les identifiants numériques en clés sémantiques stables et établir une table entre ressources naturelles et biens : potentiel forestier → accès à du bois ; céréales → récolte ; minerai → production extractive ; ingrédients → produits transformés. Le gisement naturel, son exploitation et le bien transportable restent distincts. Le chantier ressources décrit un prototype WIP séparé ; il ne constitue pas encore ce registre économique Chronos. [Suivi R0–R5][C3]

Plusieurs règles demandent une adaptation explicite :

- Azgaar attribue un bonus de ressource à une cellule et utilise des expressions JavaScript pour les distributions. Les champs et agrégats Neural Earth doivent fournir les ressources ; traduire les règles utiles en données typées, sans évaluateur JavaScript dans le cœur Rust.
- Les identifiants de biomes, hauteurs de grille et unités des biens ne sont pas ceux de Neural Earth. Les coefficients de production et de demande sont des paramètres du modèle ; leur import ne constitue pas une calibration historique.
- La production actuelle est **une passe**, sans inventaire de ville reporté au cycle suivant. Les marchés ont des stocks pendant ce calcul, mais il manque la transition historique persistante dont Chronos a besoin.
- Le commerce inter-marchés utilise notamment un coût de distance directe de type octile entre centres. Remplacer ce coût par les trajets canoniques, franchissements, ports, saisons et capacités réellement pris en charge ; une route fermée doit affecter les échanges.
- Le prélèvement par population crédite une trésorerie sans débit correspondant ; les marchés n'ont pas de trésorerie propre. Pour des comptes conservés dans Chronos, déclarer les contreparties, les sources externes et les règles de création monétaire ; ne pas recopier ces écritures comme une comptabilité conservatrice.

**Proposition :** un petit registre daté de stocks, capacités, transferts et droits, puis une fonction de transition par bloc temporel. Choisir d'abord quelques biens et recettes, des unités et des arrondis explicites. Les droits d'extraction, taxes et monopoles appartiennent aux entités et institutions ; les flux sont enregistrés une seule fois avec leurs contreparties. Les prix peuvent rester des indices conventionnels tant qu'aucun modèle monétaire n'est défini.

La famine, l'ouverture d'une mine ou un embargo doivent lire et modifier cet état. Ils ne doivent pas coexister avec une seconde économie qui applique encore les mêmes gains ou pertes par une autre formule. Un village peu détaillé peut recevoir un budget agrégé ; son zoom doit répartir ce budget sans créer une seconde production.

### 3.6. Voyages et mobilité historique

Les [voyages][F13] possèdent segments, transports, vitesses, séjours et durées. Le calcul distingue heures de déplacement et temps calendaire ; le [générateur narratif][F14] ajoute types de voyageurs, étapes et motifs. Chronos peut reprendre ce vocabulaire et le principe des segments pour pèlerinages, caravanes, migrations, messagers et déplacements d'acteurs.

Le graphe de trajets Chronos et la géométrie Magna Urbis restent les références. Un voyage historique porte départ, arrivée, participants et cause ; l'arrivée ne précède pas la durée calculée. Le passage des heures d'Azgaar aux années du prototype doit être une conversion documentée, avec une politique d'arrondi. Les personnes importées restent soumises à leurs dates de vie et à leur fenêtre d'observation.

Une trajectoire affichée peut détailler un déplacement macro déjà réservé. Si elle doit déclencher une bataille ou changer un traité, cet effet est décidé au niveau canonique propriétaire ; découvrir une étape au zoom ne déclenche pas une nouvelle guerre.

### 3.7. Noms cartographie et édition

Le [générateur de noms][F15] construit des chaînes Markov à partir de bases éditables. Il peut compléter les pools figés de Namebench pour un profil léger et des noms inventés. Reprendre le vocabulaire utile et porter une génération à clés stables ; l'appel aux fonctions aléatoires globales de FMG ne satisfait pas le contrat d'ordre de requêtes de Chronos.

Les [labels territoriaux][F16], les couches, les notes et les [exports][F17] apportent des solutions concrètes de lisibilité et d'édition : placement des noms, formes courbes, frontière, symboles et sorties SVG/PNG/GeoJSON. Il s'agit d'une couche de présentation d'un monde à une date donnée. Elle ne doit tirer ni noms ni événements ; masquer une couche ne retire pas les faits du monde.

Conserver une spécification graphique distincte du canon historique. Si un export affirme représenter l'année Y, il doit porter les géométries et relations de Y, pas la dernière carte calculée. Les notes d'auteur peuvent devenir des contraintes versionnées, mais une annotation libre n'est pas automatiquement une règle du moteur. L'interface actuelle de Chronos est en lecture seule et n'invente aucun fait ; l'ajout d'édition nécessite un contrat de commandes explicite. [Interface actuelle][C5]

## 4. Trois façons complémentaires de l'intégrer

### 4.1. Extraire des packs de contenu pendant la préparation

**À privilégier pour commencer.** Un outil de préparation lit une révision épinglée de FMG et exporte les catalogues retenus en JSON neutre : emblèmes, lieux d'intérêt, biens, recettes, catégories religieuses et vocabulaire. L'extraction de constantes TypeScript doit être contrôlée ; les dépendances au navigateur sont retirées du chemin de préparation. Cela ne suppose pas que ces données soient toutes déjà dans des fichiers JSON autonomes.

Le pack proposé déclare : `pack_id`, `schema_version`, `upstream_revision`, liste des fichiers d'origine, empreintes SHA-256, version de transformation, licence et couverture. Il utilise des clés stables, pas les positions dans les tableaux FMG. Les données sont triées et validées à la construction. Le cœur peut recevoir le catalogue normalisé, ou une ressource embarquée ; il n'effectue aucune E/S.

Statuts de provenance : catalogue copié = `imported` ; résultat calculé par adaptation = `derived` ; règle inventée pour Chronos = `synthetic` ; source requise manquante = erreur explicite ou repli annoncé `unavailable`. Les empreintes nécessaires entrent dans les sources du canon selon le mécanisme existant. Ne pas présenter les résultats d'un port comme des objets importés d'une carte Azgaar.

### 4.2. Porter les règles métier utiles vers des fonctions Rust pures

**À privilégier pour les décisions canoniques.** Une fonction reçoit un contexte versionné et rend des propositions, validées avant application. Elle ne lit ni `pack`, ni `grid`, ni le DOM, ni une horloge. Tout tirage utilise `keys::key/unit/range_i` ; les parcours et départages sont canoniques. [Contrat actuel du hasard][C7]

Exemples de contrats proposés, à spécifier avant leur implémentation :

| Fonction proposée | Entrées | Sorties et autorité |
| --- | --- | --- |
| `assign_emblem` | Entité, épisode, catalogue, clé sémantique | Recette attribuée une fois ; le renderer ne la régénère pas |
| `propose_points_of_interest` | Scope, bloc, budgets, milieu et événements déjà réservés | Lieux et rattachements, sans modifier le canon parent |
| `bootstrap_ideas` | Foyers, population, graphe de trajets, catalogue d'idées, maturité `m` | Distributions d'adhésion initiales par dimension, organisations et appuis initiaux ; progression ultérieure par le moteur générique et des événements |
| `classify` | État de l'entité à une date, version des règles | Étiquettes (religion d'État, forme, rang, type culturel…), jamais stockées |
| `advance_economy` | État initial du bloc, capacités, recettes, demande et règles | État final et écritures réconciliables ; causes et effets explicites |
| `project_journey` | Déplacement canonique, routes, transports et calendrier | Étapes et durées compatibles avec le fait macro |

La simulation historique appartient à P2 et le raffinement à P3. Il faut donc réserver en amont ce qui a une portée régionale ou mondiale, puis utiliser les catalogues Azgaar pour détailler. Le port ne doit pas figer une nouvelle boucle annuelle par chaque maison, marché ou habitant sur toute la planète.

### 4.3. Adapter les outils cartographiques dans le client

**À privilégier pour les labels, symboles et exports.** Un adaptateur reçoit une projection en lecture seule de `World` à la date choisie, puis la convertit en primitives de dessin. Extraire les méthodes utiles sans simuler un faux `pack` global pour faire fonctionner toute l'application FMG.

Le renderer d'emblèmes et les éventuels symboles SVG doivent utiliser un catalogue contrôlé et embarqué. L'import de notes HTML FMG demande une conversion vers le sous-ensemble accepté par Chronos ; il ne justifie pas d'insérer du HTML arbitraire dans l'inspecteur. Les préférences visuelles peuvent varier sans changer `canon_id` ; une modification d'affiliation ou de contrôle territorial est, elle, une commande d'auteur ou un événement.

### 4.4. Option séparée pour importer une carte Azgaar

Un import d'instantané est utile, mais distinct des trois reprises précédentes. Le format `.map` courant utilise plusieurs enregistrements sérialisés ; le « futur modèle JSON » décrit dans la documentation n'est pas une garantie du format sauvegardé aujourd'hui. Commencer par des exports JSON/GeoJSON documentés ou un convertisseur explicitement versionné. [Sauvegarde amont][F18]

L'adaptateur normalise sites, territoires et relations dans une **nouvelle entrée de bootstrap**, avec emprise, unités, date initiale et correspondances d'identités explicites. Il ne peut pas injecter arbitrairement un monde FMG via `POST /api/import` : cet endpoint Chronos valide aujourd'hui un monde Chronos par rejeu. Un instantané externe ne prouve pas les siècles d'histoire qui l'ont produit. Les domaines non importables sont signalés, et le raccord à Neural Earth reste une opération déclarée.

## 5. Modèle de données et invariants à préserver

Le modèle cible peut accueillir des spécifications de contenu et des épisodes datés, sans les confondre : `GoodSpec`, `RecipeSpec`, `IdeaSpec` (générique, la dimension *croyance* remplaçant un `BeliefSpec` dédié), `EmblemSpec`, `PointOfInterest`, `Journey`, `EconomicState` et liens vers `SocialEntity`. Ces noms sont **des propositions**, absentes du contrat JSON actuel. Les petits ajouts avant P0 doivent prévoir leur migration plutôt que créer des identités concurrentes.

Principes communs :

1. **Identité stable.** Un lieu, une croyance ou un bien garde son ID quand on ouvre une autre région ; les versions de catalogue et correspondances sont explicites.
2. **Propriétaire unique.** Chaque fait et chaque budget ont un scope, un bloc et un slot. Un temple, son clergé et ses fidèles sont reliés, sans dupliquer le lieu ni compter plusieurs fois les habitants.
3. **Validité temporelle.** Fondation, existence, fermeture, occupation et destruction sont distinctes. Le contrat v1 comporte des fins d'événement inclusives ; tout intervalle `[début, fin)` proposé doit avoir une conversion explicite, sans changer silencieusement les dates actuelles.
4. **Causalité.** Un récit cite ses faits ; les causes d'événements respectent l'ordre canonique. Les vies et filiations importées de Lineage Lab ne sont pas réécrites pour rendre une description plus intéressante.
5. **Budgets conservés.** Le détail répartit les populations, marchandises et droits réservés par le macro. Les conventions de création, consommation et perte sont déclarées ; un zoom ne produit pas de nouvelles richesses.
6. **Éditions préservées.** Nom et emblème acceptés, ville épinglée et récit d'auteur sont des données ou contraintes versionnées. Une nouvelle recette propose une révision ; elle n'efface pas ces choix.
7. **Rejeu et provenance.** Les catalogues, règles et entrées qui influencent le canon participent à son identité. Toute évolution de sémantique change `ALGORITHM_VERSION` ; tout changement de contrat publié nécessite la politique de schéma et de migration correspondante.

## 6. Ordre de réalisation proposé

Les lots ci-dessous complètent les paliers existants ; ils ne déclarent aucun travail métier terminé.

| Lot | Travail | Dépendance | Preuve de sortie attendue |
| --- | --- | --- | --- |
| A0 | Épingler les sources, extraire un petit pack et ses correspondances, conserver les notices | Aucune modification du moteur requise | Deux extractions identiques ; chaque entrée remonte à un fichier amont |
| A1 | Attribuer des emblèmes et compléter les noms ; renderer déterministe | A0 ; adaptateurs temporaires sur les types actuels | Même identité avant/après rechargement, sans aléa dans le client |
| A2 | Ajouter quelques lieux d'intérêt et descriptions dérivés des événements existants | A0 ; contrat d'identité et de date | Une destruction et une reconstruction produisent les états attendus du même lieu, sans nouveau décès ou habitant |
| A3 | Méta-modèle d'idées : adhésions, appuis, moteur de diffusion et classifieurs d'étiquettes (§1.1). Puis semer croyances, langues et coutumes, territoires et organisations depuis les catalogues Azgaar ; dater affiliations et relations | P0, P1 ; progression P2 et raffinement P3 pour les évolutions | Même répartition canonique quel que soit l'ordre d'ouverture ; aucun double comptage |
| A4 | Introduire quelques biens et recettes, puis stocks, droits et échanges | Contrat ressources R2, consommation R5, modèle social et progression | Bilan de matière réconcilié ; famine, mine et embargo ont des effets traçables |
| A5 | Voyages structurés, couches cartographiques et exports datés | Routes canoniques et projections temporelles | Étapes compatibles avec l'arrivée ; l'export correspond à l'année affichée |

**Premier parcours recommandé :** sur un monde de référence, attribuer un emblème à une maison ou une puissance ; matérialiser comme ruine un village détruit ; décrire une auberge à partir du contexte local ; rattacher une foire existante à la ligue marchande. Les faits antérieurs restent ceux de Chronos. Introduire ensuite un petit exemple économique, avec consommation et contreparties, avant de porter le catalogue entier.

Le choix entre carte statique importée et bootstrap généré, la granularité des échanges et le niveau de détail des croyances restent à trancher. Le travail ne dépend pas d'une migration complète de l'interface ou d'un monorepo. Les mesures de coûts et de mémoire devront porter sur chaque lot réalisé ; aucun chiffre de performance n'est déduit de la richesse de FMG.

## 7. Validation de l'intégration future

Ces contrôles sont des **critères proposés**, pas des tests exécutés dans cet audit :

- Rejeu : mêmes entrées, catalogues et versions → même projection canonique et même export.
- Ordre de visite : A→B, B→A, accès direct et zoom progressif donnent les mêmes IDs, épisodes et allocations.
- Temps : un lieu détruit n'est pas affiché comme ruine avant l'événement ; un schisme ne modifie pas les affiliations anciennes ; un titulaire n'agit pas hors de sa vie ou de sa couverture importée.
- Économie : stocks non négatifs, recettes et pertes explicites, transferts avec contreparties, demande non satisfaite visible ; un prix conventionnel n'est pas présenté comme une mesure historique.
- Mobilité : une route fermée ou un embargo contraint le flux ; un voyageur n'arrive pas avant la durée canonique.
- Population : fidèles, guildes, employés et foyers sont des appartenances à des personnes ou cohortes comptées une seule fois.
- Présentation : filtres, zoom, cache et export n'altèrent aucun fait ; aucune donnée future ne fuit dans une carte ancienne.
- Distribution : ouverture hors ligne sans checkout FMG, réseau externe ou Node ; `chronos-core` reste sans E/S et compatible WASM.

## 8. Sources et traçabilité des reprises

Le [fichier de licence amont][F19] porte la licence MIT. Toute extraction doit conserver la notice et un manifeste des fichiers repris. Identifier séparément la provenance des images, polices, symboles et jeux de données utilisés, plutôt que supposer que la seule licence du code décrit tous les médias. Aucun asset ou code amont n'est recopié dans Chronos par cet audit.

| Référence | Pièce examinée | Usage dans cette proposition |
| --- | --- | --- |
| [F1][F1] | Pipeline FMG | Dépendances, séquence globale et limites de la reprise directe |
| [F2][F2] | Emblèmes | Catalogues et recettes héraldiques |
| [F3][F3], [F4][F4] | Marqueurs et vocabulaire | Lieux d'intérêt et contexte narratif |
| [F5][F5], [F6][F6] | Religions et cultures | Croyances, origines et diffusion spatiale |
| [F7][F7], [F8][F8] | États et provinces | Bootstrap territorial et relations |
| [F9][F9] | Biens | Catalogue, catégories, distributions et recettes |
| [F10][F10], [F11][F11], [F12][F12] | Production, marchés, fiscalité | Cycle économique et limites comptables |
| [F13][F13], [F14][F14] | Voyages et récits | Segments, temps calendaire et vocabulaire |
| [F15][F15], [F16][F16], [F17][F17] | Noms, labels, exports | Génération légère et projection cartographique |
| [F18][F18], [F20][F20] | Sauvegarde et modèle de données | Format courant, indices de cellules et conversions |
| [C1][C1], [C2][C2], [C5][C5] | Méthode, fixture et interface Chronos | État effectivement livré |
| [C3][C3], [C4][C4] | Suivi et conception sociale | Responsabilités, P0–P5 et articulation avec les ressources |
| [C6][C6], [C7][C7], [C8][C8] | Chronique, clés et types Chronos | Comportements économiques, déterminisme et contrat actuel |

[F0]: https://github.com/Azgaar/Fantasy-Map-Generator/tree/67d555b83f0b2f40ae50b03c9bbfd018175acc6d
[F1]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/docs/architecture/generation-pipeline.md
[F2]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/src/model/political/emblems-generator.ts
[F3]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/src/model/story/markers-generator.ts
[F4]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/src/data/markers.ts
[F5]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/src/model/society/religions-generator.ts
[F6]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/src/model/society/cultures-generator.ts
[F7]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/src/model/political/states-generator.ts
[F8]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/src/model/political/provinces-generator.ts
[F9]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/src/model/economy/goods-generator.ts
[F10]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/docs/domain/production_schema.md
[F11]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/docs/domain/trade_schema.md
[F12]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/docs/domain/taxes.md
[F13]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/src/model/story/journeys/journeys-generator.ts
[F14]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/src/model/story/journeys/journey-story.ts
[F15]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/src/model/society/names-generator.ts
[F16]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/src/renderers/labels/fit-state-label.ts
[F17]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/src/services/io/export.ts
[F18]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/src/services/io/save.ts
[F19]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/LICENSE
[F20]: https://github.com/Azgaar/Fantasy-Map-Generator/blob/67d555b83f0b2f40ae50b03c9bbfd018175acc6d/docs/architecture/data-model.md
[C1]: methods/chronology.md
[C2]: data-sources.md
[C3]: suivi-implementation.md
[C4]: reviews/vision-entites-sociales.md
[C5]: interface.md
[C6]: ../crates/chronos-core/src/generate.rs
[C7]: ../crates/chronos-core/src/keys.rs
[C8]: ../crates/chronos-core/src/model.rs
