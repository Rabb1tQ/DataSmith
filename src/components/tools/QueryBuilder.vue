<template>
  <div class="query-builder">
    <div class="builder-header">
      <h3>可视化查询构建器</h3>
      <p>通过可视化界面构建SQL查询</p>
    </div>

    <div class="builder-config">
      <a-form layout="vertical">
        <a-form-item label="数据库">
          <a-select v-model:value="selectedDatabase" placeholder="选择数据库" @change="loadTables">
            <a-select-option
              v-for="db in databases"
              :key="db.name"
              :value="db.name"
            >
              {{ db.name }}
            </a-select-option>
          </a-select>
        </a-form-item>

        <a-form-item label="主表">
          <a-select
            v-model:value="mainTable"
            placeholder="选择主表"
            :disabled="!selectedDatabase"
            @change="loadTableColumns"
          >
            <a-select-option
              v-for="table in tables"
              :key="table.name"
              :value="table.name"
            >
              {{ table.name }}
            </a-select-option>
          </a-select>
        </a-form-item>
      </a-form>

      <a-divider>查询配置</a-divider>

      <!-- SELECT 子句 -->
      <div class="query-section">
        <h4>选择列 (SELECT)</h4>
        <a-checkbox-group v-model:value="selectedColumns" style="width: 100%;">
          <a-row>
            <a-col :span="8" v-for="col in columns" :key="col.name" style="margin-bottom: 8px;">
              <a-checkbox :value="col.name">
                {{ col.name }}
                <a-tag size="small" color="blue">{{ col.data_type }}</a-tag>
              </a-checkbox>
            </a-col>
          </a-row>
        </a-checkbox-group>
        <a-button size="small" @click="selectAllColumns" style="margin-top: 8px;">
          全选
        </a-button>
        <a-button size="small" @click="clearAllColumns" style="margin-left: 8px;">
          清空
        </a-button>
      </div>

      <!-- WHERE 子句 -->
      <div class="query-section">
        <h4>
          过滤条件 (WHERE)
          <a-button size="small" type="link" @click="addCondition">
            <PlusOutlined />
            添加条件
          </a-button>
        </h4>
        <div v-for="(condition, index) in conditions" :key="index" class="condition-row">
          <a-row :gutter="8" align="middle">
            <a-col :span="5">
              <a-select v-model:value="condition.column" placeholder="列">
                <a-select-option
                  v-for="col in columns"
                  :key="col.name"
                  :value="col.name"
                >
                  {{ col.name }}
                </a-select-option>
              </a-select>
            </a-col>
            <a-col :span="4">
              <a-select v-model:value="condition.operator" placeholder="操作符">
                <a-select-option value="=">=</a-select-option>
                <a-select-option value="!=">!=</a-select-option>
                <a-select-option value=">">></a-select-option>
                <a-select-option value="<"><</a-select-option>
                <a-select-option value=">=">>=</a-select-option>
                <a-select-option value="<="><=</a-select-option>
                <a-select-option value="LIKE">LIKE</a-select-option>
                <a-select-option value="IN">IN</a-select-option>
                <a-select-option value="IS NULL">IS NULL</a-select-option>
                <a-select-option value="IS NOT NULL">IS NOT NULL</a-select-option>
              </a-select>
            </a-col>
            <a-col :span="6">
              <a-input
                v-model:value="condition.value"
                placeholder="值"
                :disabled="condition.operator === 'IS NULL' || condition.operator === 'IS NOT NULL'"
              />
            </a-col>
            <a-col :span="3">
              <a-select v-model:value="condition.logic" placeholder="逻辑">
                <a-select-option value="AND">AND</a-select-option>
                <a-select-option value="OR">OR</a-select-option>
              </a-select>
            </a-col>
            <a-col :span="2">
              <a-button type="text" danger @click="removeCondition(index)">
                <DeleteOutlined />
              </a-button>
            </a-col>
          </a-row>
        </div>
      </div>

      <!-- ORDER BY 子句 -->
      <div class="query-section">
        <h4>排序 (ORDER BY)</h4>
        <a-row :gutter="8">
          <a-col :span="12">
            <a-select v-model:value="orderByColumn" placeholder="排序列" allow-clear>
              <a-select-option
                v-for="col in columns"
                :key="col.name"
                :value="col.name"
              >
                {{ col.name }}
              </a-select-option>
            </a-select>
          </a-col>
          <a-col :span="12">
            <a-select v-model:value="orderDirection" placeholder="排序方向">
              <a-select-option value="ASC">升序 (ASC)</a-select-option>
              <a-select-option value="DESC">降序 (DESC)</a-select-option>
            </a-select>
          </a-col>
        </a-row>
      </div>

      <!-- LIMIT 子句 -->
      <div class="query-section">
        <h4>限制行数 (LIMIT)</h4>
        <a-input-number v-model:value="limitRows" :min="1" :max="10000" style="width: 200px;" />
      </div>
    </div>

    <!-- 生成的SQL -->
    <div class="generated-sql">
      <a-divider>生成的SQL</a-divider>
      <div class="sql-preview">
        <pre>{{ generatedSql }}</pre>
        <div class="sql-actions">
          <a-space>
            <a-button @click="copySql">
              <CopyOutlined />
              复制
            </a-button>
            <a-button type="primary" @click="executeSql">
              <CaretRightOutlined />
              执行
            </a-button>
          </a-space>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import {
  PlusOutlined,
  DeleteOutlined,
  CopyOutlined,
  CaretRightOutlined,
} from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{
  connectionId: string | null
}>()

