# Retour sur les commentaires concernant les groupes sociaux

6 novembre 2026 — propositions de conception, sans modification du moteur.

[Spécification détaillée issue des dernières remarques](design/chronos-social-v2/README.md). Le présent document conserve la discussion et les commentaires ; les arbitrages techniques de reprise figurent dans la spécification.

Historique de discussion : les dernières remarques ont remplacé l’import de vies indépendantes par le pilotage interne de Lineage Lab. Les frontières de la première livraison sont statistiques ; le suivi nominatif entre domaines est un lot ultérieur. Le fichier culturel TypeScript cité ici complète le catalogue natif Rust inventorié dans le design ; ces deux sources n’ont pas une couverture supposée identique.

**Tes commentaires précisent le rôle de Chronos : produire une histoire sociale statistiquement cohérente, puis en matérialiser le détail à la demande.** J’avais donné trop de poids à l’émergence ascendante et au besoin d’une simulation démographique couplée en continu. Ils deviennent des options. Le cœur serait un ensemble de distributions, d’identités durables, d’affectations et d’événements contraints par les niveaux supérieurs.

Ce retour complète le [document annoté](discussion-groupes-sociaux.md).

## 1. Étendre les collectifs et les relations

J’ai relu ton [étude sociale](https://github.com/dunkean/AI_RPG_Generator/blob/main/worldbuilder_social_engine_study.md), le [modèle Entity et Group](https://github.com/dunkean/AI_RPG_Generator/blob/main/archive/template.py), la [génération des groupes](https://github.com/dunkean/AI_RPG_Generator/blob/main/src/procedural/population.py) et le [recrutement des employés](https://github.com/dunkean/AI_RPG_Generator/blob/main/src/procedural/employees.py). Le code produit déjà familles, amis, collègues, alliés et groupes liés par un objectif ou un statut, puis recrute selon des préférences. C’est une base conceptuelle plus large que ma première présentation.

Je généraliserais ainsi les collectifs et leurs liens :

| Collectifs possibles | Relations à représenter |
|---|---|
| Famille, foyer, lignage, clan, maison | Parenté, union, adoption, appartenance, héritage |
| Cercle d’amis, bande, compagnons de voyage | Amitié, confiance, entraide, rivalité, passé commun |
| Atelier, ferme, guilde, compagnie, équipage | Emploi, apprentissage, association, fourniture, dette |
| Voisinage, communauté villageoise, diaspora | Résidence, origine, accueil, solidarité, représentation |
| Culte, confrérie, école, cercle savant | Adhésion, initiation, enseignement, patronage, opposition |
| Clientèle, faction, réseau criminel, coalition | Protection, allégeance, alliance, complicité, conflit |
| Conseil, institution, État, fédération | Composition, délégation, commandement, juridiction, tribut |

Une relation porterait ses dates, son sens, son intensité éventuelle et sa cause. Une amitié peut être asymétrique ; un cercle d’amis n’implique pas que tous ses membres soient également proches. La parenté calculable reste fournie par Lineage Lab.

**Commentaires**

parfait. Pour lineage lab en fait si nécessaire, pilote le, et on remplira ce qui reste en procédural, c'est aussi une solution.

## 2. Des populations disponibles et un recrutement sous contraintes

Ton idée est pertinente. Je parlerais surtout d’**affectation sous contraintes avec tirage pondéré** : plusieurs groupes peuvent recruter la même personne, avec certaines activités exclusives ou limitées par son temps disponible.

La préparation comporterait trois étapes : définir les besoins agrégés par lieu et période ; produire des familles et des personnes compatibles ; affecter les rôles. Les besoins doivent précéder la génération pour éviter de découvrir trop tard qu’une région manque de candidats nobles, d’adultes ou d’artisans. Ils portent sur des distributions conjointes plausibles, au-delà du seul nombre d’habitants.

Le recrutement combine des contraintes dures — vivant, âge, présence, capacité du rôle — et des préférences : compétence, métier familial, statut, proximité, liens préexistants. Une pénurie reste visible ; elle ne crée pas automatiquement un candidat supplémentaire.

[Lineage Lab](../../lineage-lab/src/lineage_lab/config.py) possède déjà des paramètres d’affinité de statut et de transmission du statut et de l’activité. Son [mode hybride](../../lineage-lab/docs/genealogy-hybrid.md) distingue personnes simulées et cohortes projetées. Produire exactement les populations requises par Chronos reste un contrat à développer, particulièrement pour les métiers et les statuts de l’arrière-plan.

La famille devient bien une entité sociale. Il faut distinguer foyer résidentiel, parenté et maison durable : leurs membres ne coïncident pas toujours. Le recrutement peut favoriser une famille entière sans imposer que tous ses membres exercent le même métier.

**Commentaires**

j'avais beaucoup travaillé la dessus dans le AI RPG generator. Je te laisse carte blanche, mais attention entre génération et simulation. Il faut du procédural si on veut scale up. 


## 3. Une population cohérente à travers les dates

Le remplissage d’un instantané est plus simple que celui d’une histoire. Une nouvelle consultation doit retrouver les mêmes personnes, leurs liens et leurs affiliations datées.

Je conserverais les vies déjà établies et affecterais les rôles parmi les personnes éligibles. Les crises générales pourraient d’abord toucher les cohortes agrégées ; les victimes nommées seraient choisies parmi les décès compatibles. Cela permet une première version sans pilotage démographique continu.

Cette formule a une limite explicite : elle ne permet pas qu’une crise tue librement un personnage dont la vie est déjà fixée. Si ce comportement devient nécessaire, on devra transmettre les contraintes historiques à Lineage Lab avant la fixation des vies, ou recalculer une branche du monde.

**Commentaires**

- De mon point de vue, il faut pas aborder le problème comme ca. La cohérence est assuré par un tirage identique pour arriver à l'info, il faut s'assurer de ca. Exemple, je veux le maire de ce village en XXXX. Pour le déterminer, on va générer la genealogie et l'histoire du village (réellement, en la simulant partiellement pour retier des branches mortes et en suivant les marqueurs de pop) et on selectionne toujours de la meme facon le meme individu. Mais à dire ca je me rend compte qu'il faut que chronos utilise lineage lab comme un outils interne. Il en a besoin. Et lineage lab doit être adapté aux évenements et autres, comme ca la génération sera conjointe et pas disjointe et beaucoup plus simple à gérer. non ? 

## 4. Tes trois approches du détail local sont compatibles

**Je retiens la causalité descendante comme objectif principal.** Les personnages importants servent d’ancrages ; les événements généraux portent des impacts distribués ; les détails locaux apportent leurs propres histoires dans l’espace disponible.

Pour « la guerre a ruiné 25 % des villages », deux contrats sont possibles :

- **Proportion attendue** : chaque village possède une probabilité de destruction. Un tirage déterministe rend sa réponse stable, mais le total peut s’écarter de 25 %.
- **Quota exact** : parmi les N villages concernés, on en sélectionne exactement K selon une règle d’arrondi explicite. Cela exige une population candidate canonique ou une allocation hiérarchique de quotas sur des effectifs connus.

Je réserverais les quotas exacts aux bilans annoncés comme tels et utiliserais les probabilités pour les tendances. Il faut préciser ce qui est compté : villages, habitants, surfaces ou richesse.

Le hasard indexé existe déjà dans [keys.rs](../crates/chronos-core/src/keys.rs). Il ne suffit toutefois pas à garantir les totaux ni la cohérence des candidats. Les clés doivent dépendre du lieu canonique, de l’événement et des versions, jamais de la fenêtre consultée.

Le « premier arrivé, premier servi » conviendrait à un scénario personnalisé enregistré. Pour le monde procédural commun, je réserverais les rôles à des emplacements stables, éventuellement encore abstraits : l’ordre des visites resterait sans effet.

**Commentaires**

Que les données réelles locales divergent des données attendues depuis le global (ie. 25% des villages) ne me dérange pas. Ca sera jamais dramatique. Et ca sera peut être plus rapide que le quota qui sauf cas particuliers est inutiles. Et donc garde les deux selon les cas et comme tu dis pour un scénar personnalisé c'est nécessaire.

## 5. Une région appartient à un monde ouvert

Je proposerais trois niveaux : **région détaillée, voisinage simplifié, extérieur agrégé**. Le voisinage décrit les principaux partenaires et passages ; l’extérieur fournit des populations, puissances, marchés et traditions sans détailler tous leurs établissements. La portée dépend des trajets et des réseaux : un port lointain peut compter davantage qu’une vallée voisine.

Les échanges seraient explicites : migrants, mariages, commerce, prélèvements, armées et influences. Une arrivée possède une provenance extérieure ; une sortie conserve une destination ou un réservoir identifié. Les flux reçus et envoyés doivent être comptabilisés pour éviter de créer des habitants à chaque bord de région.

Deux régions adjacentes doivent partager les mêmes identités et événements frontaliers. Leur contexte doit donc provenir d’un support mondial stable, compatible avec celui de Neural Earth. En agrandissant la zone, on détaille les engagements extérieurs déjà connus.

Sans ce contexte commun, une région autonome reste possible, mais sa cohérence dépend d’hypothèses de frontière. Sa fusion ultérieure avec une autre région peut exiger une réconciliation : une seed identique ne résout pas seule cette difficulté.

**Commentaires**

ca marche

## 6. Des cultures raccordées à Magna Urbis

Je reprendrais les identifiants culturels de Magna Urbis comme clés de catalogue partagées. Son [modèle de culture](../../magna-urbis/web/src/gen/urban/culture.ts) définit surtout des formes d’établissement, phases urbaines et bâtiments caractéristiques. [Pecking Order](../../pecking-order/README.md) reprend déjà des identifiants de ces cultures pour les titres institutionnels. Cela donne un raccord, mais pas encore toutes les règles sociales.

Chronos ajouterait des profils explicites : groupes fréquents, tailles, recrutement, parenté, transmission des activités, mobilité et pratiques institutionnelles. Ces profils permettraient plusieurs variantes et mélanges dans une même culture. Une forme urbaine ou un nom culturel ne permet pas, à lui seul, de déduire un régime social.

Culture, ascendance biologique, religion, citoyenneté et style de nommage resteraient des attributs distincts. Leurs associations seraient paramétrées dans les catalogues.

**Commentaires**

pleins de styles culturels sont niches (merfolk, zimbabwe tower, etc.) et d'autres plus large. Essaye d'aggréger en groupes équivalent culturellement pour l'histoire. MAis tu peux en garder plein quand meme.

## 7. La datavisualisation comme outil de conception

Elle doit arriver dès le premier prototype. Je prévoirais quatre vues liées à la même date et à la même sélection :

- **Carte** des appartenances, influences et flux, avec distinction entre détail connu et extérieur agrégé. > collé sur magna urbis et/ou plus tard neural earth
- **Chronologie** des groupes, de leurs effectifs et de leurs transformations, reliée aux causes. 
- **Réseau local filtrable** autour d’un groupe ou d’une personne, montrant affiliations et relations choisies. > je vois des graphes interactifs
- **Bilans** comparant cibles et résultats : recrutement, recouvrements, métiers, statuts, vacances et échanges extérieurs.

Chaque vue devrait permettre de comprendre pourquoi un résultat a été produit et d’identifier les approximations. La crédibilité s’évalue aussi sur plusieurs seeds, en comparant les distributions et la diversité entre cultures.

Je modifierais donc l’expérience proposée : une région avec un extérieur simplifié, deux cultures contrastées, des familles, groupes informels et institutions, puis cent ans de chronologie. Le critère de réussite serait la cohérence statistique, temporelle et spatiale, accompagnée d’un détail stable au zoom. La narration élaborée pourrait ensuite exploiter ce socle.

**Commentaires généraux**

<!-- Tes remarques sur les cultures, les vues et le périmètre du prototype. -->
