<template>
  <a-modal
    :open="visible"
    title="全局搜索"
    :width="1000"
    @cancel="handleCancel"
    :footer="null"
  >
    <div class="global-search">
      <div class="search-header">
        <a-input-search
          v-model:value="searchText"
          placeholder="搜索表、列、视图、存储过程等..."
          size="large"
          @search="handleSearch"
          :loading="searching"
          allow-clear
          autofocus
        >
          <template #enterButton>
            <a-button type="primary">
              <SearchOutlined />
              搜索
            </a-button>
          </template>
        </a-input-search>
        
        <div class="search-filters" style="margin-top: 12px;">
          <a-space>
            <a-select
              v-model:value="searchScope"
              style="width: 150px"
              placeholder="搜索范围"
            >
              <a-select-option value="all">全部</a-select-option>
              <a-select-option value="tables">表</a-select-option>
              <a-select-option value="columns">列</a-select-option>
              <a-select-option value="views">视图</a-select-option>
              <a-select-option value="procedures">存储过程</a-select-option>
              <a-select-option value="functions">函数</a-select-option>
            </a-select>
            
            <a-select
              v-model:value="selectedDatabase"
              style="width: 150px"
              placeholder="选择数据库"
              allow-clear
            >
              <a-select-option value="">全部数据库</a-select-option>
              <a-select-option
                v-for="db in databases"
                :key="db.name"
                :value="db.name"
              >
                {{ db.name }}
              </a-select-option>
            </a-select>
            
            <a-checkbox v-model:checked="caseSensitive">
              区分大小写
            </a-checkbox>
          </a-space>
        </div>
      </div>

      <div class="search-results" v-if="searchResults.length > 0">
        <div class="results-summary">
          找到 <strong>{{ searchResults.length }}</strong> 个结果
        </div>
        
        <a-tabs v-model:activeKey="activeTab">
          <a-tab-pane
            v-for="type in resultTypes"
            :key="type.key"
            :tab="`${type.label} (${getResultCount(type.key)})`"
          >
            <a-list
              :data-source="getResultsByType(type.key)"
              :pagination="{ pageSize: 20 }"
            >
              <template #renderItem="{ item }">
                <a-list-item>
                  <a-list-item-meta>
                    <template #avatar>
                      <component :is="getIcon(item.type)" style="font-size: 20px; color: #1890ff;" />
                    </template>
                    <template #title>
                      <a @click="handleResultClick(item)">
                        <span v-html="highlightMatch(item.name)"></span>
                      </a>
                      <a-tag
                        v-if="item.database"
                        color="blue"
                        size="small"
                        style="margin-left: 8px;"
                      >
                        {{ item.database }}
                      </a-tag>
                      <a-tag
                        v-if="item.table"
                        color="green"
                        size="small"
                        style="margin-left: 4px;"
                      >
                        {{ item.table }}
                      </a-tag>
                    </template>
                    <template #description>
                      <div>
                        <span v-if="item.type">{{ getTypeName(item.type) }}</span>
                        <span v-if="item.dataType"> • 类型: {{ item.dataType }}</span>
                        <span v-if="item.comment"> • {{ item.comment }}</span>
                      </div>
                    </template>
                  </a-list-item-meta>
                  <template #actions>
                    <a-button
                      type="link"
                      size="small"
                      @click="handleCopyPath(item)"
                    >
                      <CopyOutlined />
                      复制路径
                    </a-button>
                    <a-button
                      v-if="item.type === 'table'"
                      type="link"
                      size="small"
                      @click="handleViewData(item)"
                    >
                      <TableOutlined />
                      查看数据
                    </a-button>
                  </template>
                </a-list-item>
              </template>
            </a-list>
          </a-tab-pane>
        </a-tabs>
      </div>

      <a-empty
        v-else-if="!searching && searchText"
        description="未找到匹配的结果"
      />
    </div>
  </a-modal>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import {
  SearchOutlined,
  CopyOutlined,
  TableOutlined,
  EyeOutlined,
  FileOutlined,
  FolderOutlined,
} from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'

