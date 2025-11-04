<template>
  <div class="table-data-grid">
    <div class="grid-toolbar">
      <a-space>
        <a-button :icon="h(ReloadOutlined)" @click="loadData" :loading="loading">
          刷新
        </a-button>
        <a-button :icon="h(PlusOutlined)" @click="addRow" type="primary">
          新增
        </a-button>
        <a-button
          :icon="h(DeleteOutlined)"
          @click="deleteSelected"
          :disabled="selectedRowKeys.length === 0"
          danger
        >
          删除 ({{ selectedRowKeys.length }})
        </a-button>
        <a-divider type="vertical" />
        <a-button :icon="h(FilterOutlined)" @click="showFilterDialog = true">
          筛选
        </a-button>
        <a-dropdown>
          <a-button :icon="h(ExportOutlined)">
            导出
          </a-button>
          <template #overlay>
            <a-menu @click="handleExport">
              <a-menu-item key="csv">导出为 CSV</a-menu-item>
              <a-menu-item key="json">导出为 JSON</a-menu-item>
              <a-menu-item key="sql">导出为 SQL</a-menu-item>
            </a-menu>
          </template>
        </a-dropdown>
      </a-space>
      <div class="toolbar-info">
        <a-tag color="blue">{{ database }}.{{ table }}</a-tag>
        <span class="row-count">{{ totalRows }} 行</span>
      </div>
    </div>

    <a-table
      :columns="columns"
      :data-source="dataSource"
      :loading="loading"
      :pagination="pagination"
      :scroll="{ x: 1200, y: 'calc(100vh - 350px)' }"
      :row-selection="rowSelection"
      :row-key="(record) => record.__rowIndex"
      size="small"
      bordered
      @change="handleTableChange"
    >
      <template #bodyCell="{ column, text, record, index }">
        <div
          class="editable-cell"
          @dblclick="startEdit(record, column.dataIndex, index)"
        >
          <div v-if="editingKey === `${record.__rowIndex}-${column.dataIndex}`" class="editing-wrapper">
            <a-textarea
              v-model:value="editingValue"
              @keyup.esc="cancelEdit"
              :auto-size="{ minRows: 2, maxRows: 8 }"
              ref="editInput"
              class="edit-input"
            />
            <div class="edit-buttons">
              <a-button 
                type="primary" 
                size="small" 
                :loading="saving"
                @click.stop="saveEdit(record, column.dataIndex)"
              >
                <template #icon><CheckOutlined /></template>
                保存
              </a-button>
              <a-button 
                size="small" 
                @click.stop="cancelEdit"
                :disabled="saving"
              >
                <template #icon><CloseOutlined /></template>
                取消
              </a-button>
            </div>
          </div>
          <div 
            v-else 
            class="cell-content"
            :title="text !== null && text !== undefined && String(text).length > 30 ? String(text) : undefined"
          >
            <span :class="{ null: text === null }">
              {{ text === null ? 'NULL' : text }}
            </span>
          </div>
        </div>
      </template>
    </a-table>

    <!-- 筛选对话框 -->
    <a-modal
      v-model:open="showFilterDialog"
      title="数据筛选"
      @ok="applyFilter"
    >
      <a-form layout="vertical">
        <a-form-item label="WHERE 条件">
          <a-textarea
            v-model:value="filterCondition"
            :rows="4"
            placeholder="例如: id > 100 AND status = 'active'"
          />
        </a-form-item>
        <a-form-item label="LIMIT">
          <a-input-number
            v-model:value="limitRows"
            :min="1"
            :max="10000"
            style="width: 100%"
          />
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { h, nextTick, ref, onMounted, watch} from 'vue'
import {
  ReloadOutlined,
  PlusOutlined,
  DeleteOutlined,
  FilterOutlined,
  ExportOutlined,
  CheckOutlined,
  CloseOutlined,
} from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import type { QueryResult } from '@/types/database'
import { useConnectionStore } from '@/stores/connection'

const props = defineProps<{
  connectionId: string
  database: string
  table: string
}>()

