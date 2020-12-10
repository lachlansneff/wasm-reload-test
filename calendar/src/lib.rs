use std::{
    sync::Mutex,
    collections::BTreeMap,
};
use chrono::*;

lazy_static::lazy_static! {
    static ref EVENTS: Mutex<BTreeMap<DateTime<Local>, String>> = Mutex::new(BTreeMap::new());
}

#[wasm_interface]
pub fn add_event(date: String, title: String) -> Result<(), ()> {
    let date = unimplemented!("parse date: {}", date);
    EVENTS.lock().unwrap().insert(date, title);

    Ok(())
}

pub fn search(substr: String) -> Option<DateTime<Local>> {
    
}
