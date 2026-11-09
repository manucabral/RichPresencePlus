//! The command line of another program, of which only declared options reach a script.
#![no_main]

use libfuzzer_sys::fuzz_target;
use rpp_host::args::{pick, split_command_line};

fuzz_target!(|text: &str| {
    let args = split_command_line(text);
    let flags = ["-p".to_owned(), "--server".to_owned(), "/c".to_owned()];
    for key in pick(&args, &flags).keys() {
        assert!(flags.contains(key));
    }
});
