# Couverture des archétypes de groupes sociaux

Synthèse lisible de `archetype-coverage.v1.json`. Le JSON fait foi et l'outil `tools/check_catalogue_v2.py` le vérifie. Toutes les valeurs sont des réglages initiaux de worldbuilding (`initial_template_not_historical_calibration`), jamais des mesures historiques.

## Chiffres

- Archétypes : 121 au total, dont 53 anciens inchangés et 68 ajoutés.
- Classes (total, dont ajoutés) : `activity_unit` 25 (12), `association` 47 (35), `community` 20 (10), `friend_circle` 3 (1), `household_view` 4 (0), `institution` 14 (8), `lineage_view` 8 (2).
- Réglage de fantastique (total, dont ajoutés) : `historical` 107 (55), `low_fantasy` 9 (8), `high_fantasy` 5 (5).
- Systèmes Pecking Order : 242 décidés, dont 206 reliés à un archétype et 36 exclus avec raison.
- Grille fonctionnelle : 86 cases du cahier des charges, toutes couvertes ; 9 cases ajoutées et justifiées ; aucune case hors périmètre.
- `base_archetype_ids` : 12, avec `base_criminal_network` et `base_charitable_order`.

## Épinglage de la source

Pecking Order, commit `7dc27d07dc38a4dcc09dc4fd20de24bd4ac581d5` (« Audit organization-role coherence and replace broken select menus »). La liste des 242 identifiants est copiée dans la grille ; le vérificateur ne lit pas le dépôt vivant.

| Fichier | SHA-256 |
|---|---|
| `data/systems.json` | `4bcb67135baf40e0d96cd9b54d8b6ab6957124b7a5005f85b3f70c85503cae32` |
| `data/systems_civic_military.json` | `a8966d52004d488a7db9f87d167953bf0e407ced6466ea779ab757faaf0e9d7f` |
| `data/systems_specialist.json` | `9a5bd0639fdc36f8458fe22386da351d5345299cdb2b76079356a6d91d2aa1df` |
| `docs/CATALOG.md` | `c2f3724526900275ce894dbd60535585bed66ec576b0f7a7fb7242e4d1ef8a87` |

## Réglage de fantastique

`setting_profile` (`historical`, `low_fantasy`, `high_fantasy`) est un paramètre obligatoire du monde (§2.1). Un archétype est actif si son `setting` est inférieur ou égal à celui du monde. Les overlays restent le mécanisme d'activation local, mais ne réactivent jamais un archétype trop fantastique : un monde historique n'en active aucun.

- `historical` : 107 archétypes.
- `low_fantasy` : `magical_school`, `arcane_cabal`, `alchemists_lodge`, `druid_circle`, `monster_hunters_lodge`, `adventuring_company`, `infernal_cult`, `necromancers_circle`, `ancient_site_guardians`.
- `high_fantasy` : `mage_order`, `fey_court`, `dragon_court`, `nonhuman_enclave`, `monstrous_horde`.

## Grille fonctionnelle

### Subsistance et extraction

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Ferme | `subsistence_extraction.farm` | couverte | `base_farm`, `productive_community`, `farm_household` |  |
| Pâturage et nomades pastoraux | `subsistence_extraction.pastoralism` | couverte | `herding_group`, `mobile_household`, `travel_party` | Le troupeau est un capital propre (`herding_group`) ; la mobilité vient de `mobile_household` et `travel_party`. Les trajets entre domaines relèvent de S7. |
| Pêche | `subsistence_extraction.fishing` | couverte | `resource_group`, `shore_community` | Pas d'archétype dédié : `resource_group` (récolte du milieu, activité `agriculture`) et `shore_community` suffisent. Choix discutable, voir le rapport. |
| Mine et carrière | `subsistence_extraction.mining_quarry` | couverte | `mining_crew` | Activité `craft` : travail qualifié sous concession. |
| Forêt et charbonnage | `subsistence_extraction.forestry_charcoal` | couverte | `resource_group`, `forest_house` | Pas d'archétype dédié : la famille `forest_houses` associe déjà `forest_house` et `resource_group`. |
| Saline | `subsistence_extraction.saltworks` | couverte | `mining_crew` | Même mécanique de concession et de risque ; la fiscalité du sel relève de l'administration. |

### Artisanat et commerce

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Atelier | `craft_trade.workshop` | couverte | `workshop`, `base_workshop` |  |
| Guilde | `craft_trade.guild` | couverte | `craft_guild`, `craft_association`, `base_craft_association` |  |
| Marché | `craft_trade.market` | couverte | `market_association` |  |
| Caravane | `craft_trade.caravan` | couverte | `caravan` |  |
| Armateur | `craft_trade.shipowner` | couverte | `shipping_consortium`, `crew` | `shipping_consortium` porte la propriété et l'assurance ; `crew` porte les équipages. |
| Banque et prêt | `craft_trade.banking` | couverte | `banking_house` | Inclut l'assurance. L'usure est un tag de conduite, pas un type de groupe. |
| Ligue marchande | `craft_trade.merchant_league` | couverte | `merchant_league` | Couvre guilde marchande, fédération de guildes et ligue de cités. |

