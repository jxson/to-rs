#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate log;
extern crate loggerv;
extern crate to;

use std::path::PathBuf;
use prettytable::Table;
use to::{cli, dir};
use to::cli::Action;
use to::database::Database;
use to::errors::*;
use std::io::{stderr, stdout, Write};
use std::process::exit;
use log::LogLevel;

fn main() {
    if let Err(ref err) = run(cli::app().get_matches(), &mut stdout()) {
        let stderr = &mut stderr();
        let stderr_msg = "Error writing to stderr";

        for cause in err.iter() {
            writeln!(stderr, "{}", cause).expect(stderr_msg);
        }

        if let Some(backtrace) = err.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).expect(stderr_msg);
        }

        exit(1);
    };
}

fn run<T: Write + ?Sized>(matches: cli::ArgMatches, out: &mut T) -> Result<()> {
    let options = try!(cli::Options::new(matches));

    // TODO(jxson): see about fixing the name of the log.
    // TODO(jxson): configure logger based on user input.
    match loggerv::init_with_level(LogLevel::Info) {
        Ok(_) => debug!("logger initialized"),
        Err(_) => {}, // Ignored due to tests reusing the log singleton.
    }

    // --init # echo the shell script for the `to` function.
    if options.initialize {
        try!(write!(out, "{}", include_str!("to.sh")));
        return Ok(());
    }

    let config = PathBuf::from(&options.config);

    if !config.exists() {
        info!("creating config dir: {:?}", &config);
        try!(dir::mkdirp(&config));
    }

    let mut store = try!(Database::open(config));

    match options.action {
        Action::Info => info(&store, &options),
        Action::Save => store.put(options.name, options.path),
        Action::Delete => store.delete(options.name),
        Action::List => list(&store, out),
        Action::Pathname => {
            let path = try!(store.get_path(&options.name));
            try!(write!(out, "{}", path.to_string_lossy()));
            Ok(())
        }
    }
}

fn info(store: &Database, options: &cli::Options) -> Result<()> {
    match store.get(&options.name) {
        Some(bookmark) => println!("bookmark: {:?}", bookmark),
        None => println!("Not found"),
    }

    Ok(())
}

fn list<T: Write + ?Sized>(store: &Database, out: &mut T) -> Result<()> {
    let mut table = Table::new();
    table.add_row(row![ b => "Name", "Path", "Count"]);

    for (name, bookmark) in store.list() {
        let path = bookmark.directory.to_string_lossy();
        table.add_row(row![name, path, bookmark.count]);
    }

    try!(table.print(out));

    Ok(())
}

#[cfg(test)]
mod test {
    extern crate tempdir;

    use super::*;
    use self::tempdir::TempDir;
    use std::io::{self, Write};

    struct TestWriter {}

    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn config() -> String {
        let tempdir = TempDir::new("mkdirp-test").unwrap();
        format!("{:?}", tempdir.path())
    }

    fn go(mut args: Vec<&str>) -> Result<()> {
        args.insert(0, "to");
        let matches = cli::app().get_matches_from(args);
        run(matches, &mut TestWriter {})
    }

    #[test]
    fn init_flag() {
        let result = go(vec!["--init"]);
        assert!(result.is_ok());
    }

    #[test]
    fn config_flag_non_existing() {
        let dir = TempDir::new("existing-dir")
            .map(|dir| dir.into_path().join("non-existing"))
            .unwrap();
        let config = dir.to_str().unwrap();
        let matches = cli::app().get_matches_from(vec!["to", "--config", config, "--info"]);

        assert_eq!(dir.exists(), false);
        assert!(run(matches, &mut TestWriter {}).is_ok());
        assert!(dir.exists());
    }

    #[test]
    fn config_flag_existing() {
        assert!(go(vec!["--config", &config()]).is_err());
    }

    #[test]
    fn save_flag_without_value() {
        assert!(go(vec!["--save"]).is_ok());
    }

    #[test]
    fn save_flag_with_value() {
        assert!(go(vec!["--save", "foo"]).is_ok());
    }

    #[test]
    fn delete_flag_existing() {
        let config = &config();
        assert!(go(vec!["--config", config, "--save", "foo"]).is_ok());
        assert!(go(vec!["--config", config, "--delete", "foo"]).is_ok());
    }

    #[test]
    fn delete_flag_non_existing() {
        let key = String::from("foo");
        let err = go(vec!["--config", &config(), "--delete", "foo"])
            .err()
            .unwrap();

        assert_eq!(
            format!("{}", ErrorKind::BookmarkNotFound(key)),
            format!("{}", err)
        );
    }

    #[test]
    fn list_flag() {
        let config = &config();
        assert!(go(vec!["--config", config, "--save", "foo"]).is_ok());
        assert!(go(vec!["--config", config, "--list"]).is_ok());
    }

    #[test]
    fn name_option_non_existing() {
        let key = String::from("foo");
        let err = go(vec!["--config", &config(), "foo"]).err().unwrap();
        assert_eq!(
            format!("{}", ErrorKind::BookmarkNotFound(key)),
            format!("{}", err)
        );
    }

    #[test]
    fn name_option() {
        let config = &config();
        assert!(go(vec!["--config", config, "--save", "foo"]).is_ok());
        assert!(go(vec!["--config", config, "foo"]).is_ok());
    }
}
