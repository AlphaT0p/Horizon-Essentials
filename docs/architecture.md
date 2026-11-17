# Architecture

## Principes

1. **Un seul bundle** `mixins/horizon_essentials/`, mod id `horizon_essentials`, plusieurs `[[module]]`
   (visibilité `pub`, donc pas d'avertissement « code mort » pour ce que le jeu n'appelle pas encore).
   Les bundles mixin n'ont pas de système de dépendances : un Core dans un autre bundle casserait la
   compilation s'il manquait.
2. **Modules = Rust pur** (aucun type du jeu) ; **helpers = colle** de 5 à 40 lignes : lire le jeu,
   appeler le Core, peindre. Les modules se testent avec `cargo test`, sans Horizon.
3. **Un helper par hook**, qui passe par `essentials_events` (ordre maîtrisé, une injection par hook).
4. **L'interface ne modifie rien elle-même** : elle produit des `Action`, que seul le Core applique.
5. **Le thread du jeu ne touche jamais le disque** : chargement et sauvegarde passent par le thread
   « Essentials I/O ».
6. **Zéro allocation par frame** en régime établi ; un module désactivé n'est jamais appelé.
7. Fichiers préfixés plutôt qu'en sous-dossiers (le format des bundles interdit `mod` et `include!`) :
   `core_` (Core), `ui_` (interface), `mod_` (modules), `profiles_` (profils), `h_` (helpers).

## Fichiers

| Fichier | Module dans le jeu | Rôle |
|---|---|---|
| `core_types.rs` | `crate::game::essentials_types` | constantes, ancres, `Action`, `Shapes` (formes réutilisées), trait `Module` |
| `core_registry.rs` | `essentials_registry` | `ModuleId`, table `MODULES`, `Modules` + `get_mut` (seule porte vers un module) |
| `core_config.rs` | `essentials_config` | modèle versionné, lecture résiliente champ par champ, `Resolved` pour le rendu |
| `core_store.rs` | `essentials_store` | fichiers (`active`, `profiles/<nom>.json`), écriture atomique, `.bak`, thread d'E/S |
| `core_state.rs` | `essentials_state` | état unique par thread, chargement asynchrone, sauvegarde différée, fermeture |
| `core_events.rs` | `essentials_events` | frame / tick / key / apply / hud / shutdown : la seule porte des helpers |
| `ui_draw.rs` | `essentials_draw` | placement automatique (ancres) ou libre (fractions de la place libre) |
| `mod_status.rs` | `essentials_status` | module témoin |
| `h_list.rs` | hook `native_mods` | ligne dans Options > Mods |
| `h_frame.rs` | hook `native_frame` | temps qui passe, réponses du thread d'E/S |
| `h_tick.rs` | hook `native_tick` | modules, messages du Core dans le chat |
| `h_key.rs` | hook `native_key` | touches et boutons de souris |
| `h_hud.rs` | hook `native_hud` | dessin des widgets (coins via les `Corners` de Horizon) |
| `h_stopping.rs` | hook `native_client_stopping` | dernier changement écrit à la fermeture |

## Ajouter un module

1. Écrire `mod_<nom>.rs` (implémente `Module`), l'ajouter à `horizon-mixin.toml` et à `harness/lib.rs`.
2. Dans `core_registry.rs` uniquement : une variante de `ModuleId`, une ligne de `MODULES`, un champ de
   `Modules` et une branche de `get_mut`.

La configuration (réglages par défaut), l'activation, le HUD et l'interface le prennent en compte seuls.

## Flux

```
native_frame ─► events::frame ─► état : réponses d'E/S, sauvegarde différée ─► modules actifs
native_tick  ─► events::tick  ─► modules actifs ; un message du Core vers le chat
native_key   ─► events::key   ─► modules actifs
native_hud   ─► events::hud   ─► widget actif : formes en cache ou reconstruites ─► helper peint
interface    ─► Action ─► events::apply ─► config modifiée ─► Resolved recalculé, sauvegarde dans 1 s
fermeture    ─► events::shutdown ─► dernier changement envoyé, thread d'E/S attendu (2 s max)
```

## Configuration

Dans le dossier de l'instance (jamais dans celui d'une version : survit aux recompilations et mises à jour) :

```
horizon-mods/.horizon-mod-data/horizon_essentials/
  active                     nom du profil actif
  profiles/<nom>.json        un profil
  profiles/<nom>.json.bak    copie d'un fichier qui était invalide
```

```json
{ "version": 1,
  "modules": { "status": {
      "on": true,
      "layout": { "anchor": "top_right", "auto": true, "x": 0.0, "y": 0.0, "scale": 100 },
      "style":  { "background": 47, "text": "FFFFFF", "shadow": true } } } }
```

- Lecture résiliente : fichier illisible → valeurs par défaut ; champ invalide → seul ce champ prend sa
  valeur par défaut ; module inconnu → conservé ; module absent → ajouté ; un avertissement dans le chat.
- Écriture atomique (fichier temporaire puis renommage), 1 s après le dernier changement, par le thread
  d'E/S ; une demande arrivée pendant une écriture n'est jamais perdue.

## Ordre de développement

Étape 1 (Core) : 1 structure ✅, 2 types/registre/config ✅, 3 stockage/état ✅, 4 événements/rendu/témoin ✅,
optimisation E/S (branche `perf/io-thread`), 5 gestion des modules (Options > Mods, `/essentials`).
Puis : menu Échap, HUD Editor, profils, modules PvP.