const connectionStore = useConnectionStore()

// 获取当前连接的数据库类型
const dbType = computed(() => {
  const connection = connectionStore.connections.find(c => c.id === props.connectionId)
  return connection?.db_type || 'mysql'
})

// 根据数据库类型引用标识符（列名、表名）
const quoteIdentifier = (name: string) => {
  // SQLite 和 PostgreSQL 使用双引号，MySQL 使用反引号
  return dbType.value === 'sqlite' || dbType.value === 'postgresql' ? `"${name}"` : `\`${name}\``
}

// 根据数据库类型格式化表引用
const formatTableRef = () => {
  if (dbType.value === 'sqlite') {
    // SQLite 不使用 database.table 格式
    return quoteIdentifier(props.table)
  } else {
    // MySQL, PostgreSQL 使用 database.table 格式
    return `${quoteIdentifier(props.database)}.${quoteIdentifier(props.table)}`
  }
}

const loading = ref(false)
const dataSource = ref<any[]>([])
const columns = ref<any[]>([])
const selectedRowKeys = ref<string[]>([])
const totalRows = ref(0)
const showFilterDialog = ref(false)
const filterCondition = ref('')
const limitRows = ref(1000)
const tableStructure = ref<any[]>([]) // 表结构信息
const primaryKeys = ref<string[]>([]) // 主键列

// 编辑相关
const editingKey = ref('')
const editingValue = ref('')
const editInput = ref()
const saving = ref(false)

// 分页
const pagination = ref({
  current: 1,
  pageSize: 100,
  total: 0,
  showSizeChanger: true,
  showQuickJumper: true,
  pageSizeOptions: ['50', '100', '200', '500', '1000'],
  showTotal: (total: number) => `共 ${total} 行`,
})

// 行选择
const rowSelection = computed(() => ({
  selectedRowKeys: selectedRowKeys.value,
  onChange: (keys: (string | number)[]) => {
    selectedRowKeys.value = keys as string[]
  },
  getCheckboxProps: (record: any) => ({
    name: String(record.__rowIndex),
  }),
}))