### Pouvoir

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Cour | `power.court` | couverte | `court` |  |
| Seigneurie | `power.lordship` | couverte | `lordship`, `landed_house`, `territorial_house` | `lordship` est l'ancre juridictionnelle et domaniale ; les maisons sont ses vues de lignage. |
| Conseil urbain | `power.urban_council` | couverte | `civic_council` |  |
| Assemblée rurale | `power.rural_assembly` | couverte | `local_assembly`, `village_council` |  |
| Clan | `power.clan` | couverte | `kin_group` | Le clan comme lignage ; son assemblée est `local_assembly`. |
| Administration | `power.administration` | couverte | `administration`, `base_local_institution` |  |
| Ambassade | `power.embassy` | couverte | `embassy` |  |
| Famille régnante | `power.ruling_family` | couverte | `ruling_dynasty` |  |
| Prétendants | `power.pretenders` | couverte | `house_faction`, `base_faction` | Pas d'archétype dédié : une prétention est une relation `claims` portée par une faction de maisons. Choix discutable, voir le rapport. |

### Justice et ordre

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Tribunal | `justice_order.tribunal` | couverte | `law_court` |  |
| Guet | `justice_order.watch` | couverte | `town_watch` |  |
| Geôle | `justice_order.jail` | couverte | `prison` |  |
| Inquisition | `justice_order.inquisition` | couverte | `inquisition` |  |
| Prévôté | `justice_order.provostship` | couverte | `provostship` | Officier royal ou seigneurial cumulant police et justice sur un ressort. |
| Bourreau | `justice_order.executioner` | couverte | `executioner_house` | Lignage à charge héréditaire et statut d'exclu ; la conformité culturelle règle le stigmate. |

### Militaire

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Levée | `military.levy` | couverte | `militia` | Une levée est une mobilisation de la milice par un événement de guerre (S2). Choix discutable, voir le rapport. **Limite de livraison :** La mobilisation par événement de guerre n'existe qu'avec les processus généraux de S2 ; en S1 seul le stock de milice est défini. |
| Milice | `military.militia` | couverte | `militia` |  |
| Garnison | `military.garrison` | couverte | `garrison` |  |
| Mercenaires | `military.mercenaries` | couverte | `mercenary_company` |  |
| Ordre de chevalerie | `military.knightly_order` | couverte | `knightly_order` |  |
| Flotte | `military.fleet` | couverte | `war_fleet` |  |
| Pillards | `military.raiders` | couverte | `raiding_band` | Pillage saisonnier parfois toléré par la culture ; distinct du banditisme et de la piraterie. |
| Éclaireurs | `military.scouts` | couverte | `scout_company` |  |

### Religion

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Clergé séculier | `religion.secular_clergy` | couverte | `secular_clergy` |  |
| Monastère | `religion.monastery` | couverte | `monastery` |  |
| Ordre mendiant | `religion.mendicant_order` | couverte | `mendicant_friary` |  |
| Ermites | `religion.hermits` | couverte | `hermitage` |  |
| Confrérie laïque | `religion.lay_confraternity` | couverte | `lay_confraternity`, `mortuary_confraternity` |  |
| Culte hétérodoxe | `religion.heterodox_cult` | couverte | `heterodox_movement` |  |
| Secte | `religion.sect` | couverte | `closed_sect` | Emprise sur les membres, interdiction de partir ; distincte du culte hétérodoxe. |
| Culte ancestral ou païen | `religion.ancestral_cult` | couverte | `ancestral_cult` |  |
| Pèlerins | `religion.pilgrims` | couverte | `travel_party`, `base_charitable_order`, `ritual_institution` | Pas d'archétype dédié : le voyage (`travel_party`), l'accueil (`base_charitable_order`) et le sanctuaire (`ritual_institution`). |

### Savoir et soin

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| École | `knowledge_care.school` | couverte | `school` |  |
| Université | `knowledge_care.university` | couverte | `university` | Couvre aussi académies, cercles de savants et observatoires. |
| Scriptorium | `knowledge_care.scriptorium` | couverte | `scriptorium` |  |
| Hôpital et léproserie | `knowledge_care.hospital` | couverte | `hospital_house` | Maison de soin résidentielle ; l'ordre qui la dessert est `base_charitable_order`. |
| Apothicaires | `knowledge_care.apothecaries` | couverte | `healers_circle`, `craft_guild` | Pas d'archétype dédié : praticiens dans `healers_circle`, corps réglementé dans `craft_guild`. Choix discutable, voir le rapport. |
| Sages-femmes | `knowledge_care.midwives` | couverte | `healers_circle` |  |
| Guérisseurs | `knowledge_care.healers` | couverte | `healers_circle` |  |

