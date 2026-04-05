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
        <a-button
          @click="toggleWordWrap"
          :type="wordWrapEnabled ? 'primary' : 'default'"
        >
          {{ wordWrapEnabled ? '换行:开' : '换行:关' }}
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
      </div>
    </div>

    <div
      class="editor-section"
      :style="{ height: editorHeight + 'px' }"
    >
      <div ref="editorContainer" class="editor-wrapper" @contextmenu="handleEditorContextMenu"></div>
    </div>

    <!-- 拖拽分隔条 -->
    <div
      class="editor-resizer"
      @mousedown="startEditorResize"
    ></div>

    <!-- 编辑器右键菜单 -->
    <div
      v-if="editorMenuVisible"
      class="editor-context-menu-overlay"
    >
      <div
        class="editor-context-menu"
        :style="{ left: editorMenuX + 'px', top: editorMenuY + 'px' }"
        @click.stop
      >
        <a-menu @click="handleEditorMenuClick">
          <!-- 执行类 -->
          <a-menu-item key="execute-selection" v-if="hasSelection">
            <CaretRightOutlined />
            执行选中语句
            <span class="menu-shortcut">Ctrl+Enter</span>
          </a-menu-item>
          <a-menu-item key="execute-all">
            <CaretRightOutlined />
            执行全部
            <span class="menu-shortcut">F5</span>
          </a-menu-item>
          <a-menu-divider />
          <!-- 编辑类 -->
          <a-menu-item key="cut" :disabled="!hasSelection">
            <ScissorOutlined />
            剪切
            <span class="menu-shortcut">Ctrl+X</span>
          </a-menu-item>
          <a-menu-item key="copy" :disabled="!hasSelection">
            <CopyOutlined />
            复制
            <span class="menu-shortcut">Ctrl+C</span>
          </a-menu-item>
          <a-menu-item key="paste">
            <SnippetsOutlined />
            粘贴
            <span class="menu-shortcut">Ctrl+V</span>
          </a-menu-item>
          <a-menu-item key="select-all">
            <SelectOutlined />
            全选
            <span class="menu-shortcut">Ctrl+A</span>
          </a-menu-item>
          <a-menu-divider />
          <!-- 转换类 -->
          <a-menu-item key="toggle-comment">
            <MessageOutlined />
            注释/取消注释
            <span class="menu-shortcut">Ctrl+/</span>
          </a-menu-item>
          <a-menu-item key="uppercase" :disabled="!hasSelection">
            <FontSizeOutlined />
            转为大写
          </a-menu-item>
          <a-menu-item key="lowercase" :disabled="!hasSelection">
            <FontSizeOutlined />
            转为小写
          </a-menu-item>
          <a-menu-divider />
          <!-- 工具类 -->
          <a-menu-item key="format">
            <FormatPainterOutlined />
            格式化 SQL
          </a-menu-item>
          <a-menu-item key="snippets">
            <CodeOutlined />
            代码片段
          </a-menu-item>
          <a-menu-divider />
          <a-menu-item key="save">
            <SaveOutlined />
            保存查询
          </a-menu-item>
          <a-menu-item key="clear">
            <ClearOutlined />
            清空编辑器
          </a-menu-item>
        </a-menu>
      </div>
    </div>

    <div class="result-tabs">
      <a-tabs v-model:activeKey="resultTabKey">
        <a-tab-pane key="result" tab="结果">
          <div class="result-content">
            <!-- 批量执行结果摘要 -->
            <div v-if="showBatchResult && batchResult" class="batch-result-summary">
              <a-alert :type="batchResult.failed_count === 0 ? 'success' : 'warning'" show-icon>
                <template #message>
                  <a-space>
                    <span>执行完成</span>
                    <a-tag color="success">{{ batchResult.success_count }} 成功</a-tag>
                    <a-tag v-if="batchResult.failed_count > 0" color="error">{{ batchResult.failed_count }} 失败</a-tag>
                    <a-tag color="blue">{{ batchResult.total_affected_rows }} 行受影响</a-tag>
                    <a-tag color="default">{{ batchResult.total_time_ms }} ms</a-tag>
                  </a-space>
                </template>
              </a-alert>
            </div>
            
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
        <a-tab-pane v-if="showBatchResult && batchResult" key="script" tab="执行详情">
          <div class="script-result-content">
            <a-table
              :columns="[
                { title: '#', dataIndex: 'index', width: 50 },
                { title: 'SQL', dataIndex: 'sql', ellipsis: true },
                { title: '状态', dataIndex: 'status', width: 80 },
                { title: '影响行数', dataIndex: 'affected_rows', width: 100 },
                { title: '耗时(ms)', dataIndex: 'execution_time_ms', width: 100 },
                { title: '错误信息', dataIndex: 'error', ellipsis: true },
              ]"
              :data-source="batchResult.statements.map((s, i) => ({
                key: i,
                index: i + 1,
                sql: s.sql,
                status: s.success ? '成功' : '失败',
                affected_rows: s.affected_rows,
                execution_time_ms: s.execution_time_ms,
                error: s.error || '-',
              }))"
              :scroll="{ x: 'max-content' }"
              size="small"
              bordered
              :row-class-name="(record: any) => record.status === '失败' ? 'row-error' : ''"
            >
              <template #bodyCell="{ column, record }">
                <template v-if="column.dataIndex === 'status'">
                  <a-tag :color="record.status === '成功' ? 'success' : 'error'">
                    {{ record.status }}
                  </a-tag>
                </template>
                <template v-else-if="column.dataIndex === 'error'">
                  <span :class="{ 'error-text': record.error !== '-' }">{{ record.error }}</span>
                </template>
              </template>
            </a-table>
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
import { h, onMounted, onUnmounted, watch, ref, computed } from 'vue'
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
  ScissorOutlined,
  CopyOutlined,
  SnippetsOutlined,
  SelectOutlined,
  MessageOutlined,
  FontSizeOutlined,
} from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { useConnectionStore } from '@/stores/connection'
import { useAppStore } from '@/stores/app'
import type { QueryResult, BatchQueryResult } from '@/types/database'
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
const wordWrapEnabled = ref(false)

