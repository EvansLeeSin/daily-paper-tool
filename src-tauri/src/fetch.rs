use crate::config::AppConfig;
use crate::utils::{compact_whitespace, contains_forbidden_markers};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct WorkItemWithSource {
    pub content: String,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct FetchedItems {
    pub items: Vec<WorkItemWithSource>,
}

const DAILY_BULLET_MAX: usize = 4;

fn postprocess_bullets(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();

    for raw in lines {
        let cleaned = compact_whitespace(raw.trim());
        if cleaned.is_empty() || contains_forbidden_markers(&cleaned) {
            continue;
        }
        if out.iter().any(|item| item == &cleaned) {
            continue;
        }
        out.push(cleaned);
        if out.len() >= DAILY_BULLET_MAX {
            break;
        }
    }

    out
}

fn normalize_commit_title(title: &str) -> String {
    let mut text = compact_whitespace(title);
    let lower = text.to_lowercase();

    for prefix in [
        "feat:",
        "fix:",
        "chore:",
        "refactor:",
        "perf:",
        "test:",
        "docs:",
        "style:",
        "ci:",
        "build:",
    ] {
        if lower.starts_with(prefix) {
            text = text[prefix.len()..].trim().to_string();
            break;
        }
    }

    if let Some(index) = text.find(':') {
        let before = &text[..index];
        if before.contains('(') && before.contains(')') && before.len() <= 24 {
            text = text[index + 1..].trim().to_string();
        }
    }

    text
}

fn build_llm_input(date: &str, commits: &[String]) -> String {
    let mut input = String::new();
    input.push_str(&format!("【日期】{}\n", date));
    input.push_str("【硬性要求】\n");
    input.push_str("- 只输出中文工作要点，每条一行\n");
    input.push_str("- 最多输出 4 条，优先合并相同主题的提交\n");
    input.push_str("- 不要出现仓库路径、提交 hash、URL\n");
    input.push_str("- 尽量写成结果导向表达\n");
    input.push_str("\n【本地 Git 提交】\n");

    if commits.is_empty() {
        input.push_str("(无)\n");
    } else {
        for commit in commits {
            input.push_str(&format!("- {}\n", commit));
        }
    }

    input
}

fn summarize_locally(commits: &[String]) -> Vec<String> {
    if commits.is_empty() {
        return Vec::new();
    }

    let mut buckets: std::collections::BTreeMap<&'static str, Vec<String>> = std::collections::BTreeMap::new();
    let mut direct = Vec::new();

    for commit in commits {
        let normalized = normalize_commit_title(commit);
        if normalized.is_empty() {
            continue;
        }

        if normalized.contains('，')
            || normalized.contains("优化")
            || normalized.contains("修复")
            || normalized.contains("新增")
            || normalized.contains("完善")
            || normalized.contains("联调")
            || normalized.contains("重构")
        {
            direct.push(normalized);
            continue;
        }

        let lower = normalized.to_lowercase();
        let key = if lower.contains("fix") || lower.contains("bug") || normalized.contains("修复") {
            "修复"
        } else if lower.contains("refactor") || normalized.contains("重构") {
            "重构"
        } else if lower.contains("opt") || lower.contains("perf") || normalized.contains("优化") {
            "优化"
        } else if lower.contains("test") || normalized.contains("测试") {
            "测试"
        } else if lower.contains("export") || normalized.contains("导出") {
            "导出"
        } else if lower.contains("add") || lower.contains("feat") || normalized.contains("新增") {
            "新增"
        } else {
            "其他"
        };

        buckets.entry(key).or_default().push(normalized);
    }

    let mut lines = Vec::new();

    for item in direct {
        if !lines.iter().any(|line| line == &item) {
            lines.push(item);
        }
        if lines.len() >= DAILY_BULLET_MAX {
            return lines;
        }
    }

    for (bucket, items) in buckets {
        if items.is_empty() {
            continue;
        }
        let line = match bucket {
            "修复" => "修复相关功能问题并完善处理逻辑",
            "重构" => "重构相关模块代码并整理实现逻辑",
            "优化" => "优化现有功能实现与处理流程",
            "测试" => "完成相关功能测试与联调验证",
            "导出" => "完善导出相关功能与内容输出",
            "新增" => "新增业务能力并补充对应实现",
            _ => "处理日常开发任务并跟进功能实现",
        };
        if !lines.iter().any(|item| item == line) {
            lines.push(line.to_string());
        }
        if lines.len() >= DAILY_BULLET_MAX {
            break;
        }
    }

    lines
}

pub fn fetch_daily_items(config: &AppConfig, date: &str) -> Result<FetchedItems, String> {
    log::info!("开始读取本地 Git 提交: date={}", date);

    let items = crate::local_git::fetch_commits(config, date)?
        .into_iter()
        .filter_map(|commit| {
            let content = normalize_commit_title(&commit.title);
            if content.is_empty() {
                None
            } else {
                Some(WorkItemWithSource {
                    content,
                    source: "git".to_string(),
                })
            }
        })
        .collect::<Vec<_>>();

    log::info!("本地 Git 提交读取完成: {} 条", items.len());
    Ok(FetchedItems { items })
}

pub fn polish_daily_items(config: &AppConfig, date: &str, raw_items: &[WorkItemWithSource]) -> Result<Vec<String>, String> {
    let commits = raw_items
        .iter()
        .map(|item| item.content.clone())
        .filter(|content| !content.trim().is_empty())
        .collect::<Vec<_>>();

    if commits.is_empty() {
        return Ok(Vec::new());
    }

    let has_model = !config.model.base_url.trim().is_empty()
        && !config.model.api_key.trim().is_empty()
        && !config.model.model.trim().is_empty();

    if !has_model {
        return Err("请先配置模型信息（base_url / api_key / model）".to_string());
    }

    let llm_input = build_llm_input(date, &commits);
    let lines = match crate::llm::polish_with_openai(
        &config.model.base_url,
        &config.model.api_key,
        &config.model.model,
        &llm_input,
        &config.prompts.polish_system,
        &config.prompts.polish_few_shot,
    ) {
        Ok(lines) => {
            let cleaned = crate::llm::postprocess_daily_bullets(lines);
            if cleaned.is_empty() {
                summarize_locally(&commits)
            } else {
                cleaned
            }
        }
        Err(error) => {
            log::warn!("AI 润色失败，回退本地规则: {}", error);
            summarize_locally(&commits)
        }
    };

    Ok(postprocess_bullets(lines))
}
