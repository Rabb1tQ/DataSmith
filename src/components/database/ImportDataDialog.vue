<template>
  <a-modal
    v-model:open="visible"
    :title="`导入数据 - ${table}`"
    width="600px"
    @ok="handleImport"
    @cancel="handleCancel"
    :confirm-loading="importing"
  >
    <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
      <a-form-item label="文件格式" required>
        <a-radio-group v-model:value="importFormat">
          <a-radio value="csv">CSV</a-radio>
          <a-radio value="json">JSON</a-radio>
          <a-radio value="sql">SQL</a-radio>
        </a-radio-group>
      </a-form-item>

      <a-form-item label="选择文件" required>
        <a-input
          v-model:value="filePath"
          placeholder="点击选择要导入的文件"
          readonly
          @click="selectFile"
        >
          <template #suffix>
            <FileOutlined style="cursor: pointer" @click="selectFile" />
          </template>
        </a-input>
      </a-form-item>

      <a-form-item label="导入模式">
        <a-radio-group v-model:value="importMode">
          <a-radio value="insert">插入</a-radio>
          <a-radio value="replace">替换</a-radio>
          <a-radio value="truncate">清空后插入</a-radio>
        </a-radio-group>
      </a-form-item>

      <a-form-item v-if="importFormat === 'csv'" label="字段分隔符">
        <a-input v-model:value="delimiter" placeholder="默认为逗号" />
      </a-form-item>

      <a-form-item v-if="importFormat === 'csv'" label="包含表头">
        <a-switch v-model:checked="hasHeader" />
      </a-form-item>
    </a-form>

    <a-alert
      v-if="importMode === 'truncate'"
      message="警告"
      description="清空后插入模式将删除表中所有现有数据，此操作不可恢复！"
      type="warning"
      show-icon
      style="margin-top: 12px"
    />
  </a-modal>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { FileOutlined } from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

const props = defineProps<{
  modelValue: boolean
  connectionId: string
  database: string
  table: string
}>()

const emit = defineEmits(['update:modelValue', 'imported'])

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
})

const importing = ref(false)
const importFormat = ref('csv')
const importMode = ref('insert')
const filePath = ref('')
const delimiter = ref(',')
const hasHeader = ref(true)

async function selectFile() {
  const extensions: Record<string, string[]> = {
    csv: ['csv'],
    json: ['json'],
    sql: ['sql'],
  }

  const path = await open({
    filters: [{
      name: importFormat.value.toUpperCase(),
      extensions: extensions[importFormat.value],
    }],
    multiple: false,
  })

  if (path) {
    filePath.value = path as string
  }
}

async function handleImport() {
  if (!filePath.value) {
    message.error('请选择要导入的文件')
    return
  }

  if (importMode.value === 'truncate') {
    Modal.confirm({
      title: '确认清空表',
      content: '您选择了清空后插入模式，这将删除表中所有现有数据。确定继续吗？',
      okText: '确定',
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        await doImport()
      },
    })
  } else {
    await doImport()
  }
}

async function doImport() {
  importing.value = true
  try {
    // 如果是清空模式，先清空表
    if (importMode.value === 'truncate') {
      await invoke('truncate_table', {
        connectionId: props.connectionId,
        table: props.table,
        database: props.database,
      })
    }

    // 读取文件内容
    const fileContent = await invoke<string>('read_file', {
      path: filePath.value,
    })

    // 根据格式解析并导入
    if (importFormat.value === 'csv') {
      await importFromCSV(fileContent)
    } else if (importFormat.value === 'json') {
      await importFromJSON(fileContent)
    } else if (importFormat.value === 'sql') {
      await importFromSQL(fileContent)
    }

    message.success('导入成功')
    emit('imported')
    handleCancel()
  } catch (error: any) {
    message.error(`导入失败: ${error}`)
  } finally {
    importing.value = false
  }
}

async function importFromCSV(content: string) {
  const lines = content.split('\n').filter(line => line.trim())
  if (lines.length === 0) return

  let headers: string[] = []
  let startIndex = 0

  if (hasHeader.value) {
    headers = lines[0].split(delimiter.value).map(h => h.trim().replace(/^"|"$/g, ''))
    startIndex = 1
  } else {
    // 如果没有表头，使用表的列名
    const columns = await invoke<any[]>('get_table_structure', {
      connectionId: props.connectionId,
      table: props.table,
      schema: props.database,
    })
    headers = columns.map(col => col.name)
  }

  // 批量插入数据
  for (let i = startIndex; i < lines.length; i++) {
    const values = lines[i].split(delimiter.value).map(v => v.trim().replace(/^"|"$/g, ''))
    const data: Record<string, any> = {}

    headers.forEach((header, index) => {
      if (values[index] !== undefined && values[index] !== '') {
        data[header] = values[index]
      }
    })

    await invoke('insert_table_data', {
      connectionId: props.connectionId,
      database: props.database,
      table: props.table,
      data,
    })
  }
}

async function importFromJSON(content: string) {
  const data = JSON.parse(content)
  const rows = Array.isArray(data) ? data : [data]

  for (const row of rows) {
    await invoke('insert_table_data', {
      connectionId: props.connectionId,
      database: props.database,
      table: props.table,
      data: row,
    })
  }
}

async function importFromSQL(content: string) {
  // 直接执行SQL
  await invoke('execute_query', {
    connectionId: props.connectionId,
    sql: content,
    database: props.database,
  })
}

function handleCancel() {
  importFormat.value = 'csv'
  importMode.value = 'insert'
  filePath.value = ''
  delimiter.value = ','
  hasHeader.value = true
  visible.value = false
}
</script>

