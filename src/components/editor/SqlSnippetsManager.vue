<template>
  <a-modal
    :open="visible"
    title="SQL代码片段"
    :width="900"
    @cancel="handleCancel"
    :footer="null"
  >
    <div class="snippets-manager">
      <div class="snippets-toolbar">
        <a-space>
          <a-button :icon="h(PlusOutlined)" @click="handleAddSnippet" type="primary">
            新建片段
          </a-button>
          <a-input-search
            v-model:value="searchText"
            placeholder="搜索片段..."
            style="width: 200px"
          />
        </a-space>
      </div>

      <div class="snippets-content">
        <div class="snippets-list">
          <a-list
            :data-source="filteredSnippets"
            size="small"
          >
            <template #renderItem="{ item }">
              <a-list-item
                :class="{ active: selectedSnippet?.id === item.id }"
                @click="selectSnippet(item)"
                class="snippet-item"
              >
                <a-list-item-meta>
                  <template #title>
                    <div class="snippet-title">
                      {{ item.title }}
                      <a-tag v-if="item.category" size="small" color="blue">
                        {{ item.category }}
                      </a-tag>
                    </div>
                  </template>
                  <template #description>
                    {{ item.description || '无描述' }}
                  </template>
                </a-list-item-meta>
                <template #actions>
                  <a-button
                    type="text"
                    size="small"
                    :icon="h(CopyOutlined)"
                    @click.stop="copySnippet(item)"
                    title="复制"
                  />
                  <a-button
                    type="text"
                    size="small"
                    danger
                    :icon="h(DeleteOutlined)"
                    @click.stop="deleteSnippet(item)"
                    title="删除"
                  />
                </template>
              </a-list-item>
            </template>
          </a-list>
        </div>

        <div class="snippet-editor" v-if="selectedSnippet">
          <a-form :label-col="{ span: 4 }" :wrapper-col="{ span: 20 }">
            <a-form-item label="标题">
              <a-input v-model:value="selectedSnippet.title" />
            </a-form-item>
            <a-form-item label="分类">
              <a-select
                v-model:value="selectedSnippet.category"
                :options="categoryOptions"
                allow-clear
                placeholder="选择或输入分类"
                mode="tags"
                :max-tag-count="1"
              />
            </a-form-item>
            <a-form-item label="描述">
              <a-textarea v-model:value="selectedSnippet.description" :rows="2" />
            </a-form-item>
            <a-form-item label="SQL代码">
              <a-textarea
                v-model:value="selectedSnippet.sql"
                :rows="10"
                class="sql-input"
                placeholder="输入SQL代码..."
              />
            </a-form-item>
            <a-form-item label="快捷键">
              <a-input
                v-model:value="selectedSnippet.shortcut"
                placeholder="例如: Ctrl+Shift+1"
              />
            </a-form-item>
          </a-form>
          
          <div class="snippet-actions">
            <a-space>
              <a-button @click="saveSnippet" type="primary">
                <SaveOutlined />
                保存
              </a-button>
              <a-button @click="insertSnippet">
                <CodeOutlined />
                插入到编辑器
              </a-button>
              <a-button @click="copySnippet(selectedSnippet)">
                <CopyOutlined />
                复制
              </a-button>
            </a-space>
          </div>
        </div>
      </div>
    </div>
  </a-modal>
</template>

<script setup lang="ts">
import { h, ref, computed, watch } from 'vue'
import {
  PlusOutlined,
  CopyOutlined,
  DeleteOutlined,
  SaveOutlined,
  CodeOutlined,
} from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'

interface SqlSnippet {
  id: string
  title: string
  description?: string
  category?: string
  sql: string
  shortcut?: string
  createdAt: number
  updatedAt: number
}

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits(['update:visible', 'insert-snippet'])

const searchText = ref('')
const selectedSnippet = ref<SqlSnippet | null>(null)
const snippets = ref<SqlSnippet[]>([])

// 默认分类
const categoryOptions = [
  { label: 'SELECT', value: 'SELECT' },
  { label: 'INSERT', value: 'INSERT' },
  { label: 'UPDATE', value: 'UPDATE' },
  { label: 'DELETE', value: 'DELETE' },
  { label: 'DDL', value: 'DDL' },
  { label: '常用', value: '常用' },
  { label: '其他', value: '其他' },
]

