//! The optional update check (SPEC §1.2, M7).
//!
//! This is the **only** code in Margin that talks to a non-local address, and
//! it is off unless the author turns it on (or presses "Check now"):
//!
//! - one fixed HTTPS address, GitHub's "latest release" for this project;
//!   nothing the author writes, no project data and no version number is sent,
//!   only what any web request reveals (the computer's IP address and the
//!   client name `Margin-update-check`);
//! - no cookies, no redirects, a short timeout, and a capped response size;
//! - the answer is only a version and a link, and the link must stay inside
//!   this project's GitHub pages before it can be opened;
//! - nothing is downloaded or installed: the author opens the page and
//!   downloads the installer themselves.

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{MarginError, Result};

pub const RELEASES_API: &str = "https://api.github.com/repos/tracesfx/margin/releases/latest";
pub const RELEASES_PAGE_PREFIX: &str = "https://github.com/tracesfx/margin/";
const MAX_BODY: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    /// The running version.
    pub current: String,
    /// The latest published release, if there is one.
    pub latest: Option<String>,
    /// Its page on GitHub (always inside this project's pages).
    pub url: Option<String>,
    pub newer: bool,
    /// When the check ran (RFC 3339, UTC).
    pub checked_at: String,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// "v1.2.3" or "1.2.3-beta.1" as numbers. Anything after a `-` or `+` is ignored.
pub fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let core = s.split(['-', '+']).next()?;
    let mut it = core.split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next().map_or(Some(0), |p| p.parse().ok())?;
    let patch = it.next().map_or(Some(0), |p| p.parse().ok())?;
    if it.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

pub fn is_newer(current: &str, latest: &str) -> bool {
    match (parse_version(current), parse_version(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

/// Ask `api_url` for the latest release. `Ok(None)` when none is published.
/// The link in the answer must start with `page_prefix`.
async fn fetch_latest(api_url: &str, page_prefix: &str) -> Result<Option<(String, String)>> {
    let client = reqwest::Client::builder()
        .user_agent("Margin-update-check")
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| MarginError::Internal(format!("could not set up the update check: {e}")))?;
    let resp = client
        .get(api_url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| MarginError::Internal("could not reach the update server".into()))?;
    let status = resp.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(MarginError::Internal(format!(
            "the update server answered {}",
            status.as_u16()
        )));
    }
    let mut body: Vec<u8> = Vec::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|_| MarginError::Internal("the update answer was cut off".into()))?;
        if body.len() + chunk.len() > MAX_BODY {
            return Err(MarginError::Internal(
                "the update answer was too large".into(),
            ));
        }
        body.extend_from_slice(&chunk);
    }
    let release: Release = serde_json::from_slice(&body)
        .map_err(|_| MarginError::Internal("the update answer could not be read".into()))?;
    if release.draft || release.prerelease {
        return Ok(None);
    }
    if !release.html_url.starts_with(page_prefix) {
        return Err(MarginError::Internal(
            "the update answer pointed somewhere unexpected, so it was ignored".into(),
        ));
    }
    Ok(Some((release.tag_name, release.html_url)))
}

