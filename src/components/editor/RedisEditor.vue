<template>
  <div class="redis-editor-container">
    <!-- 工具栏 -->
    <div class="editor-toolbar">
      <a-space>
        <a-button
          type="primary"
          :icon="h(CaretRightOutlined)"
          @click="executeCommand"
          :loading="executing"
          :disabled="!hasActiveConnection"
        >
          执行 (Ctrl+Enter)
        </a-button>
        <a-button
          :icon="h(ClearOutlined)"
          @click="clearEditor"
        >
          清空
        </a-button>
        <a-divider type="vertical" />
        <a-button :icon="h(HistoryOutlined)" @click="showHistory = true">
          历史
        </a-button>
        <a-button :icon="h(InfoCircleOutlined)" @click="showInfo = true">
          服务器信息
        </a-button>
        <a-divider type="vertical" />
        <a-select
          v-model:value="selectedDatabase"
          placeholder="选择数据库"
          style="width: 150px"
          :disabled="!hasActiveConnection"
          @change="handleDatabaseChange"
        >
          <a-select-option
            v-for="i in 16"
            :key="i - 1"
            :value="`db${i - 1}`"
          >
            db{{ i - 1 }}
          </a-select-option>
        </a-select>
      </a-space>
      <div class="editor-info">
        <a-tag v-if="connectionInfo" color="red">
          <DatabaseOutlined /> {{ connectionInfo.name }}
        </a-tag>
        <a-tag v-if="selectedDatabase" color="orange">
          {{ selectedDatabase }}
        </a-tag>
        <span class="cursor-position">行 {{ cursorLine }}, 列 {{ cursorColumn }}</span>
      </div>
    </div>

    <!-- 命令输入编辑器 -->
    <div ref="editorContainer" class="editor-wrapper"></div>

    <!-- 结果标签页 -->
    <div class="result-tabs">
      <a-tabs style="margin-left: 12px;" v-model:activeKey="resultTabKey">
        <a-tab-pane  key="result" tab="结果">
          <div class="result-content">
            <div v-if="commandResults.length > 0" class="result-info">
              <a-space>
                <a-tag color="success">
                  执行成功
                </a-tag>
                <a-tag color="processing">
                  {{ commandResults[commandResults.length - 1]?.execution_time_ms || 0 }} ms
                </a-tag>
              </a-space>
            </div>
            <div v-if="lastResult" class="result-display">
              <a-alert
                v-if="lastResult.error"
                type="error"
                :message="lastResult.error"
                show-icon
              />
              <div v-else class="result-content-wrapper">
                <!-- 字符串结果 - 保留换行符 -->
                <pre 
                  v-if="typeof lastResult.result === 'string'" 
                  class="result-text"
                  v-html="formatResult(lastResult.result)"
                ></pre>
                <!-- JSON 结果 -->
                <pre v-else class="result-json">{{ formatResult(lastResult.result) }}</pre>
              </div>
            </div>
            <a-empty v-else description="暂无执行结果" />
          </div>
        </a-tab-pane>
        <a-tab-pane key="messages" tab="消息">
          <div class="messages-content">
            <a-timeline>
              <a-timeline-item
                v-for="(msg, index) in messages"
                :key="index"
                :color="msg.type === 'success' ? 'green' : msg.type === 'error' ? 'red' : 'blue'"
              >
                <span class="message-time">{{ msg.time }}</span>
                <span class="message-text">{{ msg.text }}</span>
              </a-timeline-item>
            </a-timeline>
          </div>
        </a-tab-pane>
      </a-tabs>
    </div>

    <!-- 历史记录对话框 -->
    <a-modal
      v-model:open="showHistory"
      title="Redis 命令历史"
      :width="800"
      :footer="null"
    >
      <a-list :data-source="commandHistory" size="small">
        <template #renderItem="{ item }">
          <a-list-item>
            <template #actions>
              <a @click="loadFromHistory(item)">加载</a>
              <a @click="removeFromHistory(item)">删除</a>
            </template>
            <a-list-item-meta>
              <template #title>
                <code>{{ item.command }}</code>
              </template>
              <template #description>
                {{ new Date(item.timestamp).toLocaleString() }} • 
                {{ item.database || '默认' }}
              </template>
            </a-list-item-meta>
          </a-list-item>
        </template>
      </a-list>
    </a-modal>

    <!-- 服务器信息对话框 -->
    <a-modal
      v-model:open="showInfo"
      title="Redis 服务器信息"
      :width="900"
      :footer="null"
    >
      <a-spin :spinning="loadingInfo">
        <a-descriptions bordered size="small" :column="2">
          <a-descriptions-item label="版本" v-if="serverInfo.redis_version">
            {{ serverInfo.redis_version }}
          </a-descriptions-item>
          <a-descriptions-item label="模式" v-if="serverInfo.redis_mode">
            {{ serverInfo.redis_mode }}
          </a-descriptions-item>
          <a-descriptions-item label="已用内存" v-if="serverInfo.used_memory_human">
            {{ serverInfo.used_memory_human }}
          </a-descriptions-item>
          <a-descriptions-item label="内存峰值" v-if="serverInfo.used_memory_peak_human">
            {{ serverInfo.used_memory_peak_human }}
          </a-descriptions-item>
          <a-descriptions-item label="已连接客户端" v-if="serverInfo.connected_clients">
            {{ serverInfo.connected_clients }}
          </a-descriptions-item>
          <a-descriptions-item label="运行时间(天)" v-if="serverInfo.uptime_in_days">
            {{ serverInfo.uptime_in_days }}
          </a-descriptions-item>
          <a-descriptions-item label="总命令数" v-if="serverInfo.total_commands_processed">
            {{ serverInfo.total_commands_processed }}
          </a-descriptions-item>
          <a-descriptions-item label="键总数" v-if="serverInfo.db0">
            {{ serverInfo.db0 }}
          </a-descriptions-item>
        </a-descriptions>
        
        <a-divider orientation="left">所有信息</a-divider>
        <pre class="server-info-detail">{{ JSON.stringify(serverInfo, null, 2) }}</pre>
      </a-spin>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { h, onMounted, onUnmounted, watch, ref, computed } from 'vue'
