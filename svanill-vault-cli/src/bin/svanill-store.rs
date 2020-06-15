use anyhow::{Context, Result};
use std::io::Read;
use structopt::StructOpt;
use svanill_store::config::Config;
use svanill_store::sdk::{ls};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "svanill-store",
    about = "Read/Write data from/to a svanill store server"
)]
struct Opt {
    /// Svanill store host
    #[structopt(short = "h", default_value = "https://api.svanill.com")]
    host: String,
    /// Svanill store username
    #[structopt(short, long)]
    username: Option<String>,
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

    let cli_name = "svanill-store-cli";
    let mut conf: Config = confy::load(&cli_name)?;
    let mut conf_updated = false;

    if conf.username == "" && opt.username != None {
        conf.username = opt.username.unwrap();
        conf_updated = true;
    }

    if conf_updated {
        confy::store(&cli_name, &conf)?;
    }

    match opt.cmd {
        Command::LIST {} => {
            println!("{:?}", ls(&conf)?);
        }
    };

    Ok(())
}
