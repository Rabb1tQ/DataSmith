<template>
  <a-modal
    :open="visible"
    title="新建数据库"
    @ok="handleCreate"
    @cancel="handleCancel"
    :confirm-loading="loading"
  >
    <a-form
      ref="formRef"
      :model="formState"
      :label-col="{ span: 6 }"
      :wrapper-col="{ span: 18 }"
    >
      <a-form-item
        label="数据库名称"
        name="databaseName"
        :rules="[
          { required: true, message: '请输入数据库名称' },
          { pattern: /^[a-zA-Z0-9_]+$/, message: '只能包含字母、数字和下划线' }
        ]"
      >
        <a-input
          v-model:value="formState.databaseName"
          placeholder="例如: my_database"
          @pressEnter="handleCreate"
        />
      </a-form-item>

      <a-form-item
        v-if="isMysql"
        label="字符集"
        name="charset"
      >
        <a-select v-model:value="formState.charset">
          <a-select-option value="utf8mb4">utf8mb4 (推荐)</a-select-option>
          <a-select-option value="utf8">utf8</a-select-option>
          <a-select-option value="latin1">latin1</a-select-option>
          <a-select-option value="gbk">gbk</a-select-option>
        </a-select>
      </a-form-item>

      <a-form-item
        v-if="isMysql"
        label="排序规则"
        name="collation"
      >
        <a-select v-model:value="formState.collation">
          <a-select-option value="utf8mb4_general_ci">utf8mb4_general_ci</a-select-option>
          <a-select-option value="utf8mb4_unicode_ci">utf8mb4_unicode_ci</a-select-option>
          <a-select-option value="utf8mb4_bin">utf8mb4_bin</a-select-option>
          <a-select-option value="utf8_general_ci">utf8_general_ci</a-select-option>
          <a-select-option value="utf8_unicode_ci">utf8_unicode_ci</a-select-option>
        </a-select>
      </a-form-item>
    </a-form>
  </a-modal>
</template>

<script setup lang="ts">
import { reactive, ref, watch, computed } from 'vue'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import type { FormInstance } from 'ant-design-vue'

const props = defineProps<{
  visible: boolean
  connectionId: string
  dbType?: string
}>()

const emit = defineEmits(['update:visible', 'created'])

const formRef = ref<FormInstance>()
const loading = ref(false)

// 判断是否为 MySQL
const isMysql = computed(() => {
  return props.dbType?.toLowerCase() === 'mysql'
})

const formState = reactive({
  databaseName: '',
  charset: 'utf8mb4',
  collation: 'utf8mb4_general_ci',
})

// 重置表单
function resetForm() {
  formState.databaseName = ''
  formState.charset = 'utf8mb4'
  formState.collation = 'utf8mb4_general_ci'
  formRef.value?.resetFields()
}

// 创建数据库
async function handleCreate() {
  try {
    await formRef.value?.validate()
    
    loading.value = true
    
    // SQLite 不支持 CREATE DATABASE 语句
    const dbType = props.dbType?.toLowerCase()
    if (dbType === 'sqlite') {
      message.error('SQLite 不支持通过 SQL 创建数据库，请直接创建新的连接指向新文件')
      return
    }
    
    // 根据数据库类型构建 CREATE DATABASE 语句
    let sql = ''
    if (dbType === 'mysql') {
      // MySQL 语法
      sql = `CREATE DATABASE \`${formState.databaseName}\` 
        CHARACTER SET ${formState.charset} 
        COLLATE ${formState.collation}`
    } else if (dbType === 'postgresql') {
      // PostgreSQL 语法（不支持 CHARSET 和 COLLATE 在 CREATE DATABASE 语句中）
      sql = `CREATE DATABASE "${formState.databaseName}"`
    } else {
      // 默认使用 MySQL 语法
      sql = `CREATE DATABASE \`${formState.databaseName}\` 
        CHARACTER SET ${formState.charset} 
        COLLATE ${formState.collation}`
    }
    
    await invoke('execute_query', {
      connectionId: props.connectionId,
      sql,
    })
    
    message.success(`数据库 "${formState.databaseName}" 创建成功`)
    
    emit('created', formState.databaseName)
    emit('update:visible', false)
    
    resetForm()
  } catch (error: any) {
    if (error.errorFields) {
      // 表单验证错误
      return
    }
    message.error(`创建数据库失败: ${error}`)
  } finally {
    loading.value = false
  }
}

// 取消
function handleCancel() {
  emit('update:visible', false)
  resetForm()
}

// 监听对话框关闭
watch(() => props.visible, (visible) => {
  if (!visible) {
    resetForm()
  }
})
</script>

<style scoped>
/* 样式可以根据需要添加 */
</style>

