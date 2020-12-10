use std::{collections::HashMap, convert::TryFrom, fs::File, io::Read, convert::TryInto, io::Write, ops::Deref, ops::DerefMut, path::Path};
use walrus::{ExportItem, ImportKind};
use wasmtime::*;
use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
enum SerializableVal {
    I32(i32),
    I64(i64),
    F32(u32),
    F64(u64),
}

impl TryFrom<Val> for SerializableVal {
    type Error = ();
    fn try_from(val: Val) -> Result<Self, Self::Error> {
        Ok(match val {
            Val::I32(x) => Self::I32(x),
            Val::I64(x) => Self::I64(x),
            Val::F32(x) => Self::F32(x),
            Val::F64(x) => Self::F64(x),
            _ => return Err(()),
        })
    }
}

impl From<SerializableVal> for Val {
    fn from(val: SerializableVal) -> Self {
        match val {
            SerializableVal::I32(x) => Val::I32(x),
            SerializableVal::I64(x) => Val::I64(x),
            SerializableVal::F32(x) => Val::F32(x),
            SerializableVal::F64(x) => Val::F64(x),
        }
    }
}

pub struct PersistentInstance {
    instance: Instance,
}

impl Deref for PersistentInstance {
    type Target = Instance;
    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}
impl DerefMut for PersistentInstance {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.instance
    }
}

impl PersistentInstance {
    pub fn new_from_file<P: AsRef<Path>>(store: &Store, path: P) -> Result<(Module, PersistentInstance)> {
        let (module, instance) = Self::load(store, path, None)?;
        
        Ok((
            module,
            Self {
                instance,
            }
        ))
    }

    pub fn load_from_file<P: AsRef<Path>>(store: &Store, path: P, globals: P, memory: P) -> Result<(Module, PersistentInstance)> {
        let (module, instance) = Self::load(store, path, Some((globals, memory)))?;
        
        Ok((
            module,
            Self {
                instance,
            }
        ))
    }

    pub fn save<P: AsRef<Path>>(&self, globals: P, memory: P) -> Result<()> {
        let mut global_map: HashMap<_, SerializableVal> = HashMap::new();
        let mut memory_bin = File::create(memory)?;
        let mut globals_json = File::create(globals)?;

        for export in self.instance.exports() {
            let name = export.name();
            match export.into_extern() {
                Extern::Memory(memory) => {
                    // println!("Saving memory to `memory.bin`");

                    let data = unsafe { memory.data_unchecked() };
                    memory_bin.write_all(data)?;
                },
                Extern::Global(global) => {
                    global_map.insert(name, global.get().try_into().unwrap());
                },
                _ => {},
            }
        }

        // println!("Saving globals to `globals.json`");
        serde_json::to_writer(&mut globals_json, &global_map)?;

        Ok(())
    }

    fn load<P: AsRef<Path>>(store: &Store, path: P, reload: Option<(P, P)>) -> Result<(Module, Instance)> {
        let mut m = walrus::Module::from_file(path.as_ref())?;

        // Export all the internal, mutable globals.
        let mut counter = 0;
        for global in m.globals.iter() {
            if global.mutable && m.exports.get_exported_global(global.id()).is_none() {
                m.exports.add(&format!("$probed_global:{}", counter), global.id());
                counter += 1;
            }
        }

        if reload.is_some(){
            let old_data_ids: Vec<_> = m.data.iter().map(|data| data.id()).collect();
            // Delete all the data initializers.
            for id in old_data_ids {
                m.data.delete(id);
            }

            let old_exports: Vec<_> = m.exports
                .iter()
                .filter_map(|ex| {
                    let kind = match ex.item {
                        ExportItem::Memory(id) => Some(ImportKind::Memory(id)),
                        ExportItem::Global(id) => Some(ImportKind::Global(id)),
                        _ => None,
                    }?;
                    Some((ex.id(), ex.name.clone(), kind))
                })
                .collect();
            
            // Move all exports to imports.
            for (id, name, import_kind) in old_exports {
                m.exports.delete(id);
                let import_id = m.imports.add("cursed-imports", &name, import_kind.clone());
                match import_kind {
                    ImportKind::Memory(id) => {
                        m.memories.get_mut(id).import = Some(import_id);
                    },
                    _ => {},
                }
            }
        }

        let wasm = m.emit_wasm();
        let module = Module::from_binary(store.engine(), &wasm)?;

        // Pretty print all the exports and imports, just for fun.
        {
            let exports = module.exports();
            if exports.len() > 0 {
                println!("Exports:");
                for export in exports {
                    println!("  {:?}", export);
                }
            }

            let imports = module.imports();
            if imports.len() > 0 {
                println!("Imports:");
                for import in imports {
                    println!("  {:?}", import);
                }
            }
        }


        let mut externs: Vec<Extern> = vec![];

        if let Some((globals_path, memory_path)) = reload {
            let (globals_path, memory_path) = (globals_path.as_ref(), memory_path.as_ref());
            // Pull all the fake imports from the void.
            // println!("Loading globals from `globals.json`");
            let globals: HashMap<String, SerializableVal> = serde_json::from_reader(&mut File::open(globals_path)?)?;

            for import in module.imports() {
                match import.ty() {
                    ExternType::Global(ty) => {
                        let global = Global::new(&store, ty, globals[import.name()].clone().into())?;
                        externs.push(global.into());
                    }
                    ExternType::Memory(ty) => {
                        let memory = Memory::new(&store, ty);

                        // println!("Loading memory from `memory.bin`");
                        let mut f = File::open(memory_path)?;
                        let saved_memory_len = (f.metadata()?.len() / 0x10000) as u32;
                        assert!(saved_memory_len >= memory.size());
                        let delta = saved_memory_len - memory.size();
                        memory.grow(delta)?;

                        let data = unsafe { memory.data_unchecked_mut() };
                        f.read_exact(data)?;
                        externs.push(memory.into());
                    },
                    _ => unimplemented!()
                }
            }
        }

        let instance = Instance::new(store, &module, &externs)?;

        Ok((module, instance))
    }
}