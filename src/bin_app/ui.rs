use ani_lib::{
    AniError, CatalogProvider, HistoryStore, Result, SearchResult, TranslationType,
    provider_from_show_id,
};
use dialoguer::{FuzzySelect, Input, MultiSelect, Select, theme::ColorfulTheme};

use super::app::{PlaybackContext, ProviderClients, adjacent_episode, play_or_download};

pub fn format_search_label(value: &SearchResult) -> String {
    if value.provider == CatalogProvider::Anikoto2 {
        let total = ani_lib::anikoto2_total_episodes(&value.id).unwrap_or(value.episodes as u32);
        let current = value.episodes as u32;
        if total > 0 {
            return format!("{} ({current}/{total})", value.name);
        }
    }
    format!("{} ({} episodes)", value.name, value.episodes)
}

pub fn prompt_text(prompt: &str) -> Result<String> {
    Input::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact_text()
        .map_err(dialog_error)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionPurpose {
    Watch,
    Download,
}

impl SelectionPurpose {
    pub fn anime_prompt(self) -> &'static str {
        match self {
            Self::Watch => "Select anime (Esc: back, type to filter)",
            Self::Download => "Select anime / season to download (Esc: back, type to filter)",
        }
    }

    pub fn episode_prompt(self) -> &'static str {
        match self {
            Self::Watch => "Select episode (Esc: back, type to filter)",
            Self::Download => "Select episode to download (Esc: back, type to filter)",
        }
    }

    pub fn multiple_episode_prompt(self) -> &'static str {
        match self {
            Self::Watch => "Select episodes (Space: toggle, Enter: confirm, Esc: back)",
            Self::Download => {
                "Select episodes to download (Space: toggle, Enter: confirm, Esc: back)"
            }
        }
    }
}

pub fn select_search_result(
    results: &[SearchResult],
    nth: Option<usize>,
    purpose: SelectionPurpose,
) -> Result<Option<SearchResult>> {
    if results.is_empty() {
        return Err(AniError::NoSearchResults);
    }
    let index = if let Some(index) = nth {
        index
            .checked_sub(1)
            .filter(|index| *index < results.len())
            .ok_or(AniError::SelectionOutOfRange {
                max: results.len(),
                selected: nth,
            })?
    } else if results.len() == 1 {
        return Ok(Some(results[0].clone()));
    } else {
        let mut items = vec!["← Back to search".to_owned()];
        items.extend(results.iter().map(format_search_label));
        let Some(index) = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt(purpose.anime_prompt())
            .items(&items)
            .default(1)
            .interact_opt()
            .map_err(dialog_error)?
        else {
            return Ok(None);
        };
        if index == 0 {
            return Ok(None);
        }
        index - 1
    };
    Ok(Some(results[index].clone()))
}

pub fn select_episode(episodes: &[String]) -> Result<Option<String>> {
    if episodes.is_empty() {
        return Err(AniError::NoEpisodesAvailable { anime: None });
    }
    let mut items = vec!["← Back to anime results".to_owned()];
    items.extend(episodes.iter().cloned());
    let index = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select episode (Esc: back, type to filter)")
        .items(&items)
        .default(episodes.len())
        .interact_opt()
        .map_err(dialog_error)?;
    Ok(index.and_then(|index| index.checked_sub(1).map(|index| episodes[index].clone())))
}

pub fn select_initial_episodes(
    episodes: &[String],
    multi: bool,
    purpose: SelectionPurpose,
) -> Result<Option<String>> {
    if episodes.is_empty() {
        return Err(AniError::NoEpisodesAvailable { anime: None });
    }
    if multi {
        return select_multiple_episodes(episodes, purpose);
    }

    let mut items = vec![
        "← Back to anime results".to_owned(),
        "☑ Select multiple episodes".to_owned(),
    ];
    items.extend(episodes.iter().cloned());
    let Some(index) = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(purpose.episode_prompt())
        .items(&items)
        .default(episodes.len() + 1)
        .interact_opt()
        .map_err(dialog_error)?
    else {
        return Ok(None);
    };

    match index {
        0 => Ok(None),
        1 => select_multiple_episodes(episodes, purpose),
        index => Ok(Some(episodes[index - 2].clone())),
    }
}

