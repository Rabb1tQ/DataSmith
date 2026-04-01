<template>
  <div class="redis-key-viewer">
    <a-spin :spinning="loading">
      <a-card v-if="keyData" size="small">
        <template #title>
          <div class="key-title">
            <span>键: {{ keyName }}</span>
            <a-button type="link" size="small" @click="showRenameModal = true">
              <EditOutlined /> 重命名
            </a-button>
          </div>
        </template>
        <template #extra>
          <a-space>
            <a-tag :color="getTypeColor(keyData.key_type)">
              {{ keyData.key_type }}
            </a-tag>
            <a-tag v-if="keyData.ttl > 0" color="orange" style="cursor: pointer" @click="showTtlModal = true">
              TTL: {{ keyData.ttl }}秒
            </a-tag>
            <a-tag v-else-if="keyData.ttl === -1" color="blue" style="cursor: pointer" @click="showTtlModal = true">
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
            <a-button v-if="!editing" type="primary" @click="editing = true">编辑</a-button>
            <template v-else>
              <a-button type="primary" @click="handleSaveString">保存</a-button>
              <a-button @click="cancelEdit">取消</a-button>
            </template>
          </a-space>
        </div>
        
        <!-- 列表类型 -->
        <div v-else-if="keyData.key_type === 'list'">
          <div class="list-editor">
            <div v-for="(item, index) in editedListItems" :key="index" class="list-item-row">
              <span class="item-index">[{{ index }}]</span>
              <a-input
                v-model:value="editedListItems[index]"
                :placeholder="`元素 ${index}`"
                style="flex: 1"
              />
              <a-button type="text" danger @click="removeListItem(index)">
                <DeleteOutlined />
              </a-button>
            </div>
            <a-button type="dashed" block @click="addListItem" style="margin-top: 8px">
              <PlusOutlined /> 添加元素
            </a-button>
          </div>
          <a-space style="margin-top: 12px">
            <a-button v-if="!editing" type="primary" @click="startEditList">编辑</a-button>
            <template v-else>
              <a-button type="primary" @click="handleSaveList">保存</a-button>
              <a-button @click="cancelEdit">取消</a-button>
            </template>
          </a-space>
        </div>
        
        <!-- 集合类型 -->
        <div v-else-if="keyData.key_type === 'set'">
          <div class="set-editor">
            <div v-for="(item, index) in editedSetItems" :key="index" class="set-item-row">
              <a-input
                v-model:value="editedSetItems[index]"
                :placeholder="`成员 ${index + 1}`"
                style="flex: 1"
              />
              <a-button type="text" danger @click="removeSetItem(index)">
                <DeleteOutlined />
              </a-button>
            </div>
            <a-button type="dashed" block @click="addSetItem" style="margin-top: 8px">
              <PlusOutlined /> 添加成员
            </a-button>
          </div>
          <a-space style="margin-top: 12px">
            <a-button v-if="!editing" type="primary" @click="startEditSet">编辑</a-button>
            <template v-else>
              <a-button type="primary" @click="handleSaveSet">保存</a-button>
              <a-button @click="cancelEdit">取消</a-button>
            </template>
          </a-space>
        </div>
        
        <!-- 有序集合类型 -->
        <div v-else-if="keyData.key_type === 'zset'">
          <div class="zset-editor">
            <div v-for="(item, index) in editedZsetItems" :key="index" class="zset-item-row">
              <a-input-number
                v-model:value="item.score"
                placeholder="分数"
                style="width: 120px"
              />
              <a-input
                v-model:value="item.member"
                placeholder="成员"
                style="flex: 1"
              />
              <a-button type="text" danger @click="removeZsetItem(index)">
                <DeleteOutlined />
              </a-button>
            </div>
            <a-button type="dashed" block @click="addZsetItem" style="margin-top: 8px">
              <PlusOutlined /> 添加成员
            </a-button>
          </div>
          <a-space style="margin-top: 12px">
            <a-button v-if="!editing" type="primary" @click="startEditZset">编辑</a-button>
            <template v-else>
              <a-button type="primary" @click="handleSaveZset">保存</a-button>
              <a-button @click="cancelEdit">取消</a-button>
            </template>
          </a-space>
        </div>
        
        <!-- 哈希类型 -->
        <div v-else-if="keyData.key_type === 'hash'">
          <div class="hash-editor">
            <div v-for="(item, index) in editedHashItems" :key="index" class="hash-item-row">
              <a-input
                v-model:value="item.field"
                placeholder="字段"
                style="width: 150px"
              />
              <a-input
                v-model:value="item.value"
                placeholder="值"
                style="flex: 1"
              />
              <a-button type="text" danger @click="removeHashItem(index)">
                <DeleteOutlined />
              </a-button>
            </div>
            <a-button type="dashed" block @click="addHashItem" style="margin-top: 8px">
              <PlusOutlined /> 添加字段
            </a-button>
          </div>
          <a-space style="margin-top: 12px">
            <a-button v-if="!editing" type="primary" @click="startEditHash">编辑</a-button>
            <template v-else>
              <a-button type="primary" @click="handleSaveHash">保存</a-button>
              <a-button @click="cancelEdit">取消</a-button>
            </template>
          </a-space>
        </div>
        
        <!-- 未知类型 -->
        <div v-else>
          <pre>{{ JSON.stringify(keyData.value, null, 2) }}</pre>
        </div>
      </a-card>
      
      <a-empty v-else description="选择一个键查看详情" />
    </a-spin>
    
    <!-- TTL 设置对话框 -->
    <a-modal
      v-model:open="showTtlModal"
      title="设置 TTL"
      @ok="handleSetTtl"
      :confirm-loading="ttlLoading"
    >
      <a-form layout="vertical">
        <a-form-item label="过期时间（秒）">
          <a-input-number
            v-model:value="newTtl"
            :min="-1"
            style="width: 100%"
            placeholder="-1 表示永不过期"
          />
        </a-form-item>
        <a-form-item label="常用预设">
          <a-space wrap>
            <a-button size="small" @click="newTtl = -1">永不过期</a-button>
            <a-button size="small" @click="newTtl = 60">1分钟</a-button>
            <a-button size="small" @click="newTtl = 3600">1小时</a-button>
            <a-button size="small" @click="newTtl = 86400">1天</a-button>
            <a-button size="small" @click="newTtl = 604800">7天</a-button>
          </a-space>
        </a-form-item>
      </a-form>
    </a-modal>
    
    <!-- 重命名对话框 -->
    <a-modal
      v-model:open="showRenameModal"
      title="重命名键"
      @ok="handleRename"
      :confirm-loading="renameLoading"
    >
      <a-form layout="vertical">
        <a-form-item label="新键名">
          <a-input
            v-model:value="newKeyName"
            placeholder="请输入新的键名"
          />
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { message, Modal } from 'ant-design-vue'
import { DeleteOutlined, PlusOutlined, EditOutlined } from '@ant-design/icons-vue'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{
  connectionId: string
  keyName: string
}>()

