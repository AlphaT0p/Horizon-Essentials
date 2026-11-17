// Fermeture du jeu (une fois, avant la sauvegarde finale) : le dernier changement
// de configuration est écrit et le thread d'E/S est attendu (au plus 2 s).
// Pas de test `native_on` : un changement fait avant de couper le mod doit être gardé.
fn essentials_stopping(game: &crate::game::Game<'_>) {
    if !game.native_window() {
        return;
    }
    crate::game::essentials_events::shutdown();
}
