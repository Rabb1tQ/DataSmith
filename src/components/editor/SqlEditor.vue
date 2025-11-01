<template>
  <div class="sql-editor-container">
    <div class="editor-toolbar">
      <a-space>
        <a-button
          type="primary"
          :icon="h(CaretRightOutlined)"
          @click="executeQuery"
          :loading="executing"
          :disabled="!hasActiveConnection"
        >
          执行 (F5)
        </a-button>
        <a-button
          :icon="h(StopOutlined)"
          @click="stopExecution"
          :disabled="!executing"
        >
          停止
        </a-button>
        <a-divider type="vertical" />
        <a-button :icon="h(FormatPainterOutlined)" @click="formatSql">
          格式化
        </a-button>
        <a-button :icon="h(ClearOutlined)" @click="clearEditor">
          清空
        </a-button>
        <a-divider type="vertical" />
        <a-button :icon="h(HistoryOutlined)" @click="showHistory = true">
          历史
        </a-button>
        <a-button :icon="h(SaveOutlined)" @click="saveQuery">
          保存
        </a-button>
        <a-button :icon="h(CodeOutlined)" @click="showSnippets = true">
          代码片段
        </a-button>
        <a-button :icon="h(ReloadOutlined)" @click="refreshAutocomplete" :loading="refreshingAutocomplete">
          刷新补全
        </a-button>
        <a-divider type="vertical" />
        <a-select
          v-model:value="selectedDatabase"
          placeholder="选择数据库"
          style="width: 150px"
          :disabled="!hasActiveConnection || loadingDatabases"
          :loading="loadingDatabases"
          @change="handleDatabaseChange"
        >
          <a-select-option value="">默认</a-select-option>
          <a-select-option 
            v-for="db in availableDatabases" 
            :key="db.name" 
            :value="db.name"
          >
            {{ db.name }}
          </a-select-option>
        </a-select>
      </a-space>
      <div class="editor-info">
        <a-tag v-if="connectionInfo" color="blue">
          {{ connectionInfo.name }}
        </a-tag>
        <a-tag v-if="selectedDatabase" color="green">
          {{ selectedDatabase }}
        </a-tag>
        <span class="cursor-position">行 {{ cursorLine }}, 列 {{ cursorColumn }}</span>
      </div>
    </div>

    <div ref="editorContainer" class="editor-wrapper"></div>

    <div class="result-tabs">
      <a-tabs v-model:activeKey="resultTabKey">
        <a-tab-pane key="result" tab="结果">
          <div class="result-content">
            <div v-if="queryResults.length > 0" class="result-info">
              <a-space>
                <a-tag color="success">
                  {{ queryResults[currentResultIndex]?.affected_rows || 0 }} 行
                </a-tag>
                <a-tag color="processing">
                  {{ queryResults[currentResultIndex]?.execution_time_ms || 0 }} ms
                </a-tag>
                <a-dropdown v-if="queryResults.length > 1">
                  <a-button size="small">
                    结果集 {{ currentResultIndex + 1 }}/{{ queryResults.length }}
                    <DownOutlined />
                  </a-button>
                  <template #overlay>
                    <a-menu @click="switchResult">
                      <a-menu-item
                        v-for="(result, index) in queryResults"
                        :key="index"
                      >
                        结果集 {{ index + 1 }} ({{ result.affected_rows }} 行)
                      </a-menu-item>
                    </a-menu>
                  </template>
                </a-dropdown>
              </a-space>
            </div>
            <a-table
              v-if="currentResult"
              :columns="resultColumns"
              :data-source="currentResult.rows"
              :scroll="{ x: 'max-content', y: 400 }"
              :pagination="{ pageSize: 100, showSizeChanger: true }"
              size="small"
              bordered
            />
            <a-empty v-else description="暂无查询结果" />
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
      title="SQL 执行历史"
      :width="800"
      :footer="null"
    >
      <a-list :data-source="sqlHistory" size="small">
        <template #renderItem="{ item }">
          <a-list-item>
            <template #actions>
              <a @click="loadFromHistory(item)">加载</a>
              <a @click="removeFromHistory(item)">删除</a>
            </template>
            <a-list-item-meta>
              <template #title>
                <code>{{ item.sql.substring(0, 100) }}...</code>
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

    <!-- 保存查询对话框 -->
    <SaveQueryDialog
      v-model="showSaveDialog"
      :sql="editor?.getValue() || ''"
      @saved="handleQuerySaved"
    />

    <!-- SQL代码片段管理器 -->
    <SqlSnippetsManager
      v-model:visible="showSnippets"
      @insert-snippet="insertSnippet"
    />
  </div>