import * as monaco from 'monaco-editor'
import { registerRedisCompletionProvider } from '@/services/redisAutocomplete'
import {
  CaretRightOutlined,
  ClearOutlined,
  HistoryOutlined,
  InfoCircleOutlined,
  DatabaseOutlined,
} from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { useConnectionStore } from '@/stores/connection'
import { useAppStore } from '@/stores/app'

const connectionStore = useConnectionStore()
const appStore = useAppStore()

const editorContainer = ref<HTMLElement>()
let editor: monaco.editor.IStandaloneCodeEditor | null = null

const executing = ref(false)
const commandResults = ref<any[]>([])
const resultTabKey = ref('result')
const showHistory = ref(false)
const showInfo = ref(false)
const cursorLine = ref(1)
const cursorColumn = ref(1)
const selectedDatabase = ref('db0')
const loadingInfo = ref(false)
const serverInfo = ref<Record<string, any>>({})
let keepAliveTimer: number | null = null

interface Message {
  type: 'success' | 'error' | 'info'
  text: string
  time: string
}

const messages = ref<Message[]>([])

interface CommandHistoryItem {
  command: string
  timestamp: number
  database?: string
}

const commandHistory = ref<CommandHistoryItem[]>([])

// 最后的执行结果
const lastResult = computed(() => {
  return commandResults.value.length > 0
    ? commandResults.value[commandResults.value.length - 1]
    : null
})

// 连接信息
const connectionInfo = computed(() => {
  const activeId = connectionStore.activeConnectionId
  if (!activeId) return null
  return connectionStore.connections.find((c) => c.id === activeId)
})

const hasActiveConnection = computed(() => !!connectionStore.activeConnectionId)

// 格式化结果显示
function formatResult(result: any): string {
  if (result === null || result === undefined) {
    return 'null'
  }
  
  // 如果是字符串，处理换行符并进行 HTML 转义
  if (typeof result === 'string') {
    // HTML 转义防止 XSS
    const escaped = result
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;')
    
    // 处理 Redis 的换行符（\r\n 和 \n）
    return escaped
      .replace(/\\r\\n/g, '\n')
      .replace(/\\n/g, '\n')
      .replace(/\\r/g, '\n')
  }
  
  // 其他类型使用 JSON 格式化
  return JSON.stringify(result, null, 2)
}

