use crate::models::CodeDebtItem;
use chrono::{DateTime, Utc};
use git2::{BlameOptions, Repository};
use log::{debug, warn};

pub struct GitAnalyzer;

impl GitAnalyzer {
    pub fn add_git_information(git_repo: Option<&Repository>, items: &mut [CodeDebtItem]) {
        if let Some(repo) = git_repo {
            debug!("Adding git blame information to {} items", items.len());
            let mut success_count = 0;
            let mut error_count = 0;

            for item in items.iter_mut() {
                match item
                    .file_path
                    .strip_prefix(repo.workdir().unwrap_or_else(|| std::path::Path::new(".")))
                {
                    Ok(relative_path) => {
                        match repo.blame_file(relative_path, Some(&mut BlameOptions::new())) {
                            Ok(blame) => {
                                if let Some(hunk) = blame.get_line(item.line_number) {
                                    let sig = hunk.final_signature();
                                    let oid = hunk.final_commit_id();

                                    item.author = sig.name().map(|s| s.to_string());
                                    item.commit_hash = Some(oid.to_string());

                                    if let Ok(commit) = repo.find_commit(oid) {
                                        let timestamp = commit.time().seconds();
                                        let datetime = DateTime::from_timestamp(timestamp, 0)
                                            .unwrap_or_else(Utc::now);
                                        item.created_at = Some(datetime);
                                        let now = Utc::now();
                                        let duration = now.signed_duration_since(datetime);
                                        item.age_days = Some(duration.num_days());
                                        success_count += 1;
                                    }
                                }
                            }
                            Err(e) => {
                                debug!(
                                    "Failed to get git blame for {}:{} - {}",
                                    item.file_path.display(),
                                    item.line_number,
                                    e
                                );
                                error_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        debug!(
                            "Failed to get relative path for {} - {}",
                            item.file_path.display(),
                            e
                        );
                        error_count += 1;
                    }
                }
            }

            if error_count > 0 {
                warn!(
                    "Git blame completed with {} successes and {} errors",
                    success_count, error_count
                );
            } else {
                debug!(
                    "Git blame completed successfully for all {} items",
                    success_count
                );
            }
        }
    }
}
