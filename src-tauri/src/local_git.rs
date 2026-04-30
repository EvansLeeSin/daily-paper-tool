use crate::config::AppConfig;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct LocalCommitInfo {
    pub title: String,
}

fn configured_repo_paths(config: &AppConfig) -> Vec<String> {
    config
        .local_git
        .repo_paths
        .iter()
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
        .collect()
}

fn author_matches(config: &AppConfig, author_name: &str, author_email: &str) -> bool {
    let configured_name = config.local_git.author_name.trim();
    let configured_email = config.local_git.author_email.trim();

    let name_match = configured_name.is_empty() || author_name.eq_ignore_ascii_case(configured_name);
    let email_match = configured_email.is_empty() || author_email.eq_ignore_ascii_case(configured_email);

    name_match && email_match
}

pub fn fetch_commits(config: &AppConfig, date: &str) -> Result<Vec<LocalCommitInfo>, String> {
    let repo_paths = configured_repo_paths(config);
    if repo_paths.is_empty() {
        return Err("请先配置本地 Git 仓库路径".to_string());
    }

    if config.local_git.author_name.trim().is_empty() && config.local_git.author_email.trim().is_empty() {
        return Err("请至少配置 Git 作者名或作者邮箱".to_string());
    }

    let mut commits = Vec::new();
    let since = format!("{} 00:00:00", date);
    let until = format!("{} 23:59:59", date);

    for repo_path in repo_paths {
        let output = Command::new("git")
            .args([
                "-C",
                &repo_path,
                "log",
                "--no-merges",
                "--since",
                &since,
                "--until",
                &until,
                "--pretty=format:%an%x1f%ae%x1f%s",
            ])
            .output()
            .map_err(|e| format!("读取本地 Git 提交失败: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(format!("仓库 {} 读取失败: {}", repo_path, stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let mut parts = line.split('\u{001f}');
            let author_name = parts.next().unwrap_or_default().trim();
            let author_email = parts.next().unwrap_or_default().trim();
            let title = parts.next().unwrap_or_default().trim();

            if title.is_empty() || !author_matches(config, author_name, author_email) {
                continue;
            }

            commits.push(LocalCommitInfo {
                title: title.to_string(),
            });
        }
    }

    Ok(commits)
}
