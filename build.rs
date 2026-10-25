//! The assets' stamp for an Android build (`assets::unpack`): a hash of every file's path and
//! contents, so the phone writes them out again whenever any of them changed. A stamp of paths
//! and lengths alone missed an edit that kept a file's length ("paint": 2.0 to 0.0), and the phone
//! went on drawing the old scenes.
//!
//! Changing an asset reruns this, and the new stamp rebuilds the library, so `include_dir` takes
//! the new files in too.

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=build.rs");
    let stamp = if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") { stamp(Path::new("assets")) } else { 0 };
    println!("cargo:rustc-env=HOLLOW_ROAD_ASSETS_STAMP={stamp:016x}");
}

/// FNV-1a over every file under `root`, in path order: its path within `root`, then its bytes.
fn stamp(root: &Path) -> u64 {
    fn walk(d: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() { walk(&p, out) } else { out.push(p) }
        }
    }
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    let eat = |h: u64, bytes: &[u8]| bytes.iter().fold(h, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3));
    files.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, f| {
        let rel = f.strip_prefix(root).unwrap_or(f).to_string_lossy().replace('\\', "/");
        let h = eat(h, rel.as_bytes());
        eat(h, &std::fs::read(f).unwrap_or_default())
    })
}
