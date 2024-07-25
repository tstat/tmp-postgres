mod cli;
use clap::Parser;
use console::Style;
use dialoguer::Confirm;
use process_muxer::{
    ChildInfo, Error as MuxerError, Muxer as ProcessMuxer, Pid, PrintInfo, Result as MuxerResult,
    Signal,
};
use regex::Regex;
use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

/// A simple wrapper over a `ProcessMuxer` that captures additional info for
/// spawning postgresql binaries and cleaning up.
struct PgMuxer {
    process_muxer: ProcessMuxer,
    // A directory the user optionally specifies that contains all postgres
    // binaries under `<pg_dir>/bin`
    pg_dir: Option<PathBuf>,

    // the directory that we create to initialize a pg cluster inside, if any
    created_db_path: Option<PathBuf>,
}

impl PgMuxer {
    fn new(silent: bool) -> io::Result<Self> {
        let pg_dir = match env::var("PG_DIR") {
            Ok(val) => {
                let pathbuf = PathBuf::from(val);
                Some(pathbuf)
            }
            Err(_) => None,
        };
        let mut process_muxer = ProcessMuxer::new()?;
        if !silent {
            let print_info = PrintInfo::new();
            process_muxer.add_hook(print_info);
        }
        Ok(Self {
            process_muxer,
            pg_dir,
            created_db_path: None,
        })
    }
    fn pg_command(&self, cmd: &'static str) -> Command {
        match self.pg_dir {
            None => Command::new(cmd),
            Some(ref p) => {
                let mut path = PathBuf::from(p);
                path.push("bin");
                path.push(cmd);
                Command::new(path)
            }
        }
    }
    fn init_db<'b>(&mut self, path: &'b Path) -> Result<(), Error<'b>> {
        let mut cmd = self.pg_command("initdb");
        cmd.arg("-D").arg(path);
        let init_db_id = self
            .process_muxer
            .forward(cmd)
            .map_err(|error| Error::InitDbError { error })?;
        // todo
        let _exit_code = self.process_muxer.wait(&init_db_id)?;
        Ok(())
    }

    fn create_db(&mut self, path: &Path, port: Option<u16>) -> MuxerResult<()> {
        let mut cmd = self.pg_command("createdb");
        cmd.arg("--host");
        cmd.arg(path);
        if let Some(p) = port {
            cmd.arg("--port");
            cmd.arg(format!("{p}"));
        }
        // todo
        let child_info = self.process_muxer.forward(cmd).unwrap();
        let _exit_code = self.process_muxer.wait(&child_info)?;
        Ok(())
    }

    fn launch_psql(&mut self, path: &Path, port: Option<u16>) -> io::Result<ChildInfo> {
        let mut cmd = self.pg_command("psql");
        cmd.arg("--host");
        cmd.arg(path);
        if let Some(p) = port {
            cmd.arg("--port");
            cmd.arg(format!("{p}"));
        }
        let pid = self.process_muxer.control(cmd).unwrap();
        Ok(pid)
    }
    fn run_postgres(&mut self, db_path: &Path, port: Option<u16>) -> MuxerResult<()> {
        let mut cmd = self.pg_command("postgres");
        cmd.arg("-D").arg(db_path);
        if let Some(port) = port {
            cmd.arg("-p").arg(port.to_string().as_str());
        }
        // todo
        let child_info = self.process_muxer.forward(cmd).unwrap();
        self.process_muxer.wait_for_match(
            &child_info,
            Regex::new(r"database system is ready to accept connections").unwrap(),
        )?;
        Ok(())
    }
    fn ensure_db_dir_exists(&mut self, args: &cli::Args) -> Result<(), DirError> {
        let p: &Path = &args.directory;
        if p.exists() {
            if p.is_dir() {
                if !args.use_existing_dir {
                    match p.read_dir().unwrap().next() {
                        None => {}
                        Some(_) => return Err(DirError::IsNonempty),
                    }
                }
            } else {
                return Err(DirError::IsFile);
            }
        } else {
            fs::create_dir_all(p).map_err(DirError::CreateFailed)?;
            self.created_db_path = Some(PathBuf::from(p));
        }
        Ok(())
    }
    fn cleanup(&mut self, should_remove: &Option<bool>) -> io::Result<()> {
        self.process_muxer.cleanup()?;
        if let Some(db_path) = self.created_db_path.take() {
            match should_remove {
                None => {
                    let style = Style::new().bright().green();
                    let prompt_string = format!("Remove {}?", db_path.display());
                    let prompt_string = format!("{}", style.apply_to(prompt_string));
                    if Confirm::new().with_prompt(prompt_string).interact()? {
                        fs::remove_dir_all(db_path)?;
                    }
                }
                Some(should_remove) => {
                    if *should_remove {
                        fs::remove_dir_all(db_path)?;
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn main() {
    let args = cli::Args::parse();
    let mut pg_muxer = PgMuxer::new(args.silent).unwrap();
    match run(&args, &mut pg_muxer) {
        Ok(()) => {
            pg_muxer.cleanup(&args.should_remove).unwrap();
        }
        Err(e) => {
            eprintln!("Error: {e}");
            let mut should_remove: Option<bool> = args.should_remove;
            if let Error::Interrupt {
                signal: Signal::Hangup,
            } = e
            {
                if should_remove.is_none() {
                    should_remove = Some(true);
                }
            }
            pg_muxer.cleanup(&should_remove).unwrap();
            std::process::exit(1);
        }
    }
}

fn run<'a>(args: &'a cli::Args, muxer: &'a mut PgMuxer) -> Result<(), Error<'a>> {
    if cfg!(debug_assertions) {
        println!("args: {args:?}");
    }
    muxer
        .ensure_db_dir_exists(args)
        .map_err(|e| (&args.directory as &'a Path, e))?;
    if !args.use_existing_dir {
        muxer.init_db(&args.directory)?;
        tweak_postgresql_conf(&args.directory, args.port).unwrap();
    }
    muxer.run_postgres(&args.directory, args.port)?;
    if !args.use_existing_dir {
        muxer.create_db(&args.directory, args.port).unwrap();
    }
    let controlling_child = if args.psql {
        let psql = muxer.launch_psql(&args.directory, args.port).unwrap();
        Some(psql)
    } else {
        let mut around_args = args.around.iter();
        match around_args.next() {
            Some(bin) => {
                let mut cmd = Command::new(bin);
                let prog_path: PathBuf = cmd.get_program().into();
                cmd.args(around_args);
                let child = muxer.process_muxer.control(cmd);
                let child = child.map_err(|error| Error::AroundError { prog_path, error })?;
                Some(child)
            }
            None => None,
        }
    };
    if let Some(controlling_child) = controlling_child {
        loop {
            match muxer.process_muxer.wait(&controlling_child) {
                Ok(_) => break,
                Err(
                    err @ MuxerError::UnexpectedSignal {
                        signal: Signal::Hangup,
                    },
                ) => return Err(err.into()),
                Err(MuxerError::UnexpectedSignal { .. }) => continue,
                Err(err) => return Err(err.into()),
            }
        }
    } else {
        let signal = muxer.process_muxer.wait_for_signal();
        return Err(Error::Interrupt { signal });
    };
    Ok(())
}

fn tweak_postgresql_conf(db_path: &Path, port: Option<u16>) -> io::Result<()> {
    let mut conf_path = PathBuf::from(db_path);
    conf_path.push("postgresql.conf");
    let file = OpenOptions::new().append(true).open(conf_path)?;
    let mut file = BufWriter::new(file);
    let port_options: &[(&str, &str)] = match port {
        None => &[("listen_addresses", "")],
        Some(_) => &[],
    };
    let options: &[(&str, &str)] = &[
        ("unix_socket_directories", db_path.to_str().unwrap()),
        ("log_connections", "on"),
        ("log_disconnections", "on"),
    ];
    for options in [port_options, options] {
        for (k, v) in options {
            writeln!(file, "{k} = '{v}'")?;
        }
    }
    file.flush()?;
    Ok(())
}

#[derive(Debug)]
pub enum Error<'a> {
    DirError {
        dir_path: &'a Path,
        error: DirError,
    },
    Interrupt {
        signal: Signal,
    },
    UnexpectedChildTerminated {
        pid: Pid,
        prog_path: PathBuf,
        exit_status: ExitStatus,
    },
    InitDbError {
        error: io::Error,
    },
    AroundError {
        prog_path: PathBuf,
        error: io::Error,
    },
    InitDbBadExit,
}

impl<'a> From<MuxerError> for Error<'a> {
    fn from(err: MuxerError) -> Error<'a> {
        match err {
            MuxerError::UnexpectedSignal { signal } => Error::Interrupt { signal },
            MuxerError::UnexpectedChildTermination {
                pid,
                prog_path,
                exit_status,
            } => Error::UnexpectedChildTerminated {
                pid,
                prog_path,
                exit_status,
            },
        }
    }
}

impl<'a> From<Signal> for Error<'a> {
    fn from(signal: Signal) -> Error<'a> {
        Error::Interrupt { signal }
    }
}

impl<'a> From<(&'a Path, DirError)> for Error<'a> {
    fn from((dir_path, error): (&'a Path, DirError)) -> Error<'a> {
        Error::DirError { dir_path, error }
    }
}

#[derive(Debug)]
pub enum DirError {
    CreateFailed(io::Error),
    IsFile,
    IsNonempty,
}

impl std::fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DirError {
                dir_path: path,
                error: derr,
            } => match derr {
                DirError::CreateFailed(err) => {
                    write!(f, "create {} dir failed with {err}", path.display())
                }
                DirError::IsFile => {
                    write!(
                        f,
                        "{} is a file, but a directory was expected",
                        path.display()
                    )
                }
                DirError::IsNonempty => {
                    write!(f, "{} is a nonempty directory", path.display())
                }
            },
            Error::InitDbError { error, .. } => {
                write!(f, "initdb failed with {error}")
            }
            Error::InitDbBadExit => {
                write!(f, "initdb exited with a non-zero code")
            }
            Error::Interrupt { signal, .. } => {
                write!(f, "Interrupted by signal: {signal:?}")
            }
            Error::UnexpectedChildTerminated {
                pid,
                prog_path,
                exit_status,
            } => {
                write!(
                    f,
                    "{} [{pid}] terminated unexpectedly with {}",
                    prog_path.display(),
                    exit_status
                )
            }
            Error::AroundError { prog_path, error } => {
                write!(f, "Failed to spawn {}: {error}", prog_path.display())
            }
        }
    }
}

impl std::error::Error for Error<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::DirError {
                error: DirError::CreateFailed(err),
                ..
            } => Some(err),
            Error::InitDbError { error, .. } => Some(error),
            _ => None,
        }
    }
}