</template>

<script setup lang="ts">
import { h, onMounted, onUnmounted, watch } from 'vue'
import * as monaco from 'monaco-editor'
import { registerSqlCompletionProvider, type SqlCompletionProvider } from '@/services/sqlAutocomplete'

// 配置 Monaco Editor 环境（禁用 worker 以避免加载问题）
(window as any).MonacoEnvironment = {
  getWorker: () => {
    return new Worker(
      URL.createObjectURL(
        new Blob([''], { type: 'application/javascript' })
      )
    )
  }
}
import {
  CaretRightOutlined,
  StopOutlined,
  FormatPainterOutlined,
  ClearOutlined,
  HistoryOutlined,
  SaveOutlined,
  CodeOutlined,
  DownOutlined,
  ReloadOutlined,
} from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { useConnectionStore } from '@/stores/connection'
import { useAppStore } from '@/stores/app'
import type { QueryResult } from '@/types/database'
import SaveQueryDialog from './SaveQueryDialog.vue'
import SqlSnippetsManager from './SqlSnippetsManager.vue'

const connectionStore = useConnectionStore()
const appStore = useAppStore()

const editorContainer = ref<HTMLElement>()
let editor: monaco.editor.IStandaloneCodeEditor | null = null
let completionProvider: SqlCompletionProvider | null = null

const executing = ref(false)
const queryResults = ref<QueryResult[]>([])
const currentResultIndex = ref(0)
const resultTabKey = ref('result')
const showHistory = ref(false)
const showSaveDialog = ref(false)
const showSnippets = ref(false)
const cursorLine = ref(1)
const cursorColumn = ref(1)
const refreshingAutocomplete = ref(false)

// 数据库选择相关
const selectedDatabase = ref('')
const availableDatabases = ref<any[]>([])
const loadingDatabases = ref(false)

interface Message {
  type: 'success' | 'error' | 'info'
  text: string
  time: string
}

const messages = ref<Message[]>([])

interface SqlHistoryItem {
  sql: string
  timestamp: number
  database?: string
}

const sqlHistory = ref<SqlHistoryItem[]>([])

// 获取当前结果集
const currentResult = computed(() => {
  return queryResults.value[currentResultIndex.value] || null
})

// 生成表格列
const resultColumns = computed(() => {
  if (!currentResult.value) return []
  return currentResult.value.columns.map((col) => ({
    title: col,
    dataIndex: col,
    key: col,
    ellipsis: true,
    width: 150,
  }))
})

// 连接信息
const connectionInfo = computed(() => {
  const activeId = connectionStore.activeConnectionId
  if (!activeId) return null
  return connectionStore.connections.find((c) => c.id === activeId)
})

const hasActiveConnection = computed(() => !!connectionStore.activeConnectionId)

// 加载可用数据库列表
async function loadAvailableDatabases() {
  if (!connectionStore.activeConnectionId) {
    availableDatabases.value = []
    return
  }

  loadingDatabases.value = true
  try {
    const databases = await invoke<any[]>('get_databases', {
      connectionId: connectionStore.activeConnectionId,
    })
    availableDatabases.value = databases
  } catch (error: any) {
    console.error('加载数据库列表失败:', error)
    availableDatabases.value = []
  } finally {
    loadingDatabases.value = false
  }
}

// 处理数据库变化
function handleDatabaseChange(database: any) {
  let dbStr = ''
  if (database && typeof database === 'object' && 'value' in database) {
    // LabeledValue 类型
    dbStr = String(database.value || '')
  } else if (Array.isArray(database)) {
    dbStr = String(database[0] || '')
  } else {
    dbStr = String(database || '')
  }
  selectedDatabase.value = dbStr
  console.log('切换到数据库:', dbStr)
  
  // 更新自动补全提供程序的当前数据库
  if (completionProvider) {
    completionProvider.setCurrentDatabase(dbStr || null)
  }
  
  if (dbStr) {
    message.success(`已切换到数据库: ${dbStr}`)
  }
}

