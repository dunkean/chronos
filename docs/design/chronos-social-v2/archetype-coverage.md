# Couverture des archétypes de groupes sociaux

Synthèse lisible de `archetype-coverage.v1.json`. Le JSON fait foi et l'outil `tools/check_catalogue_v2.py` le vérifie. Les cases et les décisions attestent la forme du catalogue : elles ne prouvent ni l'exactitude historique ni la calibration. Toutes les valeurs sont des réglages initiaux de worldbuilding (`initial_template_not_historical_calibration`).

## Chiffres

- Archétypes : 133 au total, dont 53 anciens inchangés et 80 ajoutés.
- Classes (total, dont ajoutés) : `activity_unit` 29 (16), `association` 55 (43), `community` 23 (13), `friend_circle` 3 (1), `household_view` 4 (0), `institution` 11 (5), `lineage_view` 8 (2).
- Réglage de fantastique (total, dont ajoutés) : `historical` 117 (65), `low_fantasy` 10 (9), `high_fantasy` 6 (6).
- Systèmes Pecking Order : 242 décidés, dont 212 reliés à un archétype et 30 exclus avec raison.
- Grille fonctionnelle : 86 cases du cahier des charges, toutes couvertes ; 19 cases ajoutées, dont 2 hors périmètre avec raison.
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

`setting_profile` (`historical`, `low_fantasy`, `high_fantasy`) est un paramètre obligatoire du monde (§2.1). Un archétype est actif si son `setting` est inférieur ou égal à celui du monde. Les overlays fantastiques portent `world_activation_min_setting` : ils s'activent au niveau du monde, sans dépendre des 79 raccords. Un monde historique n'active aucun archétype fantastique.

- `historical` : 117 archétypes.
- `low_fantasy` : `magical_school`, `arcane_cabal`, `witch_coven`, `druid_circle`, `monster_hunters_lodge`, `adventuring_company`, `holy_order`, `infernal_cult`, `necromancers_circle`, `ancient_site_guardians`.
- `high_fantasy` : `mage_order`, `fey_court`, `dragon_court`, `nonhuman_enclave`, `enchanters_workshop`, `arcane_authority`.

## Grille fonctionnelle

### Subsistance et extraction

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Ferme | `subsistence_extraction.farm` | couverte | `base_farm`, `productive_community`, `farm_household` |  |
| Pâturage et nomades pastoraux | `subsistence_extraction.pastoralism` | couverte | `herding_group`, `mobile_household`, `travel_party` | Le troupeau est un capital propre (`herding_group`, contexte `pasture`) ; la mobilité vient de `mobile_household` et `travel_party`. Les trajets entre domaines relèvent de S7. |
| Pêche | `subsistence_extraction.fishing` | couverte | `fishing_crew`, `shore_community` | Bateau et quotas portés par `fishing_crew` ; contexte de site `coastal` ou `river`. |
| Mine et carrière | `subsistence_extraction.mining_quarry` | couverte | `mining_crew` | Activité `craft` : travail qualifié sous concession ; contextes de site `mineral` ou `saltpan`. |
| Forêt et charbonnage | `subsistence_extraction.forestry_charcoal` | couverte | `woodland_crew`, `forest_house` | Charbonnage et bûcheronnage classés `craft` ; contexte de site `forest`. |
| Saline | `subsistence_extraction.saltworks` | couverte | `mining_crew` | Contexte de site `saltpan` ; même mécanique de concession et de risque qu'une mine. |

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
| Clan | `power.clan` | couverte | `kin_group`, `local_assembly` | Le clan comme lignage (`kin_group`, famille proche seulement) et son assemblée de lignées (`local_assembly`) qui négocie et arbitre. |
| Administration | `power.administration` | couverte | `administration`, `base_local_institution` |  |
| Ambassade | `power.embassy` | couverte | `embassy` | Association née d'un événement `diplomatic_contact` (à définir en S2) ; seuil minimal de 1 500 adultes attendus. |
| Famille régnante | `power.ruling_family` | couverte | `ruling_dynasty` |  |
| Prétendants | `power.pretenders` | couverte | `house_faction`, `base_faction` | Pas d'archétype dédié : une prétention est une relation `claims` portée par une faction de maisons. Choix discutable, voir le rapport. |

### Justice et ordre

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Tribunal | `justice_order.tribunal` | couverte | `law_court` |  |
| Guet | `justice_order.watch` | couverte | `town_watch` |  |
| Geôle | `justice_order.jail` | couverte | `prison` |  |
| Inquisition | `justice_order.inquisition` | couverte | `inquisition` | Association née d'un événement `heresy_trial`, absente à la fondation du site ; seuil minimal de 600 adultes attendus. |
| Prévôté | `justice_order.provostship` | couverte | `law_court` | La prévôté fusionne dans `law_court` : le prévôt est un office de justice et de police déléguées sur un ressort. |
| Bourreau | `justice_order.executioner` | couverte | `executioner_house`, `stigmatized_caste` | Le stigmate vient du tag `impure_trade` (toléré au pire, jamais proscrit) et de la communauté `stigmatized_caste`. |

