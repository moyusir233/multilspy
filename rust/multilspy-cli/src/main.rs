//! Multilspy CLI entry point

fn main() -> anyhow::Result<()> {
    println!("Multilspy CLI v{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