### Culture et sociabilité

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Troupe itinérante | `culture_sociability.troupe` | couverte | `itinerant_troupe` |  |
| Ménestrels | `culture_sociability.minstrels` | couverte | `itinerant_troupe` | Petite troupe sous patronage ; pas d'archétype séparé. Choix discutable, voir le rapport. |
| Abbaye de jeunesse | `culture_sociability.youth_abbey` | couverte | `festive_society` |  |
| Cercle de femmes | `culture_sociability.women_circle` | couverte | `women_circle` | Cercle informel de classe `friend_circle` ; le filtre de sexe est une règle de recrutement, jamais une ascendance. |
| Fraternité de quartier | `culture_sociability.neighborhood_brotherhood` | couverte | `base_neighborhood`, `neighborhood`, `lay_confraternity` |  |
| Tavernes | `culture_sociability.taverns` | couverte | `tavern_house` |  |
| Cercle d'amis | `culture_sociability.friend_circle` | couverte | `companionship`, `base_friend_circle` |  |

### Crime et marges

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Brigands | `crime_margins.brigands` | couverte | `bandit_band` |  |
| Guilde de voleurs | `crime_margins.thieves_guild` | couverte | `thieves_guild`, `base_criminal_network` |  |
| Contrebandiers | `crime_margins.smugglers` | couverte | `smuggling_ring` |  |
| Pirates | `crime_margins.pirates` | couverte | `pirate_crew` |  |
| Racket | `crime_margins.racket` | couverte | `protection_racket` |  |
| Assassins | `crime_margins.assassins` | couverte | `assassin_brotherhood` |  |
| Faussaires | `crime_margins.forgers` | couverte | `forgery_ring` |  |
| Marchands d'esclaves | `crime_margins.slavers` | couverte | `slave_trading_ring` | Peut être licite dans une juridiction et proscrit dans une autre : la légalité reste relative (§27.3). |
| Recéleurs | `crime_margins.fences` | couverte | `smuggling_ring`, `thieves_guild` | Pas d'archétype dédié : nœud de revente du commerce illicite. Choix discutable, voir le rapport. |
| Mendiants organisés | `crime_margins.organized_beggars` | couverte | `beggar_brotherhood` |  |
| Proscrits | `crime_margins.outlaws` | couverte | `outlaw_company` | Hors-la-loi de statut juridique, de la bande prédatrice à la résistance ; la conduite est tirée par slot. |
| Réfugiés | `crime_margins.refugees` | couverte | `refugee_community` | **Limite de livraison :** Le déplacement forcé vient des événements de S2 et des flux de S3 ; en S1 seul l'archétype est défini. |
| Enclaves minoritaires | `crime_margins.minority_enclaves` | couverte | `minority_enclave` | Défini par la situation sociale, pas par une ascendance. |
| Société secrète | `crime_margins.secret_society` | couverte | `secret_society` |  |

### Fantastique

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Ordre de mages | `fantasy.mage_order` | couverte | `mage_order` |  |
| Cabale | `fantasy.cabal` | couverte | `arcane_cabal` |  |
| Alchimistes | `fantasy.alchemists` | couverte | `alchemists_lodge` | Pratique réputée historique mais classée `low_fantasy` : elle exige un effet surnaturel déclaré. Voir le rapport. |
| Cercle druidique | `fantasy.druid_circle` | couverte | `druid_circle` |  |
| Chasseurs de monstres | `fantasy.monster_hunters` | couverte | `monster_hunters_lodge` |  |
| Compagnie d'aventuriers | `fantasy.adventurers` | couverte | `adventuring_company` |  |
| Culte démoniaque | `fantasy.demonic_cult` | couverte | `infernal_cult` |  |
| Nécromanciens | `fantasy.necromancers` | couverte | `necromancers_circle` |  |
| Cour féerique | `fantasy.fey_court` | couverte | `fey_court` | Membres au profil d'ascendance explicite fourni par l'auteur. **Limite de livraison :** Exige un profil d'ascendance explicite ; un profil non biparental est refusé par le fournisseur Lineage Lab avant S8 (§1.3). |
| Cour draconique | `fantasy.dragon_court` | couverte | `dragon_court` | Membres au profil d'ascendance explicite fourni par l'auteur. **Limite de livraison :** Exige un profil d'ascendance explicite ; un profil non biparental est refusé par le fournisseur Lineage Lab avant S8 (§1.3). |
| Enclave non humaine | `fantasy.nonhuman_enclave` | couverte | `nonhuman_enclave` | Ni bonne ni mauvaise par nature : base d'alignement nulle (§6.2, §27.1). **Limite de livraison :** Exige un profil d'ascendance explicite ; un profil non biparental est refusé par le fournisseur Lineage Lab avant S8 (§1.3). |
| Horde monstrueuse | `fantasy.monstrous_horde` | couverte | `monstrous_horde` | Définie par sa conduite (pillage, violence) et non par une espèce ; l'écart laisse place aux hordes ordinaires. **Limite de livraison :** Exige un profil d'ascendance explicite ; un profil non biparental est refusé par le fournisseur Lineage Lab avant S8 (§1.3). |
| Gardiens d'un site ancien | `fantasy.ancient_site_guardians` | couverte | `ancient_site_guardians` | Exige un site marqué comme ancien par l'hôte spatial ; le catalogue d'atlas S0 n'en porte pas encore. **Limite de livraison :** Dépend d'un marqueur de site ancien absent du catalogue d'atlas S0 ; activable par overlay local seulement. |

