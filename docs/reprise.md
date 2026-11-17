# Reprise du développement

État au 17/11/2026, 13 h 15. Développement mis en pause au milieu du **commit 5** (gestion des modules) :
code terminé et testé automatiquement, build installé, **aucun test en jeu fait**.

## Branches

| Branche | Dernier commit | Rôle |
|---|---|---|
| `main` | `31e500b` = `origin/main` (GitHub) | Core 1 à 4, optimisation E/S, docs et bench — tout ce qui est poussé |
| `perf/io-thread` | `31e500b` | Conservée (déjà intégrée à `main` par avance rapide) |
| **`feat/gestion-modules`** | **`26595cf`** (ce rapport) ; dernier code : `dc8567c` | **Commit 5, local, non poussé** — c'est ici qu'il faut reprendre |

Stash conservé (ne pas supprimer sans accord) : `stash@{0}` « commit 5 en attente : page de réglages et
/essentials » — les 5 fichiers d'origine, aussi présents (identiques) dans le commit `18449b3`.

## Commits du commit 5 (`main..feat/gestion-modules`)

| Commit | Contenu |
|---|---|
| `18449b3` (1/n) | Restauration des 5 fichiers mis de côté (`git stash apply`, identiques octet pour octet) |
| `45207c7` (2/n) | `essentials_events::view` : instantané en lecture seule du Core, None pendant le chargement |
| `e5fa772` (3/n) | Page Options > Mods et `/essentials` branchées au Core ; « Chargement de la configuration… » ; modules `ui_settings` / `ui_command` et hooks `native_settings`, `native_settings_click`, `native_command` déclarés |
| `d82c898` (4/n) | `/essentials reset` → avertissement puis `/essentials reset confirm` sous 15 s ; `reset <module>` ; « Confirmer (15 s) » sur la page |
| `dc8567c` (5/n) | `harness/integration.rs` : tests page ↔ commande ↔ Core avec le vrai thread d'E/S |

## Tests réussis (automatiques uniquement)

- `cargo test` : **68/68**, trois passes identiques.
- `cargo clippy --all-targets -- -D warnings` : ok.
- `native.py check` sur Horizon 9f42064 : `"check": "ok"`.
- Banc de mesure (`cargo run --release --manifest-path bench/Cargo.toml`) : frame + HUD 57 ns et 0 allocation,
  pire frame pendant une sauvegarde 21 µs — inchangé par rapport à l'optimisation.
- Build launcher : ok (ci-dessous).

## Build installé

- Instance **Essentials Dev** → `native-9f42064-ad4e79711d7f` (commit `dc8567c`, mod `horizon_essentials 0.1.0`),
  construit le 17/11 à 13 h 09 en 15 min 2 s, profil performance.
- Instance **Horizon** (principale) : inchangée, `native-9f42064-b8ab2b688772` (ton build avec le Zoom officiel).

Avant de jouer : **fermer complètement le launcher et le rouvrir** (une fenêtre ouverte avant le build lance
l'ancien binaire). Le **premier lancement** de ce build prendra environ 1 minute (cache de données de Horizon
reconstruit), les suivants environ 7 s.

## Tests en jeu restant à faire (commit 5)

Dans Essentials Dev, dans un monde :

1. Options > Mods > Horizon Essentials > Settings : la page s'affiche (« Témoin Essentials : ACTIVÉ »,
   « Recharger la configuration », « Tout réinitialiser »).
2. Clic sur le témoin : DÉSACTIVÉ, la boîte en haut à droite disparaît ; nouveau clic : elle revient.
3. `/essentials list`, `/essentials` (état), `/essentials help`.
4. `/essentials off status`, `on status`, `toggle status` : la boîte suit, la page aussi.
5. `/essentials off radar` : « Module inconnu » ; `/essentials danse` : aide.
6. `/essentials reset` : avertissement ; `/essentials reset confirm` dans les 15 s : tout par défaut ;
   après 15 s : refusé. Même chose avec les deux clics de la page.
7. `/essentials reload` après avoir modifié `profiles/default.json` à la main.
8. Fermer le jeu juste après un changement, relancer : le changement est conservé (sauvegarde à la fermeture).
9. Recompiler (nouveau build), relancer : la configuration est conservée.
10. F1 et F3 : la boîte disparaît.
11. FPS comparables avec le mod activé et coupé (Options > Mods).

Fichier de configuration : `%APPDATA%\horizon\instances\essentials-dev\horizon-mods\.horizon-mod-data\horizon_essentials\`.

## Travaux non terminés

- **Validation en jeu du commit 5** (liste ci-dessus), puis éventuelles corrections.
- **Push** de `feat/gestion-modules` : seulement après validation en jeu et accord explicite.
- **Interface « Centre Essentials »** : proposition présentée (liste de modules à catégories, écran ouvert par
  une touche en jeu, panneau compact sur le menu Échap), **en attente de validation, rien n'est codé**.
- Ensuite (validé) : premiers modules CPS Counter, Keystrokes, Zoom, FPS Counter — en vérifiant d'abord ce que
  les mods officiels Zoom et FPS Counter de Horizon font déjà, pour ne rien dupliquer.
- Pas commencés, volontairement : Horizon Graphics, Horizon Visuals.

## Environnement à retrouver

- Projet : `Desktop\Horizon-Essentials` ; Ubuntu (WSL) avec le lien `~/horizon-mods/native/horizon-essentials`
  vers ce dossier ; commandes dans [environnement.md](environnement.md).
- Vérifier un `native.py check` par la ligne `"check": "ok"` (le code de sortie affiché par certains scripts
  peut être trompeur).
- Un worktree temporaire (`he-commit4`, détaché sur `1e4d857`) existe dans le dossier temporaire de session :
  il ne sert plus ; s'il disparaît avec ce dossier, `git worktree prune` nettoie la référence.
