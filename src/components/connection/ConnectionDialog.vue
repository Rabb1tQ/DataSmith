<template>
  <a-modal
    v-model:open="dialogVisible"
    :title="props.editingConnection ? '编辑连接' : '新建连接'"
    :width="600"
    @ok="handleSubmit"
    @cancel="handleCancel"
  >
    <a-form
      ref="formRef"
      :model="formData"
      :rules="rules"
      :label-col="{ span: 6 }"
      :wrapper-col="{ span: 18 }"
    >
      <a-form-item label="连接名称" name="name">
        <a-input v-model:value="formData.name" placeholder="请输入连接名称" />
      </a-form-item>

      <a-form-item label="数据库类型" name="db_type">
        <a-select v-model:value="formData.db_type" placeholder="请选择数据库类型">
          <a-select-option value="mysql">MySQL</a-select-option>
          <a-select-option value="postgresql">PostgreSQL</a-select-option>
          <a-select-option value="sqlite">SQLite</a-select-option>
          <a-select-option value="mongodb">MongoDB</a-select-option>
          <a-select-option value="redis">Redis</a-select-option>
        </a-select>
      </a-form-item>

      <a-form-item 
        v-if="formData.db_type !== 'sqlite'" 
        label="主机" 
        name="host"
      >
        <a-input v-model:value="formData.host" placeholder="localhost" />
      </a-form-item>

      <a-form-item 
        v-if="formData.db_type !== 'sqlite'" 
        label="端口" 
        name="port"
      >
        <a-input-number
          v-model:value="formData.port"
          :min="1"
          :max="65535"
          style="width: 100%"
        />
      </a-form-item>

      <a-form-item 
        v-if="formData.db_type !== 'sqlite'" 
        label="用户名" 
        name="username"
      >
        <a-input 
          v-model:value="formData.username" 
          :placeholder="formData.db_type === 'redis' || formData.db_type === 'mongodb' ? '可选' : 'root'" 
        />
      </a-form-item>

      <a-form-item 
        v-if="formData.db_type !== 'sqlite'" 
        label="密码" 
        name="password"
      >
        <a-input-password 
          v-model:value="formData.password" 
          :placeholder="formData.db_type === 'redis' ? '可选，留空表示无密码' : '请输入密码'" 
        />
      </a-form-item>

      <a-form-item label="数据库" name="database">
        <a-input-group 
          v-if="formData.db_type === 'sqlite'" 
          compact
        >
          <a-input
            v-model:value="formData.database"
            placeholder="数据库文件路径，例如：C:\data\mydb.db 或 :memory:"
            style="width: calc(100% - 80px)"
          />
          <a-button @click="handleSelectFile">选择文件</a-button>
        </a-input-group>
        <a-input
          v-else-if="formData.db_type === 'redis'"
          v-model:value="formData.database"
          placeholder="数据库编号 (0-15)，默认为 0"
        />
        <a-input
          v-else
          v-model:value="formData.database"
          placeholder="可选，留空连接到服务器"
        />
      </a-form-item>

      <a-form-item label="SSL 连接" name="ssl">
        <a-switch v-model:checked="formData.ssl" />
      </a-form-item>

      <a-form-item label="连接超时(秒)" name="connection_timeout">
        <a-input-number
          v-model:value="formData.connection_timeout"
          :min="1"
          :max="300"
          style="width: 100%"
        />
      </a-form-item>
    </a-form>

    <template #footer>
      <a-space>
        <a-button @click="handleCancel">取消</a-button>
        <a-button :loading="testing" @click="handleTest">测试连接</a-button>
        <a-button type="primary" :loading="submitting" @click="handleSubmit">
          保存
        </a-button>
      </a-space>
    </template>
  </a-modal>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'
import { message, Modal } from 'ant-design-vue'
import { useConnectionStore } from '@/stores/connection'
import type { ConnectionConfig, DatabaseType } from '@/types/database'
import { open } from '@tauri-apps/plugin-dialog'

const props = defineProps<{
  visible: boolean
  editingConnection?: any
}>()

const emit = defineEmits(['update:visible', 'close'])

const connectionStore = useConnectionStore()
const formRef = ref()
const testing = ref(false)
const submitting = ref(false)

const dialogVisible = computed({
  get: () => props.visible,
  set: (val) => emit('update:visible', val),
})

// 表单数据
const formData = reactive<{
  name: string
  db_type: DatabaseType
  host: string
  port: number
  username: string
  password: string
  database: string
  ssl: boolean
  connection_timeout: number
  pool_size: number
}>({
  name: '',
  db_type: 'mysql',
  host: 'localhost',
  port: 3306,
  username: 'root',
  password: '',
  database: '',
  ssl: false,
  connection_timeout: 10,
  pool_size: 10,
})

// 表单验证规则
const rules = computed(() => {
  const baseRules: any = {
    name: [{ required: true, message: '请输入连接名称' }],
    db_type: [{ required: true, message: '请选择数据库类型' }],
  }
  
  // SQLite 不需要主机和端口
  if (formData.db_type !== 'sqlite') {
    baseRules.host = [{ required: true, message: '请输入主机地址' }]
    baseRules.port = [{ required: true, message: '请输入端口号' }]
  }
  
  // Redis、MongoDB 和 SQLite 的用户名可选，其他数据库类型必填
  if (formData.db_type !== 'redis' && formData.db_type !== 'mongodb' && formData.db_type !== 'sqlite') {
    baseRules.username = [{ required: true, message: '请输入用户名' }]
  }
  
  return baseRules
})

