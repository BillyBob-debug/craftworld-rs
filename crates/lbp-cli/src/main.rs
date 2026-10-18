use std::process::ExitCode;

fn main() -> ExitCode {
    lbp_cli::main_with_args(std::env::args_os().skip(1))
}
