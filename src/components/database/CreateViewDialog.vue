<template>
  <a-modal
    v-model:open="visible"
    title="新建视图"
    width="800px"
    @ok="handleCreate"
    @cancel="handleCancel"
    :confirm-loading="creating"
  >
    <a-form :label-col="{ span: 4 }" :wrapper-col="{ span: 20 }">
      <a-form-item label="视图名" required>
        <a-input v-model:value="viewName" placeholder="请输入视图名" />
      </a-form-item>
      
      <a-form-item label="SQL查询" required>
        <div ref="editorContainer" style="height: 300px; border: 1px solid #d9d9d9; border-radius: 4px;"></div>
      </a-form-item>
      
      <a-form-item label="注释">
        <a-input v-model:value="comment" placeholder="请输入视图注释" />
      </a-form-item>
    </a-form>

    <a-alert
      message="提示"
      description="请输入SELECT查询语句来定义视图内容，例如：SELECT id, name, email FROM users WHERE active = 1"
      type="info"
      show-icon
      style="margin-top: 12px"
    />
  </a-modal>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import * as monaco from 'monaco-editor'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'

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
const viewName = ref('')
const comment = ref('')
const editorContainer = ref<HTMLElement>()
let editor: monaco.editor.IStandaloneCodeEditor | null = null

onMounted(() => {
  if (!editorContainer.value) return

  editor = monaco.editor.create(editorContainer.value, {
    value: 'SELECT * FROM table_name',
    language: 'sql',
    theme: 'vs',
    automaticLayout: true,
    fontSize: 13,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    lineNumbers: 'on',
  })
})

onUnmounted(() => {
  editor?.dispose()
})

function generateCreateViewSql(): string {
  if (!viewName.value || !editor) {
    return ''
  }

  const selectSql = editor.getValue().trim()
  let sql = `CREATE VIEW \`${viewName.value}\` AS ${selectSql}`
  
  return sql
}

async function handleCreate() {
  if (!viewName.value.trim()) {
    message.error('请输入视图名')
    return
  }

  const selectSql = editor?.getValue().trim()
  if (!selectSql) {
    message.error('请输入SELECT查询语句')
    return
  }

  creating.value = true
  try {
    const sql = generateCreateViewSql()
    
    await invoke('execute_query', {
      connectionId: props.connectionId,
      sql,
      database: props.database,
    })

    message.success('视图创建成功')
    emit('created')
    handleCancel()
  } catch (error: any) {
    message.error(`创建视图失败: ${error}`)
  } finally {
    creating.value = false
  }
}

function handleCancel() {
  viewName.value = ''
  comment.value = ''
  editor?.setValue('SELECT * FROM table_name')
  visible.value = false
}
</script>

<style scoped>
:deep(.ant-form-item-control-input-content) {
  line-height: 1;
}
</style>

