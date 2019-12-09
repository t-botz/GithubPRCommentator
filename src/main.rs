mod github;

use std::fs;
use std::io::{self, Read};
use std::str::FromStr;

use anyhow::{Context, Result};
use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg, ArgMatches};
use env_logger;
use github::{get_repo_info_from_url, GithubAPI, DEFAULT_GITHUB_API_URL};
use log::{debug, info};
use strum_macros::{EnumString, EnumVariantNames};
use url::Url;

#[derive(Debug)]
enum CommentSource {
    StrArg { comment: String },
    Standard(io::Stdin),
    File(fs::File),
}

impl CommentSource {
    pub fn retrieve(self) -> Result<String> {
        match self {
            CommentSource::StrArg { comment } => Ok(comment),
            CommentSource::Standard(mut stdin) => {
                debug!("Reading stdin for comment");
                let mut buffer = String::new();
                stdin
                    .read_to_string(&mut buffer)
                    .map(|_| buffer)
                    .context("Failed to read comment from stdin")
            }
            CommentSource::File(mut file) => {
                debug!("Reading file for comment");
                let mut buffer = String::new();
                file.read_to_string(&mut buffer)
                    .map(|_| buffer)
                    .context("Failed to read comment from file")
            }
        }
    }
}

/// Define the behaviour when writing the comment on the PR
#[derive(Debug, EnumString, EnumVariantNames)]
enum CommentOverwriteMode {
    /// Dont check for existing generated comment, just append
    Never,
    /// Always overwrite previous generated comment
    Always,
    /// Overwrite only if previous comment was made on same commit
    OnSameCommit,
}

impl Default for CommentOverwriteMode {
    fn default() -> CommentOverwriteMode {
        CommentOverwriteMode::Always
    }
}

#[derive(Debug)]
pub struct Config {
    api: GithubAPI,
    repo_owner: String,
    repo_name: String,
    branch_name: String,
    comment_source: CommentSource,
    overwrite_mode: CommentOverwriteMode,
}

