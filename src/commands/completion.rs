use std::io;

use clap::{Args, Command};
use clap_complete::{Shell, generate};

#[derive(Args, Debug)]
pub struct CompletionArgs {
    /// Shell to generate completions for
    pub shell: Shell,
}

pub fn execute(args: &CompletionArgs, cmd: &mut Command) {
    let name = cmd.get_name().to_string();
    generate(args.shell, cmd, name, &mut io::stdout());
}
