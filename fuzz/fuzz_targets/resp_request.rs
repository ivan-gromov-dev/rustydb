#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| rustydb::fuzzing::resp_request(data));
