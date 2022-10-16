use anyhow::{Context, Error, Result};
use atty::Stream;
use std::{
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use structopt::StructOpt;
use svanill_vault_cli::config::Config;
use svanill_vault_cli::utils::gen_random_filename;
use svanill_vault_cli::{
    commands::pull::sanitize_possible_filename,
    sdk::{answer_challenge, delete, ls, request_challenge, request_upload_url, retrieve, upload},
};
use svanill_vault_openapi::RetrieveListOfUserFilesResponseContentItemContent;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "svanill-vault-cli",
    about = "Read/Write data from/to a svanill vault server"
)]
struct Opt {
    /// switch on verbosity
    #[structopt(short)]
    verbose: bool,
    /// switch on verbosity
    #[structopt(short)]
    store_conf: bool,
    /// Svanill vault host
    #[structopt(short = "h", default_value = "https://api.svanill.com")]
    host: String,
    /// Svanill vault username
    #[structopt(short, long)]
    username: Option<String>,
    /// Svanill vault answer to the challenge
    #[structopt(short, long)]
    answer: Option<String>,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "ls", alias = "list")]
    List {},
    #[structopt(name = "pull")]
    Pull {
        /// Download the remote file that has this name, by default to a local file with the same name
        #[structopt(name = "remote_name")]
        remote_name: String,
        /// Write output to <file>
        #[structopt(short = "o", long = "output", name = "file", parse(from_os_str))]
        output_file: Option<PathBuf>,
        /// Write output to stdout (overrides -o if set)
        #[structopt(short = "s", long = "stdout")]
        write_to_stdout: bool,
    },
    #[structopt(name = "push")]
    Push {
        /// Push file to a svanill-vault server.
        /// With no FILE, or when FILE is -, read standard input.
        /// When reading from standard input, -g is implied
        #[structopt(name = "FILE", parse(from_os_str))]
        maybe_input_file: Option<PathBuf>,
        /// Push the content under a random file name
        #[structopt(short = "g", long = "random-remote-name")]
        gen_random_remote_name: bool,
        /// Set a different remote file name. Override -g if both present
        #[structopt(short = "r", long = "remote-name")]
        maybe_remote_name: Option<String>,
    },
    #[structopt(name = "rm")]
    Delete {
        /// Read input from <file> instead of stdin
        #[structopt(name = "file")]
        remote_name: String,
    },
}

fn output_files_list(opt: &Opt, v: Vec<RetrieveListOfUserFilesResponseContentItemContent>) {
    print!("       Bytes");
    if opt.verbose {
        print!(" |                         Checksum");
    }
    print!(" | Filename");
    if opt.verbose {
        print!(" | Url");
    }
    println!();

    for f in v.iter() {
        print!("{:>12}", f.size);
        if opt.verbose {
            print!(" | {}", f.checksum);
        }
        print!(" | {}", f.filename);
        if opt.verbose {
            print!(" | {}", f.url);
        }
        println!();
    }
}

fn main() -> Result<()> {
    let mut opt = Opt::from_args();

    // Check some options validity before attempting network requests
    if let Command::Push {
        ref mut gen_random_remote_name,
        ref mut maybe_input_file,
        ref maybe_remote_name,
    } = opt.cmd
    {
        if Some(PathBuf::from("-")) == *maybe_input_file {
            *maybe_input_file = None;
        }

        if maybe_remote_name.is_some() && *gen_random_remote_name {
            *gen_random_remote_name = false;
        }

        if maybe_input_file.is_none() && maybe_remote_name.is_none() {
            *gen_random_remote_name = true;
        }

        match maybe_input_file {
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

    let cli_name = "svanill-vault-cli";
    let mut conf: Config = confy::load(cli_name)?;
    let mut conf_updated = false;

    if opt.username != None {
        conf.username = opt.username.clone().unwrap();
        conf_updated = true;
    }

    if conf.username.is_empty() {
        eprintln!("Error: username is missing");
        std::process::exit(1);
    }

    if opt.host != conf.base_url {
        conf.base_url = opt.host.clone();
        conf_updated = true;
    }

    // If the base_url is localhost (with or without port)
    // we prefix it with http:// to avoid "URL Scheme is not allowed"
    // error from the reqwest library.
    if conf.base_url.starts_with("localhost") {
        conf.base_url = format!("http://{}", conf.base_url);
    }

    let challenge = request_challenge(&conf)?;

    if opt.answer == None && conf.challenges.get(&challenge) == None {
        eprintln!("Cannot authenticate, missing answer to the server's challenge.");
        eprintln!("Decrypt the challenge to get the answer:");
        eprintln!("  svanill -i <(echo {}) dec", challenge);
        eprintln!();
        eprintln!("Then re-run svanill-vault-cli passing --username and --answer. They will be stored in a config file so you won't have to provide them again");
        std::process::exit(1);
    }

    if let Some(x) = &opt.answer {
        conf.challenges.insert(challenge.clone(), x.clone());
        conf_updated = true;
    }

    let answer = conf.challenges.get(&challenge).unwrap();
    conf.token = answer_challenge(&conf, answer)?;

    if conf_updated && opt.store_conf {
        confy::store(cli_name, &conf)?;
    }

    match opt.cmd {
        Command::List {} => {
            output_files_list(&opt, ls(&conf)?);
        }
        Command::Delete { remote_name } => {
            delete(&conf, &remote_name)?;
            println!("Success: deleted file \"{}\"", remote_name);
        }
        Command::Pull {
            output_file,
            write_to_stdout,
            remote_name,
        } => {
            let files = ls(&conf)?;

            let f = &files
                .iter()
                .find(|x| x.filename == remote_name)
                .ok_or_else(|| Error::msg(String::from("remote file not found")))?;

            let stdout = io::stdout();
            let mut handle: Box<dyn Write> = if write_to_stdout {
                Box::new(stdout.lock())
            } else {
                let path = if let Some(path) = output_file {
                    path
                } else {
                    sanitize_possible_filename(&f.filename)?
                };

                Box::new(
                    File::create(&path)
                        .with_context(|| format!("cannot create file {:?}", path))?,
                )
            };

            let mut reader = retrieve(&f.url)?;
            std::io::copy(&mut reader, &mut handle)?;
        }
        Command::Push {
            gen_random_remote_name,
            maybe_input_file,
            maybe_remote_name,
        } => {
            let mut local_content = Vec::new();

            match maybe_input_file {
                Some(ref path) if path.exists() => {
                    File::open(path)
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

            let remote_name = if gen_random_remote_name {
                gen_random_filename()
            } else if maybe_remote_name.is_some() {
                maybe_remote_name.unwrap()
            } else {
                Path::new(&maybe_input_file.unwrap())
                    .file_name()
                    .map(PathBuf::from)
                    .unwrap()
                    .to_string_lossy()
                    .into()
            };

            let links = request_upload_url(&conf, &remote_name)?;

            upload(
                *links.upload_url,
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
