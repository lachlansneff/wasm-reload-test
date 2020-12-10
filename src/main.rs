use wasmtime::*;
use anyhow::Result;

mod wasm;
use wasm::PersistentInstance;

fn save() -> Result<()> {
    let store = Store::default();
    let (_, instance) = PersistentInstance::new_from_file(
        &store,
        "test.wasm"
    )?;

    let set = instance
        .get_func("set")
        .ok_or(anyhow::format_err!("failed to find `set` function export"))?
        .get1::<u32, ()>()?;
    
    println!("Calling exported wasm function: `set(42)`");
    set(42)?;
    instance.save("globals.json", "memory.bin")?;

    Ok(())
}

fn reload() -> Result<()> {
    let store = Store::default();
    let (_, instance) = PersistentInstance::load_from_file(
        &store,
        "test.wasm",
        "globals.json",
        "memory.bin"
    )?;

    let get = instance
        .get_func("get")
        .ok_or(anyhow::format_err!("failed to find `get` function export"))?
        .get0::<u32>()?;

    let x = get()?;
    println!("calling exported wasm function: `get()` => {}", x);
    assert_eq!(x, 42);

    Ok(())
}

fn main() -> Result<()> {
    let load_new = match std::env::args().nth(1).as_deref() {
        Some("reload") => true,
        Some("save") => false,
        _ => unimplemented!()
    };

    if load_new {
        save()?;
    } else {
        reload()?;
    }

    Ok(())
}
