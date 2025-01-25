use std::process::ExitCode;

use anyhow::Result;
use camino::Utf8PathBuf;
use clap::crate_version;
use git_cache::GitCache;

fn clap() -> clap::Command {
    use clap::Command;
    Command::new("git-cache")
        .version(crate_version!())
        .author("Kaspar Schleiser <kaspar@schleiser.de>")
        .about("A git repository cache tool")
        .infer_subcommands(true)
        .arg(git_cache::clap_git_cache_dir_arg())
        .subcommand(git_cache::clap_clone_command("clone"))
        .subcommand(
            // this is a noop, we keep it for backwards compatibility with the
            // previous shell implementation
            Command::new("init").hide(true),
        )
}

fn main() -> Result<ExitCode> {
    let matches = clap().get_matches();

    let cache_dir = Utf8PathBuf::from(&shellexpand::tilde(
        matches.get_one::<Utf8PathBuf>("git_cache_dir").unwrap(),
    ));

    match matches.subcommand() {
        Some(("clone", matches)) => {
            let repository = matches.get_one::<String>("repository").unwrap();
            let target_path = matches.get_one::<Utf8PathBuf>("target_path").cloned();
            let wanted_commit = matches.get_one::<String>("commit");
            let sparse_paths = matches
                .get_many::<String>("sparse-add")
                .map(|v| v.into_iter().cloned().collect::<Vec<String>>());

            let recurse_submodules = matches
                .get_many::<String>("recurse-submodules")
                .map(|v| v.into_iter().cloned().collect::<Vec<String>>());

            let recurse_all_submodules = recurse_submodules
                .as_ref()
                .is_some_and(|submodules| submodules.is_empty())
                && matches.contains_id("recurse-submodules");

            let shallow_submodules = matches.get_flag("shallow-submodules");
            if shallow_submodules {
                println!("git-cache: warning: shallow submodule clones not supported");
            }

            let git_cache = GitCache::new(cache_dir)?;
            git_cache
                .cloner()
                .commit(wanted_commit.cloned())
                .extra_clone_args_from_matches(matches)
                .repository_url(repository.clone())
                .sparse_paths(sparse_paths)
                .target_path(target_path)
                .update(matches.get_flag("update"))
                .recurse_submodules(recurse_submodules)
                .recurse_all_submodules(recurse_all_submodules)
                .shallow_submodules(shallow_submodules)
                .do_clone()?;
        }
        Some(("other", _matches)) => {}
        _ => {}
    }

    Ok(0.into())
}