### Cases ajoutées

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Entraide mutuelle et assurance | `added.mutual_aid` | couverte | `mutual_aid_society`, `mortuary_confraternity` | Sociétés de secours, caisses mutuelles, associations d'anciens combattants (6 systèmes Pecking Order). Distinct de la charité : on secourt ses pairs, pas des tiers. |
| Compagnons, journaliers et salariés | `added.journeymen` | couverte | `journeymen_fellowship` | Syndicats de travailleurs et compagnonnage : intérêt distinct des maîtres de la guilde, souvent proscrit. |
| Renseignement et espionnage | `added.intelligence` | couverte | `intelligence_network` | Réseaux d'espions et services de renseignement (4 systèmes Pecking Order) : ni société secrète, ni ambassade. |
| Biens communs, eau, voies et fisc | `added.commons_infrastructure` | couverte | `water_association`, `local_assembly`, `administration` | Commissions de l'eau, fiducies des communs, péages, travaux publics (12 systèmes civic et economic). |
| Rites funéraires | `added.funerary_rites` | couverte | `mortuary_confraternity` | Sociétés funéraires, passeurs, maisons de purification (4 systèmes religieux et civiques). |
| Suite armée et garde rapprochée | `added.retinue_bodyguard` | couverte | `retinue` | Maisons de clientèle et gardes du corps (2 systèmes). |
| Hospitalité et accueil des voyageurs | `added.hospitality_travelers` | couverte | `tavern_house`, `base_charitable_order` | Maisons d'étape, gardiens de col, lanternes de route. |
| Fêtes, processions et sport | `added.festivals_sport` | couverte | `festive_society` | Clubs sportifs, conseils des fêtes, compagnies processionnelles (3 systèmes). |
| Insurrection et résistance | `added.insurgents` | couverte | `outlaw_company`, `secret_society` | Réseaux de résistance et mouvements de renversement (2 systèmes) : groupes proscrits à projet politique. |

## Archétypes ajoutés