// 初始化编辑器
onMounted(() => {
  if (!editorContainer.value) return

  // 创建编辑器
  editor = monaco.editor.create(editorContainer.value, {
    value: '-- 在此输入 SQL 查询\nSELECT * FROM users LIMIT 10;',
    language: 'sql',
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

  // 注册 SQL 自动补全提供程序
  completionProvider = registerSqlCompletionProvider()
  
  // 如果已经有活动连接，立即设置
  if (connectionStore.activeConnectionId) {
    completionProvider.setConnectionId(connectionStore.activeConnectionId)
    if (selectedDatabase.value) {
      completionProvider.setCurrentDatabase(selectedDatabase.value)
    }
  }

  // 监听光标位置变化
  editor.onDidChangeCursorPosition((e) => {
    cursorLine.value = e.position.lineNumber
    cursorColumn.value = e.position.column
  })

  // 添加快捷键
  editor.addCommand(monaco.KeyCode.F5, () => {
    executeQuery()
  })

  // 加载历史记录
  loadHistory()
  
  // 加载数据库列表
  loadAvailableDatabases()
})

onUnmounted(() => {
  editor?.dispose()
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
  (newConnectionId, oldConnectionId) => {
    // 连接变化时清空结果
    queryResults.value = []
    messages.value = []
    
    // 只有在真正切换连接时才重置数据库选择
    if (newConnectionId !== oldConnectionId) {
      selectedDatabase.value = ''
    }
    
    // 更新自动补全提供程序的连接 ID
    if (completionProvider) {
      completionProvider.setConnectionId(newConnectionId)
    }
    
    // 加载新连接的数据库列表
    if (newConnectionId) {
      loadAvailableDatabases()
    } else {
      availableDatabases.value = []
    }
  }
)

// 执行查询
async function executeQuery() {
  if (!hasActiveConnection.value) {
    message.warning('请先选择一个数据库连接')
    return
  }

  const sql = editor?.getValue().trim()
  if (!sql) {
    message.warning('请输入 SQL 语句')
    return
  }

  executing.value = true
  queryResults.value = []
  currentResultIndex.value = 0
  resultTabKey.value = 'result'

  const databaseInfo = selectedDatabase.value ? ` (数据库: ${selectedDatabase.value})` : ''
  addMessage('info', `执行查询...${databaseInfo}`)
  
  // 调试信息
  console.log('执行查询 - 选中的数据库:', selectedDatabase.value)
  console.log('执行查询 - 传递的database参数:', selectedDatabase.value || null)

  try {
    const result = await invoke<QueryResult>('execute_query', {
      connectionId: connectionStore.activeConnectionId,
      sql,
      database: selectedDatabase.value || null,
    })

    queryResults.value = [result]
    const databaseInfo = selectedDatabase.value ? ` (数据库: ${selectedDatabase.value})` : ''
    addMessage(
      'success',
      `查询成功！影响 ${result.affected_rows} 行，耗时 ${result.execution_time_ms} ms${databaseInfo}`
    )

    // 保存到历史
    saveToHistory(sql)
  } catch (error: any) {
    // 查询失败时清空之前的结果，避免用户误以为还在显示旧数据
    queryResults.value = []
    currentResultIndex.value = 0
    
    const databaseInfo = selectedDatabase.value ? ` (数据库: ${selectedDatabase.value})` : ''
    addMessage('error', `查询失败${databaseInfo}: ${error}`)
    message.error(`查询失败: ${error}`)
  } finally {
    executing.value = false
  }
}

// 停止执行
function stopExecution() {
  // TODO: 实现查询取消
  executing.value = false
  addMessage('info', '已停止执行')
}

// 格式化 SQL
function formatSql() {
  if (!editor) return
  const sql = editor.getValue()
  // 简单格式化（后续可集成专业 SQL 格式化库）
  const formatted = sql
    .replace(/\bSELECT\b/gi, '\nSELECT')
    .replace(/\bFROM\b/gi, '\nFROM')
    .replace(/\bWHERE\b/gi, '\nWHERE')
    .replace(/\bORDER BY\b/gi, '\nORDER BY')
    .replace(/\bGROUP BY\b/gi, '\nGROUP BY')
    .trim()
  
  editor.setValue(formatted)
  message.success('SQL 已格式化')
}

// 清空编辑器
function clearEditor() {
  editor?.setValue('')
  queryResults.value = []
  messages.value = []
}

// 保存查询
function saveQuery() {
  const sql = editor?.getValue()
  if (!sql || !sql.trim()) {
    message.warning('没有可保存的内容')
    return
  }
  showSaveDialog.value = true
}

// 查询保存成功回调
function handleQuerySaved() {
  message.success('查询已保存')
}

// 插入代码片段
function insertSnippet(sql: string) {
  if (!editor) return
  
  const selection = editor.getSelection()
  if (selection) {
    editor.executeEdits('insert-snippet', [{
      range: selection,
      text: sql,
    }])
  } else {
    const position = editor.getPosition()
    if (position) {
      editor.executeEdits('insert-snippet', [{
        range: new monaco.Range(position.lineNumber, position.column, position.lineNumber, position.column),
        text: sql,
      }])
    }
  }
  editor.focus()
}

// 切换结果集
function switchResult({ key }: { key: string | number }) {
  currentResultIndex.value = typeof key === 'number' ? key : parseInt(String(key))
}

// 添加消息
function addMessage(type: Message['type'], text: string) {
  messages.value.unshift({
    type,
    text,
    time: new Date().toLocaleTimeString(),
  })
}

// 保存到历史
function saveToHistory(sql: string) {
  sqlHistory.value.unshift({
    sql,
    timestamp: Date.now(),
    database: connectionInfo.value?.database,
  })
  // 限制历史记录数量
  if (sqlHistory.value.length > 100) {
    sqlHistory.value = sqlHistory.value.slice(0, 100)
  }
  localStorage.setItem('sql_history', JSON.stringify(sqlHistory.value))
}

// 加载历史
function loadHistory() {
  const stored = localStorage.getItem('sql_history')
  if (stored) {
    try {
      sqlHistory.value = JSON.parse(stored)
    } catch (e) {
      console.error('加载历史记录失败', e)
    }
  }
}

// 从历史加载
function loadFromHistory(item: SqlHistoryItem) {
  editor?.setValue(item.sql)
  showHistory.value = false
  message.success('已加载历史记录')
}

// 从历史删除
function removeFromHistory(item: SqlHistoryItem) {
  sqlHistory.value = sqlHistory.value.filter((h) => h.timestamp !== item.timestamp)
  localStorage.setItem('sql_history', JSON.stringify(sqlHistory.value))
}

// 刷新自动补全数据
async function refreshAutocomplete() {
  if (!completionProvider || !connectionStore.activeConnectionId) {
    message.warning('请先连接到数据库')
    return
  }
  
  refreshingAutocomplete.value = true
  try {
    await completionProvider.refresh()
    message.success('自动补全数据已刷新')
  } catch (error: any) {
    message.error(`刷新失败: ${error}`)
  } finally {
    refreshingAutocomplete.value = false
  }
}

// 设置选中的数据库（供外部调用）
async function setSelectedDatabase(database: string) {
  console.log('=== SqlEditor.setSelectedDatabase 被调用 ===')
  console.log('目标数据库名:', database)
  console.log('当前活动连接:', connectionStore.activeConnectionId)
  console.log('当前可用数据库数量:', availableDatabases.value.length)
  
  // 确保数据库列表已加载
  if (availableDatabases.value.length === 0) {
    console.log('数据库列表为空，重新加载...')
    await loadAvailableDatabases()
    console.log('重新加载后的数据库数量:', availableDatabases.value.length)
  }
  
  // 检查数据库是否在可用列表中
  const dbExists = availableDatabases.value.some(db => db.name === database)
  console.log('数据库是否存在于列表中:', dbExists)
  
  if (!dbExists && database) {
    console.warn('数据库不在可用列表中，尝试重新加载:', database)
    // 重新加载数据库列表
    await loadAvailableDatabases()
    const dbExistsAfterReload = availableDatabases.value.some(db => db.name === database)
    console.log('重新加载后数据库是否存在:', dbExistsAfterReload)
    
    if (!dbExistsAfterReload) {
      console.error('重新加载后仍未找到数据库')
      message.warning(`数据库 ${database} 不在可用列表中`)
      return
    }
  }
  
  selectedDatabase.value = database
  
  // 更新自动补全提供程序的当前数据库
  if (completionProvider) {
    completionProvider.setCurrentDatabase(database || null)
  }
  
  console.log('已设置 selectedDatabase.value:', selectedDatabase.value)
  console.log('可用数据库列表:', availableDatabases.value.map(db => db.name))
  console.log('=== 数据库设置完成 ===')
}

// 暴露方法供父组件调用
defineExpose({
  setSelectedDatabase,
})
</script>

<style scoped>
.sql-editor-container {
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
  line-height: 1;
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

.result-tabs :deep(.ant-tabs-nav) {
  padding-left: 12px;
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

.message-time {
  color: #8c8c8c;
  margin-right: 8px;
}

.message-text {
  font-family: monospace;
}
</style>

