use assert_cmd::prelude::*; // Add methods on commands
use paste::paste;
use std::process::Command; // Run programs

#[macro_export]
macro_rules! bitte {
    ($($arg:ident)*) => {
        #[test]
        paste::paste! {
            fn [<bitte_ $($arg)_*>]() -> Result<()> {
                bitte()
                .args(&[$(stringify!{$arg}),*])
                    .assert()
                    .failure()
                    .stderr(predicate::str::contains("Unknown command"));

                Ok(())
            }
        }
    };
}

pub fn bitte() -> Command {
    Command::cargo_bin("bitte").unwrap()
}