| Archétype | Classe | Setting | Kind de nom | Rôle |
|---|---|---|---|---|
| `herding_group` | `activity_unit` | `historical` | `organization.household` | Troupeau et pâtre : unité d'activité dont le capital est le bétail. |
| `mining_crew` | `activity_unit` | `historical` | `organization.community` | Équipe d'extraction sous concession : mine, carrière, marais salants. |
| `shipping_consortium` | `association` | `historical` | `organization.company` | Armateurs qui mutualisent navires, cargaisons et assurance. |
| `banking_house` | `activity_unit` | `historical` | `organization.company` | Maison de dépôt, de change, de prêt et d'assurance. |
| `merchant_league` | `association` | `historical` | `organization.guild` | Guilde marchande ou ligue de cités : privilèges, convois, arbitrage. |
| `lordship` | `institution` | `historical` | `organization.domain` | Seigneurie : justice banale, redevances et tenures sur un domaine. |
| `embassy` | `institution` | `historical` | `organization.office` | Mission installée auprès d'une cour ou d'une cité étrangère. |
| `ruling_dynasty` | `lineage_view` | `historical` | `organization.house` | Lignage qui détient la charge souveraine. |
| `law_court` | `institution` | `historical` | `organization.tribunal` | Juridiction qui instruit et tranche : tribunal villageois, de métier ou d'appel. |
| `town_watch` | `institution` | `historical` | `organization.office` | Guet et veille : patrouille, alarme, garde du feu ou des côtes. |
| `prison` | `institution` | `historical` | `organization.office` | Geôle et administration pénitentiaire. |
| `inquisition` | `institution` | `historical` | `organization.tribunal` | Tribunal doctrinal qui poursuit l'hétérodoxie. |
| `provostship` | `institution` | `historical` | `organization.office` | Prévôté : police et justice déléguées par un souverain sur un ressort. |
| `executioner_house` | `lineage_view` | `historical` | `organization.house` | Lignage chargé des exécutions et des basses œuvres de justice. |
| `militia` | `association` | `historical` | `organization.military_unit` | Force de citoyens ou de tenanciers mobilisable pour la défense locale. |
| `garrison` | `activity_unit` | `historical` | `organization.military_unit` | Détachement permanent d'un fort ou d'une ville. |
| `mercenary_company` | `activity_unit` | `historical` | `organization.company` | Force soldée qui négocie solde, durée et ravitaillement. |
| `knightly_order` | `association` | `historical` | `organization.order` | Ordre armé sous serment, devoirs d'hospitalité et patronage. |
| `war_fleet` | `activity_unit` | `historical` | `organization.crew` | Flotte de guerre : escadres, équipages, intendance. |
| `raiding_band` | `association` | `historical` | `organization.band` | Parti de razzia saisonnière, parfois reconnu par sa culture. |
| `scout_company` | `activity_unit` | `historical` | `organization.company` | Compagnie légère de reconnaissance et d'exploration. |
| `secular_clergy` | `institution` | `historical` | `organization.cult` | Clergé de paroisse et de mission. |
| `monastery` | `community` | `historical` | `organization.monastery` | Maison de vie commune régie par une règle. |
| `mendicant_friary` | `association` | `historical` | `organization.order` | Fraternité sans domaine foncier vivant d'aumônes. |
| `hermitage` | `community` | `historical` | `organization.monastery` | Retraite de contemplation reliée à un village voisin. |
| `lay_confraternity` | `association` | `historical` | `organization.confraternity` | Association de laïcs pour une dévotion, des œuvres ou des secours. |
| `heterodox_movement` | `association` | `historical` | `organization.cult` | Mouvement de croyance divergente. |
| `closed_sect` | `association` | `historical` | `organization.cult` | Groupe fermé à emprise forte sur ses membres. |
| `ancestral_cult` | `association` | `historical` | `organization.cult` | Culte des ancêtres, des esprits du lieu ou des cycles. |
| `base_charitable_order` | `association` | `historical` | `organization.order` | Ordre ou œuvre de charité : aumône, accueil, soin. |
| `university` | `community` | `historical` | `organization.school` | Communauté d'étude sous charte : facultés, grades, débats. |
| `scriptorium` | `activity_unit` | `historical` | `organization.workshop` | Atelier de copie, d'enluminure et de conservation de manuscrits. |
| `hospital_house` | `community` | `historical` | `organization.hospital` | Maison de soin résidentielle : hôpital, léproserie, convalescence. |
| `healers_circle` | `association` | `historical` | `organization.circle` | Sages-femmes, guérisseurs et préparateurs de remèdes. |
| `itinerant_troupe` | `activity_unit` | `historical` | `organization.troupe` | Troupe de spectacle ou de ménestrels qui voyage. |
| `festive_society` | `association` | `historical` | `organization.circle` | Abbaye de jeunesse, club de jeux, comité de fêtes. |
| `women_circle` | `friend_circle` | `historical` | `organization.circle` | Cercle informel de femmes. |
| `tavern_house` | `activity_unit` | `historical` | `organization.tavern` | Taverne ou auberge : hébergement, boisson, jeu. |
| `base_criminal_network` | `association` | `historical` | `organization.criminal` | Réseau criminel générique (§27.6). |
| `thieves_guild` | `association` | `historical` | `organization.criminal` | Guilde urbaine de vol et de recel qui arbitre ses membres. |
| `protection_racket` | `association` | `historical` | `organization.criminal` | Famille ou bande qui rançonne un quartier. |
| `bandit_band` | `association` | `historical` | `organization.band` | Bande de brigands de grand chemin. |
| `smuggling_ring` | `association` | `historical` | `organization.criminal` | Filière de contrebande et de revente de biens volés. |
| `pirate_crew` | `association` | `historical` | `organization.crew` | Équipage hors-la-loi qui vit de prises en mer. |
| `assassin_brotherhood` | `association` | `historical` | `organization.secret_society` | Confrérie d'assassins à gages. |
| `forgery_ring` | `association` | `historical` | `organization.criminal` | Atelier clandestin de fausse monnaie, de faux et de contrefaçons. |
| `slave_trading_ring` | `association` | `historical` | `organization.criminal` | Maison qui capture et vend des personnes. |
| `beggar_brotherhood` | `association` | `historical` | `organization.confraternity` | Fraternité de mendiants organisée par quartier. |
| `outlaw_company` | `association` | `historical` | `organization.band` | Compagnie de proscrits : bannis, insurgés, résistants. |
| `refugee_community` | `community` | `historical` | `organization.community` | Communauté déplacée qui reçoit abri, vivres et protection. |
| `minority_enclave` | `community` | `historical` | `organization.community` | Quartier ou village d'une minorité culturelle ou religieuse. |
| `secret_society` | `association` | `historical` | `organization.secret_society` | Association fermée à signes de reconnaissance et projet caché. |
| `mage_order` | `association` | `high_fantasy` | `organization.order` | Collège ou ordre de mages : apprentissage, registre, responsabilité. |
| `arcane_cabal` | `association` | `low_fantasy` | `organization.secret_society` | Cercle fermé de praticiens occultes. |
| `alchemists_lodge` | `activity_unit` | `low_fantasy` | `organization.workshop` | Loge d'alchimistes : laboratoire, recettes, secret. |
| `druid_circle` | `association` | `low_fantasy` | `organization.cult` | Cercle gardien d'un territoire et de ses cycles. |
| `monster_hunters_lodge` | `association` | `low_fantasy` | `organization.company` | Loge qui traque et recense les prédateurs surnaturels. |
| `adventuring_company` | `activity_unit` | `low_fantasy` | `organization.company` | Équipe sous contrat qui explore et pille des sites dangereux. |
| `infernal_cult` | `association` | `low_fantasy` | `organization.cult` | Culte lié par pacte à une puissance hostile ou interdite. |
| `necromancers_circle` | `association` | `low_fantasy` | `organization.secret_society` | Cercle qui travaille avec les morts et leurs restes. |
| `fey_court` | `community` | `high_fantasy` | `organization.court` | Cour de puissances surnaturelles liées par hospitalité et dette. |
| `dragon_court` | `community` | `high_fantasy` | `organization.court` | Cour de puissances anciennes : tribut, territoire, serments. |
| `nonhuman_enclave` | `community` | `high_fantasy` | `organization.community` | Communauté fermée dont les membres relèvent d'un profil d'ascendance explicite. |
| `monstrous_horde` | `community` | `high_fantasy` | `organization.band` | Troupe nombreuse et mobile qui vit de pillage et de prédation. |
| `ancient_site_guardians` | `association` | `low_fantasy` | `organization.order` | Ordre qui garde un lieu ou des objets anciens. |
| `mutual_aid_society` | `association` | `historical` | `organization.confraternity` | Société de secours entre pairs : caisse, assurance, entraide. |
| `journeymen_fellowship` | `association` | `historical` | `organization.guild` | Fraternité de compagnons et salariés d'un métier. |
| `intelligence_network` | `association` | `historical` | `organization.secret_society` | Réseau d'informateurs et de rapports. |

