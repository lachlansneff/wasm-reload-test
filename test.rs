static mut N: usize = 0;
static mut ITEMS: Vec<usize> = Vec::new();

#[no_mangle]
pub extern "C" fn set(x: usize) {
    unsafe { N = x }
}

#[no_mangle]
pub extern "C" fn get() -> usize {
    unsafe { N }
}

#[no_mangle]
pub extern "C" fn push(x: usize) {
    unsafe { ITEMS.push(x) }
}

#[no_mangle]
pub extern "C" fn sum() -> usize {
    unsafe { ITEMS.iter().sum() }
}
