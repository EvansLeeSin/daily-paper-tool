<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { message } from "ant-design-vue";
import dayjs, { type Dayjs } from "dayjs";
import isoWeek from "dayjs/plugin/isoWeek";
import { invoke } from "@tauri-apps/api/core";
import {
  getWeekSummary,
  initDb,
  listWorkItems,
  moveWorkItemToDate,
  replaceWorkItems,
  saveWeekSummary,
  type WorkItem,
} from "../db";
import { buildWeekDates } from "../utils/date";

dayjs.extend(isoWeek);

interface FetchedItem {
  content: string;
  source: "git";
}

interface DailyCard {
  date: Date;
  dateStr: string;
  weekday: string;
  items: WorkItem[];
  fetchLoading: boolean;
  polishLoading: boolean;
}

const cards = ref<DailyCard[]>([]);
const loading = ref(false);
const fetchWeekLoading = ref(false);
const exportLoading = ref(false);
const summaryLoading = ref(false);
const weekSummary = ref("");

const selectedWeekStart = ref<Dayjs>(dayjs().isoWeekday(1).startOf("day"));
const thisWeekStart = dayjs().isoWeekday(1).startOf("day");

const editModalOpen = ref(false);
const editDate = ref<Dayjs>(dayjs());
const editContents = ref<string[]>([""]);
const editSaving = ref(false);

const draggingItemId = ref<number | null>(null);
const draggingFromDate = ref<string>("");
const dropTargetDate = ref<string>("");
const movingItem = ref(false);

const weekLabel = computed(() => {
  const start = selectedWeekStart.value;
  const end = start.add(6, "day");
  return start.isSame(thisWeekStart, "day")
    ? `本周（${start.format("MM/DD")} - ${end.format("MM/DD")}）`
    : `${start.format("YYYY/MM/DD")} - ${end.format("MM/DD")}`;
});

const isCurrentWeek = computed(() => selectedWeekStart.value.isSame(thisWeekStart, "day"));

function disabledWeek(date: Dayjs) {
  return date.isoWeekday(1).startOf("day").isAfter(thisWeekStart);
}

function onWeekChange(date: Dayjs | null) {
  if (!date) return;
  selectedWeekStart.value = date.isoWeekday(1).startOf("day");
  loadWeek();
}

function prevWeek() {
  selectedWeekStart.value = selectedWeekStart.value.subtract(1, "week");
  loadWeek();
}

function nextWeek() {
  const next = selectedWeekStart.value.add(1, "week");
  if (next.isAfter(thisWeekStart)) return;
  selectedWeekStart.value = next;
  loadWeek();
}

onMounted(async () => {
  try {
    await initDb();
    await loadWeek();
  } catch (error) {
    message.error(`初始化失败: ${error}`);
  }
});

async function loadWeek() {
  loading.value = true;
  try {
    const week = buildWeekDates(selectedWeekStart.value.toDate());
    const rows = await listWorkItems(week[0].dateStr, week[6].dateStr);
    cards.value = week.map((day) => ({
      ...day,
      items: rows.filter((row) => row.work_date === day.dateStr),
      fetchLoading: false,
      polishLoading: false,
    }));

    const summaryRecord = await getWeekSummary(selectedWeekStart.value.format("YYYY-MM-DD"));
    weekSummary.value = summaryRecord?.summary || "";
  } finally {
    loading.value = false;
  }
}

async function fetchDay(dateStr: string) {
  const result = await invoke<{ items: FetchedItem[] }>("fetch_daily_items", { date: dateStr });
  const items = result?.items || [];
  await replaceWorkItems(
    dateStr,
    items.map((item) => ({
      content: item.content,
      source: item.source,
    }))
  );
  return items.length;
}

async function handleAutoFetch(card: DailyCard) {
  card.fetchLoading = true;
  try {
    const count = await fetchDay(card.dateStr);
    await loadWeek();
    message.success(`已读取 ${count} 条本地 Git 记录`);
  } catch (error) {
    message.error(`读取失败: ${error}`);
  } finally {
    card.fetchLoading = false;
  }
}

