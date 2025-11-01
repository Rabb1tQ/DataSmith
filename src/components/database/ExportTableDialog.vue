<template>
  <a-modal
    v-model:open="visible"
    :title="`导出表 - ${table}`"
    width="600px"
    @ok="handleExport"
    @cancel="handleCancel"
    :confirm-loading="exporting"
  >
    <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
      <a-form-item label="导出格式" required>
        <a-radio-group v-model:value="exportFormat">
          <a-radio value="csv">CSV</a-radio>
          <a-radio value="json">JSON</a-radio>
          <a-radio value="sql">SQL</a-radio>
        </a-radio-group>
      </a-form-item>

      <a-form-item label="导出内容">
        <a-radio-group v-model:value="exportType">
          <a-radio value="data">仅数据</a-radio>
          <a-radio value="structure">仅结构</a-radio>
          <a-radio value="both">结构和数据</a-radio>
        </a-radio-group>
      </a-form-item>

      <a-form-item label="保存位置" required>
        <a-input
          v-model:value="savePath"
          placeholder="点击选择保存位置"
          readonly
          @click="selectSavePath"
        >
          <template #suffix>
            <FolderOpenOutlined style="cursor: pointer" @click="selectSavePath" />
          </template>
        </a-input>
      </a-form-item>

      <a-form-item label="限制行数">
        <a-input-number
          v-model:value="limit"
          :min="0"
          placeholder="0表示无限制"
          style="width: 100%"
        />
      </a-form-item>
    </a-form>
  </a-modal>
</template>

<script setup lang="ts">
import { FolderOpenOutlined } from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'

const props = defineProps<{
  modelValue: boolean
  connectionId: string
  database: string
  table: string
}>()

const emit = defineEmits(['update:modelValue', 'exported'])

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
})

const exporting = ref(false)
const exportFormat = ref('csv')
const exportType = ref('both')
const savePath = ref('')
const limit = ref(0)

async function selectSavePath() {
  const extensions: Record<string, string[]> = {
    csv: ['csv'],
    json: ['json'],
    sql: ['sql'],
  }

  const path = await save({
    defaultPath: `${props.table}.${exportFormat.value}`,
    filters: [{
      name: exportFormat.value.toUpperCase(),
      extensions: extensions[exportFormat.value],
    }],
  })

  if (path) {
    savePath.value = path
  }
}

async function handleExport() {
  if (!savePath.value) {
    message.error('请选择保存位置')
    return
  }

  exporting.value = true
  try {
    // 先查询数据
    let sql = `SELECT * FROM \`${props.table}\``
    if (limit.value > 0) {
      sql += ` LIMIT ${limit.value}`
    }

    const result = await invoke<any>('execute_query', {
      connectionId: props.connectionId,
      sql,
      database: props.database,
    })

    // 根据格式导出
    if (exportFormat.value === 'csv') {
      await invoke('export_to_csv', {
        data: result,
        path: savePath.value,
      })
    } else if (exportFormat.value === 'json') {
      await invoke('export_to_json', {
        data: result,
        path: savePath.value,
      })
    } else if (exportFormat.value === 'sql') {
      await invoke('export_to_sql', {
        data: result,
        tableName: props.table,
        path: savePath.value,
      })
    }

    message.success('导出成功')
    emit('exported')
    handleCancel()
  } catch (error: any) {
    message.error(`导出失败: ${error}`)
  } finally {
    exporting.value = false
  }
}

function handleCancel() {
  exportFormat.value = 'csv'
  exportType.value = 'both'
  savePath.value = ''
  limit.value = 0
  visible.value = false
}
</script>