fn select_multiple_episodes(
    episodes: &[String],
    purpose: SelectionPurpose,
) -> Result<Option<String>> {
    let Some(selected) = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt(purpose.multiple_episode_prompt())
        .items(episodes)
        .interact_opt()
        .map_err(dialog_error)?
    else {
        return Ok(None);
    };
    Ok(multiple_episode_selection(episodes, &selected))
}

fn multiple_episode_selection(episodes: &[String], selected: &[usize]) -> Option<String> {
    (!selected.is_empty()).then(|| {
        selected
            .iter()
            .filter_map(|index| episodes.get(*index))
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    })
}

pub async fn continue_selection(
    clients: &ProviderClients,
    history: &HistoryStore,
    mode: TranslationType,
    nth: Option<usize>,
) -> Result<Option<(SearchResult, Vec<String>, Option<String>)>> {
    let mut candidates = Vec::new();
    for entry in history.entries().await? {
        let provider = provider_from_show_id(&entry.show_id);
        let episodes = clients.episodes(&entry.show_id, provider, mode).await?;
        if let Some(position) = episodes.iter().position(|value| value == &entry.episode)
            && let Some(next) = episodes.get(position + 1)
        {
            let next = next.clone();
            candidates.push((
                SearchResult {
                    id: entry.show_id,
                    name: entry.title,
                    episodes: episodes.len() as f64,
                    provider,
                },
                episodes,
                next,
            ));
        }
    }
    if candidates.is_empty() {
        return Err(AniError::NoSearchResults);
    }
    let index = if let Some(index) = nth {
        index
            .checked_sub(1)
            .filter(|index| *index < candidates.len())
            .ok_or(AniError::SelectionOutOfRange {
                max: candidates.len(),
                selected: nth,
            })?
    } else {
        let mut items = vec!["← Cancel".to_owned()];
        items.extend(
            candidates
                .iter()
                .map(|(show, _, next)| format!("{} - episode {next}", show.name)),
        );
        let Some(index) = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Continue anime (Esc: cancel, type to filter)")
            .items(&items)
            .default(1)
            .interact_opt()
            .map_err(dialog_error)?
        else {
            return Ok(None);
        };
        if index == 0 {
            return Ok(None);
        }
        index - 1
    };
    let (show, episodes, next) = candidates.remove(index);
    Ok(Some((show, episodes, Some(next))))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterPlayAction {
    Done,
    Search,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlaybackAction {
    Next,
    Replay,
    Previous,
    Select,
    ChangeQuality,
    BackToSearch,
    Quit,
}

impl PlaybackAction {
    fn label(self, quality: &str) -> String {
        match self {
            Self::Next => "Next episode".into(),
            Self::Replay => "Replay episode".into(),
            Self::Previous => "Previous episode".into(),
            Self::Select => "Choose another episode".into(),
            Self::ChangeQuality => format!("Change quality (current: {quality})"),
            Self::BackToSearch => "Back to anime search".into(),
            Self::Quit => "Quit ani-cli-rs".into(),
        }
    }
}

fn playback_actions(episodes: &[String], current: &str) -> Vec<PlaybackAction> {
    let position = episodes.iter().position(|episode| episode == current);
    let mut actions = Vec::with_capacity(6);
    if position.is_some_and(|index| index + 1 < episodes.len()) {
        actions.push(PlaybackAction::Next);
    }
    actions.push(PlaybackAction::Replay);
    if position.is_some_and(|index| index > 0) {
        actions.push(PlaybackAction::Previous);
    }
    actions.extend([
        PlaybackAction::Select,
        PlaybackAction::ChangeQuality,
        PlaybackAction::BackToSearch,
        PlaybackAction::Quit,
    ]);
    actions
}

pub fn should_offer_playback_controls(
    selected_count: usize,
    terminal: bool,
    download: bool,
    exit_after_play: bool,
) -> bool {
    selected_count == 1 && terminal && !download && !exit_after_play
}

pub async fn interactive_after_play(
    context: &PlaybackContext<'_>,
    first: &str,
    initial_quality: &str,
) -> Result<AfterPlayAction> {
    let mut episode = first.to_owned();
    let mut quality = initial_quality.to_owned();
    loop {
        let actions = playback_actions(context.episodes, &episode);
        let labels: Vec<_> = actions
            .iter()
            .map(|action| action.label(&quality))
            .collect();
        let Some(choice) = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "Episode {episode} controls · {} (j/k: move, q/Esc: quit)",
                context.show.name
            ))
            .items(&labels)
            .default(0)
            .interact_opt()
            .map_err(dialog_error)?
        else {
            break Ok(AfterPlayAction::Done);
        };
        match actions[choice] {
            PlaybackAction::Next => episode = adjacent_episode(context.episodes, &episode, 1)?,
            PlaybackAction::Replay => {}
            PlaybackAction::Previous => {
                episode = adjacent_episode(context.episodes, &episode, -1)?;
            }
            PlaybackAction::Select => {
                let Some(selected) = select_episode(context.episodes)? else {
                    continue;
                };
                episode = selected;
            }
            PlaybackAction::ChangeQuality => {
                let streams = context
                    .clients
                    .streams(
                        &context.show.id,
                        context.show.provider,
                        &episode,
                        context.mode,
                    )
                    .await?;
                let choices: Vec<_> = streams
                    .iter()
                    .map(|value| format!("{} · {}", value.resolution, value.provider))
                    .collect();
                let Some(index) = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select quality (j/k: move, q/Esc: back)")
                    .items(&choices)
                    .interact_opt()
                    .map_err(dialog_error)?
                else {
                    continue;
                };
                quality = streams[index].resolution.clone();
            }
            PlaybackAction::BackToSearch => break Ok(AfterPlayAction::Search),
            PlaybackAction::Quit => break Ok(AfterPlayAction::Done),
        }
        play_or_download(context, &episode, &quality, false).await?;
    }
}

