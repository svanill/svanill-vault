use anyhow::{Context, Result};
use std::io::Read;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "svanill-store",
    about = "Read/Write data from/to a svanill store server"
)]
struct Opt {
    /// Svanill store host
    #[structopt(short = "h", default_value = "https://api.svanill.com")]
    host: String,
    /// Svanill store key
    #[structopt(short, long)]
    key: String,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "ls", alias = "list")]
    LIST {},
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let mut key = Vec::new();
    std::io::stdin()
        .read_to_end(&mut key)
        .with_context(|| "Couldn't read from STDIN")?;

    match opt.cmd {
        Command::LIST {} => {
            println!("{}", "let's jam");
        }
    };

    Ok(())
}