## Décision pour chaque système Pecking Order

### Systèmes reliés à un archétype

| Archétype | Systèmes |
|---|---|
| `administration` | `canal_authority`, `civil_service`, `disaster_commission`, `harbor_authority`, `ministry`, `province`, `public_works_office`, `road_wardens`, `rural_district`, `sanitation_board`, `toll_keepers`, `village_federation` |
| `caravan` | `caravan_company`, `merchant_caravan` |
| `civic_council` | `chartered_town`, `city`, `city_state`, `free_city`, `guild_republic`, `local_council`, `merchant_republic`, `siege_defense_council` |
| `court` | `crown_council`, `royal_court`, `royal_household` |
| `craft_association` | `artisan_confraternity` |
| `craft_guild` | `craft_guild` |
| `crew` | `merchant_ship_crew` |
| `household_compound` | `joint_family` |
| `kin_group` | `adoptive_house`, `clan`, `lineage`, `lineage_memory_archive`, `matrilineal_house` |
| `landed_house` | `noble_house` |
| `local_assembly` | `clan_council`, `commons_trust`, `hamlet`, `sacred_grove_moot`, `tribal_council` |
| `local_colony` | `frontier_settlement` |
| `market_association` | `grain_exchange`, `labor_exchange`, `market_authority`, `public_market_board` |
| `productive_community` | `farm_collective` |
| `resource_group` | `communal_oath_hunt`, `fishing_cooperative`, `migration_omen_watch` |
| `retinue` | `bodyguard`, `retainer_household` |
| `ritual_group` | `dream_interpretation_circle`, `exorcist_lodge`, `forge_initiation_order`, `mystery_cult`, `possession_witness_circle`, `vow_release_circle` |
| `ritual_institution` | `dream_incubation_sanctuary`, `oracle_house`, `oracle_network`, `pilgrimage_shrine`, `relic_authentication_chamber`, `sacred_calendar_council`, `solar_clock_custodians`, `temple_city`, `temple_granary_office`, `vow_renewal_court` |
| `school` | `apprentice_school`, `diviner_apprentice_house`, `guild_school`, `village_school` |
| `village_council` | `village` |
| `ward_community` | `ward` |
| `water_association` | `irrigation_council`, `spring_spirit_compact`, `water_board` |
| `workshop` | `apprenticeship_workshop`, `cooperative` |
| `base_trade_company` | `trading_company` |
| `base_religious_community` | `congregation`, `cult` |
| `base_neighborhood` | `roadside_lantern_brotherhood` |
| `mortuary_confraternity` | `burial_society`, `funerary_boat_brotherhood`, `funerary_guild`, `mortuary_purification_house` |
| `mining_crew` | `mining_company` |
| `shipping_consortium` | `shipping_consortium` |
| `banking_house` | `bank`, `insurance_house` |
| `merchant_league` | `chartered_company`, `city_league`, `colonial_company`, `guild_federation`, `merchant_guild` |
| `lordship` | `barony`, `county`, `duchy`, `estate_administration`, `frontier_march`, `principality`, `temple_estate` |
| `embassy` | `diplomatic_mission` |
| `ruling_dynasty` | `dynasty` |
| `law_court` | `appeals_court`, `arbitration_chamber`, `court_of_justice`, `customary_council`, `guild_court`, `inheritance_court`, `inquest_commission`, `mediation_circle`, `village_court`, `watch_court` |
| `town_watch` | `coast_watch`, `fire_watch`, `neighborhood_watch` |
| `prison` | `prison_administration` |
| `inquisition` | `heresy_court` |
| `militia` | `militia` |
| `garrison` | `army`, `border_guard`, `combat_company`, `garrison`, `siege_train` |
| `mercenary_company` | `mercenary_company` |
| `knightly_order` | `knightly_order`, `military_order` |
| `war_fleet` | `naval_fleet`, `naval_squadron`, `navy`, `warship_crew` |
| `raiding_band` | `raiding_party` |
| `scout_company` | `expedition`, `scout_company` |
| `secular_clergy` | `mission_station`, `priesthood` |
| `monastery` | `monastery`, `religious_order` |
| `mendicant_friary` | `mendicant_almonry`, `mendicant_preaching_network` |
| `hermitage` | `hermitage` |
| `lay_confraternity` | `lay_brotherhood`, `plague_vow_fellowship` |
| `heterodox_movement` | `cult_cell`, `itinerant_prophets_movement` |
| `ancestral_cult` | `ancestor_feast_compact`, `ancestor_shrine`, `shamanic_circle` |
| `base_charitable_order` | `pass_shrine_wardens`, `pilgrimage_wayhouse_network`, `relief_society` |
| `university` | `academy`, `astral_calendar_consortium`, `observatory`, `scholars_circle`, `university` |
| `scriptorium` | `library`, `scriptorium` |
| `hospital_house` | `epidemic_recovery_house`, `healing_house` |
| `itinerant_troupe` | `theatrical_troupe`, `traveling_circus` |
| `festive_society` | `festival_council`, `procession_company`, `sports_club` |
| `base_criminal_network` | `prison_gang` |
| `thieves_guild` | `thieves_guild` |
| `protection_racket` | `crime_family`, `street_gang` |
| `smuggling_ring` | `smugglers_ring` |
| `pirate_crew` | `pirate_flotilla` |
| `assassin_brotherhood` | `assassins_order` |
| `outlaw_company` | `resistance_network`, `revolutionary_party` |
| `refugee_community` | `refuge_camp` |
| `secret_society` | `secret_society`, `underground_press` |
| `mage_order` | `arcane_registry`, `golem_workshop`, `guild_of_enchanters`, `mage_college`, `planar_gate_authority` |
| `arcane_cabal` | `witch_coven` |
| `druid_circle` | `druid_grove` |
| `monster_hunters_lodge` | `monster_hunters_lodge` |
| `adventuring_company` | `adventurers_guild`, `adventuring_company` |
| `infernal_cult` | `demon_pact_court` |
| `necromancers_circle` | `necromantic_college` |
| `fey_court` | `elemental_conclave`, `fey_court`, `spirit_court` |
| `dragon_court` | `dragon_clan`, `dragon_court`, `dragonflight` |
| `monstrous_horde` | `monster_pack`, `undead_legion` |
| `ancient_site_guardians` | `relic_custodians` |
| `mutual_aid_society` | `credit_union`, `mutual_aid_cell`, `veterans_association` |
| `journeymen_fellowship` | `workers_union` |
| `intelligence_network` | `intelligence_service`, `spy_cell`, `spy_ring` |

