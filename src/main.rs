mod github;

use std::fs;
use std::io::{self, Read};
use std::str::FromStr;

use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg, ArgMatches};
use github::GithubAPI;
use url::Url;

enum CommentSource {
    StrArg { comment: String },
    Standard(io::Stdin),
    File(fs::File),
}

impl CommentSource {
    pub fn retrieve(self) -> io::Result<String> {
        match self {
            CommentSource::StrArg { comment } => Ok(comment),
            CommentSource::Standard(mut stdin) => {
                let mut buffer = String::new();
                stdin.read_to_string(&mut buffer).map(|_| buffer)
            }
            CommentSource::File(mut file) => {
                let mut buffer = String::new();
                file.read_to_string(&mut buffer).map(|_| buffer)
            }
        }
    }
}

pub struct Config {
    api: GithubAPI,
    repo_owner: String,
    repo_name: String,
    branch_name: String,
    comment_source: CommentSource,
}

fn parse_cli() -> Config {
    fn get_arg(app: &ArgMatches, arg: &Arg) -> String {
        app.value_of(arg.b.name).unwrap().to_owned()
    }

    let api_url_arg = Arg::with_name("Api Url")
        .long("api-url")
        .help("The Github api base url")
        .default_value("https://api.github.com/")
        .takes_value(true);
    let token_arg = Arg::with_name("token")
        .long("token")
        .help("The Github token to use")
        .required(true)
        .takes_value(true);
    let org_arg = Arg::with_name("GitHub organization")
        .long("org")
        .required(true)
        .help("The Github organisation or username containing the repo")
        .takes_value(true);
    let repo_arg = Arg::with_name("Repo name")
        .long("repo")
        .required(true)
        .help("The repository name")
        .takes_value(true);
    let branch_arg = Arg::with_name("Branch")
        .long("branch")
        .short("b")
        .required(true)
        .help("The branch name to retrieve the PR number")
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
        .arg(&api_url_arg)
        .arg(&token_arg)
        .arg(&org_arg)
        .arg(&repo_arg)
        .arg(&branch_arg)
        .arg(&comment_arg)
        .arg(&comment_file_arg)
        .arg(&std_in_arg)
        .get_matches();

    let comment_source: CommentSource = if let Some(comment) = app.value_of(&comment_arg.b.name) {
        CommentSource::StrArg {
            comment: comment.to_owned(),
        }
    } else if let Some(comment_file) = app.value_of(&comment_file_arg.b.name) {
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

    Config {
        api: GithubAPI {
            base_url: Url::from_str(&get_arg(&app, &api_url_arg)).unwrap(),
            token: get_arg(&app, &token_arg),
        },
        repo_owner: get_arg(&app, &org_arg),
        repo_name: get_arg(&app, &repo_arg),
        branch_name: get_arg(&app, &branch_arg),
        comment_source: comment_source,
    }
}

fn main() -> Result<(), String> {
    let config = parse_cli();

    let comment = config
        .comment_source
        .retrieve()
        .map_err(|err| format!("Failed to read comment : {}", err))?;
    let pr_number = config.api.find_pr_for_branch(
        &config.repo_owner,
        &config.repo_name,
        &config.branch_name,
    )?;
    config
        .api
        .comment(&config.repo_owner, &config.repo_name, pr_number, &comment)
}