// 加载数据
async function loadData() {
  console.log('=== 开始加载表数据 ===')
  console.log('连接ID:', props.connectionId)
  console.log('数据库:', props.database)
  console.log('表名:', props.table)
  
  loading.value = true
  try {
    // 先获取表结构以识别主键
    console.log('步骤1: 获取表结构...')
    const structure = await invoke<any[]>('get_table_structure', {
      connectionId: props.connectionId,
      table: props.table,
      schema: props.database,
    })
    console.log('表结构获取成功:', structure)
    
    tableStructure.value = structure
    primaryKeys.value = structure
      .filter((col: any) => col.is_primary_key)
      .map((col: any) => col.name)
    
    console.log('主键列:', primaryKeys.value)
    
    // 如果没有主键，发出警告
    if (primaryKeys.value.length === 0) {
      console.warn('该表没有主键')
      message.warning('该表没有主键，编辑和删除功能可能不可用')
    }
    
    // 使用格式化函数生成 SQL
    let sql = `SELECT * FROM ${formatTableRef()}`
    
    if (filterCondition.value) {
      sql += ` WHERE ${filterCondition.value}`
    }
    
    sql += ` LIMIT ${limitRows.value}`
    
    console.log('步骤2: 执行查询...')
    console.log('SQL:', sql)

    const result = await invoke<QueryResult>('execute_query', {
      connectionId: props.connectionId,
      sql,
    })
    
    console.log('查询结果:', {
      columns: result.columns,
      rowCount: result.rows.length,
      sampleRow: result.rows[0]
    })

    // 设置列 - 智能计算列宽
    columns.value = result.columns.map((col) => {
      // 计算列名宽度（中文按2个字符，英文按1个字符）
      const getTextWidth = (text: string) => {
        let width = 0
        for (let i = 0; i < text.length; i++) {
          // 中文、全角字符占用更多空间
          width += text.charCodeAt(i) > 255 ? 14 : 8
        }
        return width
      }
      
      let columnWidth = getTextWidth(col) + 60 // 列名宽度 + padding + 排序图标空间
      
      // 采样前50行数据来估算最佳列宽
      let maxContentWidth = 0
      const sampleSize = Math.min(50, result.rows.length)
      
      for (let i = 0; i < sampleSize; i++) {
        const cellValue = result.rows[i][col]
        if (cellValue !== null && cellValue !== undefined) {
          const strValue = String(cellValue)
          const contentWidth = getTextWidth(strValue) + 30 // 内容宽度 + padding
          maxContentWidth = Math.max(maxContentWidth, contentWidth)
        }
      }
      
      // 取列名宽度和内容宽度的较大值
      columnWidth = Math.max(columnWidth, maxContentWidth)
      
      // 限制列宽范围：最小80px，最大300px
      // 控制单列宽度，防止占用过多空间
      if (columnWidth < 80) {
        columnWidth = 80
      } else if (columnWidth > 300) {
        columnWidth = 300
      }
      
      return {
        title: col,
        dataIndex: col,
        key: col,
        width: columnWidth,
        ellipsis: {
          showTitle: false, // 使用自定义tooltip
        },
        resizable: true,
        sorter: (a: any, b: any) => {
          const aVal = a[col]
          const bVal = b[col]
          if (aVal === null) return -1
          if (bVal === null) return 1
          return aVal > bVal ? 1 : aVal < bVal ? -1 : 0
        },
      }
    })

    // 设置数据源 - 使用原始行数据作为key
    dataSource.value = result.rows.map((row, index) => ({
      __rowIndex: index, // 内部索引
      ...row,
    }))

    totalRows.value = result.rows.length
    pagination.value.total = result.rows.length

    console.log('步骤3: 数据加载完成')
    console.log('总行数:', totalRows.value)
    console.log('列数:', columns.value.length)
    console.log('=== 加载完成 ===')
    
    message.success(`加载了 ${result.rows.length} 行数据`)
  } catch (error: any) {
    console.error('=== 加载数据失败 ===')
    console.error('错误详情:', error)
    console.error('错误类型:', typeof error)
    console.error('错误字符串:', String(error))
    message.error(`加载数据失败: ${error}`)
  } finally {
    loading.value = false
  }
}

// 表格变化处理
function handleTableChange(pag: any, _filters: any, _sorter: any) {
  pagination.value = pag
}

// 开始编辑
function startEdit(record: any, field: string | number | readonly (string | number)[] | undefined, _index: number, _event?: MouseEvent) {
  if (!field) return
  const fieldStr = Array.isArray(field) ? String(field[0]) : String(field)
  editingKey.value = `${record.__rowIndex}-${fieldStr}`
  editingValue.value = record[fieldStr] === null ? '' : String(record[fieldStr])
  
  nextTick(() => {
    if (editInput.value) {
      // 聚焦并选中所有文本
      const textarea = editInput.value.$el?.querySelector('textarea') || editInput.value.$el
      if (textarea) {
        textarea.focus()
        textarea.select()
      }
    }
  })
}

