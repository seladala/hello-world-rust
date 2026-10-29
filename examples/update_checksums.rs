//! Re-pins every protected file in src/core/security/checksums.sha256.
//!
//! Run after any intentional change to a protected file: `cargo run --example update_checksums`

use hello_world::core::project_root;
use hello_world::core::security::integrity::{checksums_path, compute_manifest, format_manifest};

const HEADER: &str = "# SHA-256 manifest of every protected file. Regenerate after any intentional change.\n";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let manifest = compute_manifest(&root)?;
    let path = checksums_path(&root);
    std::fs::write(&path, format!("{HEADER}{}", format_manifest(&manifest)))?;
    println!("pinned {} files in {}", manifest.len(), path.display());
    Ok(())
}
