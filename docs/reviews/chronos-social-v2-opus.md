# Revue indépendante Opus — Chronos social v2

6 novembre 2026. Compte rendu de la relecture du [dossier d’implémentation](../design/chronos-social-v2/README.md).

Les dix fichiers concernés ont été soumis intégralement à **Claude Opus 5.5** lors de quatre relectures successives. Un cinquième appel vérifie les différences exactes des quatre fichiers corrigés depuis la quatrième revue ; les six autres sont identiques. Un sixième confirme l’application des dernières éditions précises demandées au §26.6 ; les neuf autres fichiers restent identiques. Ces appels du CLI Claude sont en lecture seule, sans outils ni modification des fichiers par le relecteur. Le modèle demandé était `opus` ; les six résultats indiquent le modèle effectif `claude-opus-5-5`. Les contrôles locaux ont confirmé les dix empreintes finales ci-dessous après réception du verdict.

Ce rapport retranscrit le verdict indépendant et consigne les vérifications. Il accompagne une spécification proposée ; l’implémentation sociale v2 n’a pas commencé.

## Traitement des revues

| Tour | Résultat et traitement |
|---|---|
| 1 | Blocages entre canon, vies, recrutements et frontières : contrats autonomes, ordre causal, crises, résidence et table complète de candidats précisés. |
| 2 | S0–S2 prêts ; compléments sur le catalogue de base, les ménages, la mortalité, la mobilité et les variantes déterministes du noyau intégrés au §25. |
| 3 | Architecture transmissible, aucun blocage architectural ; cinq compléments locaux et neuf remarques mineures intégrés au §26 et aux JSON. |
| 4 | Prêt à transmettre ; derniers compléments locaux sur cohésion, plafond des gardiens et nœuds de base indépendants des crises intégrés avant livraison. |
| 5 | Contrats fermés sur le fond ; ordre de sélection des crises et état après scission précisés suivant les éditions exactes demandées. |
| 6 | Confirmation ciblée de ces éditions et des contrôles de reliquats ; verbatim ci-dessous. |

Les corrections du troisième tour ajoutent le graphe de mobilité au plan autonome, affectent les dépendants fondateurs à des gardiens sans inventer de filiation, distinguent paire potentielle et union effective, complètent les dix archétypes de base et définissent les 21 templates démographiques avec identité dérivée des paramètres. Les conversions d’effets, phases, projections de marqueurs, sièges et consignes de reprise sont alignées. Les commentaires utilisateur des deux discussions sont conservés.

Les corrections du quatrième tour définissent la variation annuelle de cohésion et son ordre de calcul, transmettent le plafond des gardiens dans le profil de parenté du plan, retirent les frontières de crise de la grille de marqueurs, précisent les variantes culturelles par épisode et alignent les canaux d’influence. Le correcteur de fécondité agit exclusivement sur les paires actives. Le cinquième tour contrôle ces modifications.

À la suite du cinquième tour, les crises retenues sont ordonnées par identifiant UTF-8 ; après une scission, cohésion initiale, compteurs et prochaine année éligible des deux groupes sont explicites. La constante de version du déclencheur et la responsabilité de la couche sociale Rust sont nommées. Les contrôles locaux confirment l’absence des anciens nœuds de crise et la validité de toutes les variantes culturelles de l’exemple.

## Vérifications effectuées localement

- Schéma JSON Schema Draft 2020-12 valide et exemple conforme.
- Treize mutations invalides rejetées : choc sans cible, capacité nulle, part du noyau excessive, trop de tentatives, profondeur excessive, pont sans plan, plan de pont sur port statistique, champ inconnu, choc encodé dans la base, coordonnée flottante, graphe de mobilité absent, trajet négatif et plafond des gardiens absent.
- 79 styles Magna Urbis reliés une seule fois à 21 familles ; empreinte du catalogue source contrôlée ; 53 archétypes dont dix bases, 21 templates démographiques et références culturelles contrôlés.
- Titres institutionnels : 41 correspondances explicites Pecking Order, 38 replis génériques déclarés.
- Identité de société de l’exemple recalculée depuis ses paramètres et poids ; champs canoniques sans flottants.
- Liens locaux, clôture des blocs et exemples JSON des sept Markdown contrôlés ; `git diff --check` passé.

Ces contrôles vérifient les documents et fixtures. Les recettes démographiques, tests de rejeu, compilations et mesures de performance du nouveau moteur restent à exécuter pendant les lots S0–S8. Aucun code moteur n’a changé dans cette livraison.

## Corpus exact du dernier tour

SHA-256 des octets UTF-8 soumis au relecteur, avant normalisation éventuelle des fins de ligne par Git.

