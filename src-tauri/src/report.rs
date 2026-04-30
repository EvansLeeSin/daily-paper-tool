use crate::config::{ensure_config_dir, AppConfig};
use chrono::{Datelike, Duration, NaiveDate};
use docx_rs::*;

const MAX_ITEMS_PER_DAY: usize = 4;
const ROWS_PER_DAY: usize = 4;
const LIGHT_BLUE_FILL: &str = "DEEAF6";
const DATE_COL_WIDTH: usize = 1129;
const CONTENT_COL_WIDTH: usize = 2552;
const DURATION_COL_WIDTH: usize = 1701;
const PROGRESS_COL_WIDTH: usize = 2914;
const HEADER_ROW_HEIGHT: f32 = 360.0;
const BODY_ROW_HEIGHT: f32 = 430.0;
const SUMMARY_ROW_HEIGHT: f32 = 620.0;

pub struct WeeklyWorkItem {
    pub date: String,
    pub contents: Vec<String>,
}

fn compact_spacing(line: i32) -> LineSpacing {
    LineSpacing::new()
        .before(0)
        .after(0)
        .line(line)
        .line_rule(LineSpacingType::Auto)
}

fn make_text_paragraph(text: &str, center: bool, size: usize, bold: bool) -> Paragraph {
    let run = if bold {
        Run::new().add_text(text).bold().size(size)
    } else {
        Run::new().add_text(text).size(size)
    };

    let paragraph = Paragraph::new()
        .line_spacing(compact_spacing(300))
        .add_run(run);
    if center {
        paragraph.align(AlignmentType::Center)
    } else {
        paragraph
    }
}

fn title_paragraph(text: &str) -> Paragraph {
    Paragraph::new()
        .align(AlignmentType::Center)
        .line_spacing(compact_spacing(360))
        .add_run(Run::new().add_text(text).size(52))
}

fn employee_paragraph(text: &str) -> Paragraph {
    Paragraph::new()
        .align(AlignmentType::Center)
        .line_spacing(compact_spacing(300))
        .add_run(Run::new().add_text(text).size(32))
}

fn table_borders() -> TableBorders {
    TableBorders::new()
        .set(TableBorder::new(TableBorderPosition::Top).border_type(BorderType::Single).size(4))
        .set(TableBorder::new(TableBorderPosition::Bottom).border_type(BorderType::Single).size(4))
        .set(TableBorder::new(TableBorderPosition::Left).border_type(BorderType::Single).size(4))
        .set(TableBorder::new(TableBorderPosition::Right).border_type(BorderType::Single).size(4))
        .set(TableBorder::new(TableBorderPosition::InsideH).border_type(BorderType::Single).size(4))
        .set(TableBorder::new(TableBorderPosition::InsideV).border_type(BorderType::Single).size(4))
}

fn cell_borders() -> TableCellBorders {
    TableCellBorders::new()
        .set(TableCellBorder::new(TableCellBorderPosition::Top).border_type(BorderType::Single).size(4))
        .set(TableCellBorder::new(TableCellBorderPosition::Bottom).border_type(BorderType::Single).size(4))
        .set(TableCellBorder::new(TableCellBorderPosition::Left).border_type(BorderType::Single).size(4))
        .set(TableCellBorder::new(TableCellBorderPosition::Right).border_type(BorderType::Single).size(4))
}

fn header_cell(text: &str, width: usize) -> TableCell {
    TableCell::new()
        .width(width, WidthType::Dxa)
        .vertical_align(VAlignType::Center)
        .set_borders(cell_borders())
        .add_paragraph(make_text_paragraph(text, true, 22, false))
}

fn body_cell(text: &str, width: usize, center: bool, shaded: bool) -> TableCell {
    let paragraph = if center {
        make_text_paragraph(text, true, 22, false)
    } else {
        make_text_paragraph(text, false, 22, false)
    };

    let cell = TableCell::new()
        .width(width, WidthType::Dxa)
        .vertical_align(VAlignType::Center)
        .set_borders(cell_borders())
        .add_paragraph(paragraph);

    if shaded {
        cell.shading(Shading::new().shd_type(ShdType::Clear).fill(LIGHT_BLUE_FILL))
    } else {
        cell
    }
}

fn merged_date_cell(label: &str, width: usize, restart: bool, shaded: bool) -> TableCell {
    let cell = TableCell::new()
        .width(width, WidthType::Dxa)
        .vertical_align(VAlignType::Center)
        .set_borders(cell_borders())
        .vertical_merge(if restart {
            VMergeType::Restart
        } else {
            VMergeType::Continue
        });

    let cell = if restart {
        cell.add_paragraph(make_text_paragraph(label, true, 22, false))
    } else {
        cell.add_paragraph(Paragraph::new())
    };

    if shaded {
        cell.shading(Shading::new().shd_type(ShdType::Clear).fill(LIGHT_BLUE_FILL))
    } else {
        cell
    }
}