// 保存编辑
async function saveEdit(record: any, field: string | number | readonly (string | number)[] | undefined) {
  if (!field) return
  const fieldStr = Array.isArray(field) ? String(field[0]) : String(field)
  console.log('=== 开始保存编辑 ===')
  console.log('字段:', fieldStr)
  console.log('旧值:', record[fieldStr])
  console.log('新值:', editingValue.value)
  
  if (saving.value) {
    console.log('正在保存中，忽略重复点击')
    return
  }
  
  if (primaryKeys.value.length === 0) {
    message.error('该表没有主键，无法更新数据')
    cancelEdit()
    return
  }
  
  const oldValue = record[fieldStr]
  const newValue = editingValue.value === '' ? null : editingValue.value
  
  // 如果值没有变化，直接取消编辑
  if (String(oldValue) === String(newValue)) {
    console.log('值未变化，取消编辑')
    message.info('数据未修改')
    cancelEdit()
    return
  }
  
  saving.value = true
  try {
    console.log('构建WHERE条件...')
    // 构建WHERE条件（基于主键）
    const whereConditions = primaryKeys.value.map(pk => {
      const value = record[pk]
      if (value === null) {
        return `${quoteIdentifier(pk)} IS NULL`
      }
      return `${quoteIdentifier(pk)} = '${String(value).replace(/'/g, "''")}'`
    })
    const whereClause = whereConditions.join(' AND ')
    console.log('WHERE条件:', whereClause)
    
    console.log('调用后端更新...')
    // 调用后端更新数据 - 将值转换为字符串
    await invoke('update_table_data', {
      connectionId: props.connectionId,
      database: props.database,
      table: props.table,
      column: fieldStr,
      value: newValue === null ? null : String(newValue),
      whereClause,
    })
    
    console.log('更新成功，更新本地数据')
    // 更新本地数据
    record[fieldStr] = newValue
    
    editingKey.value = ''
    message.success('数据已更新')
    console.log('=== 保存完成 ===')
  } catch (error: any) {
    console.error('=== 保存失败 ===')
    console.error('错误:', error)
    message.error(`更新失败: ${error}`)
  } finally {
    saving.value = false
  }
}

// 取消编辑
function cancelEdit() {
  editingKey.value = ''
  editingValue.value = ''
}

// 新增行
async function addRow() {
  if (tableStructure.value.length === 0) {
    message.error('表结构信息未加载')
    return
  }
  
  // 创建一个包含所有列默认值的新行
  const newRow: any = { __rowIndex: -1, __isNew: true }
  
  tableStructure.value.forEach((col: any) => {
    // 设置默认值
    if (col.default_value !== null && col.default_value !== undefined) {
      newRow[col.name] = col.default_value
    } else if (col.is_auto_increment) {
      newRow[col.name] = 'AUTO'
    } else if (!col.nullable) {
      // 非空字段设置默认值
      if (col.data_type.includes('int')) {
        newRow[col.name] = 0
      } else if (col.data_type.includes('varchar') || col.data_type.includes('text')) {
        newRow[col.name] = ''
      } else if (col.data_type.includes('date') || col.data_type.includes('time')) {
        newRow[col.name] = new Date().toISOString().split('T')[0]
      } else {
        newRow[col.name] = ''
      }
    } else {
      newRow[col.name] = null
    }
  })
  
  // 将新行添加到数据源开头
  dataSource.value.unshift(newRow)
  totalRows.value++
  
  message.info('已添加新行，请编辑后刷新保存到数据库')
}

// 删除选中行
async function deleteSelected() {
  if (selectedRowKeys.value.length === 0) {
    message.warning('请先选择要删除的行')
    return
  }
  
  if (primaryKeys.value.length === 0) {
    message.error('该表没有主键，无法删除数据')
    return
  }
  
  Modal.confirm({
    title: '确认删除',
    content: `确定要删除选中的 ${selectedRowKeys.value.length} 行数据吗？此操作不可恢复！`,
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    async onOk() {
      try {
        let deletedCount = 0
        
        for (const key of selectedRowKeys.value) {
          const record = dataSource.value.find((item) => item.__rowIndex === key)
          if (!record) continue
          
          // 如果是新增但未保存的行，直接从列表中移除
          if (record.__isNew) {
            dataSource.value = dataSource.value.filter((item) => item.__rowIndex !== key)
            deletedCount++
            continue
          }
          
          // 构建WHERE条件
          const whereConditions = primaryKeys.value.map(pk => {
            const value = record[pk]
            if (value === null) {
              return `${quoteIdentifier(pk)} IS NULL`
            }
            return `${quoteIdentifier(pk)} = '${String(value).replace(/'/g, "''")}'`
          })
          const whereClause = whereConditions.join(' AND ')
          
          // 调用后端删除数据
          await invoke('delete_table_data', {
            connectionId: props.connectionId,
            database: props.database,
            table: props.table,
            whereClause,
          })
          
          deletedCount++
        }
        
        // 重新加载数据
        await loadData()
        selectedRowKeys.value = []
        
        message.success(`成功删除 ${deletedCount} 行数据`)
      } catch (error: any) {
        message.error(`删除失败: ${error}`)
      }
    },
  })
}

