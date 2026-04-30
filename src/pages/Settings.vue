<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { message } from "ant-design-vue";

interface AppConfig {
  local_git: {
    repo_paths: string[];
    author_name: string;
    author_email: string;
  };
  model: {
    base_url: string;
    api_key: string;
    model: string;
  };
  prompts: {
    polish_system: string;
    polish_few_shot: string;
    summary_system: string;
  };
  report: {
    employee_name: string;
    daily_hours: number;
    default_progress: string;
    summary_note: string;
  };
}

const config = ref<AppConfig>({
  local_git: {
    repo_paths: [],
    author_name: "",
    author_email: "",
  },
  model: {
    base_url: "",
    api_key: "",
    model: "",
  },
  prompts: {
    polish_system: "",
    polish_few_shot: "",
    summary_system: "",
  },
  report: {
    employee_name: "",
    daily_hours: 8,
    default_progress: "100%",
    summary_note: "",
  },
});

const repoPathsText = ref("");
const loading = ref(false);
const saveStatus = ref<"idle" | "saving" | "saved" | "error">("idle");

let saveTimer: ReturnType<typeof setTimeout> | null = null;
let fadeTimer: ReturnType<typeof setTimeout> | null = null;

function hydrateRepoPaths() {
  repoPathsText.value = config.value.local_git.repo_paths.join("\n");
}

function buildConfigToSave(): AppConfig {
  return {
    ...config.value,
    local_git: {
      ...config.value.local_git,
      repo_paths: repoPathsText.value
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter(Boolean),
    },
    report: {
      ...config.value.report,
      daily_hours: Number(config.value.report.daily_hours) || 8,
    },
  };
}

onMounted(async () => {
  loading.value = true;
  try {
    const result = await invoke<AppConfig>("load_config");
    if (result) {
      config.value = {
        local_git: result.local_git || { repo_paths: [], author_name: "", author_email: "" },
        model: result.model || { base_url: "", api_key: "", model: "" },
        prompts: result.prompts || { polish_system: "", polish_few_shot: "", summary_system: "" },
        report: result.report || { employee_name: "", daily_hours: 8, default_progress: "100%", summary_note: "" },
      };
      hydrateRepoPaths();
    }
  } catch (error) {
    message.error(`加载配置失败: ${error}`);
  } finally {
    loading.value = false;
  }
});

function handleBlur() {
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    saveStatus.value = "saving";
    if (fadeTimer) clearTimeout(fadeTimer);
    try {
      await invoke("save_config", { config: buildConfigToSave() });
      saveStatus.value = "saved";
    } catch (_error) {
      saveStatus.value = "error";
    }
    fadeTimer = setTimeout(() => {
      saveStatus.value = "idle";
    }, 2000);
  }, 300);
}
</script>

