<template>
  <a-modal
    v-model:open="visible"
    :title="`插入记录 - ${table}`"
    width="700px"
    @ok="handleInsert"
    @cancel="handleCancel"
    :confirm-loading="inserting"
  >
    <a-spin :spinning="loadingColumns">
      <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
        <a-form-item
          v-for="col in columns"
          :key="col.name"
          :label="col.name"
          :required="!col.nullable && !col.is_auto_increment"
        >
          <a-input
            v-if="['VARCHAR', 'CHAR', 'TEXT'].includes(col.data_type)"
            v-model:value="formData[col.name]"
            :placeholder="getPlaceholder(col)"
            :disabled="col.is_auto_increment"
          />
          <a-input-number
            v-else-if="['INT', 'BIGINT', 'SMALLINT', 'TINYINT', 'DECIMAL', 'FLOAT', 'DOUBLE'].includes(col.data_type)"
            v-model:value="formData[col.name]"
            style="width: 100%"
            :placeholder="getPlaceholder(col)"
            :disabled="col.is_auto_increment"
          />
          <a-date-picker
            v-else-if="['DATE', 'DATETIME', 'TIMESTAMP'].includes(col.data_type)"
            v-model:value="formData[col.name]"
            show-time
            style="width: 100%"
            :placeholder="getPlaceholder(col)"
            :disabled="col.is_auto_increment"
          />
          <a-switch
            v-else-if="['BOOLEAN', 'TINYINT(1)'].includes(col.data_type)"
            v-model:checked="formData[col.name]"
            :disabled="col.is_auto_increment"
          />
          <a-textarea
            v-else
            v-model:value="formData[col.name]"
            :placeholder="getPlaceholder(col)"
            :disabled="col.is_auto_increment"
            :rows="3"
          />
          <div v-if="col.comment" style="font-size: 12px; color: #999; margin-top: 4px;">
            {{ col.comment }}
          </div>
        </a-form-item>
      </a-form>
    </a-spin>
  </a-modal>
</template>

<script setup lang="ts">
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'

interface Column {
  name: string
  data_type: string
  nullable: boolean
  is_auto_increment: boolean
  default_value: string | null
  comment: string
}

const props = defineProps<{
  modelValue: boolean
  connectionId: string
  database: string
  table: string
}>()

const emit = defineEmits(['update:modelValue', 'inserted'])

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
})

const inserting = ref(false)
const loadingColumns = ref(false)
const columns = ref<Column[]>([])
const formData = ref<Record<string, any>>({})

function getPlaceholder(col: Column): string {
  if (col.is_auto_increment) {
    return '自动生成'
  }
  if (col.default_value) {
    return `默认: ${col.default_value}`
  }
  if (col.nullable) {
    return '可选'
  }
  return '必填'
}

async function loadTableStructure() {
  if (!props.table) return

  loadingColumns.value = true
  try {
    const result = await invoke<Column[]>('get_table_structure', {
      connectionId: props.connectionId,
      table: props.table,
      schema: props.database,
    })

    columns.value = result
    formData.value = {}
  } catch (error: any) {
    message.error(`加载表结构失败: ${error}`)
  } finally {
    loadingColumns.value = false
  }
}

async function handleInsert() {
  // 验证必填字段
  for (const col of columns.value) {
    if (!col.nullable && !col.is_auto_increment && !formData.value[col.name]) {
      message.error(`请填写必填字段: ${col.name}`)
      return
    }
  }

  inserting.value = true
  try {
    // 过滤掉自增字段和空值
    const data: Record<string, any> = {}
    for (const col of columns.value) {
      if (!col.is_auto_increment && formData.value[col.name] !== undefined && formData.value[col.name] !== '') {
        data[col.name] = formData.value[col.name]
      }
    }

    await invoke('insert_table_data', {
      connectionId: props.connectionId,
      database: props.database,
      table: props.table,
      data,
    })

    message.success('记录插入成功')
    emit('inserted')
    handleCancel()
  } catch (error: any) {
    message.error(`插入记录失败: ${error}`)
  } finally {
    inserting.value = false
  }
}

function handleCancel() {
  formData.value = {}
  visible.value = false
}

watch(visible, (newVal) => {
  if (newVal) {
    loadTableStructure()
  }
})
</script>

<style scoped>
:deep(.ant-form-item) {
  margin-bottom: 16px;
}
</style>

