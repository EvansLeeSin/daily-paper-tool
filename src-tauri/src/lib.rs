mod config;
mod fetch;
mod llm;
mod local_git;
mod report;
mod utils;

use config::AppConfig;
use report::WeeklyWorkItem;
use simplelog::*;
use std::fs::OpenOptions;

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    config::save_config(&config)
}

#[tauri::command]
fn load_config() -> Result<AppConfig, String> {
    config::load_config()
}

#[tauri::command]
async fn fetch_daily_items(date: String) -> Result<fetch::FetchedItems, String> {
    let config = config::load_config()?;

    tauri::async_runtime::spawn_blocking(move || fetch::fetch_daily_items(&config, &date))
        .await
        .map_err(|e| format!("任务执行失败: {}", e))?
}

#[tauri::command]
async fn polish_daily_items(date: String, items_json: String) -> Result<Vec<String>, String> {
    let config = config::load_config()?;

    #[derive(serde::Deserialize)]
    struct RawItem {
        content: String,
        source: String,
    }

    let raw: Vec<RawItem> = serde_json::from_str(&items_json)
        .map_err(|e| format!("解析数据失败: {}", e))?;

    let items = raw
        .into_iter()
        .map(|item| fetch::WorkItemWithSource {
            content: item.content,
            source: item.source,
        })
        .collect::<Vec<_>>();

    tauri::async_runtime::spawn_blocking(move || fetch::polish_daily_items(&config, &date, &items))
        .await
        .map_err(|e| format!("任务执行失败: {}", e))?
}

#[tauri::command]
fn export_week_report(
    start_date: String,
    end_date: String,
    items_json: String,
    summary: String,
) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    struct DayItems {
        date: String,
        contents: Vec<String>,
    }

    let config = config::load_config()?;
    let day_items: Vec<DayItems> = serde_json::from_str(&items_json)
        .map_err(|e| format!("解析工作内容失败: {}", e))?;

    let weekly_items = day_items
        .into_iter()
        .map(|item| WeeklyWorkItem {
            date: item.date,
            contents: item.contents,
        })
        .collect::<Vec<_>>();

    report::generate_week_docx(&config, &start_date, &end_date, &weekly_items, &summary)?;

    let file_name = format!("周报_{}_{}.docx", start_date, end_date);
    let src_path = crate::config::CONFIG_DIR.lock().unwrap().join(&file_name);

    let save_path = rfd::FileDialog::new()
        .set_file_name(&file_name)
        .add_filter("Word 文件", &["docx"])
        .save_file();

    match save_path {
        Some(dest) => {
            std::fs::copy(&src_path, &dest).map_err(|e| format!("保存文件失败: {}", e))?;

            #[cfg(target_os = "macos")]
            std::process::Command::new("open").arg("-R").arg(&dest).spawn().ok();

            #[cfg(target_os = "windows")]
            std::process::Command::new("explorer")
                .arg("/select,")
                .arg(&dest)
                .spawn()
                .ok();

            #[cfg(target_os = "linux")]
            {
                if let Some(parent) = dest.parent() {
                    std::process::Command::new("xdg-open").arg(parent).spawn().ok();
                }
            }

            Ok(dest.to_string_lossy().to_string())
        }
        None => Err("已取消".to_string()),
    }
}

fn check_model_config(config: &AppConfig) -> Result<(), String> {
    if config.model.base_url.trim().is_empty()
        || config.model.api_key.trim().is_empty()
        || config.model.model.trim().is_empty()
    {
        return Err("请先配置模型信息（base_url / api_key / model）".to_string());
    }
    Ok(())
}

#[tauri::command]
async fn summarize_week(items_json: String) -> Result<String, String> {
    let config = config::load_config()?;
    check_model_config(&config)?;

    let items: Vec<String> = serde_json::from_str(&items_json)
        .map_err(|e| format!("解析数据失败: {}", e))?;

    if items.is_empty() {
        return Err("本周暂无工作内容".to_string());
    }

    tauri::async_runtime::spawn_blocking(move || {
        llm::summarize_week_with_openai(
            &config.model.base_url,
            &config.model.api_key,
            &config.model.model,
            &items,
            &config.prompts.summary_system,
        )
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

#[tauri::command]
fn get_log_path() -> Result<String, String> {
    let log_dir = config::CONFIG_DIR.lock().unwrap().clone();
    let log_path = log_dir.join("daily-paper-generator.log");
    Ok(log_path.to_string_lossy().to_string())
}

#[tauri::command]
fn read_log_file() -> Result<String, String> {
    let log_dir = config::CONFIG_DIR.lock().unwrap().clone();
    let log_path = log_dir.join("daily-paper-generator.log");

    if !log_path.exists() {
        return Ok("日志文件不存在".to_string());
    }

    std::fs::read_to_string(&log_path).map_err(|e| format!("读取日志文件失败: {}", e))
}

fn init_logger() {
    let log_dir = config::CONFIG_DIR.lock().unwrap().clone();
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("daily-paper-generator.log");

    if let Ok(file) = OpenOptions::new().append(true).create(true).open(&log_path) {
        let _ = WriteLogger::init(LevelFilter::Info, Config::default(), file);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logger();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .setup(|_app| Ok(()))
        .invoke_handler(tauri::generate_handler![
            save_config,
            load_config,
            fetch_daily_items,
            polish_daily_items,
            summarize_week,
            export_week_report,
            get_log_path,
            read_log_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
