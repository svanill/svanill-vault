use anyhow::{Context, Result};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use structopt::StructOpt;
use svanill_store::config::Config;
use svanill_store::{
    models::RetrieveListOfUserFilesResponseContentItemContent,
    sdk::{answer_challenge, ls, request_challenge, retrieve},
};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "svanill-store",
    about = "Read/Write data from/to a svanill store server"
)]
struct Opt {
    /// switch on verbosity
    #[structopt(short)]
    verbose: bool,
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
    #[structopt(name = "pull")]
    PULL {
        /// Output file, use stdout if not present
        #[structopt(short = "o", name = "output file", parse(from_os_str))]
        output: Option<PathBuf>,
        /// Write on disk reusing the external filename
        #[structopt(short, long)]
        use_external_name: bool,
    },
}

fn output_files_list(opt: &Opt, v: Vec<RetrieveListOfUserFilesResponseContentItemContent>) {
    for f in v.iter() {
        println!("checksum: {}", f.checksum);
        println!("filename: {}", f.filename);
        println!("size: {}", f.size);
        if opt.verbose {
            println!("url: {}", f.url);
        }
        println!("---");
    }
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
        conf.username = opt.username.clone().unwrap();
        conf_updated = true;
    }

    let challenge = request_challenge(&conf)?;

    if opt.answer == None && conf.challenges.get(&challenge) == None {
        eprintln!("Cannot authenticate. Challenge is:");
        eprintln!("{}", challenge);
        eprintln!("You need to provide the answer with the --answer option (will be stored in your config on success)");
        std::process::exit(1);
    }

    if let Some(x) = &opt.answer {
        conf.challenges.insert(challenge.clone(), x.clone());
        conf_updated = true;
    }

    let answer = conf.challenges.get(&challenge).unwrap();
    conf.token = answer_challenge(&conf, answer)?;

    if conf_updated {
        confy::store(&cli_name, &conf)?;
    }

    match opt.cmd {
        Command::LIST {} => {
            output_files_list(&opt, ls(&conf)?);
        }
        Command::PULL {
            output,
            use_external_name,
        } => {
            let files = ls(&conf)?;
            if !files.is_empty() {
                let f = &files[0];
                let mut f_content: &[u8] = &retrieve(&f.url)?;

                let opt_path: Option<PathBuf> = match use_external_name {
                    // ensure we don't get a path by extracting the filename
                    true => Path::new(&f.filename).file_name().map(PathBuf::from),
                    false => output,
                };

                let stdout = io::stdout();
                let mut handle: Box<dyn Write> = match opt_path {
                    Some(path) => Box::new(
                        File::create(&path)
                            .with_context(|| format!("trying to write onto file {:?}", path))?,
                    ),
                    None => Box::new(stdout.lock()),
                };

                std::io::copy(&mut f_content, &mut handle)?;
            }
        }
    };

    Ok(())
}