| Fichier | SHA-256 |
|---|---|
| [README.md](../../README.md) | `08561a5465dc7537d653586b9c954755c26aa3e774de8751629a8273da8a4567` |
| [AGENTS.md](../../AGENTS.md) | `feae0d55f0ce336ca2d362ce5f93eac2550b20737f19ce69bd3b97b8677c8056` |
| [CLAUDE.md](../../CLAUDE.md) | `eb0c67d6470bdd79bb5fd4d74610dc7c9910c5728ab9c924aab3a4e18dd775ad` |
| [docs/suivi-implementation.md](../../docs/suivi-implementation.md) | `18ef5528eebc82891e80af16aaccf82f56b1d2046d7030e10f7e45d9fc5ce746` |
| [docs/discussion-groupes-sociaux.md](../../docs/discussion-groupes-sociaux.md) | `4c057e56a0d32b2b7e7755082565bc706ab59b1d1cfc2f93852e09e01144228a` |
| [docs/retour-commentaires-groupes-sociaux.md](../../docs/retour-commentaires-groupes-sociaux.md) | `2e25437d1c3ba361c0b08fd70118529c35abad73677e94245363099c8934eb5b` |
| [docs/design/chronos-social-v2/README.md](../../docs/design/chronos-social-v2/README.md) | `456b064c270f4a7f27c29faeaf39651ad006b33c0e58f89db13fff6be82c35e0` |
| [docs/design/chronos-social-v2/culture-bindings.v1.json](../../docs/design/chronos-social-v2/culture-bindings.v1.json) | `853e55c22033555c071b957e2fb297e20f51bcfdd4b5e68f2440ab64eb40a9e7` |
| [docs/design/chronos-social-v2/population-plan.schema.json](../../docs/design/chronos-social-v2/population-plan.schema.json) | `0495fbfd2267405d594c39c0c9b4c649c8fe2aa74239f1bdb4391496d3f2d738` |
| [docs/design/chronos-social-v2/village-population-plan.example.json](../../docs/design/chronos-social-v2/village-population-plan.example.json) | `47c0e1e5e39267537fb70a3474794c457b248f695a2255465b791478f358ef1f` |

## Rapport du sixième tour — texte du relecteur

> **Verdict : oui, les deux éditions sont correctement appliquées et le design peut être transmis. Cette fermeture n'identifie aucune autre correction documentaire indispensable.**
>
> - **Sélection des crises : appliquée.** La précision « selon l'ordre UTF-8 » renforce le déterminisme.
> - **État après scission : appliqué.** Cohésion initiale, compteurs et cooldown sont définis. `next_split_year` rend explicite la non-éligibilité des deux groupes, sans contredire le texte demandé. La cohésion du groupe source n'étant pas réinitialisée, elle conserve C(Y).
> - **Reliquats : fermés sur la foi de vos contrôles.** Les ajouts du §26.6 couvrent bien `split_rule_version` et l'attribution de `cohesion_effects` à la couche sociale Rust. Le diff n'introduit aucune contradiction avec le §26.6 ; la ligne vide finale est sans effet.
>
> **Limites de portée :** ma vérification repose uniquement sur le diff, le §26.6 final et vos déclarations. Je n'ai lancé aucun outil, relu aucun fichier, recalculé aucune empreinte ni rejoué aucune assertion. L'identité des neuf autres fichiers et les contrôles locaux restent donc déclarés, non vérifiés par moi.
>
> Cette fermeture ne prouve pas le comportement du moteur : les recettes prévues restent à exécuter pendant l'implémentation, et les réglages de cohésion à mesurer en S4.

## Reçus techniques

Métadonnées extraites des résultats du CLI, sans données de compte ni de session :

```json
[
  {
    "round": 1,
    "model_reported": [
      "claude-opus-5-5"
    ],
    "duration_ms": 583280,
    "review_sha256": "7d74d45e03278840033935e91204260a57e72526c53ec8227186c42b7dfec269"
  },
  {
    "round": 2,
    "model_reported": [
      "claude-opus-5-5"
    ],
    "duration_ms": 383879,
    "review_sha256": "405c762a530a05c4defb67cd4d55300d71bd38ffb3789ce44b281371fc30038e"
  },
  {
    "round": 3,
    "model_reported": [
      "claude-opus-5-5"
    ],
    "duration_ms": 282670,
    "review_sha256": "51f240489b997f3d63a83e1b81a44b2ea19ffc06d2b7882034e22baad6d40ce4"
  },
  {
    "round": 4,
    "model_reported": [
      "claude-opus-5-5"
    ],
    "duration_ms": 178744,
    "review_sha256": "69aee4f9470fa9fd732b2e6327f9ee03172956eb2bf823e97f382190e7d40157"
  },
  {
    "round": 5,
    "model_reported": [
      "claude-opus-5-5"
    ],
    "duration_ms": 101918,
    "review_sha256": "71d3a3ece25b91aa21fcc290ed640bb21c55ef992f5d1122a49c575362fb03a0"
  },
  {
    "round": 6,
    "model_reported": [
      "claude-opus-5-5"
    ],
    "duration_ms": 11257,
    "review_sha256": "5b9bc36505e57763f0ae53bdb77d3c3f66ee573937cdbbcd8e63656bae35890c"
  }
]
```
