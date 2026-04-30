use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub static CONFIG_DIR: Lazy<Mutex<PathBuf>> = Lazy::new(|| {
    let dir = dirs_next::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("daily-paper-generator");
    Mutex::new(dir)
});

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalGitConfig {
    #[serde(default)]
    pub repo_paths: Vec<String>,
    #[serde(default)]
    pub author_name: String,
    #[serde(default)]
    pub author_email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelConfig {
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsConfig {
    #[serde(default = "default_polish_system")]
    pub polish_system: String,
    #[serde(default = "default_polish_few_shot")]
    pub polish_few_shot: String,
    #[serde(default = "default_summary_system")]
    pub summary_system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    #[serde(default)]
    pub employee_name: String,
    #[serde(default = "default_daily_hours")]
    pub daily_hours: u32,
    #[serde(default = "default_default_progress")]
    pub default_progress: String,
    #[serde(default)]
    pub summary_note: String,
}

fn default_daily_hours() -> u32 {
    8
}

fn default_default_progress() -> String {
    "100%".to_string()
}

fn default_polish_system() -> String {
    "你是工作周报助手。请将本地 Git 提交信息整理成可直接写入日报/周报的中文工作要点。\n\
硬性规则：\n\
1) 只输出要点列表，每条一行，不要标题、解释或编号。\n\
2) 每天最多输出 4 条，优先合并同主题的零碎提交。\n\
3) 每条尽量以动词开头，如：新增、修复、优化、完善、联调、重构。\n\
4) 输出中禁止出现仓库路径、分支名、提交 hash、URL 等技术噪音。\n\
5) 不要编造未发生的工作，信息不足时宁可少写。".to_string()
}

fn default_polish_few_shot() -> String {
    "【示例输入】\n\
【日期】2026-04-21\n\
【本地 Git 提交】\n\
- feat: add valve config form validation\n\
- fix: topology export null pointer\n\
- refactor: optimize sandbox mapper copy logic\n\
\n\
【示例输出】\n\
新增阀门配置表单校验能力\n\
修复拓扑导出空指针问题\n\
优化沙箱参数复制逻辑".to_string()
}

fn default_summary_system() -> String {
    "你是工作总结助手。请将本周工作内容整合为一段精炼的中文周总结。\n\
硬性规则：\n\
1) 输出一段连贯总结，不超过200字。\n\
2) 总结重点放在本周的主要工作方向、结果和价值。\n\
3) 禁止出现仓库路径、提交 hash、URL 等技术细节。\n\
4) 如实总结，不要编造。".to_string()
}

impl Default for PromptsConfig {
    fn default() -> Self {
        Self {
            polish_system: default_polish_system(),
            polish_few_shot: default_polish_few_shot(),
            summary_system: default_summary_system(),
        }
    }
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            employee_name: String::new(),
            daily_hours: default_daily_hours(),
            default_progress: default_default_progress(),
            summary_note: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub local_git: LocalGitConfig,
    #[serde(default)]
    pub model: ModelConfig,
    #[serde(default)]
    pub prompts: PromptsConfig,
    #[serde(default)]
    pub report: ReportConfig,
}

pub fn get_config_path() -> PathBuf {
    CONFIG_DIR.lock().unwrap().join("config.json")
}

pub fn ensure_config_dir() -> Result<(), String> {
    let dir = CONFIG_DIR.lock().unwrap().clone();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    Ok(())
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    ensure_config_dir()?;
    let path = get_config_path();
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write config file: {}", e))?;
    log::info!("Config saved to {:?}", path);
    Ok(())
}

pub fn load_config() -> Result<AppConfig, String> {
    let path = get_config_path();
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    let config: AppConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config: {}", e))?;
    log::info!("Config loaded from {:?}", path);
    Ok(config)
}
