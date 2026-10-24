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
    #[cfg(target_os = "android")]
    return DIR.get_or_init(unpack);
    #[cfg(not(target_os = "android"))]
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

/// On Android: the assets built into the library, written out under the app's own folder (the
/// working directory the platform layer sets) the first time this build runs, and again only when
/// a build carries different ones.
#[cfg(target_os = "android")]
fn unpack() -> PathBuf {
    static BUILT: include_dir::Dir<'static> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/assets");
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join("assets");
    // What this build carries, in a line: every file's path and length.
    fn walk(d: &include_dir::Dir<'static>, out: &mut Vec<&'static include_dir::File<'static>>) {
        for e in d.entries() { match e { include_dir::DirEntry::Dir(d) => walk(d, out), include_dir::DirEntry::File(f) => out.push(f) } }
    }
    let mut files = Vec::new();
    walk(&BUILT, &mut files);
    let stamp = files.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, f| {
        let h = f.path().to_string_lossy().bytes().fold(h, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
        (h ^ f.contents().len() as u64).wrapping_mul(0x100_0000_01b3)
    }).to_string();
    let mark = root.join(".unpacked");
    if std::fs::read_to_string(&mark).ok().as_deref() == Some(stamp.as_str()) { return root; }
    let t = std::time::Instant::now();
    let _ = std::fs::remove_dir_all(&root);
    for f in &files {
        let to = root.join(f.path());
        if let Some(parent) = to.parent() { let _ = std::fs::create_dir_all(parent); }
        if let Err(e) = std::fs::write(&to, f.contents()) { log::error!("[hollow_road] could not unpack {}: {e}", to.display()); }
    }
    let _ = std::fs::write(&mark, &stamp);
    log::info!("[hollow_road] unpacked {} assets to {} in {:.1} s", files.len(), root.display(), t.elapsed().as_secs_f32());
    root
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
