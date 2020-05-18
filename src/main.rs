mod standardfile;

use anyhow::{Context, Result};
use standardfile::Root;

fn main() -> Result<()> {
    let filename = "test.json";
    let contents = std::fs::read_to_string(filename)
        .with_context(|| format!("Could not open {}.", filename))?;

    let root: Root = serde_json::from_str(&contents)?;
    let pass = std::env::var("SF_PASS").unwrap();

    for item in root.notes(&pass)? {
        println!("{}", item.item.updated_at);
        match item.note.title {
            None => println!("n/a"),
            Some(x) => println!("{}", x),
        }
    }

    Ok(())
}