const emit = defineEmits(['execute-query'])

const databases = ref<any[]>([])
const selectedDatabase = ref('')
const tables = ref<any[]>([])
const mainTable = ref('')
const columns = ref<any[]>([])
const selectedColumns = ref<string[]>([])
const conditions = ref<any[]>([])
const orderByColumn = ref('')
const orderDirection = ref('ASC')
const limitRows = ref(100)

// 生成SQL
const generatedSql = computed(() => {
  if (!selectedDatabase.value || !mainTable.value) {
    return '-- 请选择数据库和表'
  }
  
  let sql = 'SELECT '
  
  // SELECT 子句
  if (selectedColumns.value.length === 0) {
    sql += '*'
  } else {
    sql += selectedColumns.value.map(col => `\`${col}\``).join(', ')
  }
  
  // FROM 子句
  sql += `\nFROM \`${selectedDatabase.value}\`.\`${mainTable.value}\``
  
  // WHERE 子句
  if (conditions.value.length > 0) {
    const whereConditions = conditions.value
      .filter(c => c.column && c.operator)
      .map((c, index) => {
        let condition = ''
        
        if (c.operator === 'IS NULL' || c.operator === 'IS NOT NULL') {
          condition = `\`${c.column}\` ${c.operator}`
        } else if (c.operator === 'LIKE') {
          condition = `\`${c.column}\` LIKE '%${c.value}%'`
        } else if (c.operator === 'IN') {
          condition = `\`${c.column}\` IN (${c.value})`
        } else {
          condition = `\`${c.column}\` ${c.operator} '${c.value}'`
        }
        
        if (index > 0 && c.logic) {
          return `${c.logic} ${condition}`
        }
        return condition
      })
      
    if (whereConditions.length > 0) {
      sql += `\nWHERE ${whereConditions.join('\n  ')}`
    }
  }
  
  // ORDER BY 子句
  if (orderByColumn.value) {
    sql += `\nORDER BY \`${orderByColumn.value}\` ${orderDirection.value}`
  }
  
  // LIMIT 子句
  sql += `\nLIMIT ${limitRows.value}`
  
  sql += ';'
  
  return sql
})

// 加载数据库列表
async function loadDatabases() {
  if (!props.connectionId) return
  
  try {
    const dbs = await invoke<any[]>('get_databases', {
      connectionId: props.connectionId,
    })
    databases.value = dbs
  } catch (error: any) {
    message.error(`加载数据库列表失败: ${error}`)
  }
}

// 加载表列表
async function loadTables() {
  if (!selectedDatabase.value || !props.connectionId) return
  
  try {
    const tbls = await invoke<any[]>('get_tables', {
      connectionId: props.connectionId,
      database: selectedDatabase.value,
    })
    tables.value = tbls
  } catch (error: any) {
    message.error(`加载表列表失败: ${error}`)
  }
}

// 加载表列
async function loadTableColumns() {
  if (!mainTable.value || !selectedDatabase.value || !props.connectionId) return
  
  try {
    const cols = await invoke<any[]>('get_table_structure', {
      connectionId: props.connectionId,
      table: mainTable.value,
      schema: selectedDatabase.value,
    })
    columns.value = cols
    selectedColumns.value = []
    conditions.value = []
  } catch (error: any) {
    message.error(`加载列失败: ${error}`)
  }
}

// 选择所有列
function selectAllColumns() {
  selectedColumns.value = columns.value.map(col => col.name)
}

// 清空所有列
function clearAllColumns() {
  selectedColumns.value = []
}

// 添加条件
function addCondition() {
  conditions.value.push({
    column: '',
    operator: '=',
    value: '',
    logic: 'AND',
  })
}

// 移除条件
function removeCondition(index: number) {
  conditions.value.splice(index, 1)
}

// 复制SQL
function copySql() {
  navigator.clipboard.writeText(generatedSql.value)
  message.success('SQL已复制到剪贴板')
}

// 执行SQL
function executeSql() {
  emit('execute-query', generatedSql.value)
  message.success('SQL已发送到编辑器')
}

// 初始化
watch(() => props.connectionId, (id) => {
  if (id) {
    loadDatabases()
  }
}, { immediate: true })
</script>

<style scoped>
.query-builder {
  padding: 24px;
  max-width: 1200px;
  margin: 0 auto;
}

.builder-header {
  margin-bottom: 24px;
}

.builder-header h3 {
  margin: 0 0 8px 0;
  font-size: 18px;
  font-weight: 600;
}

.builder-header p {
  margin: 0;
  color: #666;
  font-size: 14px;
}

.query-section {
  margin-bottom: 24px;
  padding: 16px;
  background: #fafafa;
  border-radius: 6px;
}

.dark-mode .query-section {
  background: #1a1a1a;
}

.query-section h4 {
  margin: 0 0 12px 0;
  font-size: 14px;
  font-weight: 600;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.condition-row {
  margin-bottom: 12px;
}

.generated-sql {
  margin-top: 24px;
}

.sql-preview {
  position: relative;
  background: #f5f5f5;
  border: 1px solid #e8e8e8;
  border-radius: 6px;
  padding: 16px;
}

.dark-mode .sql-preview {
  background: #262626;
  border-color: #303030;
}

.sql-preview pre {
  margin: 0;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
}

.sql-actions {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid #e8e8e8;
}

.dark-mode .sql-actions {
  border-top-color: #303030;
}
</style>

