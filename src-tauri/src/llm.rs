use crate::utils::{compact_whitespace, contains_forbidden_markers};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    content: String,
}

const DAILY_BULLET_MAX: usize = 4;

fn clean_line(line: &str) -> String {
    line.trim()
        .trim_start_matches("- ")
        .trim_start_matches("• ")
        .trim_start_matches("* ")
        .trim_start_matches("· ")
        .trim()
        .to_string()
}

pub fn postprocess_daily_bullets(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();

    for raw in lines {
        let cleaned = compact_whitespace(&clean_line(&raw));
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

fn post_json(base_url: &str, api_key: &str, req: &ChatRequest) -> Result<ChatResponse, String> {
    let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
    let client = Client::new();

    let res = client
        .post(url)
        .bearer_auth(api_key)
        .json(req)
        .send()
        .map_err(|e| format!("模型请求失败: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("模型接口错误: {}", res.status()));
    }

    res.json().map_err(|e| format!("模型响应解析失败: {}", e))
}

pub fn summarize_week_with_openai(
    base_url: &str,
    api_key: &str,
    model: &str,
    week_items: &[String],
    system_prompt: &str,
) -> Result<String, String> {
    let prompt = format!(
        "【本周工作内容】\n- {}\n\n请生成本周工作总结（不超过200字）：",
        week_items.join("\n- ")
    );

    let req = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
        temperature: 0.3,
    };

    let data = post_json(base_url, api_key, &req)?;
    let content = data
        .choices
        .first()
        .map(|choice| choice.message.content.clone())
        .unwrap_or_default();

    Ok(content.trim().to_string())
}

pub fn polish_with_openai(
    base_url: &str,
    api_key: &str,
    model: &str,
    input: &str,
    system_prompt: &str,
    few_shot_prompt: &str,
) -> Result<Vec<String>, String> {
    let prompt = format!(
        "{}\n\n{}\n\n【开始处理】\n{}",
        "请严格按规则输出。",
        few_shot_prompt,
        input
    );

    let req = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
        temperature: 0.2,
    };

    let data = post_json(base_url, api_key, &req)?;
    let content = data
        .choices
        .first()
        .map(|choice| choice.message.content.clone())
        .unwrap_or_default();

    Ok(postprocess_daily_bullets(
        content.lines().map(|line| line.to_string()).collect(),
    ))
}