fn dialog_error(error: dialoguer::Error) -> AniError {
    eprintln!("Interactive selection failed: {error}");
    AniError::SelectionOutOfRange {
        max: 0,
        selected: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiple_episode_selection_preserves_episode_order() {
        let episodes = vec!["1".into(), "2".into(), "2.5".into(), "10".into()];

        assert_eq!(
            multiple_episode_selection(&episodes, &[0, 2, 3]).as_deref(),
            Some("1 2.5 10")
        );
        assert_eq!(multiple_episode_selection(&episodes, &[]), None);
    }

    #[test]
    fn playback_actions_hide_unavailable_episode_directions() {
        let episodes = vec!["1".into(), "2".into(), "3".into()];
        assert_eq!(
            playback_actions(&episodes, "1"),
            vec![
                PlaybackAction::Next,
                PlaybackAction::Replay,
                PlaybackAction::Select,
                PlaybackAction::ChangeQuality,
                PlaybackAction::BackToSearch,
                PlaybackAction::Quit,
            ]
        );
        assert_eq!(
            playback_actions(&episodes, "3"),
            vec![
                PlaybackAction::Replay,
                PlaybackAction::Previous,
                PlaybackAction::Select,
                PlaybackAction::ChangeQuality,
                PlaybackAction::BackToSearch,
                PlaybackAction::Quit,
            ]
        );
    }

    #[test]
    fn playback_controls_remain_available_for_one_interactive_episode() {
        assert!(should_offer_playback_controls(1, true, false, false));
        assert!(!should_offer_playback_controls(2, true, false, false));
        assert!(!should_offer_playback_controls(1, false, false, false));
        assert!(!should_offer_playback_controls(1, true, true, false));
        assert!(!should_offer_playback_controls(1, true, false, true));
    }

    #[test]
    fn download_prompts_name_the_anime_entry_as_a_season() {
        assert!(
            SelectionPurpose::Download
                .anime_prompt()
                .contains("anime / season")
        );
        assert!(
            SelectionPurpose::Download
                .episode_prompt()
                .contains("download")
        );
    }
}
