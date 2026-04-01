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
import { message, Modal } from 'ant-design-vue'
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

    message.success(`导入完成！成功: ${successCount}，失败: ${errorCount}`)
    emit('imported')
    handleCancel()
  } catch (error: any) {
    message.error(`导入失败: ${error}`)
  } finally {
    importing.value = false
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
  skipErrors.value = false
  visible.value = false
}
</script>