// 初始化编辑器
onMounted(() => {
  if (!editorContainer.value) return

  // 创建编辑器
  editor = monaco.editor.create(editorContainer.value, {
    value: '# 在此输入 Redis 命令\n# PING - 测试连接是否正常\n# INFO - 查看服务器信息\n# GET key - 获取键值\n# SET key value - 设置键值\n\nPING',
    language: 'shell',
    theme: 'vs',
    automaticLayout: true,
    fontSize: 14,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    lineNumbers: 'on',
    renderLineHighlight: 'all',
    quickSuggestions: {
      other: true,
      comments: false,
      strings: false
    },
    suggestOnTriggerCharacters: true,
    acceptSuggestionOnCommitCharacter: true,
    acceptSuggestionOnEnter: 'on',
    tabCompletion: 'on',
  })

  // 注册 Redis 自动补全提供程序
  registerRedisCompletionProvider()

  // 监听光标位置变化
  editor.onDidChangeCursorPosition((e) => {
    cursorLine.value = e.position.lineNumber
    cursorColumn.value = e.position.column
  })

  // 添加快捷键 Ctrl+Enter 执行命令
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
    executeCommand()
  })

  // 加载历史记录
  loadHistory()
  
  // 如果已有活动连接，启动保活
  if (connectionStore.activeConnectionId) {
    startKeepAlive()
  }
})

onUnmounted(() => {
  editor?.dispose()
  stopKeepAlive()
})

// 监听主题变化
watch(
  () => appStore.theme,
  (newTheme) => {
    if (editor) {
      monaco.editor.setTheme(newTheme === 'dark' ? 'vs-dark' : 'vs')
    }
  }
)

// 监听连接变化
watch(
  () => connectionStore.activeConnectionId,
  (newId) => {
    commandResults.value = []
    messages.value = []
    
    // 重启保活定时器
    if (newId) {
      startKeepAlive()
    } else {
      stopKeepAlive()
    }
  }
)

// 执行 Redis 命令
async function executeCommand() {
  if (!hasActiveConnection.value) {
    message.warning('请先选择一个 Redis 连接')
    return
  }

  const command = editor?.getValue().trim()
  if (!command) {
    message.warning('请输入 Redis 命令')
    return
  }

  executing.value = true
  resultTabKey.value = 'result'

  const dbInfo = selectedDatabase.value ? ` (${selectedDatabase.value})` : ''
  addMessage('info', `执行命令...${dbInfo}`)

  try {
    const result = await invoke<any>('execute_redis_command', {
      connectionId: connectionStore.activeConnectionId,
      command,
    })

    commandResults.value.push(result)

    if (result.success) {
      addMessage('success', `命令执行成功！耗时 ${result.execution_time_ms} ms${dbInfo}`)
      
      // 保存到历史
      saveToHistory(command)
    } else {
      addMessage('error', `命令执行失败${dbInfo}: ${result.error}`)
      message.error(`执行失败: ${result.error}`)
    }
  } catch (error: any) {
    addMessage('error', `命令执行失败${dbInfo}: ${error}`)
    message.error(`执行失败: ${error}`)
  } finally {
    executing.value = false
  }
}

// 清空编辑器
function clearEditor() {
  editor?.setValue('')
  commandResults.value = []
  messages.value = []
}

// 切换数据库
async function handleDatabaseChange(database: any) {
  const dbStr = String(database || 'db0')
  selectedDatabase.value = dbStr
  
  // 提取数据库编号
  const dbNum = dbStr.replace('db', '')
  
  try {
    await invoke('execute_redis_command', {
      connectionId: connectionStore.activeConnectionId,
      command: `SELECT ${dbNum}`,
    })
    message.success(`已切换到 ${dbStr}`)
  } catch (error: any) {
    message.error(`切换数据库失败: ${error}`)
  }
}

// 加载服务器信息
async function loadServerInfo() {
  if (!connectionStore.activeConnectionId) return
  
  loadingInfo.value = true
  try {
    const info = await invoke<Record<string, string>>('get_redis_info', {
      connectionId: connectionStore.activeConnectionId,
    })
    serverInfo.value = info
  } catch (error: any) {
    message.error(`获取服务器信息失败: ${error}`)
  } finally {
    loadingInfo.value = false
  }
}

// 监听显示服务器信息
watch(showInfo, (visible) => {
  if (visible) {
    loadServerInfo()
  }
})

// 添加消息
function addMessage(type: Message['type'], text: string) {
  messages.value.unshift({
    type,
    text,
    time: new Date().toLocaleTimeString(),
  })
}

