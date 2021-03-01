
#[macro_export]
macro_rules! test {
    // ($bitte_binary:ident $($bitte_args:ident)* ;
    // expect $success_or_failure:ident
    // where $stderr_or_stdout:ident
    // $str_predicate:ident $($str_predicate_args:ident)*) => {
    //     paste::paste! {
    //         #[test]
    //         fn [<$bitte_binary _ $($bitte_args)_*>]() -> Result<()> {
    //             let bitte_args: &[&str] = &[$(stringify!{$bitte_args}),*];

    //             std::process::Command::cargo_bin(stringify!{$bitte_binary})?
    //                .args(bitte_args)
    //                .assert()
    //                .$success_or_failure()
    //                .$stderr_or_stdout(predicate::str::$str_predicate(stringify!{$str_predicate_args}));
    //             Ok(())
    //         }
    //     }
    // };
    (run $binary:ident) => {
                std::process::Command::cargo_bin(stringify!{$bitte_binary})?
                   .args(bitte_args)
                   .assert()
                   .$success_or_failure()
                   .$stderr_or_stdout(predicate::str::$str_predicate(stringify!{$str_predicate_args}));
                Ok(())
     } 
}