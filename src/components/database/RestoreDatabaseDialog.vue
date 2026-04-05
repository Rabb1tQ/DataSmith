<template>
  <a-modal
    v-model:open="visible"
    :title="`导入SQL - ${database}`"
    width="600px"
    @ok="handleImport"
    @cancel="handleCancel"
    :confirm-loading="importing"
    ok-text="执行"
  >
    <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
      <a-form-item label="SQL文件" required>
        <a-input
          v-model:value="filePath"
          placeholder="点击选择SQL文件"
          readonly
          @click="selectFile"
        >
          <template #suffix>
            <FileOutlined style="cursor: pointer" @click="selectFile" />
          </template>
        </a-input>
      </a-form-item>

      <a-form-item label="跳过错误">
        <a-switch v-model:checked="skipErrors" />
        <span style="margin-left: 8px; color: #999; font-size: 12px;">
          遇到错误时继续执行后续语句
        </span>
      </a-form-item>
    </a-form>

    <a-alert
      message="提示"
      description="将执行SQL文件中的所有语句，请确保文件内容可信。"
      type="info"
      show-icon
      style="margin-top: 12px"
    />
  </a-modal>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { FileOutlined } from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

const props = defineProps<{
  modelValue: boolean
  connectionId: string
  database: string
}>()

const emit = defineEmits(['update:modelValue', 'imported'])

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
})

const importing = ref(false)
const filePath = ref('')
const skipErrors = ref(false)

async function selectFile() {
  const path = await open({
    filters: [{
      name: 'SQL文件',
      extensions: ['sql'],
    }],
    multiple: false,
  })

  if (path) {
    filePath.value = path as string
  }
}

async function handleImport() {
  if (!filePath.value) {
    message.error('请选择SQL文件')
    return
  }

  await doImport()
}

async function doImport() {
  importing.value = true
  try {
    // 读取SQL文件
    const sqlContent = await invoke<string>('read_file', {
      path: filePath.value,
    })

    // 使用后端的 execute_sql_script 命令，与SQL编辑器保持一致
    const result = await invoke<{
      success_count: number
      failed_count: number
      total_affected_rows: number
      total_time_ms: number
    }>('execute_sql_script', {
      connectionId: props.connectionId,
      sql: sqlContent,
      database: props.database,
    })

    if (result.failed_count > 0) {
      if (skipErrors.value) {
        message.warning(`导入完成！成功: ${result.success_count}，失败: ${result.failed_count}`)
      } else {
        message.error(`导入失败！成功: ${result.success_count}，失败: ${result.failed_count}`)
      }
    } else {
      message.success(`导入完成！成功: ${result.success_count}，耗时: ${result.total_time_ms}ms`)
    }
    
    emit('imported')
    handleCancel()
  } catch (error: any) {
    message.error(`导入失败: ${error}`)
  } finally {
    importing.value = false
  }
}

function handleCancel() {
  filePath.value = ''
  skipErrors.value = false
  visible.value = false
}
</script>