<template>
  <div class="settings-container">
    <div class="settings-grid">
      <a-card title="本地 Git 配置" class="config-card">
        <a-form layout="vertical" :model="config" class="config-form">
          <a-form-item label="仓库路径">
            <a-textarea
              v-model:value="repoPathsText"
              :rows="6"
              placeholder="每行一个本地 Git 仓库路径"
              @blur="handleBlur"
            />
          </a-form-item>
          <a-form-item label="Git 作者名">
            <a-input
              v-model:value="config.local_git.author_name"
              placeholder="例如：zhangsan"
              @blur="handleBlur"
            />
          </a-form-item>
          <a-form-item label="Git 作者邮箱">
            <a-input
              v-model:value="config.local_git.author_email"
              placeholder="例如：zhangsan@example.com"
              @blur="handleBlur"
            />
          </a-form-item>
        </a-form>
      </a-card>

      <a-card title="模型配置" class="config-card">
        <a-form layout="vertical" :model="config" class="config-form">
          <a-form-item label="Base URL">
            <a-input
              v-model:value="config.model.base_url"
              placeholder="https://api.example.com"
              @blur="handleBlur"
            />
          </a-form-item>
          <a-form-item label="API Key">
            <a-input-password
              v-model:value="config.model.api_key"
              placeholder="sk-..."
              @blur="handleBlur"
            />
          </a-form-item>
          <a-form-item label="Model">
            <a-input
              v-model:value="config.model.model"
              placeholder="gpt-4o-mini"
              @blur="handleBlur"
            />
          </a-form-item>
        </a-form>
      </a-card>

      <a-card title="周报导出配置" class="config-card">
        <a-form layout="vertical" :model="config" class="config-form">
          <a-form-item label="员工姓名">
            <a-input
              v-model:value="config.report.employee_name"
              placeholder="导出 Word 时会写入员工栏"
              @blur="handleBlur"
            />
          </a-form-item>
          <a-form-item label="默认日工时">
            <a-input-number
              v-model:value="config.report.daily_hours"
              :min="1"
              :max="24"
              style="width: 100%"
              @blur="handleBlur"
            />
          </a-form-item>
          <a-form-item label="默认完成度">
            <a-input
              v-model:value="config.report.default_progress"
              placeholder="例如：100%"
              @blur="handleBlur"
            />
          </a-form-item>
          <a-form-item label="总结备注">
            <a-textarea
              v-model:value="config.report.summary_note"
              :rows="3"
              placeholder="导出周报最后一列备注内容"
              @blur="handleBlur"
            />
          </a-form-item>
        </a-form>
      </a-card>

      <a-card title="提示词配置" class="config-card prompts-card">
        <a-form layout="vertical" :model="config">
          <a-form-item>
            <template #label>
              <span>AI 润色 System Prompt</span>
              <span class="prompt-hint">留空则使用默认提示词</span>
            </template>
            <a-textarea
              v-model:value="config.prompts.polish_system"
              :rows="6"
              placeholder="控制每日工作内容润色规则"
              @blur="handleBlur"
            />
          </a-form-item>
          <a-form-item>
            <template #label>
              <span>AI 润色 Few-shot 示例</span>
              <span class="prompt-hint">留空则使用默认示例</span>
            </template>
            <a-textarea
              v-model:value="config.prompts.polish_few_shot"
              :rows="8"
              placeholder="提供润色输入输出示例"
              @blur="handleBlur"
            />
          </a-form-item>
          <a-form-item>
            <template #label>
              <span>周总结 System Prompt</span>
              <span class="prompt-hint">留空则使用默认提示词</span>
            </template>
            <a-textarea
              v-model:value="config.prompts.summary_system"
              :rows="6"
              placeholder="控制周总结输出风格"
              @blur="handleBlur"
            />
          </a-form-item>
        </a-form>
      </a-card>
    </div>

    <transition name="fade">
      <div v-if="saveStatus !== 'idle'" class="save-indicator" :class="saveStatus">
        <span v-if="saveStatus === 'saving'">保存中...</span>
        <span v-else-if="saveStatus === 'saved'">已保存</span>
        <span v-else>保存失败</span>
      </div>
    </transition>
  </div>
</template>

<style>
.settings-container {
  display: flex;
  flex-direction: column;
  gap: 16px;
  position: relative;
}

.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
  gap: 16px;
}

.config-card {
  width: 100%;
}

.config-form {
  max-width: 520px;
}

.prompts-card {
  grid-column: 1 / -1;
}

.prompt-hint {
  margin-left: 8px;
  font-size: 12px;
  color: var(--text-muted);
  font-weight: normal;
}

.save-indicator {
  position: fixed;
  right: 24px;
  bottom: 24px;
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 13px;
  color: #fff;
  z-index: 1000;
  pointer-events: none;
}

.save-indicator.saving {
  background: #1677ff;
}

.save-indicator.saved {
  background: #52c41a;
}

.save-indicator.error {
  background: #ff4d4f;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
