use crate::store;

pub fn execute() -> std::io::Result<()> {
    match store::restore() {
        Ok(msg) => println!("{msg}"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => println!("Nothing to undo"),
        Err(e) => return Err(e),
    }
    Ok(())
}