async function handleFetchWeek() {
  fetchWeekLoading.value = true;
  try {
    for (const card of cards.value) {
      await fetchDay(card.dateStr);
    }
    await loadWeek();
    message.success("本周 Git 记录已更新");
  } catch (error) {
    message.error(`批量读取失败: ${error}`);
  } finally {
    fetchWeekLoading.value = false;
  }
}

async function handlePolish(card: DailyCard) {
  if (card.items.length === 0) {
    message.warning("请先准备当天工作内容");
    return;
  }

  card.polishLoading = true;
  try {
    const rawItems = card.items.map((item) => ({
      content: item.content,
      source: item.source || "manual",
    }));
    const polished = await invoke<string[]>("polish_daily_items", {
      date: card.dateStr,
      itemsJson: JSON.stringify(rawItems),
    });

    if (!polished.length) {
      message.warning("AI 润色未返回有效结果，原记录已保留");
      return;
    }

    await replaceWorkItems(
      card.dateStr,
      polished.map((content) => ({
        content,
        source: "manual" as const,
      }))
    );
    await loadWeek();
    message.success(`AI 润色完成，已整理为 ${polished.length} 条`);
  } catch (error) {
    message.error(`AI 润色失败: ${error}`);
  } finally {
    card.polishLoading = false;
  }
}

function openEditModal(card: DailyCard) {
  editDate.value = dayjs(card.dateStr);
  editContents.value = card.items.length ? card.items.map((item) => item.content) : [""];
  editModalOpen.value = true;
}

function addEditRow() {
  editContents.value.push("");
}

function removeEditRow(index: number) {
  if (editContents.value.length === 1) {
    editContents.value[0] = "";
    return;
  }
  editContents.value.splice(index, 1);
}

async function handleEditSave() {
  const rows = editContents.value.map((value) => value.trim()).filter(Boolean);
  if (rows.length === 0) {
    message.warning("请至少保留一条工作内容");
    return;
  }

  editSaving.value = true;
  try {
    await replaceWorkItems(
      editDate.value.format("YYYY-MM-DD"),
      rows.map((content) => ({ content, source: "manual" as const }))
    );
    editModalOpen.value = false;
    await loadWeek();
    message.success("已保存");
  } finally {
    editSaving.value = false;
  }
}

async function handleSummarizeWeek() {
  const allItems = cards.value.flatMap((card) => card.items.map((item) => item.content)).filter(Boolean);
  if (allItems.length === 0) {
    message.warning("本周暂无工作内容");
    return;
  }

  summaryLoading.value = true;
  try {
    const summary = await invoke<string>("summarize_week", {
      itemsJson: JSON.stringify(allItems),
    });
    weekSummary.value = summary;
    await saveWeekSummary(selectedWeekStart.value.format("YYYY-MM-DD"), summary);
    message.success("周总结已生成");
  } catch (error) {
    message.error(`总结失败: ${error}`);
  } finally {
    summaryLoading.value = false;
  }
}

async function exportWeek() {
  exportLoading.value = true;
  try {
    const week = buildWeekDates(selectedWeekStart.value.toDate());
    await invoke("export_week_report", {
      startDate: week[0].dateStr,
      endDate: week[6].dateStr,
      itemsJson: JSON.stringify(
        cards.value.map((card) => ({
          date: card.dateStr,
          contents: card.items.map((item) => item.content).slice(0, 4),
        }))
      ),
      summary: weekSummary.value,
    });
    message.success("周报已导出");
  } catch (error) {
    const text = String(error);
    if (text.includes("已取消")) return;
    message.error(`导出失败: ${error}`);
  } finally {
    exportLoading.value = false;
  }
}

function sourceLabel(source: string) {
  return source === "git" ? "Git" : null;
}

