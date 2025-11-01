<template>
  <a-modal
    v-model:open="visible"
    title="新建表"
    width="900px"
    @ok="handleCreate"
    @cancel="handleCancel"
    :confirm-loading="creating"
  >
    <a-form :label-col="{ span: 4 }" :wrapper-col="{ span: 20 }">
      <a-form-item label="表名" required>
        <a-input v-model:value="tableName" placeholder="请输入表名" />
      </a-form-item>
      
      <a-form-item label="字符集">
        <a-select v-model:value="charset" placeholder="选择字符集">
          <a-select-option value="utf8mb4">utf8mb4</a-select-option>
          <a-select-option value="utf8">utf8</a-select-option>
          <a-select-option value="latin1">latin1</a-select-option>
        </a-select>
      </a-form-item>
      
      <a-form-item label="存储引擎">
        <a-select v-model:value="engine" placeholder="选择存储引擎">
          <a-select-option value="InnoDB">InnoDB</a-select-option>
          <a-select-option value="MyISAM">MyISAM</a-select-option>
        </a-select>
      </a-form-item>
      
      <a-form-item label="表注释">
        <a-input v-model:value="comment" placeholder="请输入表注释" />
      </a-form-item>
      
      <a-divider>字段定义</a-divider>
      
      <a-button type="dashed" block @click="addColumn" style="margin-bottom: 12px">
        <PlusOutlined />
        添加字段
      </a-button>
      
      <a-table
        :columns="columnTableColumns"
        :data-source="columns"
        :pagination="false"
        size="small"
        bordered
      >
        <template #bodyCell="{ column, record, index }">
          <template v-if="column.key === 'name'">
            <a-input v-model:value="record.name" placeholder="字段名" size="small" />
          </template>
          <template v-else-if="column.key === 'type'">
            <a-select v-model:value="record.type" size="small" style="width: 100%">
              <a-select-option value="INT">INT</a-select-option>
              <a-select-option value="BIGINT">BIGINT</a-select-option>
              <a-select-option value="VARCHAR">VARCHAR</a-select-option>
              <a-select-option value="TEXT">TEXT</a-select-option>
              <a-select-option value="DATETIME">DATETIME</a-select-option>
              <a-select-option value="TIMESTAMP">TIMESTAMP</a-select-option>
              <a-select-option value="DECIMAL">DECIMAL</a-select-option>
              <a-select-option value="BOOLEAN">BOOLEAN</a-select-option>
            </a-select>
          </template>
          <template v-else-if="column.key === 'length'">
            <a-input v-model:value="record.length" placeholder="长度" size="small" />
          </template>
          <template v-else-if="column.key === 'nullable'">
            <a-checkbox v-model:checked="record.nullable" />
          </template>
          <template v-else-if="column.key === 'primary'">
            <a-checkbox v-model:checked="record.primary" />
          </template>
          <template v-else-if="column.key === 'autoIncrement'">
            <a-checkbox v-model:checked="record.autoIncrement" />
          </template>
          <template v-else-if="column.key === 'defaultValue'">
            <a-input v-model:value="record.defaultValue" placeholder="默认值" size="small" />
          </template>
          <template v-else-if="column.key === 'comment'">
            <a-input v-model:value="record.comment" placeholder="注释" size="small" />
          </template>
          <template v-else-if="column.key === 'action'">
            <a-button type="link" danger size="small" @click="removeColumn(index)">
              删除
            </a-button>
          </template>
        </template>
      </a-table>
    </a-form>
  </a-modal>
</template>

<script setup lang="ts">
import { PlusOutlined } from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'

interface Column {
  name: string
  type: string
  length: string
  nullable: boolean
  primary: boolean
  autoIncrement: boolean
  defaultValue: string
  comment: string
}

const props = defineProps<{
  modelValue: boolean
  connectionId: string
  database: string
}>()

const emit = defineEmits(['update:modelValue', 'created'])

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
})

const creating = ref(false)
const tableName = ref('')
const charset = ref('utf8mb4')
const engine = ref('InnoDB')
const comment = ref('')
const columns = ref<Column[]>([])

const columnTableColumns = [
  { title: '字段名', key: 'name', width: 120 },
  { title: '类型', key: 'type', width: 100 },
  { title: '长度', key: 'length', width: 80 },
  { title: '允许空', key: 'nullable', width: 70 },
  { title: '主键', key: 'primary', width: 60 },
  { title: '自增', key: 'autoIncrement', width: 60 },
  { title: '默认值', key: 'defaultValue', width: 100 },
  { title: '注释', key: 'comment', width: 120 },
  { title: '操作', key: 'action', width: 80 },
]

function addColumn() {
  columns.value.push({
    name: '',
    type: 'VARCHAR',
    length: '255',
    nullable: true,
    primary: false,
    autoIncrement: false,
    defaultValue: '',
    comment: '',
  })
}

function removeColumn(index: number) {
  columns.value.splice(index, 1)
}

function generateCreateTableSql(): string {
  if (!tableName.value || columns.value.length === 0) {
    return ''
  }

  const columnDefs = columns.value.map(col => {
    let def = `\`${col.name}\` ${col.type}`
    
    if (col.length && ['VARCHAR', 'CHAR', 'DECIMAL'].includes(col.type)) {
      def += `(${col.length})`
    }
    
    if (!col.nullable) {
      def += ' NOT NULL'
    }
    
    if (col.autoIncrement) {
      def += ' AUTO_INCREMENT'
    }
    
    if (col.defaultValue) {
      def += ` DEFAULT '${col.defaultValue}'`
    }
    
    if (col.comment) {
      def += ` COMMENT '${col.comment}'`
    }
    
    return def
  })

  const primaryKeys = columns.value.filter(col => col.primary).map(col => col.name)
  if (primaryKeys.length > 0) {
    columnDefs.push(`PRIMARY KEY (\`${primaryKeys.join('`, `')}\`)`)
  }

  let sql = `CREATE TABLE \`${tableName.value}\` (\n  ${columnDefs.join(',\n  ')}\n)`
  
  sql += ` ENGINE=${engine.value} DEFAULT CHARSET=${charset.value}`
  
  if (comment.value) {
    sql += ` COMMENT='${comment.value}'`
  }

  return sql
}

async function handleCreate() {
  if (!tableName.value.trim()) {
    message.error('请输入表名')
    return
  }

  if (columns.value.length === 0) {
    message.error('请至少添加一个字段')
    return
  }

  // 验证字段名
  for (const col of columns.value) {
    if (!col.name.trim()) {
      message.error('请填写所有字段名')
      return
    }
  }

  creating.value = true
  try {
    const sql = generateCreateTableSql()
    
    await invoke('execute_query', {
      connectionId: props.connectionId,
      sql,
      database: props.database,
    })

    message.success('表创建成功')
    emit('created')
    handleCancel()
  } catch (error: any) {
    message.error(`创建表失败: ${error}`)
  } finally {
    creating.value = false
  }
}

function handleCancel() {
  tableName.value = ''
  charset.value = 'utf8mb4'
  engine.value = 'InnoDB'
  comment.value = ''
  columns.value = []
  visible.value = false
}

// 初始化一个默认字段
watch(visible, (newVal) => {
  if (newVal && columns.value.length === 0) {
    addColumn()
    columns.value[0].name = 'id'
    columns.value[0].type = 'INT'
    columns.value[0].primary = true
    columns.value[0].autoIncrement = true
    columns.value[0].nullable = false
  }
})
</script>

<style scoped>
:deep(.ant-table) {
  font-size: 12px;
}
</style>