fn parse_cli() -> Result<Config> {
    fn get_arg(app: &ArgMatches, arg: &Arg) -> String {
        app.value_of(arg.b.name).unwrap().to_owned()
    }

    let repo_url_arg = Arg::with_name("Repo Url")
        .long("repo-url")
        .help(
            "The repository url, used to deduce the repo name, api url and \
             organization. This is evaluated first if present and can be overridden",
        )
        .takes_value(true);
    let api_url_arg = Arg::with_name("Api Url")
        .long("api-url")
        .help("The Github api base url")
        .takes_value(true);
    let token_arg = Arg::with_name("token")
        .long("token")
        .help("The Github token to use")
        .required(true)
        .takes_value(true);
    let org_arg = Arg::with_name("GitHub organization")
        .long("org")
        .required_unless(repo_url_arg.b.name)
        .help("The Github organization or username containing the repo")
        .takes_value(true);
    let repo_arg = Arg::with_name("Repo name")
        .long("repo")
        .required_unless(repo_url_arg.b.name)
        .help("The repository name")
        .takes_value(true);
    let branch_arg = Arg::with_name("Git reference")
        .long("ref")
        .required(true)
        .help("The reference name to retrieve the PR number (e.g. 'refs/head/my_branch')")
        .takes_value(true);
    let comment_file_arg = Arg::with_name("Comment Input File")
        .long("comment-file")
        .help("A file containing the countent of the comment")
        .takes_value(true);
    let std_in_arg = Arg::with_name("Stdin flag")
        .long("use-stdin")
        .help("If no comment provided, allow the program to read from stdin");
    let comment_arg = Arg::with_name("Comment")
        .long("comment")
        .help("The content of the comment")
        .required_unless_one(&[comment_file_arg.b.name, std_in_arg.b.name])
        .takes_value(true);
    let overwrite_mode_arg = Arg::with_name("PR Comment Overwrite Mode")
        .long("overwrite")
        .possible_values(&CommentOverwriteMode::variants())
        .help("Whether previous comment in the PR should be overwritten");
    let app = App::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .author(crate_authors!())
        .long_about(
            format!(
                "The content comment can be provided in several way. \
                 The program will first look for the `{}` arg, \
                 if absent try to get the content from a file specified by the {} arg, \
                 if absent and {} arg program, it will read from stdin, \
                 otherwise exit unsucessfully",
                comment_arg.s.long.unwrap(),
                comment_file_arg.s.long.unwrap(),
                std_in_arg.s.long.unwrap()
            )
            .as_ref(),
        )
        .arg(&repo_url_arg)
        .arg(&api_url_arg)
        .arg(&token_arg)
        .arg(&org_arg)
        .arg(&repo_arg)
        .arg(&branch_arg)
        .arg(&comment_arg)
        .arg(&comment_file_arg)
        .arg(&std_in_arg)
        .arg(&overwrite_mode_arg)
        .get_matches();

    let repo_info = app.value_of(&repo_url_arg.b.name).map(|repo_url| {
        Url::from_str(repo_url)
            .with_context(|| format!("Invalid url `{}", repo_url))
            .and_then(get_repo_info_from_url)
            .unwrap_or_else(|err| {
                clap::Error {
                    message: format!("Invalid repo url {} : {}", repo_url, err),
                    kind: clap::ErrorKind::ValueValidation,
                    info: None,
                }
                .exit()
            })
    });

    let (repo_info_api_url, repo_info_name, repo_info_org) = if let Some(repo_info) = repo_info {
        (
            Some(repo_info.api_url),
            Some(repo_info.name),
            Some(repo_info.org),
        )
    } else {
        (None, None, None)
    };

    let api_url = app
        .value_of(api_url_arg.b.name)
        .map(|url| {
            Url::from_str(url).unwrap_or_else(|err| {
                clap::Error {
                    message: format!("Invalid repo url {} : {}", url, err),
                    kind: clap::ErrorKind::ValueValidation,
                    info: None,
                }
                .exit()
            })
        })
        .or(repo_info_api_url)
        .unwrap_or_else(|| DEFAULT_GITHUB_API_URL.clone());

    let repo = app
        .value_of(&repo_arg.b.name)
        .map(ToOwned::to_owned)
        .or(repo_info_name)
        .unwrap_or_else(|| {
            clap::Error {
                message: "Missing repo name!".to_owned(),
                kind: clap::ErrorKind::ArgumentNotFound,
                info: None,
            }
            .exit()
        });
    let org = app
        .value_of(&org_arg.b.name)
        .map(ToOwned::to_owned)
        .or(repo_info_org)
        .unwrap_or_else(|| {
            clap::Error {
                message: "Missing repo name!".to_owned(),
                kind: clap::ErrorKind::ArgumentNotFound,
                info: None,
            }
            .exit()
        });

    let comment_source: CommentSource = if let Some(comment) = app.value_of(&comment_arg.b.name) {
        CommentSource::StrArg {
            comment: comment.to_owned(),
        }
    } else if let Some(comment_file) = app.value_of(&comment_file_arg.b.name) {
        debug!("Opening file {}", comment_file);
        CommentSource::File(
            fs::OpenOptions::new()
                .read(true)
                .open(&comment_file)
                .unwrap_or_else(|err| {
                    clap::Error {
                        message: format!(
                            "Could not open file input containing comment
    path: {}
    error: {}",
                            &comment_file, err
                        ),
                        kind: clap::ErrorKind::ValueValidation,
                        info: None,
                    }
                    .exit()
                }),
        )
    } else {
        CommentSource::Standard(io::stdin())
    };

    let overwrite_mode = app
        .value_of(&overwrite_mode_arg.b.name)
        .map(|m| {
            CommentOverwriteMode::from_str(m).unwrap_or_else(|_| {
                clap::Error {
                    message: format!("Invalid overwrite Mode: {}", m,),
                    kind: clap::ErrorKind::ArgumentNotFound,
                    info: None,
                }
                .exit()
            })
        })
        .unwrap_or_default();

    Ok(Config {
        api: GithubAPI {
            base_url: api_url,
            token: get_arg(&app, &token_arg),
        },
        repo_owner: org,
        repo_name: repo,
        branch_name: get_arg(&app, &branch_arg),
        comment_source,
        overwrite_mode,
    })
}

fn main() -> Result<()> {
    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();

    debug!("Parsing Command line");
    let config = parse_cli()?;
    debug!("Config parsed as: {:?}", &config);

    debug!("Evaluating comment content");
    let comment = config
        .comment_source
        .retrieve()
        .context("Failed to read comment")?;

    debug!("Determining PR number");
    let pr_number =
        config
            .api
            .find_pr_for_ref(&config.repo_owner, &config.repo_name, &config.branch_name)?;

    debug!("Commenting back to PR#{}", pr_number);
    config
        .api
        .comment(&config.repo_owner, &config.repo_name, pr_number, &comment)
        .context("Failed to publish comment")
        .map(|_| info!("Successfully commented back to PR#{}", pr_number))
}
