use crate::store;

pub fn execute() -> std::io::Result<()> {
    let msg = store::restore()?;
    println!("{msg}");
    Ok(())
}