### Systèmes exclus

- `polity_scale` (23) : Souveraineté ou gouvernement d'un État entier. Chronos modélise la polity et ses offices supérieurs (`home_domain`, §2) en dehors des archétypes de groupes locaux ; un domaine S0-S5 ne dépasse pas 20 000 habitants (§17). Systèmes : `kingdom`, `empire`, `aristocratic_nation`, `democratic_nation`, `federation`, `confederation`, `tributary_realm`, `elective_monarchy`, `protectorate`, `occupied_territory`, `nomadic_confederation`, `island_polity`, `parliament`, `republic`, `bureaucratic_state`, `theocracy`, `military_junta`, `revolutionary_council`, `colonial_administration`, `sacred_kingdom`, `nation`, `war_council`, `religious_empire`.
- `command_echelon` (9) : Niveau interne de commandement d'une force permanente, sans mécanique sociale distincte de l'archétype `garrison` (ou `war_fleet`) qui le contient. Aucun domaine S0-S5 n'atteint cette échelle. Systèmes : `army_group`, `army_corps`, `division`, `brigade`, `regiment`, `battalion`, `company`, `platoon`, `squad`.
- `modern_form` (4) : Forme institutionnelle étrangère au cadre médiéval ou fantastique de Chronos (aviation, parti de masse, société par actions, fédération syndicale interprofessionnelle). Systèmes : `air_force`, `political_party`, `trade_union_federation`, `corporation`.

