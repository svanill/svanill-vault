use anyhow::{Context, Result};
use std::io::Read;
use structopt::StructOpt;
use svanill_store::config::Config;
use svanill_store::sdk::{answer_challenge, ls, request_challenge};

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
    /// Svanill store answer to the challenge
    #[structopt(short, long)]
    answer: Option<String>,
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

    let challenge = request_challenge(&conf)?;

    if opt.answer == None && conf.challenges.get(&challenge) == None {
        eprintln!("Cannot authenticate. Challenge is:");
        eprintln!("{}", challenge);
        eprintln!("You need to provide the answer with the --answer option (will be stored in your config on success)");
        std::process::exit(1);
    }

    if let Some(x) = opt.answer {
        conf.challenges.insert(challenge.clone(), x);
        conf_updated = true;
    }

    let answer = conf.challenges.get(&challenge).unwrap();
    conf.token = answer_challenge(&conf, answer)?;

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
