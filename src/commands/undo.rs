use crate::store::Store;

pub fn execute(store: &Store) -> std::io::Result<()> {
    match store.restore() {
        Ok(msg) => println!("{msg}"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => println!("Nothing to undo"),
        Err(e) => return Err(e),
    }
    Ok(())
}