async fn check_against(api_url: &str, page_prefix: &str, current: &str) -> Result<UpdateStatus> {
    let found = fetch_latest(api_url, page_prefix).await?;
    let newer = found
        .as_ref()
        .is_some_and(|(tag, _)| is_newer(current, tag));
    Ok(UpdateStatus {
        current: current.to_string(),
        latest: found
            .as_ref()
            .map(|(t, _)| t.trim_start_matches(['v', 'V']).to_string()),
        url: found.map(|(_, u)| u),
        newer,
        checked_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// The check the app runs: this project's releases, compared with this build.
pub async fn check_now() -> Result<UpdateStatus> {
    check_against(
        RELEASES_API,
        RELEASES_PAGE_PREFIX,
        env!("CARGO_PKG_VERSION"),
    )
    .await
}

/// Whether `url` may be opened from the update notice.
pub fn is_release_page(url: &str) -> bool {
    url.starts_with(RELEASES_PAGE_PREFIX)
        && url.len() < 300
        && url
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/:.-_~%".contains(c))
}

/// Open a release page in the default browser.
pub fn open_release_page(url: &str) -> Result<()> {
    if !is_release_page(url) {
        return Err(MarginError::Invalid(
            "only this project's release pages can be opened from here".into(),
        ));
    }
    #[cfg(windows)]
    {
        std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .map_err(|e| MarginError::Io(e.to_string()))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err(MarginError::Invalid(
            "opening links is only built for Windows".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// A one-shot local HTTP server that answers every request with `response`.
    async fn serve(response: String) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = listener.accept().await {
                let response = response.clone();
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    let _ = sock.read(&mut buf).await;
                    let _ = sock.write_all(response.as_bytes()).await;
                    let _ = sock.shutdown().await;
                });
            }
        });
        format!("http://{addr}/latest")
    }

    fn ok(json: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",
            json.len()
        )
    }

    const PAGE: &str = "https://github.com/tracesfx/margin/";

    #[test]
    fn versions_compare_as_numbers() {
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("0.10"), Some((0, 10, 0)));
        assert_eq!(parse_version("2.0.0-beta.1"), Some((2, 0, 0)));
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("nightly"), None);
        assert!(is_newer("0.1.0", "v0.2.0"));
        assert!(is_newer("0.9.0", "0.10.0"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.3.0", "0.2.9"));
        assert!(!is_newer("0.1.0", "nightly"));
    }

    #[test]
    fn only_this_projects_pages_can_be_opened() {
        assert!(is_release_page(&format!("{PAGE}releases/tag/v0.2.0")));
        for bad in [
            "https://evil.example/tracesfx/margin/",
            "https://github.com/someone-else/repo/releases",
            "http://github.com/tracesfx/margin/",
            "https://github.com/tracesfx/margin/ & calc",
            "https://github.com/tracesfx/margin/\"x",
            "file:///C:/Windows/System32/calc.exe",
        ] {
            assert!(!is_release_page(bad), "{bad}");
        }
        assert!(open_release_page("https://example.com").is_err());
    }

    #[tokio::test]
    async fn a_newer_release_is_reported_with_its_link() {
        let url = serve(ok(&format!(
            r#"{{"tag_name":"v0.3.0","html_url":"{PAGE}releases/tag/v0.3.0","draft":false,"prerelease":false}}"#
        )))
        .await;
        let s = check_against(&url, PAGE, "0.1.0").await.unwrap();
        assert!(s.newer);
        assert_eq!(s.latest.as_deref(), Some("0.3.0"));
        assert_eq!(
            s.url.as_deref(),
            Some(&*format!("{PAGE}releases/tag/v0.3.0"))
        );
        let same = check_against(&url, PAGE, "0.3.0").await.unwrap();
        assert!(!same.newer && same.latest.as_deref() == Some("0.3.0"));
    }

    #[tokio::test]
    async fn no_release_a_draft_or_a_prerelease_is_not_an_update() {
        let none = serve(
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
        )
        .await;
        let s = check_against(&none, PAGE, "0.1.0").await.unwrap();
        assert!(!s.newer && s.latest.is_none() && s.url.is_none());
        let pre = serve(ok(&format!(
            r#"{{"tag_name":"v9.0.0","html_url":"{PAGE}releases/tag/v9","prerelease":true}}"#
        )))
        .await;
        assert!(!check_against(&pre, PAGE, "0.1.0").await.unwrap().newer);
    }

    #[tokio::test]
    async fn a_link_outside_the_project_a_big_or_broken_answer_is_refused() {
        let off = serve(ok(
            r#"{"tag_name":"v9.0.0","html_url":"https://evil.example/download"}"#,
        ))
        .await;
        assert!(check_against(&off, PAGE, "0.1.0").await.is_err());
        let junk = serve(ok("not json")).await;
        assert!(check_against(&junk, PAGE, "0.1.0").await.is_err());
        let big = serve(ok(&format!(r#"{{"pad":"{}"}}"#, "x".repeat(MAX_BODY + 10)))).await;
        assert!(check_against(&big, PAGE, "0.1.0").await.is_err());
        let boom =
            serve("HTTP/1.1 500 Oops\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into())
                .await;
        assert!(check_against(&boom, PAGE, "0.1.0").await.is_err());
        // Nothing listening: an error, not a hang.
        assert!(
            check_against("http://127.0.0.1:1/latest", PAGE, "0.1.0")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn redirects_are_not_followed() {
        let r = serve(
            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
        )
        .await;
        assert!(check_against(&r, PAGE, "0.1.0").await.is_err());
    }

    #[test]
    fn the_fixed_address_is_https_on_github() {
        assert!(RELEASES_API.starts_with("https://api.github.com/repos/tracesfx/"));
        assert!(RELEASES_PAGE_PREFIX.starts_with("https://github.com/tracesfx/"));
    }

    /// Opt-in: talks to the real GitHub. `cargo test --lib live_update -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn live_update_check_reaches_github() {
        let s = check_now().await.unwrap();
        println!("{s:?}");
        assert_eq!(s.current, env!("CARGO_PKG_VERSION"));
    }
}
