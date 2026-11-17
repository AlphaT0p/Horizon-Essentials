# Horizon Essentials

Extension complémentaire pour [Horizon](https://playhorizon.dev) (Minecraft: Java Edition 26.3 réécrit en Rust) :
un mod natif (`.hnmod`) qui ajoute ce que Horizon n'a pas encore, sans refaire ce qu'il a déjà
(viseur, Hit Color, armure, effets, ping, éditeur de HUD… sont déjà intégrés à Horizon).

Cible : **Horizon Nightly 2026-11-13 · `9f42064`** (hook points `NATIVE_HOOKS_API = 415`).

## Fonctionnalités prévues

| Fonction | Ce qu'elle ajoute à Horizon | État |
|---|---|---|
| Essentials Core | Registre de modules, configuration résiliente et persistante, E/S hors du rendu | ✅ |
| Module témoin | Une boîte qui prouve que le Core tourne | ✅ |
| Gestion des modules | Page Options > Mods, commande `/essentials` | en cours |
| Menu Échap | Panneau latéral dessiné sur le menu pause | prévu |
| HUD Customizer | Placement, taille, couleurs de chaque widget, éditeur glisser-déposer | prévu |
| HUD Profiles | PvP, Survival, Minimaliste, Personnalisé | prévu |
| FPS Statistics, CPS History, Advanced Keystrokes, Memory Display | Modules PvP | prévu |

## Organisation du dépôt

```
mixins/horizon_essentials/   le mod (un seul bundle horizon-mixins)
  horizon-mixin.toml           manifeste : modules et hooks
  core_*.rs                    Essentials Core (types, registre, config, stockage, état, événements)
  ui_*.rs                      interface (placement, plus tard menu et HUD Editor)
  mod_*.rs                     modules (un fichier par module)
  h_*.rs                       helpers : la colle avec les hook points de Horizon, un par hook
harness/lib.rs               banc de test : `cargo test` sans compiler Horizon
bench/                       protocole de mesure des performances
compatibility.json           commit Horizon et horizon-mixins testés
docs/                        architecture, API vérifiées, environnement, performances
```

## Compiler et tester

```bash
cargo test                                               # logique (Windows ou Linux), quelques secondes
cargo clippy --all-targets -- -D warnings
cargo run --release --manifest-path bench/Cargo.toml     # mesures de performance
```

Vérification contre le code de Horizon et build du jeu : voir [docs/environnement.md](docs/environnement.md).

## Documentation

- [Architecture](docs/architecture.md)
- [API Horizon vérifiées](docs/api-verifiee.md)
- [Environnement de développement](docs/environnement.md)
- [Performances](docs/performance.md)

Projet personnel et d'apprentissage, non affilié à Horizon, Mojang ou Microsoft. Licence MIT.
