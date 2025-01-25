use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::BufRead;
use std::{fs::File, process::Command};

use anyhow::{anyhow, bail, Context as _, Error, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Arg, ArgAction, ArgMatches, ValueHint};

pub struct GitCache {
    cache_base_dir: Utf8PathBuf,
}

impl GitCache {
    pub fn new(cache_base_dir: Utf8PathBuf) -> Result<Self, Error> {
        std::fs::create_dir_all(&cache_base_dir)
            .with_context(|| format!("creating git cache base directory {cache_base_dir}"))?;

        Ok(Self { cache_base_dir })
    }

    pub fn cloner(&self) -> GitCacheClonerBuilder {
        let mut cloner = GitCacheClonerBuilder::default();
        cloner.cache_base_dir(self.cache_base_dir.clone());
        cloner
    }
}

#[macro_use]
extern crate derive_builder;

#[derive(Builder)]
pub struct GitCacheCloner {
    cache_base_dir: Utf8PathBuf,
    #[builder(setter(custom))]
    repository_url: String,
    #[builder(default = "true")]
    cached: bool,
    #[builder(default)]
    update: bool,
    #[builder(default)]
    target_path: Option<Utf8PathBuf>,
    #[builder(default)]
    sparse_paths: Option<Vec<String>>,
    #[builder(default)]
    recurse_submodules: Option<Vec<String>>,
    #[builder(default)]
    recurse_all_submodules: bool,
    #[builder(default)]
    shallow_submodules: bool,
    #[builder(default)]
    commit: Option<String>,
    #[builder(default)]
    extra_clone_args: Option<Vec<String>>,
}

impl GitCacheClonerBuilder {
    pub fn repository_url(&mut self, url: String) -> &mut Self {
        if self.cached.is_none() {
            self.cached = Some(!repo_is_local(&url));
        }
        self.repository_url = Some(url);
        self
    }

    pub fn do_clone(&mut self) -> Result<(), Error> {
        self.build()
            .expect("GitCacheCloner builder correctly set up")
            .do_clone()
    }
    pub fn extra_clone_args_from_matches(&mut self, matches: &ArgMatches) -> &mut Self {
        self.extra_clone_args(Some(get_pass_through_args(matches)))
    }
}

/// returns `true` if the git repo url points to a local path
///
/// This function tries to mimic Git's notion of a local repository.
///
/// Some things to watch out for:
/// - this does not take bundles into account
fn repo_is_local(url: &str) -> bool {
    if let Ok(url) = url::Url::parse(url) {
        url.scheme() == "file"
    } else {
        (url.starts_with("./") || url.starts_with('/'))
            || (!url_is_scp_scheme(url))
            || std::path::Path::new(url).exists()
    }
}

fn url_is_scp_scheme(url: &str) -> bool {
    let at = url.find('@');
    let colon = url.find(':');

    if let Some(colon_pos) = colon {
        if let Some(at_pos) = at {
            if at_pos < colon_pos {
                return true;
            }
        }
    }

    false
}