### Militaire

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Levée | `military.levy` | couverte | `militia` | Une levée est une mobilisation de la milice par un événement de guerre (S2). Choix discutable, voir le rapport. **Limite de livraison :** La mobilisation par événement de guerre n'existe qu'avec les processus généraux de S2 ; en S1 seul le stock de milice est défini. |
| Milice | `military.militia` | couverte | `militia` |  |
| Garnison | `military.garrison` | couverte | `garrison` |  |
| Mercenaires | `military.mercenaries` | couverte | `mercenary_company` |  |
| Ordre de chevalerie | `military.knightly_order` | couverte | `knightly_order` |  |
| Flotte | `military.fleet` | couverte | `war_fleet` | Unité d'activité `services` à contrat protégé, de 30 à 400 membres, présente seulement dans un port de 3 000 adultes attendus ou plus. |
| Pillards | `military.raiders` | couverte | `raiding_band` | Pillage saisonnier dont la conformité dépend du poids `raiding` de la culture (proscrit, toléré ou licite) ; distinct du banditisme et de la piraterie. |
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
| Pèlerins | `religion.pilgrims` | couverte | `lay_confraternity`, `base_charitable_order`, `ritual_institution` | Pas d'archétype de marche : un flux de pèlerins est de la mobilité (S7) et non un groupe. Confrérie, accueil et sanctuaire sont portés par ces trois archétypes. |

### Savoir et soin

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| École | `knowledge_care.school` | couverte | `school` |  |
| Université | `knowledge_care.university` | couverte | `university` | Couvre aussi académies, cercles de savants et observatoires. |
| Scriptorium | `knowledge_care.scriptorium` | couverte | `scriptorium` |  |
| Hôpital et léproserie | `knowledge_care.hospital` | couverte | `hospital_house` | Maison de soin résidentielle en temps budgété ; l'ordre qui la dessert est `base_charitable_order`. |
| Apothicaires | `knowledge_care.apothecaries` | couverte | `healers_circle`, `craft_guild` | Pas d'archétype dédié : praticiens dans `healers_circle`, corps réglementé dans `craft_guild`. Choix discutable, voir le rapport. |
| Sages-femmes | `knowledge_care.midwives` | couverte | `healers_circle`, `women_circle` | Cercle de soin informel ; le filtre de sexe d'un groupe de sages-femmes est porté par `women_circle`. Approximation à confirmer par la recette de S4. |
| Guérisseurs | `knowledge_care.healers` | couverte | `healers_circle` |  |

### Culture et sociabilité

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Troupe itinérante | `culture_sociability.troupe` | couverte | `itinerant_troupe` |  |
| Ménestrels | `culture_sociability.minstrels` | couverte | `itinerant_troupe` | Petite troupe sous patronage ; pas d'archétype séparé. Choix discutable, voir le rapport. |
| Abbaye de jeunesse | `culture_sociability.youth_abbey` | couverte | `youth_abbey` | Association avec filtre d'âge de 14 à 25 ans, classes d'âge incluses. |
| Cercle de femmes | `culture_sociability.women_circle` | couverte | `women_circle` | Cercle informel de classe `friend_circle` avec filtre `sex: female` ; le filtre est une règle de recrutement et jamais une ascendance. |
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
| Réfugiés | `crime_margins.refugees` | couverte | `refugee_community` | **Limite de livraison :** Le déplacement forcé vient des événements de S2 et des flux de S3 ; en S1 seul l'archétype est défini (événement `displacement`). |
| Enclaves minoritaires | `crime_margins.minority_enclaves` | couverte | `minority_enclave` | Défini par la situation sociale, pas par une ascendance. |
| Société secrète | `crime_margins.secret_society` | couverte | `secret_society` |  |

### Fantastique

