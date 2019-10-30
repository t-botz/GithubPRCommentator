use anyhow::{anyhow, Context, Result};
use github_types::ShortCommit;
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;
use reqwest::{Method, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::FromIterator;
use std::str::FromStr;
use url::Url;

lazy_static! {
    pub static ref DEFAULT_GITHUB_API_URL: Url = Url::from_str("https://api.github.com/").unwrap();
    pub static ref PR_BRANCH_GITHUB_PATTERN: Regex =
        Regex::new(r"^refs/pull/(\d+)/(?:head|merge)$").unwrap();
}

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

fn mask_token(token: &mut String) -> &mut String {
    if token.len() > 8 {
        token.replace_range(
            std::ops::Range {
                start: 2,
                end: token.len() - 2,
            },
            "************",
        );
    } else {
        token.replace_range(std::ops::RangeFull, "************");
    };
    token
}

impl fmt::Debug for GithubAPI {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GithubAPI {{ base_url: '{}',  token: '{}' }}",
            self.base_url,
            mask_token(&mut self.token.clone())
        )
    }
}

impl GithubAPI {
    pub fn request(&self, method: Method, url: &str) -> RequestBuilder {
        let full_url = self.base_url.join(url).unwrap(); // TODO: Unwrap yuk
        debug!("{} {}", method, full_url);
        reqwest::Client::new()
            .request(method, full_url)
            .header("Authorization", "token ".to_owned() + &self.token)
            .header("Accept", "application/vnd.github.v3+json")
    }

    pub fn find_pr_for_ref(&self, repo_owner: &str, repo_name: &str, git_ref: &str) -> Result<u64> {
        if let Some(capture) = PR_BRANCH_GITHUB_PATTERN.captures(git_ref) {
            debug!("Extracting PR number from branch name [{}]", git_ref);
            return u64::from_str(&capture[1]).with_context(|| {
                // In practice should never happen
                format!(
                    "Reference {} identified as PR but failing to parse",
                    git_ref
                )
            });
        }

        self.request(
            Method::GET,
            &format!(
                "repos/{}/{}/pulls?state=open&sort=updated&direction=desc",
                repo_owner, repo_name
            ),
        )
        .send()
        .context("Failed to send Github Request")
        .and_then(|mut r| {
            r.json()
                .with_context(|| format!("Failed to parse Response: {:?}", r))
        })
        .and_then(|prs: Vec<PullRequestSummary>| {
            if let Some(pr) = prs.iter().find(|pr| pr.head.commit_ref == git_ref) {
                Ok(pr.number)
            } else {
                Err(anyhow!("No PRs are matching the branch name"))
            }
        })
    }

    pub fn comment<T: Into<String>>(
        &self,
        repo_owner: &str,
        repo_name: &str,
        issue_number: u64,
        comment: T,
    ) -> Result<()> {
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
        .context("Creating comment failed")
        .and_then(|res| {
            if res.status() == 201 {
                Ok(())
            } else {
                Err(anyhow!(
                    "Github returned unexpected status : {}",
                    res.status()
                ))
            }
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RepoInfo {
    pub api_url: Url,
    pub org: String,
    pub name: String,
}

pub fn get_repo_info_from_url(url: Url) -> Result<RepoInfo> {
    if url.query().is_some() || url.fragment().is_some() {
        return Err(anyhow!("Url {} has unexpected query args or fragment", url));
    }
    if let Some(segments) = url.path_segments() {
        let seg_vec = Vec::from_iter(segments);
        if seg_vec.len() != 2 {
            Err(anyhow!(
                "Url {} doesn't have the expected 2 path segments (org, repo name)",
                url
            ))
        } else if let Some(host) = url.host_str() {
            let api_url = if host == "github.com" {
                DEFAULT_GITHUB_API_URL.clone()
            } else {
                url.join("/api/v3/")
                    .with_context(|| format!("Couldnt determine api url for {}", url))?
            };
            let repo_name = if seg_vec[1].ends_with(".git") {
                seg_vec[1][..seg_vec[1].len() - 4].to_owned()
            } else {
                seg_vec[1].to_owned()
            };
            Ok(RepoInfo {
                api_url: api_url,
                org: seg_vec[0].to_owned(),
                name: repo_name,
            })
        } else {
            Err(anyhow!("Url {} has no host???", url))
        }
    } else {
        Err(anyhow!("Url {} is not a supported github repo url", url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(url: &str) -> Result<RepoInfo> {
        Url::from_str(url)
            .context("Can't parse URL")
            .and_then(get_repo_info_from_url)
    }

    #[test]
    fn test_get_repo_info() {
        let good_github_repo = RepoInfo {
            api_url: Url::from_str("https://api.github.com/").unwrap(),
            org: "thibaultdelor".to_owned(),
            name: "GithubPRCommentator".to_owned(),
        };
        assert_eq!(
            repo("https://github.com/thibaultdelor/GithubPRCommentator").unwrap(),
            good_github_repo
        );
        assert_eq!(
            repo("https://github.com/thibaultdelor/GithubPRCommentator.git").unwrap(),
            good_github_repo
        );
    }

    #[test]
    fn test_get_repo_info_ghe() {
        let good_github_repo = RepoInfo {
            api_url: Url::from_str("https://my.github.internal/api/v3/").unwrap(),
            org: "thibaultdelor".to_owned(),
            name: "GithubPRCommentator".to_owned(),
        };
        assert_eq!(
            repo("https://my.github.internal/thibaultdelor/GithubPRCommentator").unwrap(),
            good_github_repo
        );
        assert_eq!(
            repo("https://my.github.internal/thibaultdelor/GithubPRCommentator.git").unwrap(),
            good_github_repo
        );
    }

    #[test]
    fn test_unsupported_url() {
        // git url not supported yet
        assert!(repo("git@github.com:thibaultdelor/GithubPRCommentator.git").is_err());
        assert!(repo("https://github.com/thibaultdelor/GithubPRCommentator?some_params").is_err());
    }

    #[test]
    fn test_github_pr_branch_pattern() {
        assert!(!PR_BRANCH_GITHUB_PATTERN.is_match("refs/heads/my_branch"));
        assert_eq!(
            u32::from_str(
                &PR_BRANCH_GITHUB_PATTERN
                    .captures("refs/pull/1/head")
                    .unwrap()[1]
            ),
            Ok(1)
        );
        assert_eq!(
            u32::from_str(
                &PR_BRANCH_GITHUB_PATTERN
                    .captures("refs/pull/1/merge")
                    .unwrap()[1]
            ),
            Ok(1)
        );
    }
}
