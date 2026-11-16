# Architecture (validée le 2026-11-16)

## Principes

1. **Un seul bundle** `mixins/horizon_essentials/`, mod id `horizon_essentials`, plusieurs `[[module]]`.
   Les bundles mixin n'ont pas de système de dépendances : un Core dans un autre bundle casserait la compilation s'il manquait.
2. **Modules = Rust pur** (aucun type du jeu) ; **helpers = colle** de 5 à 15 lignes : lire le jeu, appeler un module, dessiner.
   Les modules se testent donc avec `cargo test`, sans Horizon.
3. **Un helper par hook**, qui distribue aux widgets (ordre maîtrisé, une seule injection par hook).
4. **Aucune lecture disque pendant une frame** ; écriture différée au tick, atomique.
5. **Zéro allocation par frame** quand rien ne change : formes et largeurs de texte en cache, buffers circulaires de taille fixe.
6. Le code interne non stable (`scale_since`, `process_memory`) n'est appelé que dans **un** helper chacun, avec repli.

## Modules

| Fichier | Module dans le jeu | Rôle |
|---|---|---|
| `core.rs` | `crate::game::essentials_core` | état global, répartition des événements, commande, sauvegarde différée |
| `config.rs` | `essentials_config` | modèle serde, défauts, bornes, migration, repli si corrompu |
| `profiles.rs` | `essentials_profiles` | 4 préréglages, changement de profil, copie vers Personnalisé |
| `draw.rs` | `essentials_draw` | `Shape`, cache, layout (ancre + fractions), opacité |
| `fps.rs` | `essentials_fps` | temps de frame : min, max, moyenne, 1 % low, graphe |
| `cps.rs` | `essentials_cps` | horodatage des clics : actuel, pic, moyenne, historique |
| `keystrokes.rs` | `essentials_keystrokes` | WASD, espace, Shift, clics, CPS ; styles par touche |
| `memory.rs` | `essentials_memory` | formatage mémoire, échantillonnage 1×/s |
| `editor.rs` | `essentials_editor` | éditeur : sélection, glisser, molette, reset |

## Flux

```
native_frame ─► core.on_frame(dt) ─► fps
native_key   ─► core.on_key       ─► cps / keystrokes / touche éditeur
native_tick  ─► core.on_tick      ─► memory (1×/s), sauvegarde si modifié depuis ≥ 1 s
native_hud   ─► core.draw         ─► widget sale ? reconstruire : réutiliser ─► helper peint
Options>Mods ─► list / settings / click ─► config ─► « à sauvegarder »
Éditeur      ─► screen_* ─► editor ─► config.layout
```

## Configuration

`horizon-mods/.horizon-mod-data/horizon_essentials/` :

```
active                       nom du profil actif
profiles/pvp.json survival.json minimal.json custom.json
```

JSON versionné (`"version": 1`), positions en fractions de la place libre, taille 50–200 %.

## Ordre de développement

0. Environnement + compilation du mod officiel `fps-overlay` inchangé (porte d'entrée)
1. Core + config  2. Draw  3. FPS  4. CPS  5. Keystrokes  6. Memory  7. Profils  8. Éditeur  9. Mesures, docs, paquet

Chaque étape : `cargo test` ✔, `native.py check` ✔, contrôle en jeu (F1, F3, solo, serveur, mod coupé), FPS avec/sans.