| Case | Identifiant | Statut | Archétypes | Remarque |
|---|---|---|---|---|
| Ordre de mages | `fantasy.mage_order` | couverte | `mage_order` |  |
| Cabale | `fantasy.cabal` | couverte | `arcane_cabal` |  |
| Alchimistes | `fantasy.alchemists` | couverte | `alchemists_lodge` | Atelier d'alchimistes historique, `historical` et sans tag `sorcery` ; l'effet surnaturel relève de `enchanters_workshop`. |
| Cercle druidique | `fantasy.druid_circle` | couverte | `druid_circle` |  |
| Chasseurs de monstres | `fantasy.monster_hunters` | couverte | `monster_hunters_lodge` |  |
| Compagnie d'aventuriers | `fantasy.adventurers` | couverte | `adventuring_company` |  |
| Culte démoniaque | `fantasy.demonic_cult` | couverte | `infernal_cult` |  |
| Nécromanciens | `fantasy.necromancers` | couverte | `necromancers_circle` |  |
| Cour féerique | `fantasy.fey_court` | couverte | `fey_court` | Membres au profil d'ascendance explicite fourni par l'auteur. **Limite de livraison :** Exige un profil d'ascendance explicite ; un profil non biparental est refusé par le fournisseur Lineage Lab avant S8 (§1.3). |
| Cour draconique | `fantasy.dragon_court` | couverte | `dragon_court` | Membres au profil d'ascendance explicite fourni par l'auteur. **Limite de livraison :** Exige un profil d'ascendance explicite ; un profil non biparental est refusé par le fournisseur Lineage Lab avant S8 (§1.3). |
| Enclave non humaine | `fantasy.nonhuman_enclave` | couverte | `nonhuman_enclave` | Ni bonne ni mauvaise par nature : base d'alignement nulle (§6.2, §27.1). **Limite de livraison :** Exige un profil d'ascendance explicite ; un profil non biparental est refusé par le fournisseur Lineage Lab avant S8 (§1.3). |
| Horde monstrueuse | `fantasy.monstrous_horde` | couverte | `raiding_band` | Fusionnée dans `raiding_band` : une horde est une bande de pillage de grande taille, activée par l'overlay `monstrous_threat`, sans exigence d'ascendance. |
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
| Paroisse rurale | `added.parish` | couverte | `parish_community`, `secular_clergy` | Communauté de fidèles et clergé de paroisse, présents dès 60 adultes attendus (la revue relevait l'absence de paroisse). |
| Chapitre cathédral et fondation pieuse | `added.cathedral_pious` | couverte | `cathedral_chapter`, `pious_foundation` | Chanoines et fondations perpétuelles (waqf, chantrerie) : corps d'Église durable des villes. |
| Béguinage | `added.beguinage` | couverte | `beguine_house` | Maison de femmes pieuses à vie commune : seule institution féminine hors d'un cercle d'amis. |
| Nation marchande étrangère | `added.foreign_merchants` | couverte | `foreign_merchant_nation` | Colonie de marchands d'une autre culture dans un port ou un carrefour : relation à la culture hôte via le tag `alien_custom`. |
| Bains et prostitution | `added.bathhouse` | couverte | `bathhouse_brothel` | Établissement de bains et commerce sexuel, avec le tag `prostitution` dont le statut varie selon la culture. |
| Ordre sacré fantastique | `added.holy_order` | couverte | `holy_order` | Pendant saint de `infernal_cult` pour le fantastique : moralité de base +3 500, présent dès `low_fantasy`. |
| Cercle de sorcières | `added.witch_coven` | couverte | `witch_coven` | Cercle ouvert de praticiennes qui négocie avec la communauté (système Pecking Order `witch_coven`) : moralité plutôt positive, conformité souvent tolérée. |
| Régulation et artisanat magiques | `added.magic_regulation` | couverte | `arcane_authority`, `enchanters_workshop` | Registres, portails et ateliers d'enchantement (6 systèmes fantastiques de Pecking Order). |
| Hôte de morts-vivants | `added.undead_host` | **hors périmètre** | aucun | Les morts-vivants liés par une volonté ne sont pas des personnes du fournisseur démographique : il ne les recrute ni ne les fait vivre. |
| Serfs et esclaves | `added.dependent_labor` | **hors périmètre** | aucun | Condition personnelle et non organisation : la dépendance se porte par un statut et par les tags `slavery` et `exploitation`, pas par un archétype de groupe. |

## Archétypes ajoutés

| Archétype | Classe | Setting | Kind de nom | Rôle |
|---|---|---|---|---|
| `herding_group` | `activity_unit` | `historical` | `organization.household` | Troupeau et pâtre : unité d'activité dont le capital est le bétail. |
| `fishing_crew` | `activity_unit` | `historical` | `organization.crew` | Équipage et bateau de pêche avec quotas. |
| `woodland_crew` | `activity_unit` | `historical` | `organization.workshop.worksite` | Bûcherons, charbonniers et chasseurs de forêt. |
| `mining_crew` | `activity_unit` | `historical` | `organization.workshop.worksite` | Équipe d'extraction sous concession : mine, carrière, marais salants. |
| `shipping_consortium` | `association` | `historical` | `organization.company` | Armateurs qui mutualisent navires, cargaisons et assurance. |
| `banking_house` | `activity_unit` | `historical` | `organization.company` | Maison de dépôt, de change, de prêt et d'assurance. |
| `merchant_league` | `association` | `historical` | `organization.company.league` | Guilde marchande ou ligue de cités : privilèges, convois, arbitrage. |
| `lordship` | `institution` | `historical` | `organization.domain` | Seigneurie : justice banale, redevances et tenures sur un domaine. |
| `embassy` | `association` | `historical` | `organization.office.embassy` | Mission installée auprès d'une cour ou d'une cité étrangère, née d'un contact diplomatique. |
| `ruling_dynasty` | `lineage_view` | `historical` | `organization.house` | Lignage qui détient la charge souveraine. |
| `law_court` | `institution` | `historical` | `organization.tribunal` | Juridiction qui instruit et tranche, avec ses officiers (dont le prévôt). |
| `town_watch` | `institution` | `historical` | `organization.office.watch` | Guet et veille : patrouille, alarme, garde du feu ou des côtes. |
| `prison` | `institution` | `historical` | `organization.office.prison` | Geôle et administration pénitentiaire. |
| `inquisition` | `association` | `historical` | `organization.tribunal.inquisition` | Tribunal doctrinal né d'un procès d'hérésie. |
| `executioner_house` | `lineage_view` | `historical` | `organization.house` | Lignage chargé des exécutions et des basses œuvres de justice. |
| `militia` | `association` | `historical` | `organization.military_unit` | Force de citoyens ou de tenanciers mobilisable pour la défense locale. |
| `garrison` | `activity_unit` | `historical` | `organization.military_unit` | Détachement permanent d'un fort ou d'une ville. |
| `mercenary_company` | `activity_unit` | `historical` | `organization.company.free_company` | Force soldée qui négocie solde, durée et ravitaillement. |
| `knightly_order` | `association` | `historical` | `organization.order` | Ordre armé sous serment, devoirs d'hospitalité et patronage. |
| `war_fleet` | `activity_unit` | `historical` | `organization.military_unit.fleet` | Flotte de guerre : escadres, équipages, intendance. |
| `raiding_band` | `association` | `historical` | `organization.band` | Bande de pillage saisonnière, de la razzia à la grande horde selon l'overlay `monstrous_threat`. |
| `scout_company` | `activity_unit` | `historical` | `organization.company.free_company` | Compagnie légère de reconnaissance et d'exploration. |
| `secular_clergy` | `institution` | `historical` | `organization.cult.clergy` | Clergé de paroisse et de mission. |
| `parish_community` | `community` | `historical` | `organization.community.parish` | Communauté de fidèles d'une paroisse. |
| `monastery` | `community` | `historical` | `organization.monastery` | Maison de vie commune régie par une règle. |
| `mendicant_friary` | `association` | `historical` | `organization.order` | Fraternité sans domaine foncier vivant d'aumônes. |
| `hermitage` | `community` | `historical` | `organization.monastery` | Retraite de contemplation reliée à un village voisin. |
| `lay_confraternity` | `association` | `historical` | `organization.confraternity` | Association de laïcs pour une dévotion, des œuvres ou des secours. |
| `heterodox_movement` | `association` | `historical` | `organization.cult.movement` | Mouvement de croyance divergente. |
| `closed_sect` | `association` | `historical` | `organization.cult.sect` | Groupe fermé à emprise forte sur ses membres. |
| `ancestral_cult` | `association` | `historical` | `organization.cult` | Culte des ancêtres, des esprits du lieu ou des cycles. |
| `base_charitable_order` | `association` | `historical` | `organization.order` | Ordre ou œuvre de charité : aumône, accueil, soin. |
| `cathedral_chapter` | `association` | `historical` | `organization.council.chapter` | Chapitre de chanoines autour d'une cathédrale. |
| `pious_foundation` | `association` | `historical` | `organization.confraternity.foundation` | Fondation perpétuelle pour une intention pieuse (waqf, chantrerie). |
| `university` | `community` | `historical` | `organization.school` | Communauté d'étude sous charte : facultés, grades, débats. |
| `scriptorium` | `activity_unit` | `historical` | `organization.workshop` | Atelier de copie, d'enluminure et de conservation de manuscrits. |
| `hospital_house` | `community` | `historical` | `organization.hospital` | Maison de soin résidentielle : hôpital, léproserie, convalescence. |
| `healers_circle` | `association` | `historical` | `organization.circle` | Sages-femmes, guérisseurs et préparateurs de remèdes. |
| `itinerant_troupe` | `activity_unit` | `historical` | `organization.troupe` | Troupe de spectacle ou de ménestrels qui voyage. |
| `festive_society` | `association` | `historical` | `organization.confraternity.festive` | Abbaye de jeunesse, club de jeux, comité de fêtes. |
| `women_circle` | `friend_circle` | `historical` | `organization.circle` | Cercle informel de femmes. |
| `youth_abbey` | `association` | `historical` | `organization.confraternity.youth` | Abbaye ou classe d'âge de jeunesse. |
| `tavern_house` | `activity_unit` | `historical` | `organization.tavern` | Taverne ou auberge : hébergement, boisson, jeu. |
| `beguine_house` | `community` | `historical` | `organization.monastery.beguinage` | Maison de femmes pieuses à vie commune sans vœux perpétuels. |
| `bathhouse_brothel` | `activity_unit` | `historical` | `organization.tavern.bathhouse` | Établissement de bains et de commerce sexuel. |
| `foreign_merchant_nation` | `community` | `historical` | `organization.community.nation` | Colonie de marchands d'une autre culture installée dans un port. |
| `stigmatized_caste` | `community` | `historical` | `organization.community.caste` | Groupe héréditaire d'un métier jugé impur. |
| `base_criminal_network` | `association` | `historical` | `organization.criminal` | Réseau criminel générique (§27.6). |
| `thieves_guild` | `association` | `historical` | `organization.criminal` | Guilde urbaine de vol et de recel qui arbitre ses membres. |
| `protection_racket` | `association` | `historical` | `organization.criminal` | Famille ou bande qui rançonne un quartier. |
| `bandit_band` | `association` | `historical` | `organization.band` | Bande de brigands de grand chemin. |
| `smuggling_ring` | `association` | `historical` | `organization.criminal` | Filière de contrebande et de revente de biens volés. |
| `pirate_crew` | `association` | `historical` | `organization.crew` | Équipage hors-la-loi qui vit de prises en mer. |
| `assassin_brotherhood` | `association` | `historical` | `organization.secret_society` | Confrérie d'assassins à gages. |
| `forgery_ring` | `association` | `historical` | `organization.criminal` | Atelier clandestin de fausse monnaie, de faux et de contrefaçons. |
| `slave_trading_ring` | `association` | `historical` | `organization.company` | Maison qui capture et vend des personnes. |
| `beggar_brotherhood` | `association` | `historical` | `organization.confraternity` | Fraternité de mendiants organisée par quartier. |
| `outlaw_company` | `association` | `historical` | `organization.band` | Compagnie de proscrits : bannis, insurgés, résistants. |
| `refugee_community` | `community` | `historical` | `organization.community` | Communauté déplacée qui reçoit abri, vivres et protection. |
| `minority_enclave` | `community` | `historical` | `organization.community` | Quartier ou village d'une minorité culturelle ou religieuse. |
| `secret_society` | `association` | `historical` | `organization.secret_society` | Association fermée à signes de reconnaissance et projet caché. |
| `intelligence_network` | `association` | `historical` | `organization.secret_society` | Réseau d'informateurs et de rapports. |
| `mutual_aid_society` | `association` | `historical` | `organization.confraternity` | Société de secours entre pairs : caisse, assurance, entraide. |
| `journeymen_fellowship` | `association` | `historical` | `organization.guild` | Fraternité de compagnons et salariés d'un métier. |
| `mage_order` | `association` | `high_fantasy` | `organization.order` | Collège ou ordre de mages : apprentissage, registre, responsabilité. |
| `arcane_cabal` | `association` | `low_fantasy` | `organization.secret_society` | Cercle fermé de praticiens occultes. |
| `alchemists_lodge` | `activity_unit` | `historical` | `organization.workshop` | Atelier d'alchimistes : laboratoire, recettes, secret. |
| `witch_coven` | `association` | `low_fantasy` | `organization.circle.coven` | Cercle ouvert de praticiennes qui négocie avec la communauté. |
| `druid_circle` | `association` | `low_fantasy` | `organization.cult` | Cercle gardien d'un territoire et de ses cycles. |
| `monster_hunters_lodge` | `association` | `low_fantasy` | `organization.company.free_company` | Loge qui traque et recense les prédateurs surnaturels. |
| `adventuring_company` | `activity_unit` | `low_fantasy` | `organization.company.free_company` | Équipe sous contrat qui explore et pille des sites dangereux. |
| `holy_order` | `association` | `low_fantasy` | `organization.order` | Ordre dont les pouvoirs surnaturels servent le bien. |
| `infernal_cult` | `association` | `low_fantasy` | `organization.cult.sect` | Culte lié par pacte à une puissance hostile ou interdite. |
| `necromancers_circle` | `association` | `low_fantasy` | `organization.secret_society` | Cercle qui travaille avec les morts et leurs restes. |
| `fey_court` | `community` | `high_fantasy` | `organization.court` | Cour de puissances surnaturelles liées par hospitalité et dette. |
| `dragon_court` | `community` | `high_fantasy` | `organization.court` | Cour de puissances anciennes : tribut, territoire, serments. |
| `nonhuman_enclave` | `community` | `high_fantasy` | `organization.community` | Communauté fermée dont les membres relèvent d'un profil d'ascendance explicite. |
| `ancient_site_guardians` | `association` | `low_fantasy` | `organization.order` | Ordre qui garde un lieu ou des objets anciens. |
| `enchanters_workshop` | `activity_unit` | `high_fantasy` | `organization.workshop.arcane` | Atelier d'enchantement et de constructs. |
| `arcane_authority` | `association` | `high_fantasy` | `organization.office.arcane` | Autorité qui enregistre et régule les pratiques magiques. |

## Décision pour chaque système Pecking Order

### Systèmes reliés à un archétype

| Archétype | Systèmes |
|---|---|
| `administration` | `canal_authority`, `civil_service`, `disaster_commission`, `harbor_authority`, `ministry`, `occupied_territory`, `province`, `public_works_office`, `road_wardens`, `rural_district`, `sanitation_board` |
| `caravan` | `caravan_company`, `merchant_caravan` |
| `civic_council` | `chartered_town`, `city`, `city_state`, `free_city`, `guild_republic`, `local_council`, `merchant_republic`, `revolutionary_council`, `siege_defense_council`, `temple_city` |
| `court` | `crown_council`, `royal_court`, `royal_household`, `war_council` |
| `craft_association` | `artisan_confraternity` |
| `craft_guild` | `craft_guild` |
| `crew` | `merchant_ship_crew` |
| `household_compound` | `joint_family` |
| `kin_group` | `adoptive_house`, `clan`, `lineage`, `lineage_memory_archive`, `matrilineal_house` |
| `landed_house` | `noble_house` |
| `local_assembly` | `clan_council`, `commons_trust`, `confederation`, `hamlet`, `island_polity`, `nomadic_confederation`, `sacred_grove_moot`, `tribal_council`, `village_federation` |
| `local_colony` | `frontier_settlement` |
| `market_association` | `grain_exchange`, `labor_exchange`, `market_authority`, `public_market_board` |
| `productive_community` | `farm_collective` |
| `retinue` | `bodyguard`, `retainer_household` |
| `ritual_group` | `dream_interpretation_circle`, `exorcist_lodge`, `forge_initiation_order`, `mystery_cult`, `possession_witness_circle`, `vow_release_circle` |
| `ritual_institution` | `dream_incubation_sanctuary`, `oracle_house`, `pilgrimage_shrine`, `relic_authentication_chamber`, `sacred_calendar_council`, `solar_clock_custodians`, `temple_granary_office`, `vow_renewal_court` |
| `school` | `apprentice_school`, `diviner_apprentice_house`, `guild_school`, `village_school` |
| `village_council` | `village` |
| `ward_community` | `ward` |
| `water_association` | `irrigation_council`, `spring_spirit_compact`, `water_board` |
| `workshop` | `apprenticeship_workshop`, `cooperative` |
| `base_trade_company` | `trading_company` |
| `base_religious_community` | `cult` |
| `base_neighborhood` | `neighborhood_watch`, `roadside_lantern_brotherhood` |
| `mortuary_confraternity` | `burial_society`, `funerary_boat_brotherhood`, `funerary_guild`, `mortuary_purification_house` |
| `fishing_crew` | `fishing_cooperative` |
| `woodland_crew` | `communal_oath_hunt`, `migration_omen_watch` |
| `mining_crew` | `mining_company` |
| `shipping_consortium` | `shipping_consortium` |
| `banking_house` | `bank`, `insurance_house`, `toll_keepers` |
| `merchant_league` | `chartered_company`, `city_league`, `colonial_company`, `guild_federation`, `merchant_guild` |
| `lordship` | `barony`, `county`, `duchy`, `estate_administration`, `frontier_march`, `principality`, `temple_estate` |
| `embassy` | `diplomatic_mission` |
| `ruling_dynasty` | `dynasty` |
| `law_court` | `appeals_court`, `arbitration_chamber`, `court_of_justice`, `customary_council`, `guild_court`, `inheritance_court`, `inquest_commission`, `mediation_circle`, `village_court`, `watch_court` |
| `town_watch` | `coast_watch`, `fire_watch` |
| `prison` | `prison_administration` |
| `inquisition` | `heresy_court` |
| `militia` | `militia` |
| `garrison` | `army`, `border_guard`, `combat_company`, `company`, `garrison`, `siege_train` |
| `mercenary_company` | `mercenary_company` |
| `knightly_order` | `knightly_order`, `military_order` |
| `war_fleet` | `naval_fleet`, `naval_squadron`, `navy`, `warship_crew` |
| `raiding_band` | `monster_pack`, `raiding_party` |
| `scout_company` | `expedition`, `scout_company` |
| `secular_clergy` | `mission_station`, `priesthood` |
| `parish_community` | `congregation` |
| `monastery` | `monastery`, `religious_order` |
| `mendicant_friary` | `mendicant_almonry`, `mendicant_preaching_network` |
| `hermitage` | `hermitage` |
| `lay_confraternity` | `lay_brotherhood`, `plague_vow_fellowship` |
| `heterodox_movement` | `itinerant_prophets_movement` |
| `ancestral_cult` | `ancestor_feast_compact`, `ancestor_shrine`, `shamanic_circle` |
| `base_charitable_order` | `pass_shrine_wardens`, `pilgrimage_wayhouse_network`, `relief_society` |
| `university` | `academy`, `astral_calendar_consortium`, `observatory`, `scholars_circle`, `university` |
| `scriptorium` | `library`, `scriptorium` |
| `hospital_house` | `epidemic_recovery_house`, `healing_house` |
| `itinerant_troupe` | `theatrical_troupe`, `traveling_circus` |
| `festive_society` | `festival_council`, `procession_company`, `sports_club` |
| `base_criminal_network` | `crime_family`, `prison_gang` |
| `thieves_guild` | `thieves_guild` |
| `protection_racket` | `street_gang` |
| `smuggling_ring` | `smugglers_ring` |
| `pirate_crew` | `pirate_flotilla` |
| `assassin_brotherhood` | `assassins_order` |
| `outlaw_company` | `resistance_network`, `revolutionary_party` |
| `refugee_community` | `refuge_camp` |
| `secret_society` | `cult_cell`, `secret_society`, `underground_press` |
| `intelligence_network` | `intelligence_service`, `spy_cell`, `spy_ring` |
| `mutual_aid_society` | `credit_union`, `mutual_aid_cell`, `veterans_association` |
| `journeymen_fellowship` | `workers_union` |
| `mage_order` | `mage_college` |
| `witch_coven` | `oracle_network`, `witch_coven` |
| `druid_circle` | `druid_grove` |
| `monster_hunters_lodge` | `monster_hunters_lodge` |
| `adventuring_company` | `adventurers_guild`, `adventuring_company` |
| `fey_court` | `elemental_conclave`, `fey_court`, `spirit_court` |
| `dragon_court` | `dragon_clan`, `dragon_court`, `dragonflight` |
| `ancient_site_guardians` | `relic_custodians` |
| `enchanters_workshop` | `golem_workshop`, `guild_of_enchanters` |
| `arcane_authority` | `arcane_registry`, `demon_pact_court`, `necromantic_college`, `planar_gate_authority` |

### Systèmes exclus

- `polity_scale` (17) : Souveraineté ou gouvernement d'un État entier. Chronos modélise la polity et ses offices supérieurs (`home_domain`, §2) en dehors des archétypes de groupes locaux. Systèmes : `kingdom`, `empire`, `aristocratic_nation`, `democratic_nation`, `federation`, `tributary_realm`, `elective_monarchy`, `protectorate`, `parliament`, `republic`, `bureaucratic_state`, `theocracy`, `military_junta`, `colonial_administration`, `sacred_kingdom`, `nation`, `religious_empire`.
- `command_echelon` (8) : Niveau hiérarchique interne d'une force permanente, sans mécanique sociale distincte de `garrison` ou `war_fleet` qui le contiennent. Systèmes : `army_group`, `army_corps`, `division`, `brigade`, `regiment`, `battalion`, `platoon`, `squad`.
- `modern_form` (4) : Forme institutionnelle étrangère au cadre médiéval ou fantastique de Chronos (aviation, parti de masse, société par actions, fédération syndicale interprofessionnelle). Systèmes : `air_force`, `political_party`, `trade_union_federation`, `corporation`.
- `non_person_members` (1) : Ses membres sont des morts-vivants liés par une volonté : ce ne sont pas des personnes du fournisseur démographique, qui ne les recrute ni ne les fait vivre. Systèmes : `undead_legion`.

## Densités attendues

Effectifs attendus (monde historique) pour deux familles extrêmes, avec les étiquettes plausibles de chaque taille de site. Les bornes vérifiées figurent dans `density_expectations`.

**Famille `agrarian_estates`**

| Archétype | Hameau (50) | Village (200) | Bourg (2000) | Ville (20000) |
|---|---:|---:|---:|---:|
| `craft_guild` | 0.00 | 0.00 | 1.20 | 12.00 |
| `parish_community` | 0.00 | 1.00 | 6.00 | 60.00 |
| `secular_clergy` | 0.00 | 1.00 | 1.00 | 1.00 |
| `law_court` | 0.00 | 1.00 | 1.00 | 1.00 |
| `town_watch` | 0.00 | 0.00 | 1.00 | 1.00 |
| `prison` | 0.00 | 0.00 | 1.00 | 1.00 |
| `lordship` | 0.00 | 0.00 | 1.00 | 1.00 |
| `university` | 0.00 | 0.00 | 0.00 | 1.00 |
| `garrison` | 0.00 | 0.00 | 0.46 | 2.00 |
| `banking_house` | 0.00 | 0.00 | 2.12 | 25.47 |
| `thieves_guild` | 0.00 | 0.00 | 0.24 | 2.00 |
| `base_criminal_network` | 0.03 | 0.12 | 1.20 | 12.00 |
| `base_charitable_order` | 0.03 | 0.12 | 1.20 | 12.00 |
| `monastery` | 0.00 | 0.04 | 0.00 | 2.00 |
| `tavern_house` | 0.00 | 0.15 | 2.77 | 54.73 |
| `fishing_crew` | 0.00 | 2.14 | 16.07 | 32.14 |
| `mining_crew` | 0.00 | 0.00 | 0.00 | 0.00 |

**Famille `market_towns`**

| Archétype | Hameau (50) | Village (200) | Bourg (2000) | Ville (20000) |
|---|---:|---:|---:|---:|
| `craft_guild` | 0.00 | 0.00 | 1.20 | 12.00 |
| `parish_community` | 0.00 | 1.00 | 6.00 | 60.00 |
| `secular_clergy` | 0.00 | 1.00 | 1.00 | 1.00 |
| `law_court` | 0.00 | 1.00 | 1.00 | 1.00 |
| `town_watch` | 0.00 | 0.00 | 1.00 | 1.00 |
| `prison` | 0.00 | 0.00 | 1.00 | 1.00 |
| `lordship` | 0.00 | 0.00 | 1.00 | 1.00 |
| `university` | 0.00 | 0.00 | 0.00 | 1.00 |
| `garrison` | 0.00 | 0.00 | 0.46 | 2.00 |
| `banking_house` | 0.00 | 0.00 | 2.12 | 25.47 |
| `thieves_guild` | 0.00 | 0.00 | 0.24 | 2.00 |
| `base_criminal_network` | 0.03 | 0.12 | 1.20 | 12.00 |
| `base_charitable_order` | 0.03 | 0.12 | 1.20 | 12.00 |
| `monastery` | 0.00 | 0.04 | 0.00 | 2.00 |
| `tavern_house` | 0.00 | 0.15 | 2.77 | 54.73 |
| `fishing_crew` | 0.00 | 2.14 | 16.07 | 32.14 |
| `mining_crew` | 0.00 | 0.00 | 0.00 | 0.00 |

## Échelle morale

Distributions exactes (convolution entière), hors base culturelle des organisations. Pourcentages de slots par classe de conduite ; seuil `saintly` des organisations à 5 500.

| Archétype | lawless | crooked | ordinary | upright | saintly |
|---|---:|---:|---:|---:|---:|
| `workshop` | 0.00 % | 14.75 % | 66.49 % | 17.33 % | 1.42 % |
| `banking_house` | 0.00 % | 14.75 % | 66.49 % | 17.33 % | 1.42 % |
| `thieves_guild` | 58.88 % | 40.96 % | 0.16 % | 0.00 % | 0.00 % |
| `protection_racket` | 69.44 % | 30.56 % | 0.00 % | 0.00 % | 0.00 % |
| `mercenary_company` | 1.17 % | 42.60 % | 49.19 % | 6.78 % | 0.26 % |
| `monastery` | 0.00 % | 2.09 % | 47.91 % | 37.85 % | 12.16 % |
| `base_charitable_order` | 0.00 % | 0.00 % | 23.63 % | 52.72 % | 23.65 % |
| `holy_order` | 0.00 % | 0.00 % | 31.77 % | 51.55 % | 16.68 % |
| `infernal_cult` | 16.98 % | 73.37 % | 9.65 % | 0.00 % | 0.00 % |

Personnes (6 tirages de ±1 500 autour de l'`alignment_base` de la famille), pour les 21 familles : 

- `lawless` : de 1 sur 6797 à 1 sur 19112.
- `saintly` : de 1 sur 5030 à 1 sur 15233.
- `crooked` : 10.7 à 14.3 % ; `upright` : 10.4 à 13.9 %.

## Kinds de noms

`name_profile` porte directement un kind Namebench, éventuellement pointé avec repli sur son parent. Le kind ne dépend ni de la famille culturelle ni de la classe morale. Les kinds marqués « proposé » sont à envoyer à l'équipe Namebench.

| Kind | Origine | Archétypes |
|---|---|---|
| `organization.household` | départ Namebench | `base_farm`, `communal_house`, `farm_household`, `herding_group`, `household_compound`, `mobile_household`, `service_household` |
| `organization.house` | départ Namebench | `craft_house`, `executioner_house`, `forest_house`, `kin_group`, `landed_house`, `merchant_house`, `ruling_dynasty`, `territorial_house` |
| `organization.guild` | départ Namebench | `base_craft_association`, `craft_association`, `craft_guild`, `journeymen_fellowship`, `market_association`, `water_association` |
| `organization.confraternity` | départ Namebench | `beggar_brotherhood`, `lay_confraternity`, `mortuary_confraternity`, `mutual_aid_society` |
| `organization.order` | départ Namebench | `ancient_site_guardians`, `base_charitable_order`, `holy_order`, `knightly_order`, `mage_order`, `mendicant_friary` |
| `organization.faction` | départ Namebench | `base_faction`, `house_faction` |
| `organization.secret_society` | départ Namebench | `arcane_cabal`, `assassin_brotherhood`, `intelligence_network`, `necromancers_circle`, `secret_society` |
| `organization.criminal` | départ Namebench | `base_criminal_network`, `forgery_ring`, `protection_racket`, `smuggling_ring`, `thieves_guild` |
| `organization.company` | départ Namebench | `banking_house`, `base_trade_company`, `caravan`, `exchange_network`, `shipping_consortium`, `slave_trading_ring` |
| `organization.retinue` | départ Namebench | `retinue` |
| `organization.school` | départ Namebench | `magical_school`, `school`, `university` |
| `organization.council` | départ Namebench | `civic_council`, `hold_council`, `local_assembly`, `village_council` |
| `organization.community` | départ Namebench | `base_neighborhood`, `local_colony`, `local_community`, `minority_enclave`, `neighborhood`, `nonhuman_enclave`, `refugee_community`, `shore_community`, `travel_party`, `ward_community` |
| `organization.workshop` | proposé | `alchemists_lodge`, `base_workshop`, `scriptorium`, `workshop` |
| `organization.crew` | proposé | `crew`, `fishing_crew`, `pirate_crew` |
| `organization.band` | proposé | `bandit_band`, `outlaw_company`, `raiding_band` |
| `organization.military_unit` | proposé | `garrison`, `militia` |
| `organization.office` | proposé | `administration`, `base_local_institution`, `redistributive_institution` |
| `organization.tribunal` | proposé | `law_court` |
| `organization.court` | proposé | `court`, `dragon_court`, `fey_court` |
| `organization.domain` | proposé | `lordship` |
| `organization.cult` | proposé | `ancestral_cult`, `base_religious_community`, `druid_circle`, `ritual_group`, `ritual_institution` |
| `organization.monastery` | proposé | `hermitage`, `monastery` |
| `organization.hospital` | proposé | `hospital_house` |
| `organization.troupe` | proposé | `itinerant_troupe` |
| `organization.tavern` | proposé | `tavern_house` |
| `organization.circle` | proposé | `base_friend_circle`, `companionship`, `healers_circle`, `women_circle` |
| `organization.circle.coven` | sous-kind de `organization.circle` | `witch_coven` |
| `organization.community.caste` | sous-kind de `organization.community` | `stigmatized_caste` |
| `organization.community.nation` | sous-kind de `organization.community` | `foreign_merchant_nation` |
| `organization.community.parish` | sous-kind de `organization.community` | `parish_community` |
| `organization.company.free_company` | sous-kind de `organization.company` | `adventuring_company`, `mercenary_company`, `monster_hunters_lodge`, `scout_company` |
| `organization.company.league` | sous-kind de `organization.company` | `merchant_league` |
| `organization.confraternity.festive` | sous-kind de `organization.confraternity` | `festive_society` |
| `organization.confraternity.foundation` | sous-kind de `organization.confraternity` | `pious_foundation` |
| `organization.confraternity.youth` | sous-kind de `organization.confraternity` | `youth_abbey` |
| `organization.council.chapter` | sous-kind de `organization.council` | `cathedral_chapter` |
| `organization.cult.clergy` | sous-kind de `organization.cult` | `secular_clergy` |
| `organization.cult.movement` | sous-kind de `organization.cult` | `heterodox_movement` |
| `organization.cult.sect` | sous-kind de `organization.cult` | `closed_sect`, `infernal_cult` |
| `organization.military_unit.fleet` | sous-kind de `organization.military_unit` | `war_fleet` |
| `organization.monastery.beguinage` | sous-kind de `organization.monastery` | `beguine_house` |
| `organization.office.arcane` | sous-kind de `organization.office` | `arcane_authority` |
| `organization.office.embassy` | sous-kind de `organization.office` | `embassy` |
| `organization.office.prison` | sous-kind de `organization.office` | `prison` |
| `organization.office.watch` | sous-kind de `organization.office` | `town_watch` |
| `organization.tavern.bathhouse` | sous-kind de `organization.tavern` | `bathhouse_brothel` |
| `organization.tribunal.inquisition` | sous-kind de `organization.tribunal` | `inquisition` |
| `organization.workshop.arcane` | sous-kind de `organization.workshop` | `enchanters_workshop` |
| `organization.workshop.service` | sous-kind de `organization.workshop` | `base_service_unit`, `service_group` |
| `organization.workshop.worksite` | sous-kind de `organization.workshop` | `mining_crew`, `productive_community`, `resource_group`, `woodland_crew` |

## Limites assumées

- Les densités, stocks, seuils et tailles sont des valeurs de départ à évaluer dans les bilans ; les bornes du vérificateur détectent les dérives grossières, pas la justesse historique.
- La formule de `conformity_bps` est une proposition de S1 : la spec §27.4 ne la fixe pas.
- Les activités restent `agriculture`, `craft`, `trade`, `services` (§24.1).
- `catalogue_version` n'a pas été modifié : la décision d'un nouveau numéro revient à l'intégrateur.