const emit = defineEmits(['deleted', 'updated', 'renamed'])

const loading = ref(false)
const keyData = ref<any>(null)
const editing = ref(false)
const editedValue = ref('')

// TTL 相关
const showTtlModal = ref(false)
const newTtl = ref(-1)
const ttlLoading = ref(false)

// 重命名相关
const showRenameModal = ref(false)
const newKeyName = ref('')
const renameLoading = ref(false)

// 列表编辑数据
const editedListItems = ref<string[]>([])

// 集合编辑数据
const editedSetItems = ref<string[]>([])

// 有序集合编辑数据
interface ZsetItem {
  member: string
  score: number
}
const editedZsetItems = ref<ZsetItem[]>([])

// 哈希编辑数据
interface HashItem {
  field: string
  value: string
}
const editedHashItems = ref<HashItem[]>([])

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
    newTtl.value = result.ttl
    newKeyName.value = props.keyName
    
    // 根据类型初始化编辑数据
    if (result.key_type === 'string') {
      editedValue.value = result.value
    } else if (result.key_type === 'list') {
      editedListItems.value = [...(result.value as string[])]
    } else if (result.key_type === 'set') {
      editedSetItems.value = [...(result.value as string[])]
    } else if (result.key_type === 'zset') {
      editedZsetItems.value = formatZsetData(result.value)
    } else if (result.key_type === 'hash') {
      editedHashItems.value = formatHashData(result.value)
    }
  } catch (error: any) {
    message.error(`获取键值失败: ${error}`)
  } finally {
    loading.value = false
  }
}

// 格式化哈希数据
function formatHashData(value: any): HashItem[] {
  if (Array.isArray(value)) {
    const result: HashItem[] = []
    for (let i = 0; i < value.length; i += 2) {
      result.push({
        field: value[i] || '',
        value: value[i + 1] || '',
      })
    }
    return result
  }
  return []
}

// 格式化有序集合数据
function formatZsetData(value: any): ZsetItem[] {
  if (Array.isArray(value)) {
    return value.map((item: any) => {
      if (Array.isArray(item) && item.length === 2) {
        return {
          member: String(item[0]),
          score: Number(item[1]),
        }
      }
      return {
        member: String(item),
        score: 0,
      }
    })
  }
  return []
}

