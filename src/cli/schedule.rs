use ani_lib::{AniError, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ScheduleSearchResponse {
    #[serde(default)]
    anime: Vec<ScheduleAnime>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleAnime {
    title: String,
    status: String,
    jpn_time: Option<String>,
    sub_time: Option<String>,
    #[serde(default)]
    names: ScheduleNames,
}

#[derive(Debug, Default, Deserialize)]
struct ScheduleNames {
    english: Option<String>,
    native: Option<String>,
}

pub(crate) async fn display_next_episode_schedule(query: &str) -> Result<()> {
    let response = reqwest::Client::builder()
        .user_agent(concat!("ani-cli-rs/", env!("CARGO_PKG_VERSION")))
        .build()?
        .get("https://animeschedule.net/api/v3/anime")
        .query(&[("q", query)])
        .send()
        .await?
        .error_for_status()?
        .json::<ScheduleSearchResponse>()
        .await?;
    if response.anime.is_empty() {
        return Err(AniError::NoSearchResults);
    }
    for anime in &response.anime {
        for line in schedule_lines(anime) {
            println!("{line}");
        }
        println!("---");
    }
    Ok(())
}

fn schedule_lines(anime: &ScheduleAnime) -> Vec<String> {
    let mut lines = Vec::with_capacity(5);
    if let Some(title) = anime.names.english.as_deref() {
        lines.push(format!("English Title: {title}"));
    } else {
        lines.push(format!("English Title: {}", anime.title));
    }
    if let Some(title) = anime.names.native.as_deref() {
        lines.push(format!("Japanese Title: {title}"));
    }
    if anime.status != "Finished" {
        if let Some(time) = anime
            .jpn_time
            .as_deref()
            .filter(|time| valid_release_time(time))
        {
            lines.push(format!("Next Raw Release: {time}"));
        }
        if let Some(time) = anime
            .sub_time
            .as_deref()
            .filter(|time| valid_release_time(time))
        {
            lines.push(format!("Next Sub Release: {time}"));
        }
    }
    lines.push(format!("Status:  {}", anime.status));
    lines
}

fn valid_release_time(time: &str) -> bool {
    !(time.starts_with("0001-") || time.starts_with("0002-"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_episode_schedule_formats_ongoing_and_finished_titles() {
        let ongoing = ScheduleAnime {
            title: "Example".into(),
            status: "Ongoing".into(),
            jpn_time: Some("2026-07-24T15:00:00Z".into()),
            sub_time: Some("2026-07-24T16:00:00Z".into()),
            names: ScheduleNames {
                english: Some("Example Anime".into()),
                native: Some("例".into()),
            },
        };
        assert_eq!(
            schedule_lines(&ongoing),
            [
                "English Title: Example Anime",
                "Japanese Title: 例",
                "Next Raw Release: 2026-07-24T15:00:00Z",
                "Next Sub Release: 2026-07-24T16:00:00Z",
                "Status:  Ongoing",
            ]
        );

        let finished = ScheduleAnime {
            title: "Old Anime".into(),
            status: "Finished".into(),
            jpn_time: Some("2020-01-01T00:00:00Z".into()),
            sub_time: Some("0001-01-01T00:00:00Z".into()),
            names: ScheduleNames::default(),
        };
        assert_eq!(
            schedule_lines(&finished),
            ["English Title: Old Anime", "Status:  Finished"]
        );
    }
}