// 批量执行结果
const batchResult = ref<BatchQueryResult | null>(null)
const showBatchResult = ref(false)

// 编辑器和结果面板高度调整
const editorHeight = ref(300) // 默认编辑器高度 300px
const isEditorResizing = ref(false)

// 编辑器右键菜单
const editorMenuVisible = ref(false)
const editorMenuX = ref(0)
const editorMenuY = ref(0)
const hasSelection = ref(false)
const refreshingAutocomplete = ref(false)

// 数据库选择相关
const selectedDatabase = ref('')
const availableDatabases = ref<any[]>([])
const loadingDatabases = ref(false)

interface Message {
  type: 'success' | 'error' | 'info' | 'warning'
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
    contextmenu: false,
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
  batchResult.value = null
  showBatchResult.value = false

  const databaseInfo = selectedDatabase.value ? ` (数据库: ${selectedDatabase.value})` : ''
  addMessage('info', `执行SQL脚本...${databaseInfo}`)
  
  // 调试信息
  console.log('执行查询 - 选中的数据库:', selectedDatabase.value)
  console.log('执行查询 - 传递的database参数:', selectedDatabase.value || null)

  try {
    // 使用新的批量执行命令，支持DELIMITER语法
    const result = await invoke<BatchQueryResult>('execute_sql_script', {
      connectionId: connectionStore.activeConnectionId,
      sql,
      database: selectedDatabase.value || null,
    })

    batchResult.value = result
    showBatchResult.value = true

    // 提取所有查询结果用于显示
    const queryStmts = result.statements.filter(s => s.is_query && s.success)
    if (queryStmts.length > 0) {
      queryResults.value = queryStmts.map(s => ({
        columns: s.columns,
        rows: s.rows,
        affected_rows: s.affected_rows,
        execution_time_ms: s.execution_time_ms,
      }))
    }

    const dbInfo = selectedDatabase.value ? ` (数据库: ${selectedDatabase.value})` : ''
    if (result.failed_count === 0) {
      addMessage(
        'success',
        `执行完成！成功 ${result.success_count} 条，影响 ${result.total_affected_rows} 行，耗时 ${result.total_time_ms} ms${dbInfo}`
      )
    } else {
      addMessage(
        'warning',
        `执行完成！成功 ${result.success_count} 条，失败 ${result.failed_count} 条，耗时 ${result.total_time_ms} ms${dbInfo}`
      )
    }

    // 保存到历史
    saveToHistory(sql)
  } catch (error: any) {
    // 查询失败时清空之前的结果，避免用户误以为还在显示旧数据
    queryResults.value = []
    currentResultIndex.value = 0
    batchResult.value = null
    
    const dbInfo = selectedDatabase.value ? ` (数据库: ${selectedDatabase.value})` : ''
    addMessage('error', `执行失败${dbInfo}: ${error}`)
    message.error(`执行失败: ${error}`)
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

// 切换自动换行
function toggleWordWrap() {
  if (!editor) return
  wordWrapEnabled.value = !wordWrapEnabled.value
  editor.updateOptions({ wordWrap: wordWrapEnabled.value ? 'on' : 'off' })
  message.success(wordWrapEnabled.value ? '已开启自动换行' : '已关闭自动换行')
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

// 编辑器右键菜单处理
function handleEditorContextMenu(e: MouseEvent) {
  e.preventDefault()
  e.stopPropagation()

  if (!editor) return

  const selection = editor.getSelection()
  hasSelection.value = !!(selection && !selection.isEmpty())

  editorMenuX.value = e.clientX
  editorMenuY.value = e.clientY
  editorMenuVisible.value = true

  nextTick(() => {
    const menuElement = document.querySelector('.editor-context-menu') as HTMLElement
    if (!menuElement) return

    const menuRect = menuElement.getBoundingClientRect()
    const windowWidth = window.innerWidth
    const windowHeight = window.innerHeight
    const padding = 10

    let x = e.clientX
    let y = e.clientY

    if (x + menuRect.width > windowWidth - padding) {
      x = windowWidth - menuRect.width - padding
    }
    if (y + menuRect.height > windowHeight - padding) {
      y = windowHeight - menuRect.height - padding
    }
    x = Math.max(padding, x)
    y = Math.max(padding, y)

    editorMenuX.value = x
    editorMenuY.value = y
  })
}

// 关闭编辑器右键菜单
function closeEditorMenu() {
  editorMenuVisible.value = false
}

// 执行选中语句
async function executeSelection() {
  if (!editor) return
  const selection = editor.getSelection()
  if (!selection || selection.isEmpty()) return

  const sql = editor.getModel()?.getValueInRange(selection)?.trim()
  if (!sql) return

  if (!hasActiveConnection.value) {
    message.warning('请先选择一个数据库连接')
    return
  }

  executing.value = true
  queryResults.value = []
  currentResultIndex.value = 0
  resultTabKey.value = 'result'

  addMessage('info', `执行选中语句...`)

  try {
    const result = await invoke<QueryResult>('execute_query', {
      connectionId: connectionStore.activeConnectionId,
      sql,
      database: selectedDatabase.value || null,
    })

    queryResults.value = [result]
    addMessage(
      'success',
      `查询成功！影响 ${result.affected_rows} 行，耗时 ${result.execution_time_ms} ms`
    )
    saveToHistory(sql)
  } catch (error: any) {
    queryResults.value = []
    currentResultIndex.value = 0
    addMessage('error', `查询失败: ${error}`)
    message.error(`查询失败: ${error}`)
  } finally {
    executing.value = false
  }
}

// 注释/取消注释
function toggleComment() {
  if (!editor) return

  const selection = editor.getSelection()
  const model = editor.getModel()
  if (!selection || !model) return

  const startLine = selection.startLineNumber
  const endLine = selection.endLineNumber

  // 检查选中区域是否所有行都已注释
  let allCommented = true
  for (let line = startLine; line <= endLine; line++) {
    const lineContent = model.getLineContent(line)
    if (!lineContent.trimStart().startsWith('--')) {
      allCommented = false
      break
    }
  }

  const edits: monaco.editor.IIdentifiedSingleEditOperation[] = []

  for (let line = startLine; line <= endLine; line++) {
    const lineContent = model.getLineContent(line)
    if (allCommented) {
      // 取消注释：移除行首的 --
      const trimmed = lineContent.trimStart()
      if (trimmed.startsWith('--')) {
        const commentIndex = lineContent.indexOf('--')
        edits.push({
          range: new monaco.Range(line, commentIndex + 1, line, commentIndex + 3),
          text: '',
        })
      }
    } else {
      // 添加注释：在行首添加 --
      edits.push({
        range: new monaco.Range(line, 1, line, 1),
        text: '-- ',
      })
    }
  }

  editor.executeEdits('toggle-comment', edits)
  editor.focus()
}

// 转大写
function convertToUppercase() {
  if (!editor) return
  const selection = editor.getSelection()
  const model = editor.getModel()
  if (!selection || !model || selection.isEmpty()) return

  const text = model.getValueInRange(selection)
  editor.executeEdits('uppercase', [{
    range: selection,
    text: text.toUpperCase(),
  }])
  editor.focus()
}

// 转小写
function convertToLowercase() {
  if (!editor) return
  const selection = editor.getSelection()
  const model = editor.getModel()
  if (!selection || !model || selection.isEmpty()) return

  const text = model.getValueInRange(selection)
  editor.executeEdits('lowercase', [{
    range: selection,
    text: text.toLowerCase(),
  }])
  editor.focus()
}

// 处理编辑器右键菜单点击
function handleEditorMenuClick({ key }: { key: string | number }) {
  closeEditorMenu()

  const keyStr = String(key)
  switch (keyStr) {
    case 'execute-selection':
      executeSelection()
      break
    case 'execute-all':
      executeQuery()
      break
    case 'cut':
      editor?.trigger('contextMenu', 'editor.action.clipboardCutAction', null)
      break
    case 'copy':
      editor?.trigger('contextMenu', 'editor.action.clipboardCopyAction', null)
      break
    case 'paste':
      editor?.trigger('contextMenu', 'editor.action.clipboardPasteAction', null)
      break
    case 'select-all':
      editor?.trigger('contextMenu', 'editor.action.selectAll', null)
      break
    case 'toggle-comment':
      toggleComment()
      break
    case 'uppercase':
      convertToUppercase()
      break
    case 'lowercase':
      convertToLowercase()
      break
    case 'format':
      formatSql()
      break
    case 'snippets':
      showSnippets.value = true
      break
    case 'save':
      saveQuery()
      break
    case 'clear':
      clearEditor()
      break
  }
}

// 注册全局事件关闭编辑器右键菜单
onMounted(() => {
  document.addEventListener('click', closeEditorMenu)
  document.addEventListener('contextmenu', closeEditorMenu)
})

onUnmounted(() => {
  document.removeEventListener('click', closeEditorMenu)
  document.removeEventListener('contextmenu', closeEditorMenu)
})

// 开始拖拽调整编辑器和结果面板高度
function startEditorResize(e: MouseEvent) {
  isEditorResizing.value = true
  const startY = e.clientY
  const startHeight = editorHeight.value

  const doResize = (e: MouseEvent) => {
    if (!isEditorResizing.value) return
    
    const delta = e.clientY - startY
    const newHeight = startHeight + delta
    
    // 限制最小高度 100px，最大高度 600px
    if (newHeight >= 100 && newHeight <= 600) {
      editorHeight.value = newHeight
    }
  }

  const stopResize = () => {
    isEditorResizing.value = false
    document.removeEventListener('mousemove', doResize)
    document.removeEventListener('mouseup', stopResize)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
  }

  document.body.style.cursor = 'row-resize'
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', doResize)
  document.addEventListener('mouseup', stopResize)
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

.editor-section {
  flex-shrink: 0;
  min-height: 100px;
  display: flex;
  flex-direction: column;
}

.editor-wrapper {
  flex: 1;
  min-height: 100px;
}

.dark-mode .editor-wrapper {
  border-bottom-color: #303030;
}

.editor-resizer {
  height: 4px;
  cursor: row-resize;
  background: #e8e8e8;
  flex-shrink: 0;
  position: relative;
  transition: background-color 0.2s;
}

.editor-resizer:hover {
  background: #1890ff;
}

.dark-mode .editor-resizer {
  background: #303030;
}

.dark-mode .editor-resizer:hover {
  background: #177ddc;
}

.editor-resizer::before {
  content: '';
  position: absolute;
  left: 0;
  right: 0;
  top: -2px;
  bottom: -2px;
}

.result-tabs {
  flex: 1;
  min-height: 100px;
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

/* 编辑器右键菜单样式 */
.editor-context-menu-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 9999;
  background: transparent;
  pointer-events: none;
}

.editor-context-menu {
  position: absolute;
  background: #fff;
  border-radius: 6px;
  box-shadow: 0 6px 16px 0 rgba(0, 0, 0, 0.08), 0 3px 6px -4px rgba(0, 0, 0, 0.12), 0 9px 28px 8px rgba(0, 0, 0, 0.05);
  z-index: 10000;
  min-width: 220px;
  pointer-events: auto;
}

.dark-mode .editor-context-menu {
  background: #1f1f1f;
  border: 1px solid #303030;
}

.menu-shortcut {
  float: right;
  color: #8c8c8c;
  font-size: 12px;
  margin-left: 40px;
}

/* 批量执行结果样式 */
.batch-result-summary {
  margin-bottom: 12px;
}

.script-result-content {
  padding: 12px;
}

.row-error {
  background-color: #fff2f0;
}

.dark-mode .row-error {
  background-color: #2c1618;
}

.error-text {
  color: #ff4d4f;
}
</style>

