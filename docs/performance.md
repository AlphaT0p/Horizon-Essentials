# Performances

## Règles

- Aucune lecture ni écriture disque sur le thread du jeu (rendu) : le thread « Essentials I/O » s'en charge.
- Zéro allocation par frame, tick ou touche en régime établi.
- Un module désactivé n'est jamais appelé.
- Rien n'est optimisé sans mesure avant/après avec le même protocole.

Les trois premières règles sont vérifiées à chaque `cargo test` (`core_events.rs`, tests
`zero_allocation_par_frame`, `un_module_desactive_n_est_jamais_appele`,
`aucune_operation_disque_sur_le_thread_du_jeu`).

## Protocole (`bench/`)

`cargo run --release --manifest-path bench/Cargo.toml` compile les modules **hors mode test**
(comme dans le jeu), avec un allocateur qui compte les allocations du thread courant :

| | Mesure |
|---|---|
| P1 | frame + HUD en régime établi, 2 000 000 frames : ns et allocations par frame, témoin actif puis désactivé |
| P2 | tick : ns et allocations par tick |
| P3 | changement de config puis frames jusqu'à l'écriture du fichier : durée de la pire frame (20 essais) |
| P4 | première frame d'un thread neuf avec un fichier de config présent (20 essais) |

Le « jeu » est simulé (mesure de texte et peinture factices) : on mesure le coût de notre code seul.
Le dessin réel revient à Horizon (`native_fill`, `native_text`), comme pour ses mods officiels.

## Résultats (Windows 11, i5-11400H)

Commit 4 (`1e4d857`) contre la branche `perf/io-thread`, 3 passes alternées, médianes :

| Mesure | Avant | Après |
|---|---|---|
| P1 frame + HUD, témoin actif | 53-57 ns, 0 alloc | 51-58 ns, 0 alloc |
| P1 frame + HUD, témoin désactivé | 43-50 ns, 0 alloc | 44-46 ns, 0 alloc |
| P2 tick | 21-22 ns, 0 alloc | 21-23 ns, 0 alloc |
| P3 pire frame pendant une sauvegarde | **1 484-1 581 µs** | **18-23 µs** |
| P4 première frame (chargement) | **339-385 µs** | **40-43 µs** |

À 1 250 FPS une frame dure 800 µs : notre code en prend moins de 0,01 %.

## Démarrage mesuré (17/11, instance Essentials Dev, même machine)

Commande `horizon-launcher play --instance essentials-dev`, du lancement de la commande jusqu'à la
ligne « first frame after » du journal (menu principal) ; mémoire relevée 2 s après « rest-ready » ;
fermeture propre par la fenêtre.

| Build | Lancement | Play → processus | Play → menu | Mémoire (working set / privée) |
|---|---|---|---|---|
| Nightly classique | 2e | 5,5 s | **7,4 s** | 552 / 965 Mo |
| Natif sans Essentials (fps-overlay officiel) | 2e | 6,0 s | 13,5 s ¹ | 916 / 1 144 Mo |
| Essentials commit 4 | 2e | 5,5 s | **7,4 s** | 895 / 1 129 Mo |
| Essentials `perf/io-thread` | 1er (cache reconstruit) | 18,1 s | 60,2 s | 963 / 2 295 Mo |
| Essentials `perf/io-thread` | 2e / 3e | 5,9 / 5,7 s | 14,2 ¹ / **7,7 s** | 963-1 026 / 1 410-1 657 Mo |

¹ Attente de 5,7-6,2 s dans la phase « first frame after » de Horizon, vue avec et sans Essentials.

- À chaud, un build avec Essentials démarre comme Horizon classique (7,4-7,7 s, dont ~5,5 s de
  préparation par le launcher avant le démarrage du jeu).
- Le premier lancement d'un nouveau build (≈ 60 s) et le passage d'une version à une autre dans la
  même instance (≈ 18 s, le launcher recopie les fichiers du jeu) relèvent du launcher.
- Un build natif occupe ~350 Mo de plus qu'une Nightly, **avec ou sans Essentials** : c'est la
  compilation native du launcher (profil `fast`, binaire de 147 Mo contre 107 Mo), pas notre code.

## Démarrage du jeu

Mesuré dans les journaux du jeu (`logs/latest.log`) et les rapports du launcher (`benchmarks/`).

| Lancement | « core-ready » | Ressources chargées | Play → 1re image |
|---|---|---|---|
| Nightly classique, lancements suivants | 0,7 s | 0,9 s | 5,0-8,0 s |
| Build natif, **1er** lancement (avec ou sans Essentials) | 28-34 s | 55-60 s | 35-40 s |
| Build natif, 2e lancement (fps-overlay officiel) | 0,7 s | 1,2 s | 5,1 s |

Cause, dans le code de Horizon (`horizon-core/src/prepared_files.rs`) : chaque build natif crée
un nouveau dossier de version (copie de 1,3 Go). Le cache `horizon-data.bin` copié contient le
chemin de l'ancien dossier, donc il est refusé ; le premier lancement relit alors ~22 000 fichiers
un par un (chacun analysé par Windows Defender) puis réécrit le cache. Les lancements suivants
sont aussi rapides qu'un Horizon classique. Notre code n'y est pour rien : il ne fait rien avant la
première frame, puis demande la lecture de sa configuration au thread d'E/S (~40 µs).