// 应用筛选
function applyFilter() {
  showFilterDialog.value = false
  pagination.value.current = 1
  loadData()
}

// 处理导出
async function handleExport({ key }: { key: string | number }) {
  try {
    let result: string
    
    // 构建SQL查询（包含筛选条件）
    let sql = `SELECT * FROM \`${props.database}\`.\`${props.table}\``
    if (filterCondition.value) {
      sql += ` WHERE ${filterCondition.value}`
    }
    
    switch (key) {
      case 'csv':
        result = await invoke<string>('export_to_csv', {
          connectionId: props.connectionId,
          database: props.database,
          table: props.table,
          query: sql,
        })
        break
      case 'json':
        result = await invoke<string>('export_to_json', {
          connectionId: props.connectionId,
          database: props.database,
          table: props.table,
          query: sql,
        })
        break
      case 'sql':
        result = await invoke<string>('export_to_sql', {
          connectionId: props.connectionId,
          database: props.database,
          table: props.table,
          query: sql,
        })
        break
      default:
        return
    }
    
    message.success(`导出成功: ${result}`)
  } catch (error: any) {
    message.error(`导出失败: ${error}`)
  }
}

// 初始加载
onMounted(() => {
  loadData()
})

// 监听表变化
watch(
  () => props.table,
  () => {
    loadData()
  }
)
</script>

<style scoped>
.table-data-grid {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.grid-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px;
  border-bottom: 1px solid #e8e8e8;
  background: #fafafa;
}

.dark-mode .grid-toolbar {
  background: #1f1f1f;
  border-bottom-color: #303030;
}

.toolbar-info {
  display: flex;
  gap: 12px;
  align-items: center;
}

.row-count {
  font-size: 13px;
  color: #8c8c8c;
}

.editable-cell {
  min-height: 32px;
  padding: 4px 8px;
  cursor: text;
  position: relative;
}

.editable-cell:hover {
  background: #f0f0f0;
}

.dark-mode .editable-cell:hover {
  background: #262626;
}

/* 编辑包装器 - 浮动弹出样式 */
.editing-wrapper {
  position: fixed;
  z-index: 9999;
  background: #fff;
  border: 2px solid #1890ff;
  border-radius: 8px;
  padding: 16px;
  box-shadow: 0 6px 24px rgba(0, 0, 0, 0.2);
  min-width: 400px;
  max-width: 600px;
  margin-top: 4px;
}

.dark-mode .editing-wrapper {
  background: #1f1f1f;
  border-color: #177ddc;
  box-shadow: 0 6px 24px rgba(0, 0, 0, 0.5);
}

/* 编辑输入框 */
.edit-input {
  width: 100%;
  margin-bottom: 12px;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 14px;
}

.edit-input :deep(textarea.ant-input) {
  min-height: 80px !important;
  line-height: 1.6;
  padding: 8px 12px;
  font-size: 14px;
}

/* 编辑按钮组 */
.edit-buttons {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  margin-top: 8px;
}

.edit-buttons .ant-btn {
  height: 32px;
  font-size: 14px;
}

.cell-content {
  min-height: 24px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.null {
  color: #bfbfbf;
  font-style: italic;
}

/* 表格优化 */
:deep(.ant-table-thead > tr > th) {
  background: #fafafa;
  font-weight: 600;
  padding: 10px 12px;
  user-select: none;
}

.dark-mode :deep(.ant-table-thead > tr > th) {
  background: #1f1f1f;
}

:deep(.ant-table-tbody > tr > td) {
  padding: 8px 12px;
}

:deep(.ant-table-tbody > tr:hover > td) {
  background: #f5f5f5 !important;
}

.dark-mode :deep(.ant-table-tbody > tr:hover > td) {
  background: #262626 !important;
}
</style>

