# Environnement de développement (Windows 11)

Mis en place et testé le 2026-11-16.

| Outil | Version | Rôle |
|---|---|---|
| Rust (Windows) | 1.100.0, `stable-x86_64-pc-windows-gnu` | `cargo test` des modules. GNU : pas besoin de Visual Studio |
| WSL Ubuntu 24.04 | Python 3.12, rsync 3.2.7, git 2.43, Rust 1.100.0 | `native.py` (vérification contre le code de Horizon) |
| `~/horizon-mods` (dans Ubuntu) | clone de `horizon-rust/horizon-mods` | outils officiels `native/tools/native.py` |
| Horizon Launcher | 0.2.69 (`%LOCALAPPDATA%\Programs\Horizon Launcher\horizon-launcher.exe`, c'est aussi la CLI) | build du jeu avec le mod |
| Rust du launcher | 1.99.1 windows-gnu (`horizon-launcher rust install`) | utilisé par `native-build` |
| Horizon installé | Nightly 2026-11-13 · `9f42064` (`NATIVE_HOOKS_API = 415`) | cible |
| Instance de test | `essentials-dev` (« Essentials Dev ») | reçoit les builds ; l'instance « Horizon » n'est jamais touchée |

## Pièges rencontrés

- Les chemins trop longs (> 260 caractères) cassent `cargo` sous Windows : travailler dans un dossier au chemin court.
- `native.py` attend un nom de dossier de `native/` ; compiler un mod externe demande de le placer (ou le lier) dans `~/horizon-mods/native/`.

## Commandes

```bash
# Tests des modules (Windows, à la racine du projet)
cargo test

# Vérification contre Horizon (Ubuntu)
wsl -d Ubuntu-24.04 -u root -- bash -lc ". ~/.cargo/env; cd ~/horizon-mods && python3 native/tools/native.py check <mod> --horizon 9f42064c11efe7ef3e5ed2baad7ddc2a3b36eea0"

# Build du jeu avec le mod, dans l'instance de test (Windows)
"%LOCALAPPDATA%\Programs\Horizon Launcher\horizon-launcher.exe" native-build <dossier mixins\...> --instance essentials-dev --trust-native-code
```

## Résultats de l'étape 0 (mod officiel `fps-overlay`, non modifié)

| Test | Résultat | Durée |
|---|---|---|
| `cargo test` Windows | ✅ 1/1 | 1,5 s |
| `cargo test` Ubuntu | ✅ 1/1 | 0,5 s |
| `native.py check` sur 9f42064 | ✅ `"check": "ok"` | 5 min 12 s (premier passage) |
| `native-build` launcher, profil performance, instance `essentials-dev` | ✅ `native-9f42064-33ffd7ce9a19 built and selected for its instance` | 24 min 48 s (premier build) |
| Lancement en jeu et affichage du compteur | ⏳ pas encore testé (Play à lancer sur « Essentials Dev ») | |

Après le build, le launcher sélectionne l'instance construite pour Play : remettre l'instance principale avec
`horizon-launcher instance select horizon`.
