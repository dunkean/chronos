# Revue d’intégration du prototype

Date : 21 octobre 2026. Périmètre : première tranche locale utilisable, pas l’intégralité des travaux de recherche de la roadmap.

## Travail et revue

La roadmap a été relue et corrigée par Claude Opus 5.5 en effort high via CLI ; le [rapport initial](roadmap-opus-5.5.md) est conservé. Trois agents Opus 5.5/high ont réalisé le moteur Rust, les adaptateurs/données et l’interface, sur des périmètres distincts. La revue principale a examiné le code, demandé les corrections ci-dessous et exécuté les contrôles d’intégration.

## Corrections retenues

- Séparer le scénario pivot imposé de l’histoire libre : des intensités nulles ne doivent pas recevoir une guerre ou une épidémie scriptée.
- Conserver l’assemblée collective issue de Pecking Order ; ne pas transformer une assemblée en personne ni simuler sa succession individuelle.
- Interpréter une liste fermée d’identifiants de succession, jamais une description textuelle par recherche de sous-chaîne.
- Préserver vies et filiations Lineage Lab ; arrêter les rôles importés hors de la couverture 1300–1420 et afficher un statut inconnu après la fin d’une trace sans décès enregistré.
- Garder un rôle parental stable pour chaque personne virtuelle.
- Employer les pools de noms des cultures propres aux sites importés et afficher le nombre de sites réellement appliqué.
- Refuser un fichier dont les faits ont été altérés sous le même canon ; distinguer données canoniques et temps de génération.
- Afficher la source de carte effectivement utilisée, y compris un éventuel repli virtuel.
- Réduire l’emprise de la légende, séparer le millésime des outils sur mobile et synchroniser fiche/année lors d’un changement du fragment d’URL.

## Génération de référence

| Élément | Résultat |
|---|---|
| Canon | `canon-3fa7f6a56d5ce200` |
| Seed et période | 42 ; 1300–1500 ; scénario `pivot` |
| Carte | Source initiale Neural Earth/Orogen ; planète de diamètre 1 600 km ; quatre tuiles de 100 km |
| Sites et liaisons | 52 ; 71 |
| Événements | 325 |
| Personnes | 1 880, dont 1 846 importées de Lineage Lab ; les autres sont signalées virtuelles |
| Institutions | Six modèles importés de Pecking Order, dont une assemblée collective |
| Noms | Pools français, britannique et fantasy préparés depuis Namebench |
| Fixture SHA-256 | `f92d2d8e3111bbf7b3c1091c4d88bd0709a70dd2c01f6b6e5456b2c73bb4aa85` |

Une requête de génération release mesurée localement a pris environ 17 ms pour ce cas déjà préparé. Cette mesure ne comprend ni la préparation des sources voisines ni une génération planétaire ; elle ne constitue pas une campagne de benchmark.

## Contrôles exécutés

```text
cargo test --workspace                                      10 tests réussis
cargo fmt --all -- --check                                  OK
cargo clippy --workspace --all-targets -- -D warnings        OK
cargo check -p chronos-core --target wasm32-unknown-unknown   OK
node --check sur les modules web                            OK
python tools/validate_fixture.py                            OK
```

API réelle : rejeu du même canon ; histoire libre avec guerres/épidémies à zéro ; monde virtuel sans source faussement importée ; export/réimport ; fichier altéré refusé avec HTTP 400 ; configuration hors limites refusée avec HTTP 400 ; office collectif sans titulaire personnel. Le scénario pivot est remis en place après les essais.

Chrome : ouverture, génération depuis les réglages, filtre d’année, peste de 1356 et bilan avant/après, héros de 1398 et liens familiaux, instance collective, dialogue des sources, téléchargement et réimportation du JSON. Une personne sans décès enregistré affiche « Inconnu » en 1450, après la couverture de 1420. Les liens directs dans l’onglet mettent à jour la fiche et l’année. Aucun message d’erreur de page ou de console.

Affichage bureau 1440 × 1000 et écran étroit 390 × 844 : carte visible, légende repliée, réglages initialement repliés sur mobile, aucun débordement horizontal ni chevauchement entre année et outils.

Les fichiers de contrôle locaux sont conservés dans `output/` (ignoré par Git) : `validation-api.json`, `chronos-export.json`, `chronos-desktop.png`, `chronos-hero.png` et `chronos-mobile.png`.

## Limites assumées

Le canon est généré intégralement pour une zone limitée ; le noyau compile pour WASM, mais l’atelier livré utilise l’exécutable natif. Les ressources sont simplifiées, les routes figées, les offices personnels sans durée de mandat fixe et les instances collectives non détaillées. La démographie de l’échantillon Lineage Lab ne se recalcule pas en fonction des crises Chronos. Le relief est grossier et n’apporte ni réseau de rivières ni lacs détaillés. Voir la [roadmap](../../Roadmap.md) pour les travaux suivants.
