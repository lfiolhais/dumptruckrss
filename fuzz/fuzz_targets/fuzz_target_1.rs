#![no_main]
use libfuzzer_sys::fuzz_target;

use dumptruckrss::query::Query;

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
    if let Ok(s) = std::str::from_utf8(data) {
        match Query::new(&s) {
            Ok(_) => {}
            Err(_) => {}
        }
    }
});
