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
