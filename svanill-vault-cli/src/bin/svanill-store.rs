use anyhow::{Context, Result};
use atty::Stream;
use std::{
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use structopt::StructOpt;
use svanill_store::config::Config;
use svanill_store::utils::gen_random_filename;
use svanill_store::{
    models::RetrieveListOfUserFilesResponseContentItemContent,
    sdk::{answer_challenge, delete, ls, request_challenge, request_upload_url, retrieve, upload},
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
        /// Write output to <file> instead of stdout
        #[structopt(short = "o", long = "output", name = "file", parse(from_os_str))]
        output_file: Option<PathBuf>,
        /// Write output to a local file named like the remote file. Existing file will be overwritten
        #[structopt(short = "O", long = "remote-name")]
        use_remote_name: bool,
    },
    #[structopt(name = "push")]
    PUSH {
        /// Read input from <file> instead of stdin
        #[structopt(short = "i", long = "input", name = "file", parse(from_os_str))]
        input_file: Option<PathBuf>,
        /// Use the local file name as remote name (requires -i to point to an existing file). If this flag is not provided a random name will be used instead
        #[structopt(short = "l", long = "local-name")]
        use_local_name: bool,
    },
    #[structopt(name = "rm")]
    DELETE {
        /// Read input from <file> instead of stdin
        #[structopt(name = "file")]
        remote_name: String,
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

    // Check some options validity before attempting network requests
    if let Command::PUSH {
        use_local_name,
        ref input_file,
    } = opt.cmd
    {
        let mut input_file = input_file.clone();
        if Some(PathBuf::from("-")) == input_file {
            input_file = None;
        }

        if use_local_name && input_file == None {
            eprintln!(
                    "ERROR: you cannot pipe data in and request to use the local filename at the same time"
                );
            std::process::exit(1);
        }

        match input_file {
            None if atty::is(Stream::Stdin) => {
                eprintln!("ERROR: no data piped in");
                std::process::exit(1);
            }
            Some(x) if !x.exists() => {
                eprintln!("ERROR: input file does not exist");
                std::process::exit(1);
            }
            _ => (),
        }
    }

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
        Command::DELETE { remote_name } => {
            delete(&conf, &remote_name)?;
            println!("Success: deleted file \"{}\"", remote_name);
        }
        Command::PULL {
            output_file,
            use_remote_name,
        } => {
            let files = ls(&conf)?;
            if !files.is_empty() {
                let f = &files[0];
                let mut f_content: &[u8] = &retrieve(&f.url)?;

                let opt_path: Option<PathBuf> = match use_remote_name {
                    // attempt to convert the remote name to a filename
                    true => Path::new(&f.filename).file_name().map(PathBuf::from),
                    false => output_file,
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
        Command::PUSH {
            ref input_file,
            use_local_name,
        } => {
            let mut input_file = input_file.clone();
            if Some(PathBuf::from("-")) == input_file {
                input_file = None;
            }

            let mut local_content = Vec::new();

            match input_file {
                Some(ref path) if path.exists() => {
                    File::open(&path)
                        .with_context(|| format!("trying to read file {:?}", path))?
                        .read_to_end(&mut local_content)?;
                }
                None => {
                    std::io::stdin()
                        .read_to_end(&mut local_content)
                        .with_context(|| "Couldn't read from STDIN")?;
                }
                _ => panic!("Input file does not exist"),
            }

            let remote_name = if use_local_name {
                Path::new(&input_file.unwrap())
                    .file_name()
                    .map(PathBuf::from)
                    .unwrap()
                    .to_string_lossy()
                    .into()
            } else {
                gen_random_filename()
            };

            let links = request_upload_url(&conf, &remote_name)?;

            upload(
                links.upload_url,
                remote_name.clone(),
                String::from_utf8(local_content)?,
            )?;

            println!(
                "Successfully pushed file, using as remote name \"{}\"",
                remote_name
            );
        }
    };

    Ok(())
}
