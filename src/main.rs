use std::str::FromStr;

use clap::{App, Arg, ArgMatches};
use github_types::ShortCommit;
use reqwest::{Method, RequestBuilder};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CommentCreateRequest {
    pub body: String,
}

// The api to retrieve the list of PR doesn't return all the fields of the PR
#[derive(Deserialize, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PullRequestSummary {
    pub number: u64,
    pub head: ShortCommit,
}

pub struct GithubAPI {
    pub base_url: Url,
    pub token: String,
}

fn req_error_to_string(req_error: reqwest::Error) -> String {
    format!("{:?}", req_error)
}

impl GithubAPI {
    pub fn request(&self, method: Method, url: &str) -> RequestBuilder {
        reqwest::Client::new()
            .request(method, self.base_url.join(url).unwrap()) // TODO: Unwrap yuk
            .header("Authorization", "token ".to_owned() + &self.token)
            .header("Accept", "application/vnd.github.v3+json")
    }

    pub fn find_pr_for_branch(
        &self,
        repo_owner: &str,
        repo_name: &str,
        branch_name: &str,
    ) -> Result<u64, String> {
        self.request(
            Method::GET,
            &format!(
                "repos/{}/{}/pulls?state=open&sort=updated&direction=desc",
                repo_owner, repo_name
            ),
        )
        .send()
        .and_then(|mut r| r.json())
        .map_err(req_error_to_string)
        .and_then(|prs: Vec<PullRequestSummary>| {
            if let Some(pr) = prs.iter().find(|pr| pr.head.commit_ref == branch_name) {
                Ok(pr.number)
            } else {
                Err("Cant find dude".to_owned())
            }
        })
    }

    pub fn comment<T: Into<String>>(
        &self,
        repo_owner: &str,
        repo_name: &str,
        issue_number: u64,
        comment: T,
    ) -> Result<(), String> {
        let body = CommentCreateRequest {
            body: comment.into(),
        };

        self.request(
            Method::POST,
            &format!(
                "repos/{}/{}/issues/{}/comments",
                repo_owner, repo_name, issue_number
            ),
        )
        .json(&body)
        .send()
        .map_err(req_error_to_string)
        .and_then(|res| {
            if res.status() == 201 {
                Ok(())
            } else {
                Err(format!("Arggggg {:?}", res))
            }
        })
    }
}

pub struct Config {
    api: GithubAPI,
    repo_owner: String,
    repo_name: String,
    branch_name: String,
    comment: String,
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
    let comment_arg = Arg::with_name("Comment")
        .long("comment")
        .required(true)
        .help("The content of the comment")
        .takes_value(true);
    let app = App::new("GitHub commentator")
        .version("v0.0.1")
        .arg(&api_url_arg)
        .arg(&token_arg)
        .arg(&org_arg)
        .arg(&repo_arg)
        .arg(&branch_arg)
        .arg(&comment_arg)
        .get_matches();

    Config {
        api: GithubAPI {
            base_url: Url::from_str(&get_arg(&app, &api_url_arg)).unwrap(),
            token: get_arg(&app, &token_arg),
        },
        repo_owner: get_arg(&app, &org_arg),
        repo_name: get_arg(&app, &repo_arg),
        branch_name: get_arg(&app, &branch_arg),
        comment: get_arg(&app, &comment_arg),
    }
}

fn main() -> Result<(), String> {
    let config = parse_cli();

    let pr_number = config.api.find_pr_for_branch(
        &config.repo_owner,
        &config.repo_name,
        &config.branch_name,
    )?;
    config.api.comment(
        &config.repo_owner,
        &config.repo_name,
        pr_number,
        &config.comment,
    )
}
