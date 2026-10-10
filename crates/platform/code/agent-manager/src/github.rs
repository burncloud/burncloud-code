//! GitHub read-only adapter using `gh api`. No token is included in command args.
use serde::Deserialize;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RemoteIssue {
    pub id: u64,
    pub node_id: String,
    pub number: u64,
    pub title: String,
    pub state: String,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RemotePr {
    pub id: u64,
    pub node_id: String,
    pub number: u64,
    pub state: String,
    pub draft: bool,
    pub head: PrHead,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PrHead { pub sha: String }

fn validate_segment(s: &str) -> Result<(),String> {
    if !s.is_empty() && s.len() <= 100 && s.chars().all(|c| c.is_ascii_alphanumeric() || c=='-' || c=='_') {
        Ok(())
    } else { Err("invalid GitHub repository path segment".into()) }
}
fn validate_number(n: u64) -> Result<(),String> {
    if n>0 {Ok(())} else {Err("GitHub number must be positive".into())}
}
pub fn issue_endpoint(owner: &str,repo: &str,number:u64)->Result<String,String>{
    validate_segment(owner)?;validate_segment(repo)?;validate_number(number)?;
    Ok(format!("repos/{owner}/{repo}/issues/{number}"))
}
pub fn pr_endpoint(owner: &str,repo: &str,number:u64)->Result<String,String>{
    validate_segment(owner)?;validate_segment(repo)?;validate_number(number)?;
    Ok(format!("repos/{owner}/{repo}/pulls/{number}"))
}
pub fn list_issues_endpoint(owner: &str,repo: &str,page:u32)->Result<String,String>{
    validate_segment(owner)?;validate_segment(repo)?;
    if page==0 || page>10_000 {return Err("invalid page".into());}
    Ok(format!("repos/{owner}/{repo}/issues?state=open&per_page=100&page={page}"))
}
pub fn list_prs_endpoint(owner: &str,repo: &str,page:u32)->Result<String,String>{
    validate_segment(owner)?;validate_segment(repo)?;
    if page==0 || page>10_000 {return Err("invalid page".into());}
    Ok(format!("repos/{owner}/{repo}/pulls?state=open&per_page=100&page={page}"))
}

/// Invokes exactly `gh api GET <validated-endpoint>` with a timeout controlled by
/// GitHub CLI. Not usable for mutations, arbitrary CLI execution, or shell commands.
pub fn gh_get(endpoint: &str) -> Result<String,String> {
    if endpoint.is_empty() || endpoint.starts_with('-')
        || !endpoint.starts_with("repos/")
        || endpoint.contains(' ') || endpoint.contains('\n') || endpoint.contains('\r')
        || endpoint.contains("..") {
        return Err("invalid GitHub read-only endpoint".into());
    }
    let output = Command::new("gh")
        .arg("api").arg("--method").arg("GET").arg(endpoint)
        .stdin(Stdio::null())
        .output().map_err(|e|format!("cannot execute gh api: {e}"))?;
    if !output.status.success(){
        return Err(format!("GitHub request failed (exit: {:?})",output.status.code()));
    }
    String::from_utf8(output.stdout).map_err(|e|format!("invalid GitHub UTF-8: {e}"))
}
pub fn get_issue(owner:&str,repo:&str,n:u64)->Result<RemoteIssue,String>{
    let raw=gh_get(&issue_endpoint(owner,repo,n)?)?;
    serde_json::from_str(&raw).map_err(|e|format!("invalid issue response: {e}"))
}
pub fn get_pr(owner:&str,repo:&str,n:u64)->Result<RemotePr,String>{
    let raw=gh_get(&pr_endpoint(owner,repo,n)?)?;
    serde_json::from_str(&raw).map_err(|e|format!("invalid PR response: {e}"))
}
pub fn get_open_issues(owner:&str,repo:&str)->Result<Vec<RemoteIssue>,String>{
    let mut items=Vec::new();
    for page in 1..=100 {
        let raw=gh_get(&list_issues_endpoint(owner,repo,page)?)?;
        let batch:Vec<serde_json::Value>=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
        let count=batch.len();
        for entry in batch {
            // GitHub issues endpoint includes pull requests; never treat them as issues.
            if entry.get("pull_request").is_none(){
                items.push(serde_json::from_value(entry).map_err(|e|format!("issue payload: {e}"))?);
            }
        }
        if count<100 {return Ok(items);}
    }
    Err("pagination limit reached; refusing incomplete issue results".into())
}
pub fn get_open_prs(owner:&str,repo:&str)->Result<Vec<RemotePr>,String>{
    let mut items=Vec::new();
    for page in 1..=100 {
        let raw=gh_get(&list_prs_endpoint(owner,repo,page)?)?;
        let mut batch:Vec<RemotePr>=serde_json::from_str(&raw).map_err(|e|e.to_string())?;
        let count=batch.len();
        items.append(&mut batch);
        if count<100 {return Ok(items);}
    }
    Err("pagination limit reached; refusing incomplete PR results".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn endpoints_are_strictly_validated(){
        assert_eq!(issue_endpoint("burncloud","burncloud",734).unwrap(),"repos/burncloud/burncloud/issues/734");
        assert!(issue_endpoint("burncloud","evil/path",1).is_err());
        assert!(pr_endpoint("burncloud","repo",0).is_err());
        assert!(list_issues_endpoint("burncloud","repo",0).is_err());
        assert!(list_prs_endpoint("burncloud","repo",1).is_ok());
        assert!(gh_get("-X DELETE").is_err());
    }
    #[test]
    fn github_response_shapes_are_typed(){
        let issue:RemoteIssue=serde_json::from_str(r#"{"id":123,"node_id":"I_1","number":7,"title":"Do work","state":"open"}"#).unwrap();
        assert_eq!(issue.number,7);
        let pr:RemotePr=serde_json::from_str(r#"{"id":10,"node_id":"PR_1","number":2,"state":"open","draft":true,"head":{"sha":"abc"}}"#).unwrap();
        assert!(pr.draft);
    }
}