// 监听编辑连接变化，填充表单
watch(
  () => props.editingConnection,
  (connection) => {
    if (connection) {
      Object.assign(formData, {
        name: connection.name || '',
        db_type: connection.db_type || 'mysql',
        host: connection.host || 'localhost',
        port: connection.port || 3306,
        username: connection.username || 'root',
        password: '', // 密码不回填，安全考虑
        database: connection.database || '',
        ssl: connection.ssl || false,
        connection_timeout: connection.connection_timeout || 10,
        pool_size: connection.pool_size || 10,
      })
    } else {
      // 如果没有编辑连接，重置表单
      resetForm()
    }
  },
  { immediate: true }
)

// 监听对话框打开/关闭
watch(
  () => props.visible,
  (visible) => {
    if (visible && !props.editingConnection) {
      // 新建模式下，确保表单已重置
      resetForm()
    }
  }
)

// 监听数据库类型变化，自动设置默认端口
watch(
  () => formData.db_type,
  (type) => {
    // 只在新建模式下自动设置端口
    if (!props.editingConnection) {
      const portMap: Record<string, number> = {
        mysql: 3306,
        postgresql: 5432,
        mongodb: 27017,
        redis: 6379,
        sqlite: 0,
      }
      formData.port = portMap[type] || 3306
    }
  }
)

// 测试连接
async function handleTest() {
  try {
    await formRef.value.validate()
    
    // 如果是编辑模式且密码为空，提示用户输入密码（Redis、MongoDB 和 SQLite 除外）
    if (props.editingConnection && !formData.password && 
        formData.db_type !== 'redis' && 
        formData.db_type !== 'mongodb' && 
        formData.db_type !== 'sqlite') {
      message.warning('请输入密码以测试连接')
      return
    }
    
    testing.value = true
    
    const config: Partial<ConnectionConfig> = {
      ...formData,
      id: '', // 测试时不需要 ID
    }
    
    const result = await connectionStore.testConnection(config as ConnectionConfig)
    message.success(`连接测试成功！响应时间: ${result.ping_time_ms}ms`)
  } catch (error: any) {
    // 提取有用的错误信息
    let errorMessage = error?.message || '连接测试失败'
    
    // 针对常见错误提供更友好的提示
    if (errorMessage.includes('timed out')) {
      errorMessage = '连接超时，请检查：\n1. 数据库服务是否正在运行\n2. 主机地址和端口是否正确\n3. 网络连接是否正常\n4. 防火墙是否允许连接'
    } else if (errorMessage.includes('Access denied')) {
      errorMessage = '访问被拒绝，请检查用户名和密码是否正确'
    } else if (errorMessage.includes('Unknown database')) {
      errorMessage = '数据库不存在，请检查数据库名称是否正确'
    } else if (errorMessage.includes('Can\'t connect')) {
      errorMessage = '无法连接到数据库服务器，请检查主机地址和端口'
    }
    
    Modal.error({
      title: '连接测试失败',
      content: errorMessage,
      width: 500,
    })
  } finally {
    testing.value = false
  }
}

// 提交保存
async function handleSubmit() {
  try {
    await formRef.value.validate()
    submitting.value = true
    
    if (props.editingConnection) {
      // 编辑模式
      const config: ConnectionConfig = {
        ...formData,
        id: props.editingConnection.id,
        tags: props.editingConnection.tags || [],
        created_at: props.editingConnection.created_at,
        updated_at: Date.now(),
      }
      
      await connectionStore.updateConnection(config, formData.password)
      message.success('连接更新成功！')
    } else {
      // 新建模式
      const id = crypto.randomUUID()
      const config: ConnectionConfig = {
        ...formData,
        id,
        tags: [],
        created_at: Date.now(),
        updated_at: Date.now(),
      }
      
      await connectionStore.saveConnection(config, formData.password)
      message.success('连接保存成功！')
    }
    
    dialogVisible.value = false
    resetForm()
  } catch (error: any) {
    message.error(error?.message || '保存连接失败')
  } finally {
    submitting.value = false
  }
}

// 取消
function handleCancel() {
  dialogVisible.value = false
  resetForm()
  emit('close')
}

// 选择 SQLite 数据库文件
async function handleSelectFile() {
  try {
    const selected = await open({
      multiple: false,
      filters: [{
        name: 'SQLite Database',
        extensions: ['db', 'sqlite', 'sqlite3', 'db3']
      }]
    })
    
    if (selected) {
      formData.database = selected as string
    }
  } catch (error: any) {
    message.error(`选择文件失败: ${error.message || error}`)
  }
}

// 重置表单
function resetForm() {
  formRef.value?.resetFields()
  // 重置为默认值
  Object.assign(formData, {
    name: '',
    db_type: 'mysql',
    host: 'localhost',
    port: 3306,
    username: 'root',
    password: '',
    database: '',
    ssl: false,
    connection_timeout: 10,
    pool_size: 10,
  })
}
</script>