// 预置的代码片段
const defaultSnippets: SqlSnippet[] = [
  {
    id: '1',
    title: 'SELECT 基本查询',
    description: '基本的SELECT查询模板',
    category: 'SELECT',
    sql: 'SELECT * FROM table_name\nWHERE condition\nORDER BY column_name\nLIMIT 100;',
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
  {
    id: '2',
    title: 'INSERT 插入数据',
    description: '插入单行数据',
    category: 'INSERT',
    sql: 'INSERT INTO table_name (column1, column2, column3)\nVALUES (value1, value2, value3);',
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
  {
    id: '3',
    title: 'UPDATE 更新数据',
    description: '更新表数据',
    category: 'UPDATE',
    sql: 'UPDATE table_name\nSET column1 = value1, column2 = value2\nWHERE condition;',
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
  {
    id: '4',
    title: 'DELETE 删除数据',
    description: '删除表数据',
    category: 'DELETE',
    sql: 'DELETE FROM table_name\nWHERE condition;',
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
  {
    id: '5',
    title: 'CREATE TABLE',
    description: '创建表',
    category: 'DDL',
    sql: `CREATE TABLE table_name (
  id INT PRIMARY KEY AUTO_INCREMENT,
  name VARCHAR(255) NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;`,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
  {
    id: '6',
    title: 'JOIN 查询',
    description: '多表关联查询',
    category: 'SELECT',
    sql: `SELECT t1.*, t2.column_name
FROM table1 t1
LEFT JOIN table2 t2 ON t1.id = t2.foreign_id
WHERE t1.condition;`,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
  {
    id: '7',
    title: '分组统计',
    description: 'GROUP BY聚合查询',
    category: 'SELECT',
    sql: `SELECT column1, COUNT(*) as count, SUM(column2) as total
FROM table_name
GROUP BY column1
HAVING COUNT(*) > 1
ORDER BY count DESC;`,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
]

// 过滤片段
const filteredSnippets = computed(() => {
  if (!searchText.value) {
    return snippets.value
  }
  const text = searchText.value.toLowerCase()
  return snippets.value.filter(
    (snippet) =>
      snippet.title.toLowerCase().includes(text) ||
      snippet.description?.toLowerCase().includes(text) ||
      snippet.sql.toLowerCase().includes(text)
  )
})

// 加载片段
function loadSnippets() {
  const saved = localStorage.getItem('sql-snippets')
  if (saved) {
    try {
      snippets.value = JSON.parse(saved)
    } catch (e) {
      console.error('加载片段失败:', e)
      snippets.value = [...defaultSnippets]
    }
  } else {
    snippets.value = [...defaultSnippets]
  }
}

// 保存片段到本地存储
function saveToStorage() {
  localStorage.setItem('sql-snippets', JSON.stringify(snippets.value))
}

// 添加新片段
function handleAddSnippet() {
  const newSnippet: SqlSnippet = {
    id: Date.now().toString(),
    title: '新建片段',
    description: '',
    category: '',
    sql: '',
    createdAt: Date.now(),
    updatedAt: Date.now(),
  }
  snippets.value.unshift(newSnippet)
  selectedSnippet.value = newSnippet
}

// 选择片段
function selectSnippet(snippet: SqlSnippet) {
  selectedSnippet.value = { ...snippet }
}

// 保存片段
function saveSnippet() {
  if (!selectedSnippet.value) return

  const index = snippets.value.findIndex((s) => s.id === selectedSnippet.value!.id)
  if (index !== -1) {
    selectedSnippet.value.updatedAt = Date.now()
    snippets.value[index] = { ...selectedSnippet.value }
    saveToStorage()
    message.success('片段已保存')
  }
}

// 删除片段
function deleteSnippet(snippet: SqlSnippet) {
  Modal.confirm({
    title: '确认删除',
    content: `确定要删除片段 "${snippet.title}" 吗？`,
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    onOk() {
      snippets.value = snippets.value.filter((s) => s.id !== snippet.id)
      if (selectedSnippet.value?.id === snippet.id) {
        selectedSnippet.value = null
      }
      saveToStorage()
      message.success('片段已删除')
    },
  })
}

// 复制片段
function copySnippet(snippet: SqlSnippet) {
  navigator.clipboard.writeText(snippet.sql)
  message.success('SQL已复制到剪贴板')
}

// 插入片段到编辑器
function insertSnippet() {
  if (!selectedSnippet.value) return
  emit('insert-snippet', selectedSnippet.value.sql)
  message.success('已插入到编辑器')
}

// 取消
function handleCancel() {
  emit('update:visible', false)
}

// 监听对话框打开
watch(() => props.visible, (visible) => {
  if (visible) {
    loadSnippets()
  }
})
</script>

<style scoped>
.snippets-manager {
  height: 600px;
  display: flex;
  flex-direction: column;
}

.snippets-toolbar {
  padding: 12px;
  border-bottom: 1px solid #e8e8e8;
}

.dark-mode .snippets-toolbar {
  border-bottom-color: #303030;
}

.snippets-content {
  display: flex;
  gap: 16px;
  flex: 1;
  overflow: hidden;
  padding: 16px;
}

.snippets-list {
  width: 300px;
  border: 1px solid #e8e8e8;
  border-radius: 6px;
  overflow-y: auto;
}

.dark-mode .snippets-list {
  border-color: #303030;
}

.snippet-item {
  cursor: pointer;
  transition: background-color 0.2s;
  padding: 12px !important;
}

.snippet-item:hover {
  background-color: #f5f5f5;
}

.dark-mode .snippet-item:hover {
  background-color: #262626;
}

.snippet-item.active {
  background-color: #e6f7ff;
}

.dark-mode .snippet-item.active {
  background-color: #111b26;
}

.snippet-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.snippet-editor {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}

.sql-input {
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 13px;
}

.snippet-actions {
  padding-top: 12px;
  border-top: 1px solid #e8e8e8;
}

.dark-mode .snippet-actions {
  border-top-color: #303030;
}
</style>