// 保存到历史
function saveToHistory(command: string) {
  commandHistory.value.unshift({
    command,
    timestamp: Date.now(),
    database: selectedDatabase.value,
  })
  // 限制历史记录数量
  if (commandHistory.value.length > 100) {
    commandHistory.value = commandHistory.value.slice(0, 100)
  }
  localStorage.setItem('redis_command_history', JSON.stringify(commandHistory.value))
}

// 加载历史
function loadHistory() {
  const stored = localStorage.getItem('redis_command_history')
  if (stored) {
    try {
      commandHistory.value = JSON.parse(stored)
    } catch (e) {
      console.error('加载历史记录失败', e)
    }
  }
}

// 从历史加载
function loadFromHistory(item: CommandHistoryItem) {
  editor?.setValue(item.command)
  showHistory.value = false
  message.success('已加载历史记录')
}

// 从历史删除
function removeFromHistory(item: CommandHistoryItem) {
  commandHistory.value = commandHistory.value.filter((h) => h.timestamp !== item.timestamp)
  localStorage.setItem('redis_command_history', JSON.stringify(commandHistory.value))
}

// 切换数据库（供外部调用）
async function switchDatabase(dbName: string) {
  selectedDatabase.value = dbName
  const dbNum = dbName.replace('db', '')
  
  try {
    await invoke('execute_redis_command', {
      connectionId: connectionStore.activeConnectionId,
      command: `SELECT ${dbNum}`,
    })
  } catch (error: any) {
    console.error('切换数据库失败:', error)
    throw error
  }
}

// 启动保活定时器（每30秒发送一次PING）
function startKeepAlive() {
  stopKeepAlive() // 先停止旧的定时器
  
  keepAliveTimer = window.setInterval(async () => {
    if (!connectionStore.activeConnectionId) {
      stopKeepAlive()
      return
    }
    
    try {
      // 静默发送 PING 命令保持连接
      await invoke('execute_redis_command', {
        connectionId: connectionStore.activeConnectionId,
        command: 'PING',
      })
      console.log('Redis 保活: PING 成功')
    } catch (error) {
      console.error('Redis 保活失败:', error)
      // 保活失败，提示用户
      message.warning('Redis 连接可能已断开，请尝试重新连接')
      stopKeepAlive()
    }
  }, 30000) // 30秒
}

// 停止保活定时器
function stopKeepAlive() {
  if (keepAliveTimer !== null) {
    clearInterval(keepAliveTimer)
    keepAliveTimer = null
  }
}

// 暴露方法供父组件调用
defineExpose({
  switchDatabase,
})
</script>

<style scoped>
.redis-editor-container {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.editor-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  border-bottom: 1px solid #e8e8e8;
  background: #fafafa;
}

.dark-mode .editor-toolbar {
  background: #1f1f1f;
  border-bottom-color: #303030;
}

.editor-info {
  display: flex;
  gap: 12px;
  align-items: center;
}

.cursor-position {
  font-size: 12px;
  color: #8c8c8c;
}

.editor-wrapper {
  flex: 1;
  min-height: 300px;
  border-bottom: 1px solid #e8e8e8;
}

.dark-mode .editor-wrapper {
  border-bottom-color: #303030;
}

.result-tabs {
  height: 450px;
  overflow: hidden;
}

.result-tabs :deep(.ant-tabs-content) {
  height: calc(100% - 46px);
}

.result-content,
.messages-content {
  height: 100%;
  overflow: auto;
  padding: 12px;
}

.result-info {
  margin-bottom: 12px;
}

.result-display {
  background: #f5f5f5;
  padding: 12px;
  border-radius: 4px;
}

.dark-mode .result-display {
  background: #1a1a1a;
}

.result-content-wrapper {
  max-height: 500px;
  overflow: auto;
}

.result-json,
.result-text {
  margin: 0;
  font-family: 'Courier New', monospace;
  font-size: 13px;
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.6;
}

.result-text {
  color: #2c3e50;
}

.dark-mode .result-text {
  color: #e0e0e0;
}

.message-time {
  color: #8c8c8c;
  margin-right: 8px;
}

.message-text {
  font-family: monospace;
}

.server-info-detail {
  background: #f5f5f5;
  padding: 12px;
  border-radius: 4px;
  font-size: 12px;
  max-height: 400px;
  overflow: auto;
}

.dark-mode .server-info-detail {
  background: #1a1a1a;
}
</style>

