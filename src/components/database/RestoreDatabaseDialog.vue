<template>
  <a-modal
    v-model:open="visible"
    :title="`还原数据库 - ${database}`"
    width="600px"
    @ok="handleRestore"
    @cancel="handleCancel"
    :confirm-loading="restoring"
  >
    <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
      <a-form-item label="备份文件" required>
        <a-input
          v-model:value="filePath"
          placeholder="点击选择SQL备份文件"
          readonly
          @click="selectFile"
        >
          <template #suffix>
            <FileOutlined style="cursor: pointer" @click="selectFile" />
          </template>
        </a-input>
      </a-form-item>

      <a-form-item label="还原模式">
        <a-radio-group v-model:value="restoreMode">
          <a-radio value="append">追加数据</a-radio>
          <a-radio value="replace">替换数据</a-radio>
        </a-radio-group>
      </a-form-item>

      <a-form-item label="跳过错误">
        <a-switch v-model:checked="skipErrors" />
        <span style="margin-left: 8px; color: #999; font-size: 12px;">
          遇到错误时继续执行
        </span>
      </a-form-item>
    </a-form>

    <a-alert
      v-if="restoreMode === 'replace'"
      message="警告"
      description="替换模式将删除现有数据，此操作不可恢复！建议先备份当前数据。"
      type="warning"
      show-icon
      style="margin-top: 12px"
    />
  </a-modal>
</template>

<script setup lang="ts">
import { FileOutlined } from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

const props = defineProps<{
  modelValue: boolean
  connectionId: string
  database: string
}>()

const emit = defineEmits(['update:modelValue', 'restored'])

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
})

const restoring = ref(false)
const filePath = ref('')
const restoreMode = ref('append')
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

async function handleRestore() {
  if (!filePath.value) {
    message.error('请选择备份文件')
    return
  }

  if (restoreMode.value === 'replace') {
    Modal.confirm({
      title: '确认还原数据库',
      content: '您选择了替换模式，这将删除数据库中的现有数据。建议先备份。确定继续吗？',
      okText: '确定',
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        await doRestore()
      },
    })
  } else {
    await doRestore()
  }
}

async function doRestore() {
  restoring.value = true
  try {
    // 读取SQL文件
    const sqlContent = await invoke<string>('read_file', {
      path: filePath.value,
    })

    // 分割SQL语句（按分号分割，但要注意字符串和注释中的分号）
    const statements = splitSqlStatements(sqlContent)

    let successCount = 0
    let errorCount = 0

    for (const statement of statements) {
      const sql = statement.trim()
      if (!sql || sql.startsWith('--')) continue

      try {
        await invoke('execute_query', {
          connectionId: props.connectionId,
          sql,
          database: props.database,
        })
        successCount++
      } catch (error: any) {
        errorCount++
        if (!skipErrors.value) {
          throw new Error(`执行SQL失败: ${error}`)
        }
        console.error('SQL执行错误（已跳过）:', error)
      }
    }

    message.success(`还原完成！成功: ${successCount}，失败: ${errorCount}`)
    emit('restored')
    handleCancel()
  } catch (error: any) {
    message.error(`还原失败: ${error}`)
  } finally {
    restoring.value = false
  }
}

function splitSqlStatements(sql: string): string[] {
  const statements: string[] = []
  let current = ''
  let inString = false
  let stringChar = ''
  let inComment = false

  for (let i = 0; i < sql.length; i++) {
    const char = sql[i]
    const nextChar = sql[i + 1]

    // 处理注释
    if (!inString && char === '-' && nextChar === '-') {
      inComment = true
      current += char
      continue
    }

    if (inComment && char === '\n') {
      inComment = false
      current += char
      continue
    }

    if (inComment) {
      current += char
      continue
    }

    // 处理字符串
    if (!inString && (char === '"' || char === "'")) {
      inString = true
      stringChar = char
      current += char
      continue
    }

    if (inString && char === stringChar && sql[i - 1] !== '\\') {
      inString = false
      current += char
      continue
    }

    // 处理分号
    if (!inString && char === ';') {
      current += char
      statements.push(current.trim())
      current = ''
      continue
    }

    current += char
  }

  if (current.trim()) {
    statements.push(current.trim())
  }

  return statements
}

function handleCancel() {
  filePath.value = ''
  restoreMode.value = 'append'
  skipErrors.value = false
  visible.value = false
}
</script>