// 取消编辑
function cancelEdit() {
  editing.value = false
  // 恢复原始数据
  if (keyData.value) {
    if (keyData.value.key_type === 'string') {
      editedValue.value = keyData.value.value
    } else if (keyData.value.key_type === 'list') {
      editedListItems.value = [...(keyData.value.value as string[])]
    } else if (keyData.value.key_type === 'set') {
      editedSetItems.value = [...(keyData.value.value as string[])]
    } else if (keyData.value.key_type === 'zset') {
      editedZsetItems.value = formatZsetData(keyData.value.value)
    } else if (keyData.value.key_type === 'hash') {
      editedHashItems.value = formatHashData(keyData.value.value)
    }
  }
}

// ===== TTL 设置 =====
async function handleSetTtl() {
  ttlLoading.value = true
  try {
    await invoke('set_redis_key_ttl', {
      connectionId: props.connectionId,
      key: props.keyName,
      ttl: newTtl.value,
    })
    
    message.success('TTL 设置成功')
    showTtlModal.value = false
    emit('updated')
    loadKeyValue()
  } catch (error: any) {
    message.error(`设置 TTL 失败: ${error}`)
  } finally {
    ttlLoading.value = false
  }
}

// ===== 重命名 =====
async function handleRename() {
  if (!newKeyName.value.trim()) {
    message.error('请输入新的键名')
    return
  }
  
  if (newKeyName.value === props.keyName) {
    message.error('新键名与原键名相同')
    return
  }
  
  renameLoading.value = true
  try {
    await invoke('rename_redis_key', {
      connectionId: props.connectionId,
      oldKey: props.keyName,
      newKey: newKeyName.value,
    })
    
    message.success('重命名成功')
    showRenameModal.value = false
    emit('renamed', { oldKey: props.keyName, newKey: newKeyName.value })
  } catch (error: any) {
    message.error(`重命名失败: ${error}`)
  } finally {
    renameLoading.value = false
  }
}

// ===== 字符串操作 =====
async function handleSaveString() {
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

// ===== 列表操作 =====
function startEditList() {
  editing.value = true
}

function addListItem() {
  editedListItems.value.push('')
}

function removeListItem(index: number) {
  editedListItems.value.splice(index, 1)
}

async function handleSaveList() {
  try {
    // 过滤空值
    const items = editedListItems.value.filter(item => item.trim() !== '')
    
    await invoke('set_redis_list_value', {
      connectionId: props.connectionId,
      key: props.keyName,
      values: items,
    })
    
    message.success('保存成功')
    editing.value = false
    emit('updated')
    loadKeyValue()
  } catch (error: any) {
    message.error(`保存失败: ${error}`)
  }
}

// ===== 集合操作 =====
function startEditSet() {
  editing.value = true
}

function addSetItem() {
  editedSetItems.value.push('')
}

function removeSetItem(index: number) {
  editedSetItems.value.splice(index, 1)
}

async function handleSaveSet() {
  try {
    // 过滤空值
    const items = editedSetItems.value.filter(item => item.trim() !== '')
    
    await invoke('set_redis_set_value', {
      connectionId: props.connectionId,
      key: props.keyName,
      members: items,
    })
    
    message.success('保存成功')
    editing.value = false
    emit('updated')
    loadKeyValue()
  } catch (error: any) {
    message.error(`保存失败: ${error}`)
  }
}

// ===== 有序集合操作 =====
function startEditZset() {
  editing.value = true
}

function addZsetItem() {
  editedZsetItems.value.push({ member: '', score: 0 })
}

function removeZsetItem(index: number) {
  editedZsetItems.value.splice(index, 1)
}

async function handleSaveZset() {
  try {
    // 过滤空成员
    const items = editedZsetItems.value.filter(item => item.member.trim() !== '')
    
    await invoke('set_redis_zset_value', {
      connectionId: props.connectionId,
      key: props.keyName,
      members: items.map(item => ({ member: item.member, score: item.score })),
    })
    
    message.success('保存成功')
    editing.value = false
    emit('updated')
    loadKeyValue()
  } catch (error: any) {
    message.error(`保存失败: ${error}`)
  }
}

// ===== 哈希操作 =====
function startEditHash() {
  editing.value = true
}

function addHashItem() {
  editedHashItems.value.push({ field: '', value: '' })
}

function removeHashItem(index: number) {
  editedHashItems.value.splice(index, 1)
}

async function handleSaveHash() {
  try {
    // 过滤空字段
    const items = editedHashItems.value.filter(item => item.field.trim() !== '')
    
    await invoke('set_redis_hash_value', {
      connectionId: props.connectionId,
      key: props.keyName,
      fields: items,
    })
    
    message.success('保存成功')
    editing.value = false
    emit('updated')
    loadKeyValue()
  } catch (error: any) {
    message.error(`保存失败: ${error}`)
  }
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

.key-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.list-item-row,
.set-item-row,
.zset-item-row,
.hash-item-row {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
  align-items: center;
}

.item-index {
  width: 50px;
  color: #666;
  font-family: monospace;
}

.zset-item-row {
  gap: 12px;
}

.hash-item-row {
  gap: 12px;
}
</style>
