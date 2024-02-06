use std::{
    fs::File,
    process::{Command, ExitCode},
};

use anyhow::{anyhow, bail, Context as _, Error, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::{crate_version, Arg, ArgAction, ArgMatches, ValueHint};

struct GitRepo {
    path: Utf8PathBuf,
}

struct GitCacheRepo {
    url: String,
    repo: GitRepo,
}

trait TrueOr {
    fn true_or(self, error: Error) -> Result<()>;
}

impl TrueOr for bool {
    fn true_or(self, error: Error) -> Result<()> {
        if self {
            Ok(())
        } else {
            Err(error)
        }
    }
}

impl GitRepo {
    fn git(&self) -> std::process::Command {
        let mut command = Command::new("git");
        command.arg("-C").arg(&self.path);

        command
    }

    fn is_initialized(&self) -> Result<bool> {
        Ok(self.path.is_dir()
            && matches!(
                self.git()
                    .arg("rev-parse")
                    .arg("--git-dir")
                    .output()?
                    .stdout
                    .as_slice(),
                b".\n" | b".git\n"
            ))
    }

    fn has_commit(&self, commit: &str) -> Result<bool> {
        Ok(self
            .git()
            .arg("cat-file")
            .arg("-e")
            .arg(format!("{}^{{commit}}", commit))
            .status()?
            .success())
    }

    fn set_config(&self, key: &str, value: &str) -> Result<()> {
        self.git()
            .arg("config")
            .arg(key)
            .arg(value)
            .status()?
            .success()
            .true_or(anyhow!("cannot set configuration value"))
    }

    fn checkout(&self, commit: &str) -> Result<()> {
        self.git()
            .arg("checkout")
            .arg(commit)
            .status()?
            .success()
            .true_or(anyhow!("error checking out commit"))
    }

    fn sparse_checkout(
        &self,
        sparse_paths: clap::parser::ValuesRef<'_, String>,
    ) -> std::result::Result<(), anyhow::Error> {
        self.git()
            .arg("sparse-checkout")
            .arg("set")
            .args(sparse_paths)
            .status()?
            .success()
            .true_or(anyhow!("error setting up sparse checkout"))
    }
}

impl GitCacheRepo {
    pub fn new(base_path: &Utf8Path, url: &str) -> Self {
        let mut path = base_path.to_path_buf();
        path.push(Self::url_to_slug(url));
        let cache_path = Utf8PathBuf::from(&path);
        Self {
            repo: GitRepo { path: cache_path },
            url: url.to_string(),
        }
    }

    fn mirror(&self) -> Result<bool> {
        if !self.repo.is_initialized()? {
            println!("git-cache: cloning {} into cache...", self.url);
            std::fs::create_dir_all(&self.repo.path)?;
            self.repo
                .git()
                .arg("clone")
                .arg("--mirror")
                .arg(&self.url)
                .arg(&self.repo.path)
                .status()?
                .success()
                .true_or(anyhow!("error mirroring repository"))?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn update(&self) -> Result<()> {
        self.repo
            .git()
            .arg("remote")
            .arg("update")
            .status()?
            .success()
            .true_or(anyhow!("error updating repository"))
    }

    fn url_to_slug(url: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        format!("{}.git", hasher.finish())
    }

    fn clone(&self, target_path: &str, matches: &ArgMatches) -> Result<()> {
        let mut clone_cmd = Command::new("git");
        clone_cmd.arg("clone").arg("--shared");

        apply_pass_through(&mut clone_cmd, matches);

        clone_cmd
            .arg("--")
            .arg(&self.repo.path)
            .arg(target_path)
            .status()?
            .success()
            .true_or(anyhow!("cloning failed"))?;

        Command::new("git")
            .arg("-C")
            .arg(target_path)
            .arg("remote")
            .arg("set-url")
            .arg("origin")
            .arg(&self.url)
            .status()?
            .success()
            .true_or(anyhow!("error updating remote url"))?;
        Ok(())
    }

    fn target_path(&self, target_path: Option<&Utf8PathBuf>) -> Result<Utf8PathBuf> {
        target_path.map(shellexpand::tilde);

        let url_path = Utf8PathBuf::from(&self.url);
        let url_path_filename = Utf8PathBuf::from(url_path.file_name().unwrap());
        let target_path = target_path.unwrap_or(&url_path_filename);

        if !target_path.is_clone_target()? {
            return Err(anyhow!(
                    "fatal: destination path '{target_path}' already exists and is not an empty directory."
                ));
        }

        Ok(target_path.clone())
    }

    // fn is_initialized(&self) -> std::result::Result<bool, anyhow::Error> {
    //     self.repo.is_initialized()
    // }

    fn has_commit(&self, commit: &str) -> std::result::Result<bool, anyhow::Error> {
        self.repo.has_commit(commit)
    }

    fn lockfile(&self) -> Result<fd_lock::RwLock<File>> {
        let lock_path = self.repo.path.with_extension("lock");
        Ok(fd_lock::RwLock::new(
            std::fs::File::create(&lock_path)
                .with_context(|| format!("creating lock file \"{lock_path}\""))?,
        ))
    }
}

fn clap() -> clap::Command {
    use clap::Command;
    Command::new("git-cache")
        .version(crate_version!())
        .author("Kaspar Schleiser <kaspar@schleiser.de>")
        .about("A git repository cache tool")
        .infer_subcommands(true)
        .arg(
            Arg::new("cache_dir")
                .short('c')
                .long("cache-dir")
                .help("git cache base directory")
                .required(false)
                .default_value("~/.gitcache")
                .value_parser(clap::value_parser!(Utf8PathBuf))
                .value_hint(ValueHint::DirPath)
                .env("GIT_CACHE_DIR")
                .num_args(1),
        )
        .subcommand(
            Command::new("clone")
                .about("clone repository")
                .arg(
                    Arg::new("repository")
                        .help("repository to clone")
                        .required(true),
                )
                .arg(
                    Arg::new("target_path")
                        .help("target path")
                        .required(false)
                        .value_parser(clap::value_parser!(Utf8PathBuf))
                        .value_hint(ValueHint::DirPath),
                )
                .arg(
                    Arg::new("update")
                        .short('U')
                        .long("update")
                        .action(ArgAction::SetTrue)
                        .help("force update of cached repo"),
                )
                .arg(
                    Arg::new("commit")
                        .long("commit")
                        .value_name("HASH")
                        .conflicts_with("branch")
                        .help("check out specific commit"),
                )
                .arg(
                    Arg::new("sparse-add")
                        .long("sparse-add")
                        .value_name("PATH")
                        .conflicts_with("branch")
                        .action(ArgAction::Append)
                        .help("do a sparse checkout, keep PATH"),
                )
                .args(pass_through_args())
                .after_help(
                    "These regular \"git clone\" options are passed through:\n
        [--template=<template-directory>]
        [-l] [-s] [--no-hardlinks] [-q] [-n] [--bare] [--mirror]
        [-o <name>] [-b <name>] [-u <upload-pack>] [--reference <repository>]
        [--dissociate] [--separate-git-dir <git-dir>]
        [--depth <depth>] [--[no-]single-branch] [--no-tags]
        [--recurse-submodules[=<pathspec>]] [--[no-]shallow-submodules]
        [--[no-]remote-submodules] [--jobs <n>] [--sparse] [--[no-]reject-shallow]
        [--filter=<filter> [--also-filter-submodules]]",
                ),
        )
        .subcommand(Command::new("init").hide(true))
}

fn pass_through_args() -> Vec<Arg> {
    let mut args = Vec::new();

    // short w/o arg
    for (short, long) in [
        ('l', "local"),
        //        ('n', "no-checkout"),
        ('q', "quiet"),
        ('s', "shared"),
        ('v', "verbose"),
    ]
    .into_iter()
    {
        args.push(
            Arg::new(long)
                .short(short)
                .long(long)
                .hide(true)
                .action(ArgAction::SetTrue),
        );
    }

    //
    args.push(
        Arg::new("no-checkout")
            .short('n')
            .long("no-checkout")
            .hide(true)
            .num_args(0)
            .default_value_if("commit", clap::builder::ArgPredicate::IsPresent, "true"),
    );

    args.push(
        Arg::new("sparse")
            .long("sparse")
            .hide(true)
            .num_args(0)
            .default_value_if("sparse-add", clap::builder::ArgPredicate::IsPresent, "true"),
    );

    // short with arg
    for (short, long) in [
        ('b', "branch"),
        ('c', "config"),
        ('j', "jobs"),
        ('o', "origin"),
        ('u', "upload-pack"),
    ]
    .into_iter()
    {
        args.push(
            Arg::new(long)
                .short(short)
                .long(long)
                .num_args(1)
                .hide(true),
        );
    }

    // long w/o arg
    for id in [
        "also-filter-submodules",
        "bare",
        "dissociate",
        "mirror",
        "no-hardlinks",
        "no-reject-shallow",
        "no-remote-submodules",
        "no-shallow-submodules",
        "no-single-branch",
        "no-tags",
        "reject-shallow",
        "remote-submodules",
        "shallow-submodules",
        "single-branch",
    ]
    .into_iter()
    {
        args.push(Arg::new(id).long(id).action(ArgAction::SetTrue).hide(true));
    }

    // long with arg always
    for id in [
        "bundle-uri",
        "depth",
        "filter",
        "reference",
        "reference-if-able",
        "separate-git-dir",
        "shallow-exclude",
        "shallow-since",
        "template",
    ]
    .into_iter()
    {
        args.push(Arg::new(id).long(id).num_args(1).hide(true));
    }

    // long with optional arg
    args.push(
        Arg::new("recurse-submodules")
            .long("recurse-submodules")
            .num_args(0..=1)
            .action(ArgAction::Append)
            .hide(true),
    );

    args
}

fn apply_pass_through(clone_cmd: &mut Command, matches: &ArgMatches) {
    // w/o arg
    for id in [
        "local",
        "no-checkout",
        "quiet",
        "shared",
        "verbose",
        "also-filter-submodules",
        "bare",
        "dissociate",
        "mirror",
        "no-hardlinks",
        "no-reject-shallow",
        "no-remote-submodules",
        "no-shallow-submodules",
        "no-single-branch",
        "no-tags",
        "reject-shallow",
        "remote-submodules",
        "shallow-submodules",
        "single-branch",
        "sparse",
    ]
    .into_iter()
    {
        if matches.get_flag(id) {
            clone_cmd.arg(format!("--{id}"));
        }
    }

    // with arg always
    for id in [
        "branch",
        "bundle-uri",
        "config",
        "depth",
        "filter",
        "jobs",
        "origin",
        "reference",
        "reference-if-able",
        "separate-git-dir",
        "shallow-exclude",
        "shallow-since",
        "template",
        "upload-pack",
    ]
    .into_iter()
    {
        if let Some(occurrences) = matches.get_occurrences::<String>(id) {
            for occurrence in occurrences.flatten() {
                clone_cmd.arg(format!("--{id}"));
                clone_cmd.arg(occurrence);
            }
        }
    }

    if let Some(submodules) = matches.get_many::<String>("recurse-submodules") {
        if submodules.len() == 0 && matches.contains_id("recurse-submodules") {
            clone_cmd.arg("--recurse-submodules");
        } else {
            for submodule in submodules {
                clone_cmd.arg(format!("--recurse-submodules={submodule}"));
            }
        }
    }
}

trait CanCloneInto {
    fn is_clone_target(&self) -> Result<bool, Error>;
}

impl CanCloneInto for camino::Utf8Path {
    fn is_clone_target(&self) -> Result<bool, Error> {
        Ok((!self.exists()) || (self.is_dir() && { self.read_dir()?.next().is_none() }))
    }
}

fn main() -> Result<ExitCode> {
    let matches = clap().get_matches();

    let cache_dir: Utf8PathBuf = Utf8PathBuf::from(&shellexpand::tilde(
        matches.get_one::<Utf8PathBuf>("cache_dir").unwrap(),
    ));

    match matches.subcommand() {
        Some(("clone", matches)) => {
            let repository = matches.get_one::<String>("repository").unwrap();
            let target_path = matches.get_one::<Utf8PathBuf>("target_path");

            let cache_repo = GitCacheRepo::new(&cache_dir, repository);

            let target_path = cache_repo.target_path(target_path)?;

            std::fs::create_dir_all(&cache_dir)
                .with_context(|| format!("creating cache base directory {cache_dir}"))?;

            let wanted_commit = matches.get_one::<String>("commit");

            let mut lock = cache_repo.lockfile()?;
            {
                let _lock = lock.write()?;
                if !cache_repo.mirror()? {
                    let try_update =
                        wanted_commit.is_some_and(|commit| !cache_repo.has_commit(commit).unwrap());

                    if matches.get_flag("update") || try_update {
                        println!("git-cache: updating cache for {repository}...");
                        cache_repo.update()?;
                    }

                    if let Some(commit) = wanted_commit {
                        if try_update && !cache_repo.has_commit(commit)? {
                            bail!("git-cache: {repository} does not contain commit {commit}");
                        }
                    }
                }
            }

            {
                let _lock = lock.read()?;
                cache_repo.clone(target_path.as_str(), matches)?;
            }
            if let Some(commit) = wanted_commit {
                let target_repo = GitRepo {
                    path: target_path.clone(),
                };
                target_repo.set_config("advice.detachedHead", "false")?;
                target_repo.checkout(commit)?;
            }
            if let Some(sparse_paths) = matches.get_many::<String>("sparse-add") {
                let target_repo = GitRepo {
                    path: target_path.clone(),
                };
                target_repo.sparse_checkout(sparse_paths)?;
            }
        }
        Some(("other", _matches)) => {}
        _ => {}
    }

    Ok(0.into())
}
