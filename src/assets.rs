//! Where the game's art, scenes and effects are read from as it runs. Built with cargo they are
//! the source tree's `assets/`; shipped, they travel with the game: inside the app bundle
//! (`Hollow Road.app/Contents/Resources/assets`, see `tools/package_mac.sh`) or in a folder beside
//! the executable. The font alone is built in.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The `assets` folder: `HOLLOW_ROAD_ASSETS` when set, else the app bundle's, else one beside the
/// executable, else the source tree it was built from. The first that holds the journey is the
/// one; looked for once.
pub fn dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let mut tries: Vec<PathBuf> = std::env::var_os("HOLLOW_ROAD_ASSETS").map(PathBuf::from).into_iter().collect();
        if let Some(bin) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
            tries.push(bin.join("../Resources/assets"));
            tries.push(bin.join("assets"));
        }
        let built = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        tries.into_iter().find(|d| d.join(JOURNEY).is_file()).unwrap_or(built)
    })
}

/// The journey PathForge walks, within `dir`.
pub const JOURNEY: &str = "scenes/world.journey.json";

/// `rel` within `dir`.
pub fn path(rel: impl AsRef<Path>) -> PathBuf { dir().join(rel) }

#[cfg(test)]
mod tests {
    #[test]
    fn built_with_cargo_the_art_is_the_source_trees() {
        assert!(super::dir().join(super::JOURNEY).is_file(), "{}", super::dir().display());
        assert!(super::path("pixellab/wayfarer_ne").is_dir());
    }
}
