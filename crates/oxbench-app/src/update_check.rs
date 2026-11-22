//! Asks GitHub whether a newer release exists. It only reports one; installing stays a choice
//! the user makes on the release page.

use std::time::Duration;

use eframe::egui;
use semver::Version;
use serde::Deserialize;

use crate::tasks::JobSlot;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const RELEASES_PAGE: &str = "https://github.com/zahedul/oxbench/releases";
const LATEST_RELEASE_API: &str = "https://api.github.com/repos/zahedul/oxbench/releases/latest";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const NOT_FOUND: u16 = 404;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateStatus {
    UpToDate,
    Available {
        version: String,
        page: String,
    },
    /// The repository has no published release yet.
    NoReleases,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub html_url: String,
}

#[derive(Default)]
pub struct UpdateChecker {
    job: JobSlot<Result<UpdateStatus, String>>,
}

impl UpdateChecker {
    pub fn start(&mut self, ctx: &egui::Context) {
        if self.job.is_running() {
            return;
        }
        self.job.start(ctx, fetch_update_status);
    }

    /// A checker that has already finished with `status`, for tests and screenshots.
    #[cfg(test)]
    pub fn finished_with(status: UpdateStatus) -> Self {
        Self {
            job: JobSlot::Finished(Ok(status)),
        }
    }

    pub fn poll(&mut self) {
        self.job.poll();
    }

    pub fn is_running(&self) -> bool {
        self.job.is_running()
    }

    /// `None` until a check has finished.
    pub fn outcome(&self) -> Option<Result<&UpdateStatus, String>> {
        match &self.job {
            JobSlot::Finished(Ok(status)) => Some(Ok(status)),
            JobSlot::Finished(Err(message)) => Some(Err(message.clone())),
            JobSlot::Lost => Some(Err("The update check stopped unexpectedly.".to_owned())),
            JobSlot::Idle | JobSlot::Running(_) => None,
        }
    }
}

fn fetch_update_status() -> Result<UpdateStatus, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .build()
        .into();
    let response = agent
        .get(LATEST_RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", &format!("Oxbench/{CURRENT_VERSION}"))
        .call();
    let mut response = match response {
        Ok(response) => response,
        Err(ureq::Error::StatusCode(NOT_FOUND)) => return Ok(UpdateStatus::NoReleases),
        Err(error) => return Err(format!("Couldn't reach GitHub: {error}")),
    };
    let release: Release = response
        .body_mut()
        .read_json()
        .map_err(|error| format!("GitHub sent a reply Oxbench couldn't read: {error}"))?;
    compare_release(CURRENT_VERSION, &release)
}

/// Decides whether `release` is newer than the running version.
pub fn compare_release(current: &str, release: &Release) -> Result<UpdateStatus, String> {
    let current = Version::parse(current)
        .map_err(|error| format!("This build's version \"{current}\" is invalid: {error}"))?;
    let tag = release.tag_name.trim();
    let tag = tag.strip_prefix('v').unwrap_or(tag);
    let latest = Version::parse(tag).map_err(|error| {
        format!(
            "The latest release tag \"{}\" isn't a version: {error}",
            release.tag_name
        )
    })?;
    if latest <= current {
        return Ok(UpdateStatus::UpToDate);
    }
    // The page is opened in the browser, so only a GitHub link is trusted.
    let page = if release.html_url.starts_with("https://github.com/") {
        release.html_url.clone()
    } else {
        RELEASES_PAGE.to_owned()
    };
    Ok(UpdateStatus::Available {
        version: latest.to_string(),
        page,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, url: &str) -> Release {
        Release {
            tag_name: tag.to_owned(),
            html_url: url.to_owned(),
        }
    }

    #[test]
    fn reports_a_newer_release() {
        let latest = release(
            "v0.2.0",
            "https://github.com/zahedul/oxbench/releases/tag/v0.2.0",
        );
        assert_eq!(
            compare_release("0.1.0", &latest),
            Ok(UpdateStatus::Available {
                version: "0.2.0".to_owned(),
                page: "https://github.com/zahedul/oxbench/releases/tag/v0.2.0".to_owned(),
            })
        );
    }

    #[test]
    fn the_same_or_an_older_release_is_up_to_date() {
        let url = "https://github.com/zahedul/oxbench/releases/tag/v0.1.0";
        assert_eq!(
            compare_release("0.1.0", &release("v0.1.0", url)),
            Ok(UpdateStatus::UpToDate)
        );
        assert_eq!(
            compare_release("0.3.0", &release("0.2.9", url)),
            Ok(UpdateStatus::UpToDate)
        );
    }

    #[test]
    fn a_pre_release_is_older_than_its_final_version() {
        let url = "https://github.com/zahedul/oxbench/releases";
        assert_eq!(
            compare_release("0.2.0", &release("v0.2.0-beta.1", url)),
            Ok(UpdateStatus::UpToDate)
        );
    }

    #[test]
    fn a_link_away_from_github_falls_back_to_the_releases_page() {
        let latest = release("v1.0.0", "https://example.com/download");
        let Ok(UpdateStatus::Available { page, .. }) = compare_release("0.1.0", &latest) else {
            panic!("expected an update");
        };
        assert_eq!(page, RELEASES_PAGE);
    }

    #[test]
    #[ignore = "contacts GitHub; run on demand"]
    fn asks_github_for_the_latest_release() {
        assert!(fetch_update_status().is_ok());
    }

    #[test]
    fn rejects_a_tag_that_is_not_a_version() {
        let latest = release("nightly", RELEASES_PAGE);
        let error = compare_release("0.1.0", &latest).unwrap_err();
        assert!(error.contains("nightly"));
    }
}
