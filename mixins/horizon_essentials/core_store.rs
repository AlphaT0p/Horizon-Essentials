// Horizon Essentials : le stockage sur disque (crate::game::essentials_store).
//
// Persistance : le dossier vient de `native_data_dir`, dans le dossier de l'instance
// (horizon-mods/.horizon-mod-data/horizon_essentials/), jamais dans celui d'une
// version : les réglages survivent aux recompilations et aux mises à jour.
//
//   active                  le nom du profil actif
//   profiles/<nom>.json     un profil (format de essentials_config)
//
// Écriture atomique : fichier temporaire puis renommage, pour qu'un arrêt brutal
// ne laisse jamais un fichier à moitié écrit. Un fichier corrompu est copié en
// `<nom>.json.bak` avant d'être remplacé : rien n'est perdu.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::game::essentials_config::Config;

pub const DEFAULT_PROFILE: &str = "default";
/// La sauvegarde attend une seconde sans changement (un glisser dans le HUD Editor
/// donne des dizaines de changements : un seul fichier écrit).
pub const SAVE_DELAY_MS: f32 = 1000.0;

/// Ce que `load` a trouvé.
#[derive(Debug)]
pub struct Loaded {
    pub profile: String,
    pub config: Config,
    pub warnings: Vec<String>,
}

/// Un nom de profil sûr pour un nom de fichier : 1 à 32 caractères `a-z`, `0-9`, `_`, `-`.
pub fn valid_profile_name(name: &str) -> bool {
    (1..=32).contains(&name.len())
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

pub fn profile_path(dir: &Path, profile: &str) -> PathBuf {
    dir.join("profiles").join(format!("{profile}.json"))
}

/// Lit le profil actif. Ne panique jamais et renvoie toujours une configuration valide.
pub fn load(dir: &Path) -> Loaded {
    let mut warnings = Vec::new();
    let profile = match fs::read_to_string(dir.join("active")) {
        Ok(text) => {
            let name = text.trim();
            if valid_profile_name(name) {
                name.to_owned()
            } else {
                warnings.push(format!("profil actif invalide ({name:?}) : profil « {DEFAULT_PROFILE} »"));
                DEFAULT_PROFILE.to_owned()
            }
        }
        Err(_) => DEFAULT_PROFILE.to_owned(),
    };
    let path = profile_path(dir, &profile);
    let config = match fs::read_to_string(&path) {
        Ok(text) => {
            let (config, found) = Config::parse(&text);
            if !found.is_empty() {
                let backup = path.with_extension("json.bak");
                if fs::copy(&path, &backup).is_ok() {
                    warnings.push(format!("ancien fichier copié dans profiles/{profile}.json.bak"));
                }
                warnings.splice(0..0, found);
            }
            config
        }
        // Premier lancement : pas de fichier, pas d'avertissement.
        Err(e) if e.kind() == io::ErrorKind::NotFound => Config::default(),
        Err(e) => {
            warnings.push(format!("profil « {profile} » illisible ({e}) : valeurs par défaut"));
            Config::default()
        }
    };
    Loaded { profile, config, warnings }
}

/// Écrit le profil, et `active` s'il a changé.
pub fn save(dir: &Path, profile: &str, config: &Config) -> io::Result<()> {
    write_atomic(&profile_path(dir, profile), config.to_json().as_bytes())?;
    let active = dir.join("active");
    if fs::read_to_string(&active).map(|a| a.trim() != profile).unwrap_or(true) {
        write_atomic(&active, profile.as_bytes())?;
    }
    Ok(())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, bytes)?;
    // Sous Windows comme ailleurs, `rename` remplace la cible existante.
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// Le délai avant sauvegarde, compté en temps de frame : les frames continuent
/// quand le menu Échap met le jeu en pause, contrairement aux ticks.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SaveTimer {
    pending: bool,
    quiet_ms: f32,
}

impl SaveTimer {
    /// Un changement vient d'avoir lieu.
    pub fn mark(&mut self) {
        self.pending = true;
        self.quiet_ms = 0.0;
    }

    /// Le temps passe. Vrai quand il faut sauvegarder maintenant.
    pub fn advance(&mut self, elapsed_ms: f32) -> bool {
        if !self.pending {
            return false;
        }
        self.quiet_ms += elapsed_ms.max(0.0);
        self.quiet_ms >= SAVE_DELAY_MS
    }

    pub fn done(&mut self) {
        *self = SaveTimer::default();
    }

    pub fn pending(&self) -> bool {
        self.pending
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::game::essentials_registry::ModuleId;
    use crate::game::essentials_types::Action;

    /// Un dossier temporaire propre à un test, supprimé à la fin.
    pub(crate) struct TempDir(pub PathBuf);
    impl TempDir {
        pub(crate) fn new(name: &str) -> TempDir {
            let dir = std::env::temp_dir().join(format!("horizon-essentials-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn premier_lancement_sans_fichier() {
        let d = TempDir::new("vide");
        let l = load(&d.0);
        assert_eq!(l.profile, DEFAULT_PROFILE);
        assert_eq!(l.config, Config::default());
        assert!(l.warnings.is_empty());
    }

    #[test]
    fn sauvegarde_puis_relecture() {
        let d = TempDir::new("aller-retour");
        let mut c = Config::default();
        c.apply(Action::Toggle(ModuleId::Status));
        save(&d.0, "pvp", &c).unwrap();
        assert_eq!(fs::read_to_string(d.0.join("active")).unwrap(), "pvp");
        assert!(!profile_path(&d.0, "pvp").with_extension("json.tmp").exists(), "pas de fichier temporaire oublié");
        let l = load(&d.0);
        assert_eq!((l.profile.as_str(), &l.config), ("pvp", &c));
        assert!(l.warnings.is_empty());
    }

    #[test]
    fn un_fichier_corrompu_est_garde_en_bak() {
        let d = TempDir::new("corrompu");
        let path = profile_path(&d.0, DEFAULT_PROFILE);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ ceci n'est pas du JSON").unwrap();
        let l = load(&d.0);
        assert_eq!(l.config, Config::default());
        assert_eq!(l.warnings.len(), 2, "{:?}", l.warnings);
        assert_eq!(fs::read_to_string(path.with_extension("json.bak")).unwrap(), "{ ceci n'est pas du JSON");
        assert!(path.exists(), "l'original n'est jamais supprimé à la lecture");
    }

    #[test]
    fn un_profil_actif_dangereux_est_refuse() {
        let d = TempDir::new("actif");
        fs::write(d.0.join("active"), "../../ailleurs").unwrap();
        let l = load(&d.0);
        assert_eq!(l.profile, DEFAULT_PROFILE);
        assert_eq!(l.warnings.len(), 1);
        assert!(valid_profile_name("pvp_2"));
        assert!(!valid_profile_name(""));
        assert!(!valid_profile_name("Majuscule"));
    }

    #[test]
    fn le_delai_de_sauvegarde() {
        let mut t = SaveTimer::default();
        assert!(!t.advance(5000.0), "rien à sauvegarder");
        t.mark();
        assert!(!t.advance(600.0));
        t.mark();
        assert!(!t.advance(600.0), "un nouveau changement relance le délai");
        assert!(t.advance(500.0));
        t.done();
        assert!(!t.pending());
    }
}
