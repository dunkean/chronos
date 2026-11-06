# Chronos et la simulation de groupes sociaux multi-échelles

Document de discussion — 6 novembre 2026.

[Retour sur tes commentaires](retour-commentaires-groupes-sociaux.md) — groupes informels, recrutement, quotas, monde ouvert, cultures et datavisualisation.

[Spécification détaillée pour l’implémentation](design/chronos-social-v2/README.md) — synthèse des décisions issues des deux tours de commentaires. Cette page conserve la discussion initiale.

Historique de discussion : l’expérience d’émergence ascendante proposée plus bas a été remplacée par une priorité à la causalité descendante et aux réalisations statistiques. Les recouvrements de groupes restent requis. Les commentaires sont conservés.

Cette orientation me paraît être le bon rôle pour Chronos : simuler comment les collectifs se forment, interagissent et transforment le monde au fil du temps. Lineage Lab apporte les personnes et les parentés ; Namebench apporte les noms ; Chronos pourrait donner une histoire à leurs appartenances : familles, communautés, métiers, religions, factions, institutions, États.

Après lecture du code et des documents, cette vision est déjà largement formulée dans l’[étude v3](../conception_generateur_histoire_multiechelle_v3.md#4a-modèle-générique-des-entités-sociales--nouveau-cœur-du-moteur) et la [revue « entités sociales »](reviews/vision-entites-sociales.md). **L’écart actuel est surtout dans l’implémentation**, avec néanmoins un choix de conception à préciser : quelle liberté donner à l’émergence locale ?

Les propositions ci-dessous sont distinctes des capacités déjà implémentées. Les zones de commentaires sont libres et peuvent être agrandies.

## 1. Un réseau de groupes qui se chevauchent

Une personne appartient simultanément à une famille, un village, une guilde, un culte et une clientèle politique. Ces appartenances ont des dates, des rôles et des obligations différents. Une guilde peut réunir plusieurs ateliers ; un ordre religieux, plusieurs communautés locales ; une coalition, plusieurs organisations indépendantes.

Il faut donc distinguer composition, adhésion, résidence, autorité et propriété. Habiter une ville ne signifie pas appartenir à son gouvernement ; partager une religion ne signifie pas obéir à une même organisation. Le support géographique peut être hiérarchique, tandis que le réseau social traverse les régions et les frontières.

Je distinguerais aussi trois formes de collectif :

- **Une organisation capable d’agir** : conseil, maison marchande, monastère, faction.
- **Une communauté aux pratiques partagées** : village, diaspora, milieu professionnel, dont l’action collective dépend d’une coordination.
- **Une catégorie descriptive** : population d’une langue ou d’une tranche d’âge, qui ne possède pas automatiquement une volonté commune.

Cette distinction évite qu’une culture entière « décide » comme un personnage. Les décisions viennent d’acteurs et de procédures ; les transformations collectives peuvent aussi résulter de milliers de comportements sans coordination.

**Commentaires**

https://github.com/dunkean/AI_RPG_Generator: regarde ce projet à moi le concept est discuté, et étend ce que tu proposes en organisation sociales (les amis par exemple) et les relations possibles entre entités sociales.  

<br>
<br>

## 2. Des mécanismes pour faire évoluer les groupes

Pour produire une évolution intéressante, les groupes doivent avoir des mécanismes : recrutement, départs, transmission des pratiques, accès aux ressources, coopération, concurrence, légitimité, conflits internes, scissions et fusions. Quelques règles bien couplées suffiraient au départ.

Par exemple : une taxe fragilise les ateliers ; une guilde coordonne leur opposition ; une faction municipale la soutient ; le pouvoir accorde un privilège ; des artisans migrent vers les villes où ce privilège s’applique. L’événement devient alors la trace d’une transformation sociale explicable.

**Commentaires**

oui

<br>
<br>

## 3. Les influences dans l’espace et dans le temps

Un groupe n’a pas une seule « zone d’influence » : il a des membres quelque part, des ressources ailleurs, une autorité reconnue sur certains lieux et une réputation qui circule plus loin. Son siège peut rester fixe pendant que son réseau se déplace.

Je représenterais quelques canaux distincts — économique, politique, culturel, religieux, militaire — propagés par les routes, les migrations et les relations sociales. Chaque canal aurait une portée, un délai et une persistance propres. Une conquête peut changer rapidement le contrôle politique ; les pratiques linguistiques ou les fidélités familiales évoluent autrement. Les affiliations multiples permettent précisément de transmettre une influence entre ces réseaux.

La cohésion interne compte également : deux groupes partageant beaucoup de membres peuvent coopérer, mais aussi provoquer des conflits de loyauté. Pour conserver cela à grande échelle, de simples effectifs par groupe ne suffisent pas toujours : il faut préserver certains recouvrements, par exemple la part des marchands appartenant à un culte donné. Additionner tous les membres des groupes compterait plusieurs fois les mêmes habitants.

**Commentaires**

oui

<br>
<br>

## 4. Ce que Chronos possède aujourd’hui

Le moteur fournit une base utile, mais pas encore cette simulation.

| Domaine | Implémentation actuelle | Évolution nécessaire |
|---|---|---|
| Société | États, organisations, personnes, offices et successions | Affiliations multiples datées, relations entre groupes, structures internes évolutives |
| Dynamique | Boucle annuelle, taux et conditions locales, événements avec effets | Actions issues des ressources, intérêts, institutions et interactions des groupes |
| Espace | Sites, routes, temps de trajet, contrôle politique | Présences et influences superposées, réseaux historiques évolutifs |
| Cohérence | Déterminisme, provenance, effets exacts, causes, rejeu | Étendre ces garanties aux appartenances et transformations sociales |

Le [modèle Rust](../crates/chronos-core/src/model.rs) reste simple : une organisation comporte essentiellement un périmètre de sites et un office ; une personne n’a pas de liste d’appartenances. Certaines crises dépendent déjà de l’état du monde, mais il n’existe pas de moteur général de décisions collectives. Le raffinement à la demande est également absent. En revanche, le déterminisme et la comptabilité des effets sont des acquis à conserver. La [méthode implémentée](methods/chronology.md) précise ces capacités et leurs limites.

**Commentaires**

<!-- Ajoute ici tes remarques, objections ou précisions. -->

<br>
<br>

## 5. Les approches et les projets existants

Plusieurs familles de travaux correspondent à des morceaux du problème. Les réseaux sociaux multicouches décrivent les relations simultanées dans plusieurs contextes ; [Nunes et Antunes](https://arxiv.org/abs/1507.03826) abordent directement ces contextes sociaux chevauchants. La simulation multi-agents multiniveau traite les interactions entre individus et collectifs ; la [revue de Morvan](https://arxiv.org/abs/1205.0561) présente ce domaine, et [GAMA](https://gama-platform.org/wiki/next/Introduction) fournit des mécanismes de modélisation multiniveau et de composition.

Pour les histoires individuelles et communautaires, [Neighborly](https://github.com/ShiJbey/neighborly) est une référence concrète de simulation d’établissements orientée vers les récits émergents ; son dépôt est désormais archivé. Ces références donnent des modèles et des techniques à examiner. Elles ne démontrent pas à elles seules la faisabilité de toute la combinaison recherchée : planète, longue durée, affiliations croisées et détail cohérent au zoom.

**Commentaires**

<!-- Ajoute ici tes remarques, objections ou précisions. -->

<br>
<br>

## 6. La faisabilité et le couplage avec Lineage Lab

La faisabilité dépend surtout du niveau de détail simulé. Un prototype régional avec quelques centaines de groupes explicites paraît accessible. Une planète sur des millénaires exige de simuler les collectifs importants et des populations agrégées, puis de détailler certaines personnes et organisations. Le [mode hybride de Lineage Lab](../../lineage-lab/docs/genealogy-hybrid.md) constitue une base pertinente, avec ses approximations documentées.

Le couplage avec Lineage Lab demande toutefois une décision. Aujourd’hui, Chronos importe des vies déjà calculées et ne peut pas provoquer la mort d’une personne importée. Pour qu’une guerre modifie réellement les familles, les migrations ou les naissances, les deux moteurs doivent partager une progression temporelle et échanger des effets. Je conserverais Lineage Lab comme moteur démographique et chercherais d’abord un contrat de pilotage commun. Une réécriture en Rust n’est pas un préalable.

**Commentaires**

Je vois ca comme un probleme de tirage aléatoire ou de bagpack. En gros on génère autant de personnes que nécessaire dasn l'espace de la région (avec ville, etc) et l'extérieur "simulé" si on est pas à l'échelle planétaire.
Puis les groupes sociaux qui sont chiffrés en taille et localisation, recrutent les individus en fonction de leur stats (tirage aléatoire ou sélection par critères). Dans AI_RPG_generator ca veut dire que des familles ont des affinités métier par exemple. 
Mais en gros l'idée est de dire que la famille est un groupe social et que les autres groupes sociaux recrutent dans ce pool. Et lineage_lab doit produire des pools d'individus qui correspondent à grose échelles aux pools d'individus nécessaire dans chronos (les familles nobles, le tier état pouilleux, etc.)

<br>
<br>

## 7. L’émergence locale et le zoom déterministe

Le point le plus délicat est l’émergence locale face au zoom déterministe. La conception actuelle prévoit que les conséquences importantes soient réservées au niveau supérieur. Cela facilite une histoire stable, mais limite la possibilité qu’un conflit local imprévu transforme le monde.

Si l’on veut cette remontée causale, il faut simuler les processus concernés avant de figer l’histoire, ou accepter un recalcul versionné. Un détail découvert tardivement ne peut pas simultanément créer une conséquence mondiale imprévue et préserver tout le passé déjà établi.

Je commencerais donc par une expérience bornée : une région, quelques villes, cinq types de groupes et cent ans. Le prototype devrait montrer des affiliations croisées, une diffusion, une scission et une crise locale ayant une conséquence régionale. On vérifierait les causes, les effectifs et le rejeu, puis on mesurerait le coût. Cette expérience permettrait de choisir ensuite comment agréger la simulation et introduire le zoom, en sachant quels mécanismes sociaux doivent rester actifs pour produire le monde recherché.

**Commentaires**

Pour la question emergence, je vois 3 approches:
- comme lineage lab peux choisir des personnages importants, ils sont reliés à une emprise locale qui peut s'appuyer sur eux pour générer du contenu, on raffine le contenu à partir d'une graine globale quoi. 
- la generation globale peut laisser des blancs d'intrigue. En généralement localement les villages remplissent ces blancs, soit purement procéduralement (très dur) soit en mode "first come first served", le premier village généré s'approprie la part de chronologie, ou alors on fait des stats. En gros l'evenement général dit: "la guerre de 45-78 a ruiné 25% des villages de cette région" quand on zoom un village, il a un tirage pour savoir si il a été ruiné à cet époque. Ca veut dire un générateur de grain random parfaitement déterministe et indexé.
- on s'en fiche, chronos doit générer l'histoire "générale" n'importe où. Si on veut de la narration plus sérieuse, on travaillera par dessus l'existant. Ca veut dire on veut de la causalité descendante (et peut être statistique) et localement on se contente de generation procédurale)

Bref l'émergence n'est pas un objectif réel, plutot voir ca comme des stats.


<br>
<br>

## Commentaires généraux et décisions à reprendre

<!-- Ajoute ici les sujets transversaux, tes priorités et les décisions à discuter. -->
C'est un problème super complexe qui méritera de la dataviz au top pour évaluer la crédibilité, inclus le dans ta réflexion. Et inclus le monde ouvert et son traitement. Je vois comment consolider à l'echelle d'un monde, mais une région doit supposer des choses autour et ca complique. Enfin, j'aimerai que ca soit vraiment typé par les cultures dispo dans magna urbis.
<br>
<br>
<br>