impl GitCacheCloner {
    fn do_clone(&self) -> Result<(), Error> {
        let repository = &self.repository_url;
        let wanted_commit = self.commit.as_ref();
        let target_path;

        if self.cached {
            let cache_repo = GitCacheRepo::new(&self.cache_base_dir, &self.repository_url);
            target_path = cache_repo.target_path(self.target_path.as_ref())?;

            let mut lock = cache_repo.lockfile()?;
            {
                let _lock = lock.write()?;
                if !cache_repo.mirror()? {
                    let try_update =
                        wanted_commit.is_some_and(|commit| !cache_repo.has_commit(commit).unwrap());

                    if self.update || try_update {
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
                cache_repo.clone(target_path.as_str(), self.extra_clone_args.as_ref())?;
            }
        } else {
            target_path =
                target_path_from_url_maybe(&self.repository_url, self.target_path.as_ref())?;

            direct_clone(
                &self.repository_url,
                target_path.as_str(),
                self.extra_clone_args.as_ref(),
            )?;
        }

        let target_repo = GitRepo {
            path: target_path.clone(),
        };

        if let Some(commit) = wanted_commit {
            target_repo.set_config("advice.detachedHead", "false")?;
            target_repo.checkout(commit)?;
        }
        if let Some(sparse_paths) = self.sparse_paths.as_ref() {
            target_repo.sparse_checkout(sparse_paths)?;
        }

        if self.recurse_all_submodules || self.recurse_submodules.is_some() {
            let filter = if !self.recurse_all_submodules {
                self.recurse_submodules.clone()
            } else {
                None
            };

            let cache = self.cache()?;

            for submodule in target_repo.get_submodules(filter)? {
                println!(
                    "recursively cloning {} into {}... (todo)",
                    submodule.url, submodule.path
                );
                target_repo.clone_submodule(
                    submodule,
                    &cache,
                    self.shallow_submodules,
                    self.update,
                )?;
            }
        };

        Ok(())
    }

    pub fn cache(&self) -> Result<GitCache, anyhow::Error> {
        GitCache::new(self.cache_base_dir.clone())
    }
}

pub struct GitRepo {
    path: Utf8PathBuf,
}

pub struct GitCacheRepo {
    url: String,
    repo: GitRepo,
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

    fn submodule_commits(&self) -> Result<HashMap<String, String>> {
        let output = self.git().arg("submodule").arg("status").output()?;

        let res = output
            .stdout
            .lines()
            .map(|line| line.unwrap())
            .map(|line| {
                // ` f47ce7b5fbbb3aa43d33d2be1f6cd3746b13d5bf some/path`
                let commit = line[1..41].to_string();
                let path = line[42..].to_string();
                (path, commit)
            })
            .collect::<HashMap<String, String>>();
        Ok(res)
    }

    fn sparse_checkout<I, S>(&self, sparse_paths: I) -> std::result::Result<(), anyhow::Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.git()
            .arg("sparse-checkout")
            .arg("set")
            .args(sparse_paths)
            .status()?
            .success()
            .true_or(anyhow!("error setting up sparse checkout"))
    }

    fn get_submodules(
        &self,
        filter: Option<Vec<String>>,
    ) -> std::result::Result<Vec<SubmoduleSpec>, anyhow::Error> {
        use gix_config::File;
        let mut path = self.path.clone();
        path.push(".gitmodules");

        if !path.exists() {
            return Ok(Vec::new());
        }

        let gitconfig = File::from_path_no_includes(path.into(), gix_config::Source::Api)?;
        let gitmodules = gitconfig.sections_by_name("submodule");

        if gitmodules.is_none() {
            return Ok(Vec::new());
        }

        let submodule_commits = self.submodule_commits()?;

        println!("{:?}", submodule_commits);

        let mut submodules = Vec::new();
        for module in gitmodules.unwrap() {
            let path = module.body().value("path");
            let url = module.body().value("url");
            let branch = module.body().value("branch").map(|b| b.to_string());

            if path.is_none() || url.is_none() {
                eprintln!("git-cache: submodule missing path or url");
                continue;
            }
            let path = path.unwrap().into_owned().to_string();
            let url = url.unwrap().into_owned().to_string();

            let commit = submodule_commits.get(&path);

            if commit.is_none() {
                eprintln!("git-cache: could not find submodule commit for path `{path}`");
            }

            if let Some(filter) = filter.as_ref() {
                if !filter.contains(&path) {
                    continue;
                }
            }

            submodules.push(SubmoduleSpec::new(
                path,
                url,
                commit.unwrap().clone(),
                branch,
            ));
        }

        Ok(submodules)
    }

    fn clone_submodule(
        &self,
        submodule: SubmoduleSpec,
        cache: &GitCache,
        shallow_submodules: bool,
        update: bool,
    ) -> std::result::Result<(), anyhow::Error> {
        let submodule_path = self.path.join(&submodule.path);

        let mut cloner = cache.cloner();

        cloner
            .repository_url(submodule.url)
            .target_path(Some(submodule_path))
            .recurse_all_submodules(true)
            .shallow_submodules(shallow_submodules)
            .commit(Some(submodule.commit))
            .update(update);

        // if let Some(branch) = submodule.branch {
        //     cloner.extra_clone_args(Some(vec!["--branch".into(), branch]));
        // }

        cloner.do_clone()?;

        self.init_submodule(&submodule.path)?;

        Ok(())
    }

    fn init_submodule(&self, path: &str) -> std::result::Result<(), anyhow::Error> {
        self.git()
            .arg("submodule")
            .arg("init")
            .arg("--")
            .arg(path)
            .status()?
            .success()
            .true_or(anyhow!("error initializing submodule"))
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
            Command::new("git")
                .arg("clone")
                .arg("--mirror")
                .arg("--")
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

    fn clone(&self, target_path: &str, pass_through_args: Option<&Vec<String>>) -> Result<()> {
        direct_clone(self.repo.path.as_str(), target_path, pass_through_args)?;

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

    pub fn target_path(&self, target_path: Option<&Utf8PathBuf>) -> Result<Utf8PathBuf> {
        target_path_from_url_maybe(&self.url, target_path)
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

fn direct_clone(
    repo: &str,
    target_path: &str,
    pass_through_args: Option<&Vec<String>>,
) -> Result<(), Error> {
    let mut clone_cmd = Command::new("git");
    clone_cmd.arg("clone").arg("--shared");
    if let Some(args) = pass_through_args {
        clone_cmd.args(args);
    }
    clone_cmd
        .arg("--")
        .arg(repo)
        .arg(target_path)
        .status()?
        .success()
        .true_or(anyhow!("cloning failed"))?;
    Ok(())
}

fn target_path_from_url_maybe(
    url: &str,
    target_path: Option<&Utf8PathBuf>,
) -> Result<Utf8PathBuf, Error> {
    target_path.map(shellexpand::tilde);

    let url_path = Utf8PathBuf::from(url);
    let url_path_filename = Utf8PathBuf::from(url_path.file_name().unwrap());
    let target_path = target_path.unwrap_or(&url_path_filename);

    if !target_path.is_clone_target()? {
        return Err(anyhow!(
            "fatal: destination path '{target_path}' already exists and is not an empty directory."
        ));
    }

    Ok(target_path.clone())
}

pub fn clap_git_cache_dir_arg() -> Arg {
    Arg::new("git_cache_dir")
        .short('c')
        .long("cache-dir")
        .help("git cache base directory")
        .required(false)
        .default_value("~/.gitcache")
        .value_parser(clap::value_parser!(Utf8PathBuf))
        .value_hint(ValueHint::DirPath)
        .env("GIT_CACHE_DIR")
        .num_args(1)
}

pub fn clap_clone_command(name: &'static str) -> clap::Command {
    use clap::Command;
    Command::new(name)
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
        .arg(
            Arg::new("recurse-submodules")
                .long("recurse-submodules")
                .value_name("pathspec")
                .action(ArgAction::Append)
                .num_args(0..=1)
                .require_equals(true)
                .help("recursively clone submodules"),
        )
        .arg(
            Arg::new("shallow-submodules")
                .long("shallow-submodules")
                .action(ArgAction::SetTrue)
                .overrides_with("no-shallow-submodules")
                .help("shallow-clone submodules"),
        )
        .arg(
            Arg::new("no-shallow-submodules")
                .long("no-shallow-submodules")
                .action(ArgAction::SetTrue)
                .overrides_with("shallow-submodules")
                .help("don't shallow-clone submodules"),
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
        )
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
        "no-single-branch",
        "no-tags",
        "reject-shallow",
        "remote-submodules",
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

    args
}

fn get_pass_through_args(matches: &ArgMatches) -> Vec<String> {
    let mut args = Vec::new();
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
        "no-single-branch",
        "no-tags",
        "reject-shallow",
        "remote-submodules",
        "single-branch",
        "sparse",
    ]
    .into_iter()
    {
        if matches.get_flag(id) {
            args.push(format!("--{id}"));
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
                args.push(format!("--{id}"));
                args.push(occurrence.clone());
            }
        }
    }

    args
}

trait CanCloneInto {
    fn is_clone_target(&self) -> Result<bool, Error>;
}

impl CanCloneInto for camino::Utf8Path {
    fn is_clone_target(&self) -> Result<bool, Error> {
        Ok((!self.exists()) || (self.is_dir() && { self.read_dir()?.next().is_none() }))
    }
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

#[derive(Debug, Clone)]
struct SubmoduleSpec {
    path: String,
    url: String,
    #[allow(dead_code)]
    branch: Option<String>,
    commit: String,
}

impl SubmoduleSpec {
    pub fn new(path: String, url: String, commit: String, branch: Option<String>) -> Self {
        Self {
            path,
            url,
            commit,
            branch,
        }
    }
}