## Kinds de noms

`name_profile` porte directement un kind Namebench. Le kind ne dépend ni de la famille culturelle ni de la classe morale. Les kinds marqués « proposé » sont à envoyer à l'équipe Namebench.

| Kind | Origine | Archétypes |
|---|---|---|
| `organization.household` | départ Namebench | `base_farm`, `communal_house`, `farm_household`, `herding_group`, `household_compound`, `mobile_household`, `service_household` |
| `organization.house` | départ Namebench | `craft_house`, `executioner_house`, `forest_house`, `kin_group`, `landed_house`, `merchant_house`, `ruling_dynasty`, `territorial_house` |
| `organization.guild` | départ Namebench | `base_craft_association`, `craft_association`, `craft_guild`, `journeymen_fellowship`, `market_association`, `merchant_league`, `water_association` |
| `organization.confraternity` | départ Namebench | `beggar_brotherhood`, `lay_confraternity`, `mortuary_confraternity`, `mutual_aid_society` |
| `organization.order` | départ Namebench | `ancient_site_guardians`, `base_charitable_order`, `knightly_order`, `mage_order`, `mendicant_friary` |
| `organization.faction` | départ Namebench | `base_faction`, `house_faction` |
| `organization.secret_society` | départ Namebench | `arcane_cabal`, `assassin_brotherhood`, `intelligence_network`, `necromancers_circle`, `secret_society` |
| `organization.criminal` | départ Namebench | `base_criminal_network`, `forgery_ring`, `protection_racket`, `slave_trading_ring`, `smuggling_ring`, `thieves_guild` |
| `organization.company` | départ Namebench | `adventuring_company`, `banking_house`, `base_service_unit`, `base_trade_company`, `caravan`, `exchange_network`, `mercenary_company`, `monster_hunters_lodge`, `scout_company`, `service_group`, `shipping_consortium` |
| `organization.retinue` | départ Namebench | `retinue` |
| `organization.school` | départ Namebench | `magical_school`, `school`, `university` |
| `organization.council` | départ Namebench | `civic_council`, `hold_council`, `local_assembly`, `village_council` |
| `organization.community` | départ Namebench | `base_neighborhood`, `local_colony`, `local_community`, `mining_crew`, `minority_enclave`, `neighborhood`, `nonhuman_enclave`, `productive_community`, `refugee_community`, `resource_group`, `shore_community`, `travel_party`, `ward_community` |
| `organization.workshop` | proposé | `alchemists_lodge`, `base_workshop`, `scriptorium`, `workshop` |
| `organization.crew` | proposé | `crew`, `pirate_crew`, `war_fleet` |
| `organization.band` | proposé | `bandit_band`, `monstrous_horde`, `outlaw_company`, `raiding_band` |
| `organization.military_unit` | proposé | `garrison`, `militia` |
| `organization.office` | proposé | `administration`, `base_local_institution`, `embassy`, `prison`, `provostship`, `redistributive_institution`, `town_watch` |
| `organization.tribunal` | proposé | `inquisition`, `law_court` |
| `organization.court` | proposé | `court`, `dragon_court`, `fey_court` |
| `organization.domain` | proposé | `lordship` |
| `organization.cult` | proposé | `ancestral_cult`, `base_religious_community`, `closed_sect`, `druid_circle`, `heterodox_movement`, `infernal_cult`, `ritual_group`, `ritual_institution`, `secular_clergy` |
| `organization.monastery` | proposé | `hermitage`, `monastery` |
| `organization.hospital` | proposé | `hospital_house` |
| `organization.troupe` | proposé | `itinerant_troupe` |
| `organization.tavern` | proposé | `tavern_house` |
| `organization.circle` | proposé | `base_friend_circle`, `companionship`, `festive_society`, `healers_circle`, `women_circle` |

## Limites assumées

- La disponibilité des archétypes non « base » dépend du contexte de site, que S1 ne spécifie pas : elle est tranchée par S2 et S4. Seuls les archétypes fantastiques ont un chemin d'activation déclaré (overlays).
- La formule de `conformity_bps` est une proposition de S1 : la spec §27.4 ne la fixe pas.
- Les activités restent `agriculture`, `craft`, `trade`, `services` (§24.1) : l'extraction et les métiers d'armes y sont rattachés pour le décompte des travailleurs.
- `catalogue_version` n'a pas été modifié : la décision d'un nouveau numéro revient à l'intégrateur.
