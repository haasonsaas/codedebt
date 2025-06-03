use crate::models::CodeDebtItem;
use chrono::{DateTime, Utc};
use git2::{BlameOptions, Repository};

pub struct GitAnalyzer;

impl GitAnalyzer {
    pub fn add_git_information(git_repo: &Option<Repository>, items: &mut [CodeDebtItem]) {
        if let Some(repo) = git_repo {
            for item in items.iter_mut() {
                if let Ok(relative_path) = item
                    .file_path
                    .strip_prefix(repo.workdir().unwrap_or_else(|| std::path::Path::new(".")))
                {
                    if let Ok(blame) =
                        repo.blame_file(relative_path, Some(&mut BlameOptions::new()))
                    {
                        if let Some(hunk) = blame.get_line(item.line_number) {
                            let sig = hunk.final_signature();
                            let oid = hunk.final_commit_id();

                            item.author = sig.name().map(|s| s.to_string());
                            item.commit_hash = Some(oid.to_string());

                            if let Ok(commit) = repo.find_commit(oid) {
                                let timestamp = commit.time().seconds();
                                let datetime =
                                    DateTime::from_timestamp(timestamp, 0).unwrap_or_else(Utc::now);
                                item.created_at = Some(datetime);
                                let now = Utc::now();
                                let duration = now.signed_duration_since(datetime);
                                item.age_days = Some(duration.num_days());
                            }
                        }
                    }
                }
            }
        }
    }
}