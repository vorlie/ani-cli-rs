use std::path::{Path, PathBuf};

use crate::{AniError, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEntry {
    pub episode: String,
    pub show_id: String,
    pub title: String,
}

#[derive(Clone, Debug)]
pub struct HistoryStore {
    path: PathBuf,
}

impl HistoryStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn platform_default() -> Result<Self> {
        if let Some(path) = std::env::var_os("ANI_CLI_HIST_DIR") {
            return Ok(Self::new(PathBuf::from(path).join("ani-hsts")));
        }
        let project = directories::ProjectDirs::from("org", "ani-cli", "ani-cli")
            .ok_or_else(|| AniError::HistoryStateDirectory)?;
        let directory = project
            .state_dir()
            .unwrap_or_else(|| project.data_local_dir());
        Ok(Self::new(directory.join("ani-hsts")))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn entries(&self) -> Result<Vec<HistoryEntry>> {
        let text = match tokio::fs::read_to_string(&self.path).await {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(error) => return Err(error.into()),
        };
        Ok(text
            .lines()
            .filter_map(|line| {
                let mut values = line.splitn(3, '\t');
                Some(HistoryEntry {
                    episode: values.next()?.into(),
                    show_id: values.next()?.into(),
                    title: values.next()?.into(),
                })
            })
            .collect())
    }

    pub async fn update(&self, entry: HistoryEntry) -> Result<()> {
        let mut entries = self.entries().await?;
        entry_valid(&entry)?;
        if let Some(existing) = entries
            .iter_mut()
            .find(|value| value.show_id == entry.show_id)
        {
            *existing = entry;
        } else {
            entries.push(entry);
        }
        self.write(&entries).await
    }

    pub async fn clear(&self) -> Result<()> {
        self.write(&[]).await
    }

    async fn write(&self, entries: &[HistoryEntry]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let text = entries
            .iter()
            .map(|value| {
                format!(
                    "{}\t{}\t{}\n",
                    value.episode,
                    value.show_id,
                    value.title.replace(['\t', '\n'], " ")
                )
            })
            .collect::<String>();
        let temporary = self.path.with_extension("new");
        tokio::fs::write(&temporary, text).await?;
        if tokio::fs::try_exists(&self.path).await? {
            tokio::fs::remove_file(&self.path).await?;
        }
        tokio::fs::rename(temporary, &self.path).await?;
        Ok(())
    }
}

fn entry_valid(entry: &HistoryEntry) -> Result<()> {
    if entry.episode.is_empty() || entry.show_id.is_empty() || entry.title.is_empty() {
        Err(AniError::History(
            "history entry contains an empty field".into(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn reads_and_updates_legacy_format() {
        let directory = tempfile::tempdir().unwrap();
        let store = HistoryStore::new(directory.path().join("ani-hsts"));
        store
            .update(HistoryEntry {
                episode: "1".into(),
                show_id: "abc".into(),
                title: "Anime".into(),
            })
            .await
            .unwrap();
        store
            .update(HistoryEntry {
                episode: "2".into(),
                show_id: "abc".into(),
                title: "Anime".into(),
            })
            .await
            .unwrap();
        assert_eq!(
            store.entries().await.unwrap(),
            vec![HistoryEntry {
                episode: "2".into(),
                show_id: "abc".into(),
                title: "Anime".into()
            }]
        );
    }
}