function sourceColor(source: string) {
  return source === "git" ? "geekblue" : "";
}

function handleDragStart(item: WorkItem) {
  draggingItemId.value = item.id;
  draggingFromDate.value = item.work_date;
}

function handleDragEnd() {
  draggingItemId.value = null;
  draggingFromDate.value = "";
  dropTargetDate.value = "";
}

function handleDragOver(card: DailyCard, event: DragEvent) {
  if (!draggingItemId.value || card.dateStr === draggingFromDate.value) {
    return;
  }
  event.preventDefault();
  dropTargetDate.value = card.dateStr;
}

function handleDragLeave(card: DailyCard) {
  if (dropTargetDate.value === card.dateStr) {
    dropTargetDate.value = "";
  }
}

async function handleDrop(card: DailyCard, event: DragEvent) {
  event.preventDefault();
  const itemId = draggingItemId.value;
  const fromDate = draggingFromDate.value;
  dropTargetDate.value = "";

  if (!itemId || !fromDate || fromDate === card.dateStr) {
    handleDragEnd();
    return;
  }

  movingItem.value = true;
  try {
    await moveWorkItemToDate(itemId, card.dateStr);
    await loadWeek();
    message.success(`已移动到 ${card.dateStr}`);
  } catch (error) {
    message.error(`移动失败: ${error}`);
  } finally {
    movingItem.value = false;
    handleDragEnd();
  }
}
</script>

<template>
  <div class="home-container">
    <div class="toolbar">
      <div class="week-nav">
        <a-button size="small" @click="prevWeek">&lt;</a-button>
        <a-date-picker
          picker="week"
          :value="selectedWeekStart"
          :disabled-date="disabledWeek"
          format="YYYY第WW周"
          :allow-clear="false"
          size="small"
          @change="onWeekChange"
        />
        <a-button size="small" :disabled="isCurrentWeek" @click="nextWeek">&gt;</a-button>
        <span class="week-label">{{ weekLabel }}</span>
      </div>
      <div class="actions">
        <a-button :loading="fetchWeekLoading" @click="handleFetchWeek">读取本周 Git</a-button>
        <a-button type="primary" :loading="exportLoading" @click="exportWeek">导出周报</a-button>
      </div>
    </div>

    <div class="panel-tip">
      自动读取不会限制每日 Git 记录数量。导出 Word 周报时，每天只取前 4 条。支持把单条记录拖拽到其他日期。
    </div>

    <div v-if="!loading" class="week-grid">
      <a-card
        v-for="card in cards"
        :key="card.dateStr"
        :title="`${card.dateStr}（${card.weekday}）`"
        class="day-card"
        :class="{ 'drop-active': dropTargetDate === card.dateStr }"
      >
        <div
          class="card-drop-zone"
          @dragover="handleDragOver(card, $event)"
          @dragleave="handleDragLeave(card)"
          @drop="handleDrop(card, $event)"
        >
          <div v-if="card.items.length === 0" class="empty-text">暂无记录，可拖拽其他日期的记录到这里</div>
          <ul v-else class="work-list">
            <li
              v-for="item in card.items"
              :key="item.id"
              class="work-item"
              :class="{ dragging: draggingItemId === item.id }"
              draggable="true"
              @dragstart="handleDragStart(item)"
              @dragend="handleDragEnd"
            >
              <a-tag v-if="sourceLabel(item.source)" :color="sourceColor(item.source)" class="source-tag">
                {{ sourceLabel(item.source) }}
              </a-tag>
              <span>{{ item.content }}</span>
            </li>
          </ul>
        </div>
        <template #actions>
          <a-button
            type="text"
            size="small"
            :loading="card.fetchLoading"
            :disabled="card.fetchLoading || card.polishLoading || fetchWeekLoading || movingItem"
            @click="handleAutoFetch(card)"
          >
            读取 Git
          </a-button>
          <a-button
            type="text"
            size="small"
            :loading="card.polishLoading"
            :disabled="card.fetchLoading || card.polishLoading || fetchWeekLoading || movingItem"
            @click="handlePolish(card)"
          >
            AI 润色
          </a-button>
          <a-button
            type="text"
            size="small"
            :disabled="card.fetchLoading || card.polishLoading || fetchWeekLoading || movingItem"
            @click="openEditModal(card)"
          >
            编辑
          </a-button>
        </template>
      </a-card>
    </div>

    <a-spin v-else />

    <div class="week-summary-section">
      <div class="summary-header">
        <span class="summary-title">{{ weekLabel }} 工作总结</span>
        <a-button type="primary" :loading="summaryLoading" @click="handleSummarizeWeek">
          AI 总结
        </a-button>
      </div>
      <a-spin v-if="summaryLoading" class="summary-spin" />
      <a-textarea
        v-else
        v-model:value="weekSummary"
        :rows="4"
        placeholder="点击「AI 总结」生成周总结，或直接手动编辑"
        @blur="saveWeekSummary(selectedWeekStart.format('YYYY-MM-DD'), weekSummary)"
      />
    </div>

    <a-modal
      v-model:open="editModalOpen"
      title="编辑工作内容"
      ok-text="保存"
      :confirm-loading="editSaving"
      @ok="handleEditSave"
    >
      <a-form layout="vertical">
        <a-form-item label="日期">
          <a-date-picker v-model:value="editDate" format="YYYY-MM-DD" disabled />
        </a-form-item>
        <a-form-item label="工作内容">
          <div class="dynamic-list">
            <div v-for="(_row, index) in editContents" :key="index" class="dynamic-row">
              <a-input v-model:value="editContents[index]" placeholder="输入一条工作内容" />
              <a-button type="text" danger @click="removeEditRow(index)">删除</a-button>
            </div>
            <a-button type="dashed" block @click="addEditRow">+ 添加一行</a-button>
          </div>
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<style>
.home-container {
  padding: 8px 12px;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.week-nav {
  display: flex;
  align-items: center;
  gap: 8px;
}

.week-label {
  font-size: 14px;
  color: var(--text-secondary);
}

.actions {
  display: flex;
  gap: 8px;
}

.panel-tip {
  margin-bottom: 16px;
  padding: 10px 12px;
  border-radius: 8px;
  background: var(--bg-section);
  border: 1px solid var(--bg-section-border);
  color: var(--text-secondary);
  font-size: 13px;
}

.week-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 16px;
}