interface SearchResult {
  type: 'table' | 'column' | 'view' | 'procedure' | 'function' | 'trigger'
  name: string
  database?: string
  table?: string
  dataType?: string
  comment?: string
}

const props = defineProps<{
  visible: boolean
  connectionId: string | null
}>()

const emit = defineEmits(['update:visible', 'view-data', 'design-table'])

const searchText = ref('')
const searching = ref(false)
const searchScope = ref('all')
const selectedDatabase = ref('')
const caseSensitive = ref(false)
const searchResults = ref<SearchResult[]>([])
const databases = ref<any[]>([])
const activeTab = ref('all')

const resultTypes = [
  { key: 'all', label: '全部' },
  { key: 'table', label: '表' },
  { key: 'column', label: '列' },
  { key: 'view', label: '视图' },
  { key: 'procedure', label: '存储过程' },
  { key: 'function', label: '函数' },
]

// 获取图标
function getIcon(type: string) {
  const iconMap: Record<string, any> = {
    table: TableOutlined,
    column: FileOutlined,
    view: EyeOutlined,
    procedure: FolderOutlined,
    function: FolderOutlined,
    trigger: FolderOutlined,
  }
  return iconMap[type] || FileOutlined
}

// 获取类型名称
function getTypeName(type: string): string {
  const nameMap: Record<string, string> = {
    table: '表',
    column: '列',
    view: '视图',
    procedure: '存储过程',
    function: '函数',
    trigger: '触发器',
  }
  return nameMap[type] || type
}

// 获取指定类型的结果数量
function getResultCount(type: string): number {
  if (type === 'all') return searchResults.value.length
  return searchResults.value.filter(r => r.type === type).length
}

// 获取指定类型的结果
function getResultsByType(type: string): SearchResult[] {
  if (type === 'all') return searchResults.value
  return searchResults.value.filter(r => r.type === type)
}

// 高亮匹配文本
function highlightMatch(text: string): string {
  if (!searchText.value) return text
  
  const regex = new RegExp(
    `(${searchText.value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`,
    caseSensitive.value ? 'g' : 'gi'
  )
  
  return text.replace(regex, '<span style="background-color: #ffc069; font-weight: bold;">$1</span>')
}

// 加载数据库列表
async function loadDatabases() {
  if (!props.connectionId) return
  
  try {
    const dbs = await invoke<any[]>('get_databases', {
      connectionId: props.connectionId,
    })
    databases.value = dbs
  } catch (error: any) {
    console.error('加载数据库列表失败:', error)
  }
}

