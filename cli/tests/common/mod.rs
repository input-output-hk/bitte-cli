use std::process::Command;
use anyhow::{Result, Context};
use assert_cmd::prelude::*;
use assert_cmd::assert::Assert;
use predicates::str::*;

#[macro_export]
// TODO figure out how to extract the common part. macros are hard!
macro_rules! test {
    // test! {
    //     VAR1="this" VAR2="that";
    //     bitte do something;
    // .   should fail with "error message"
    //  }
    ($($var:ident=$val:expr)*; $($cmdline:ident)+; should fail with $msg:expr) => {
        // The paste! crate enables
        paste::paste! {
            #[test]
            fn [<test_ $($cmdline)_+>]() -> $crate::Result<()> {
                use $crate::*;
                $crate::c!($($cmdline)+)
                    $(
                        .env(stringify!{$var}, $val)
                    )*
                    .should()
                    .fail()
                    .with($msg);
                Ok(())
            }
        }
    };

    // test! {
    //     bitte do something;
    // .   should fail with "error message"
    //  }
    ($($cmdline:ident)+; should fail with $msg:expr) => {
        paste::paste! {
            #[test]
            fn [<test_ $($cmdline)_+>]() -> $crate::Result<()> {
                use $crate::*;
                $crate::c!($($cmdline)+)
                    .should()
                    .fail()
                    .with($msg);
                Ok(())
            }
        }
    }
}

pub(crate) fn c(cargo_bin_and_args: &[&str]) -> Result<Command> {
    let cargo_bin = cargo_bin_and_args[0];
    let args = &cargo_bin_and_args[1..];
    let cmd = cc(cargo_bin, args)?;
    Ok(cmd)
}

pub(crate) fn cc(cargo_bin: &str, args: &[&str]) -> Result<Command> {
    let mut cmd = Command::cargo_bin(&cargo_bin)
    .with_context(|| format!("Cargo couldn't find {}", cargo_bin))?;
    cmd.args(args);
    Ok(cmd)
}

#[macro_export]
macro_rules! c {
    ($($args:ident)+) => { $crate::c!($(stringify!($args))+) };
    ($($args:expr),*) => { $crate::c(&[$($args),*])? };
    ($($args:expr)+) => { $crate::c!($($args),+) };
}

// For shits and giggles, allow .should() instead of .assert()
// Future reference more than anything.
pub(crate) trait OutputShouldExt {
    fn should(self) -> Assert;
}

impl<'c> OutputShouldExt for &'c mut Command {
    fn should(self) -> Assert {
        self.assert()
    }
}

// Likewise above
pub(crate) trait AssertExt {
    fn fail(self) -> Assert;
    fn with(self, pattern: &str) -> Assert;
}

impl AssertExt for Assert {
    fn fail(self) -> Assert {
        self.failure()
    }

    fn with(self, pattern: &str) -> Assert {
        // TODO decide stderr/stdout based on whether wer're asserting success or failure...
        // this isn't currently straight-forward because Assert keeps its `output` memeber private.
        self.stderr(contains(pattern))
    }
}
