<template>
  <div class="redis-key-viewer">
    <a-spin :spinning="loading">
      <a-card v-if="keyData" :title="`键: ${keyName}`" size="small">
        <template #extra>
          <a-space>
            <a-tag :color="getTypeColor(keyData.key_type)">
              {{ keyData.key_type }}
            </a-tag>
            <a-tag v-if="keyData.ttl > 0" color="orange">
              TTL: {{ keyData.ttl }}秒
            </a-tag>
            <a-tag v-else-if="keyData.ttl === -1" color="blue">
              永不过期
            </a-tag>
            <a-button size="small" danger @click="handleDelete">
              删除
            </a-button>
          </a-space>
        </template>
        
        <!-- 字符串类型 -->
        <div v-if="keyData.key_type === 'string'">
          <a-textarea
            v-model:value="editedValue"
            :rows="10"
            :disabled="!editing"
          />
          <a-space style="margin-top: 12px">
            <a-button v-if="!editing" @click="editing = true">编辑</a-button>
            <template v-else>
              <a-button type="primary" @click="handleSave">保存</a-button>
              <a-button @click="cancelEdit">取消</a-button>
            </template>
          </a-space>
        </div>
        
        <!-- 列表类型 -->
        <div v-else-if="keyData.key_type === 'list'">
          <a-list
            :data-source="(keyData.value as string[])"
            bordered
            size="small"
          >
            <template #renderItem="{ item, index }">
              <a-list-item>
                <strong>[{{ index }}]</strong> {{ item }}
              </a-list-item>
            </template>
          </a-list>
        </div>
        
        <!-- 集合类型 -->
        <div v-else-if="keyData.key_type === 'set'">
          <a-list
            :data-source="(keyData.value as string[])"
            bordered
            size="small"
          >
            <template #renderItem="{ item }">
              <a-list-item>{{ item }}</a-list-item>
            </template>
          </a-list>
        </div>
        
        <!-- 有序集合类型 -->
        <div v-else-if="keyData.key_type === 'zset'">
          <a-table
            :columns="zsetColumns"
            :data-source="formatZsetData(keyData.value)"
            :pagination="false"
            size="small"
            bordered
          />
        </div>
        
        <!-- 哈希类型 -->
        <div v-else-if="keyData.key_type === 'hash'">
          <a-table
            :columns="hashColumns"
            :data-source="formatHashData(keyData.value)"
            :pagination="false"
            size="small"
            bordered
          />
        </div>
        
        <!-- 未知类型 -->
        <div v-else>
          <pre>{{ JSON.stringify(keyData.value, null, 2) }}</pre>
        </div>
      </a-card>
      
      <a-empty v-else description="选择一个键查看详情" />
    </a-spin>
  </div>
</template>

<script setup lang="ts">
import { message, Modal } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{
  connectionId: string
  keyName: string
}>()

const emit = defineEmits(['deleted', 'updated'])

const loading = ref(false)
const keyData = ref<any>(null)
const editing = ref(false)
const editedValue = ref('')

// 哈希表列
const hashColumns = [
  {
    title: '字段',
    dataIndex: 'field',
    key: 'field',
  },
  {
    title: '值',
    dataIndex: 'value',
    key: 'value',
  },
]

// 有序集合列
const zsetColumns = [
  {
    title: '成员',
    dataIndex: 'member',
    key: 'member',
  },
  {
    title: '分数',
    dataIndex: 'score',
    key: 'score',
  },
]

// 获取类型颜色
function getTypeColor(type: string): string {
  const colors: Record<string, string> = {
    string: 'green',
    list: 'blue',
    set: 'orange',
    zset: 'purple',
    hash: 'cyan',
  }
  return colors[type] || 'default'
}

// 格式化哈希数据
function formatHashData(value: any): any[] {
  if (Array.isArray(value)) {
    const result = []
    for (let i = 0; i < value.length; i += 2) {
      result.push({
        field: value[i],
        value: value[i + 1],
      })
    }
    return result
  }
  return []
}

// 格式化有序集合数据
function formatZsetData(value: any): any[] {
  if (Array.isArray(value)) {
    return value.map((item: any) => {
      if (Array.isArray(item) && item.length === 2) {
        return {
          member: item[0],
          score: item[1],
        }
      }
      return item
    })
  }
  return []
}

// 加载键值
async function loadKeyValue() {
  if (!props.keyName) return
  
  loading.value = true
  try {
    const result = await invoke<any>('get_redis_key_value', {
      connectionId: props.connectionId,
      key: props.keyName,
    })
    
    keyData.value = result
    
    // 如果是字符串类型，初始化编辑值
    if (result.key_type === 'string') {
      editedValue.value = result.value
    }
  } catch (error: any) {
    message.error(`获取键值失败: ${error}`)
  } finally {
    loading.value = false
  }
}

// 保存编辑
async function handleSave() {
  try {
    await invoke('set_redis_key_value', {
      connectionId: props.connectionId,
      key: props.keyName,
      value: editedValue.value,
      ttl: keyData.value.ttl > 0 ? keyData.value.ttl : null,
    })
    
    message.success('保存成功')
    editing.value = false
    emit('updated')
    loadKeyValue()
  } catch (error: any) {
    message.error(`保存失败: ${error}`)
  }
}

// 取消编辑
function cancelEdit() {
  editing.value = false
  editedValue.value = keyData.value?.value || ''
}

// 删除键
function handleDelete() {
  Modal.confirm({
    title: '确认删除',
    content: `确定要删除键 "${props.keyName}" 吗？`,
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    async onOk() {
      try {
        await invoke('delete_redis_key', {
          connectionId: props.connectionId,
          key: props.keyName,
        })
        
        message.success('删除成功')
        emit('deleted')
      } catch (error: any) {
        message.error(`删除失败: ${error}`)
      }
    },
  })
}

// 监听 keyName 变化
watch(() => props.keyName, () => {
  if (props.keyName) {
    editing.value = false
    loadKeyValue()
  } else {
    keyData.value = null
  }
}, { immediate: true })
</script>

<style scoped>
.redis-key-viewer {
  padding: 16px;
  height: 100%;
  overflow: auto;
}
</style>