// 执行搜索
async function handleSearch() {
  if (!searchText.value || !props.connectionId) {
    message.warning('请输入搜索关键字')
    return
  }
  
  searching.value = true
  searchResults.value = []
  
  try {
    const results: SearchResult[] = []
    const searchPattern = caseSensitive.value ? searchText.value : searchText.value.toLowerCase()
    
    // 确定要搜索的数据库列表
    const databasesToSearch = selectedDatabase.value
      ? [{ name: selectedDatabase.value }]
      : databases.value
    
    for (const db of databasesToSearch) {
      // 搜索表
      if (searchScope.value === 'all' || searchScope.value === 'tables') {
        const tables = await invoke<any[]>('get_tables', {
          connectionId: props.connectionId,
          database: db.name,
        })
        
        for (const table of tables) {
          const tableName = caseSensitive.value ? table.name : table.name.toLowerCase()
          if (tableName.includes(searchPattern)) {
            results.push({
              type: 'table',
              name: table.name,
              database: db.name,
              comment: table.comment,
            })
          }
        }
      }
      
      // 搜索列
      if (searchScope.value === 'all' || searchScope.value === 'columns') {
        const tables = await invoke<any[]>('get_tables', {
          connectionId: props.connectionId,
          database: db.name,
        })
        
        for (const table of tables) {
          const columns = await invoke<any[]>('get_table_structure', {
            connectionId: props.connectionId,
            table: table.name,
            schema: db.name,
          })
          
          for (const column of columns) {
            const columnName = caseSensitive.value ? column.name : column.name.toLowerCase()
            if (columnName.includes(searchPattern)) {
              results.push({
                type: 'column',
                name: column.name,
                database: db.name,
                table: table.name,
                dataType: column.data_type,
                comment: column.comment,
              })
            }
          }
        }
      }
      
      // 搜索视图
      if (searchScope.value === 'all' || searchScope.value === 'views') {
        try {
          const views = await invoke<any[]>('get_views', {
            connectionId: props.connectionId,
            database: db.name,
          })
          
          for (const view of views) {
            const viewName = caseSensitive.value ? view.name : view.name.toLowerCase()
            if (viewName.includes(searchPattern)) {
              results.push({
                type: 'view',
                name: view.name,
                database: db.name,
                comment: view.comment,
              })
            }
          }
        } catch (error) {
          console.error('搜索视图失败:', error)
        }
      }
      
      // 搜索存储过程
      if (searchScope.value === 'all' || searchScope.value === 'procedures') {
        try {
          const procedures = await invoke<any[]>('get_procedures', {
            connectionId: props.connectionId,
            database: db.name,
          })
          
          for (const proc of procedures) {
            const procName = caseSensitive.value ? proc.ROUTINE_NAME : proc.ROUTINE_NAME.toLowerCase()
            if (procName.includes(searchPattern)) {
              results.push({
                type: 'procedure',
                name: proc.ROUTINE_NAME,
                database: db.name,
              })
            }
          }
        } catch (error) {
          console.error('搜索存储过程失败:', error)
        }
      }
      
      // 搜索函数
      if (searchScope.value === 'all' || searchScope.value === 'functions') {
        try {
          const functions = await invoke<any[]>('get_functions', {
            connectionId: props.connectionId,
            database: db.name,
          })
          
          for (const func of functions) {
            const funcName = caseSensitive.value ? func.ROUTINE_NAME : func.ROUTINE_NAME.toLowerCase()
            if (funcName.includes(searchPattern)) {
              results.push({
                type: 'function',
                name: func.ROUTINE_NAME,
                database: db.name,
              })
            }
          }
        } catch (error) {
          console.error('搜索函数失败:', error)
        }
      }
    }
    
    searchResults.value = results
    
    if (results.length === 0) {
      message.info('未找到匹配的结果')
    } else {
      message.success(`找到 ${results.length} 个结果`)
    }
  } catch (error: any) {
    message.error(`搜索失败: ${error}`)
  } finally {
    searching.value = false
  }
}

// 处理结果点击
function handleResultClick(item: SearchResult) {
  if (item.type === 'table' || item.type === 'view') {
    emit('view-data', {
      database: item.database,
      table: item.name,
    })
  }
}

// 查看数据
function handleViewData(item: SearchResult) {
  emit('view-data', {
    database: item.database,
    table: item.name,
  })
  emit('update:visible', false)
}

// 复制路径
function handleCopyPath(item: SearchResult) {
  let path = ''
  if (item.table) {
    path = `${item.database}.${item.table}.${item.name}`
  } else {
    path = `${item.database}.${item.name}`
  }
  
  navigator.clipboard.writeText(path)
  message.success('路径已复制到剪贴板')
}

// 取消
function handleCancel() {
  emit('update:visible', false)
}

// 监听对话框打开
watch(() => props.visible, (visible) => {
  if (visible) {
    loadDatabases()
    searchResults.value = []
    activeTab.value = 'all'
  }
})
</script>

<style scoped>
.global-search {
  min-height: 500px;
}

.search-header {
  margin-bottom: 24px;
}

.search-filters {
  display: flex;
  align-items: center;
  gap: 12px;
}

.results-summary {
  padding: 12px 0;
  color: #666;
  font-size: 14px;
}

.search-results {
  margin-top: 16px;
}

:deep(.ant-list-item-meta-title) a {
  color: inherit;
}

:deep(.ant-list-item-meta-title) a:hover {
  color: #1890ff;
}
</style>