.day-card {
  min-height: 180px;
  display: flex;
  flex-direction: column;
  transition: box-shadow 0.2s ease, transform 0.2s ease;
}

.day-card.drop-active {
  box-shadow: 0 0 0 2px #1677ff inset;
  transform: translateY(-2px);
}

.day-card :deep(.ant-card-body) {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.day-card :deep(.ant-card-actions) {
  margin-top: 0;
}

.card-drop-zone {
  flex: 1;
  min-height: 120px;
}

.dynamic-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.dynamic-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.work-list {
  padding-left: 0;
  margin: 0;
  list-style: none;
}

.work-item {
  display: flex;
  align-items: flex-start;
  gap: 4px;
  margin-bottom: 6px;
  line-height: 1.5;
  padding: 6px 8px;
  border-radius: 6px;
  cursor: grab;
}

.work-item:hover {
  background: rgba(22, 119, 255, 0.06);
}

.work-item.dragging {
  opacity: 0.45;
}

.source-tag {
  flex-shrink: 0;
  margin-top: 2px;
}

.empty-text {
  color: var(--text-muted);
}

.week-summary-section {
  margin-top: 24px;
  padding: 16px 20px;
  background: var(--bg-section);
  border: 1px solid var(--bg-section-border);
  border-radius: 8px;
}

.summary-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.summary-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.summary-spin {
  display: block;
  text-align: center;
  padding: 16px 0;
}
</style>
