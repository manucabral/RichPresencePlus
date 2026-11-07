//! Command lines come from other programs, and only declared options may pass.

use proptest::prelude::*;
use rpp_host::args::{pick, split_command_line};

proptest! {
    #[test]
    fn any_command_line_splits(line in "\\PC{0,300}") {
        let _ = split_command_line(&line);
    }

    #[test]
    fn quotes_and_backslashes_are_safe(line in "[a-z \"\\\\=-]{0,120}") {
        let args = split_command_line(&line);
        let _ = pick(&args, &["-p".to_owned()]);
    }

    #[test]
    fn only_declared_options_reach_a_script(
        args in proptest::collection::vec("-{0,2}[a-zA-Z]{1,6}(=[a-z]{0,6})?", 0..12),
        flags in proptest::collection::vec("-{1,2}[a-zA-Z]{1,6}", 0..4),
    ) {
        for key in pick(&args, &flags).keys() {
            prop_assert!(flags.contains(key));
        }
    }

    #[test]
    fn a_name_in_other_case_is_another_option(name in "[a-z]{1,8}", value in "[0-9]{1,6}") {
        let flag = format!("-{name}");
        let upper = flag.to_uppercase();
        let args = vec![upper, value];
        prop_assert!(pick(&args, &[flag]).is_empty());
    }
}