fn format_duration_text(daily_hours: u32, item_count: usize, row_index: usize) -> String {
    if item_count == 0 || row_index >= item_count {
        return String::new();
    }

    let per_item_hours = daily_hours as f32 / item_count as f32;
    if (per_item_hours.fract() - 0.0).abs() < f32::EPSILON {
        format!("（{}h）", per_item_hours as u32)
    } else {
        format!("（{:.1}h）", per_item_hours)
    }
}

fn weekday_label(date: &str) -> String {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .ok()
        .map(|parsed| format!("周{}", parsed.weekday().number_from_monday()))
        .unwrap_or_else(|| "周?".to_string())
}

fn title_range(start_date: &str) -> String {
    let start = NaiveDate::parse_from_str(start_date, "%Y-%m-%d").ok();
    if let Some(start) = start {
        let friday = start + Duration::days(4);
        return format!(
            "{}-{}",
            start.format("%Y.%m.%d"),
            friday.format("%m.%d")
        );
    }
    start_date.to_string()
}

fn total_hours(items: &[WeeklyWorkItem], daily_hours: u32) -> u32 {
    items.iter()
        .take(5)
        .filter(|item| !item.contents.is_empty())
        .count() as u32
        * daily_hours
}

pub fn generate_week_docx(
    config: &AppConfig,
    start_date: &str,
    end_date: &str,
    items: &[WeeklyWorkItem],
    summary: &str,
) -> Result<String, String> {
    ensure_config_dir()?;

    let file_name = format!("周报_{}_{}.docx", start_date, end_date);
    let file_path = crate::config::CONFIG_DIR.lock().unwrap().join(&file_name);

    let employee_name = config.report.employee_name.trim();
    let employee_line = if employee_name.is_empty() {
        "员工：".to_string()
    } else {
        format!("员工：{}", employee_name)
    };

    let mut doc = Docx::new()
        .default_fonts(
            RunFonts::new()
                .ascii("Times New Roman")
                .hi_ansi("Times New Roman")
                .east_asia("宋体")
                .cs("Times New Roman"),
        )
        .default_size(21)
        .default_spacing(0)
        .default_line_spacing(compact_spacing(300))
        .page_margin(
            PageMargin::new()
                .top(720)
                .right(860)
                .bottom(720)
                .left(860)
                .header(420)
                .footer(420),
        )
        .add_paragraph(title_paragraph(&format!("工作周报（{}）", title_range(start_date))))
        .add_paragraph(employee_paragraph(&employee_line));

    let mut rows = Vec::new();
    rows.push(TableRow::new(vec![
        header_cell("日期", DATE_COL_WIDTH),
        header_cell("工作内容", CONTENT_COL_WIDTH),
        header_cell("花费时长", DURATION_COL_WIDTH),
        header_cell("任务完成度和困难", PROGRESS_COL_WIDTH),
    ])
    .row_height(HEADER_ROW_HEIGHT)
    .cant_split());

    for item in items.iter().take(7) {
        let contents = item
            .contents
            .iter()
            .take(MAX_ITEMS_PER_DAY)
            .map(|content| content.trim().to_string())
            .filter(|content| !content.is_empty())
            .collect::<Vec<_>>();

        for row_index in 0..ROWS_PER_DAY {
            let shaded = row_index % 2 == 0;
            let date_cell = merged_date_cell(&weekday_label(&item.date), DATE_COL_WIDTH, row_index == 0, shaded);
            let content = contents.get(row_index).cloned().unwrap_or_default();
            let duration = format_duration_text(config.report.daily_hours, contents.len(), row_index);
            let progress = if content.is_empty() {
                String::new()
            } else {
                config.report.default_progress.clone()
            };

            rows.push(
                TableRow::new(vec![
                    date_cell,
                    body_cell(&content, CONTENT_COL_WIDTH, false, shaded),
                    body_cell(&duration, DURATION_COL_WIDTH, true, shaded),
                    body_cell(&progress, PROGRESS_COL_WIDTH, false, shaded),
                ])
                .row_height(BODY_ROW_HEIGHT)
                .cant_split(),
            );
        }
    }

    rows.push(
        TableRow::new(vec![
            body_cell("总结", DATE_COL_WIDTH, true, false),
            body_cell(summary.trim(), CONTENT_COL_WIDTH, false, false),
            body_cell(
                &format!("总时长{}hour", total_hours(items, config.report.daily_hours)),
                DURATION_COL_WIDTH,
                true,
                false,
            ),
            body_cell(config.report.summary_note.trim(), PROGRESS_COL_WIDTH, false, false),
        ])
        .row_height(SUMMARY_ROW_HEIGHT)
        .cant_split(),
    );

    let table = Table::new(rows)
        .set_grid(vec![DATE_COL_WIDTH, CONTENT_COL_WIDTH, DURATION_COL_WIDTH, PROGRESS_COL_WIDTH])
        .set_borders(table_borders())
        .margins(TableCellMargins::new().margin(90, 100, 90, 100))
        .align(TableAlignmentType::Center);

    doc = doc.add_table(table);

    let file = std::fs::File::create(&file_path)
        .map_err(|e| format!("创建周报文件失败: {}", e))?;

    doc.build()
        .pack(file)
        .map_err(|e| format!("生成周报文档失败: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}
