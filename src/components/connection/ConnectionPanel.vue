<template>
  <div class="connection-panel">
    <div class="panel-header">
      <span class="panel-title">连接</span>
      <a-button
        type="text"
        size="small"
        :icon="h(PlusOutlined)"
        @click="$emit('add-connection')"
      >
        新建
      </a-button>
    </div>

    <div class="panel-content">
      <a-input-search
        v-model:value="searchText"
        placeholder="搜索连接..."
        style="margin-bottom: 8px"
      />

      <div class="connection-list">
        <div
          v-for="conn in filteredConnections"
          :key="conn.id"
          class="connection-group"
        >
          <div
            class="connection-item"
            :class="{ 
              active: activeConnectionId === conn.id,
              expanded: expandedConnections.has(conn.id)
            }"
            @click="handleSelectConnection(conn)"
            @dblclick="handleToggleConnection(conn)"
            @contextmenu.prevent="handleContextMenu($event, conn)"
          >
            <div class="connection-expand-icon" @click.stop="handleToggleExpand(conn)">
              <DownOutlined 
                v-if="getConnectionStatus(conn.id) === 'connected' && expandedConnections.has(conn.id)" 
                class="expand-icon expanded"
              />
              <RightOutlined 
                v-else-if="getConnectionStatus(conn.id) === 'connected'" 
                class="expand-icon"
              />
              <span v-else class="expand-icon-placeholder"></span>
            </div>
            <div class="connection-icon">
              <DatabaseOutlined />
            </div>
            <div class="connection-info">
              <div class="connection-name">{{ conn.name }}</div>
              <div class="connection-detail">
                {{ conn.db_type }} • {{ conn.host }}:{{ conn.port }}
              </div>
            </div>
            <div class="connection-status">
              <a-badge :status="getStatusBadge(conn.id)" />
              <a-button
                v-if="getConnectionStatus(conn.id) === 'connected'"
                type="text"
                size="small"
                :icon="h(DisconnectOutlined)"
                @click.stop="handleDisconnect(conn)"
                title="断开连接"
              />
            </div>
          </div>
          
          <!-- 数据库对象树 -->
          <div 
            v-if="getConnectionStatus(conn.id) === 'connected' && expandedConnections.has(conn.id)" 
            class="database-objects"
          >
            <DatabaseTree
              :ref="el => { if (el) databaseTreeRefs.set(conn.id, el) }"
              :connection-id="conn.id"
              :db-type="conn.db_type"
              @table-selected="(data) => emit('table-selected', { ...data, connectionId: conn.id })"
              @database-selected="(data) => emit('database-selected', { ...data, connectionId: conn.id })"
              @new-query="(data) => emit('new-query', data)"
              @design-table="(data) => emit('design-table', { ...data, connectionId: conn.id })"
            />
          </div>
        </div>
      </div>

      <a-empty
        v-if="filteredConnections.length === 0"
        description="暂无连接"
        :image-style="{ height: '60px' }"
      />
    </div>

    <!-- 右键菜单 -->
    <div
      v-if="contextMenuVisible"
      class="context-menu-overlay"
      @click="contextMenuVisible = false"
    >
      <div
        class="context-menu"
        :style="{ left: contextMenuX + 'px', top: contextMenuY + 'px' }"
        @click.stop
      >
        <a-menu @click="handleMenuClick">
          <a-menu-item 
            key="connect" 
            v-if="getConnectionStatus(selectedConnection?.id || '') !== 'connected'"
          >
            <LinkOutlined />
            连接
          </a-menu-item>
          <a-menu-item 
            key="disconnect" 
            v-if="getConnectionStatus(selectedConnection?.id || '') === 'connected'"
          >
            <DisconnectOutlined />
            断开连接
          </a-menu-item>
          <a-menu-divider v-if="getConnectionStatus(selectedConnection?.id || '') === 'connected' && canCreateDatabase" />
          <a-menu-item 
            key="create-database"
            v-if="getConnectionStatus(selectedConnection?.id || '') === 'connected' && canCreateDatabase"
          >
            <DatabaseOutlined />
            新建数据库
          </a-menu-item>
          <a-menu-divider />
          <a-menu-item key="edit">
            <EditOutlined />
            编辑
          </a-menu-item>
          <a-menu-divider />
          <a-menu-item key="delete" danger>
            <DeleteOutlined />
            删除
          </a-menu-item>
        </a-menu>
      </div>
    </div>
    
    <!-- 新建数据库对话框 -->
    <CreateDatabaseDialog
      v-model:visible="showCreateDatabaseDialog"
      :connection-id="selectedConnection?.id || ''"
      :db-type="selectedConnection?.db_type"
      @created="handleDatabaseCreated"
    />
  </div>
</template>

<script setup lang="ts">
import { h, computed } from 'vue'
import { 
  DatabaseOutlined, 
  PlusOutlined,
  LinkOutlined,
  EditOutlined,
  DeleteOutlined,
  DisconnectOutlined,
  DownOutlined,
  RightOutlined,
} from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'
import { useConnectionStore } from '@/stores/connection'
import type { ConnectionConfig } from '@/types/database'
import DatabaseTree from '@/components/database/DatabaseTree.vue'
import CreateDatabaseDialog from '@/components/database/CreateDatabaseDialog.vue'

const emit = defineEmits(['add-connection', 'edit-connection', 'table-selected', 'database-selected', 'new-query', 'design-table'])

const connectionStore = useConnectionStore()
const searchText = ref('')
const activeConnectionId = computed(() => connectionStore.activeConnectionId)
const showCreateDatabaseDialog = ref(false)

// 展开的连接集合
const expandedConnections = ref<Set<string>>(new Set())

// 右键菜单
const contextMenuVisible = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const selectedConnection = ref<ConnectionConfig | null>(null)

// 数据库树引用映射
const databaseTreeRefs = new Map<string, any>()

// 判断当前选中的连接是否支持创建数据库
const canCreateDatabase = computed(() => {
  if (!selectedConnection.value) return false
  // SQLite 不支持通过 SQL 创建数据库
  const dbType = selectedConnection.value.db_type?.toLowerCase()
  return dbType !== 'sqlite'
})

// 过滤连接列表
const filteredConnections = computed(() => {
  if (!searchText.value) {
    return connectionStore.connections
  }
  const text = searchText.value.toLowerCase()
  return connectionStore.connections.filter(
    (conn) =>
      conn.name.toLowerCase().includes(text) ||
      conn.host.toLowerCase().includes(text)
  )
})

// 选择连接（仅设置为活动连接，不自动连接）
function handleSelectConnection(conn: ConnectionConfig) {
  connectionStore.setActiveConnection(conn.id)
}

// 切换展开/折叠
function handleToggleExpand(conn: ConnectionConfig) {
  const status = getConnectionStatus(conn.id)
  if (status !== 'connected') {
    // 如果未连接，先连接
    handleConnectToDatabase(conn)
    return
  }
  
  // 已连接时，切换展开/折叠
  const newExpanded = new Set(expandedConnections.value)
  if (newExpanded.has(conn.id)) {
    newExpanded.delete(conn.id)
  } else {
    newExpanded.add(conn.id)
  }
  expandedConnections.value = newExpanded
}

// 双击连接处理
async function handleToggleConnection(conn: ConnectionConfig) {
  const status = getConnectionStatus(conn.id)
  
  if (status === 'connected') {
    // 如果已连接，切换展开/折叠
    const newExpanded = new Set(expandedConnections.value)
    if (newExpanded.has(conn.id)) {
      newExpanded.delete(conn.id)
    } else {
      newExpanded.add(conn.id)
    }
    expandedConnections.value = newExpanded
  } else {
    // 如果未连接，连接并展开
    await handleConnectToDatabase(conn)
  }
}

// 连接到数据库
async function handleConnectToDatabase(conn: ConnectionConfig) {
  try {
    connectionStore.updateConnectionStatus(conn.id, 'connecting')
    await connectionStore.connectToDatabase(conn.id)
    connectionStore.setActiveConnection(conn.id)
    connectionStore.updateConnectionStatus(conn.id, 'connected')
    
    // 自动展开连接
    const newExpanded = new Set(expandedConnections.value)
    newExpanded.add(conn.id)
    expandedConnections.value = newExpanded
    
    message.success(`已连接到 ${conn.name}`)
  } catch (error: any) {
    connectionStore.updateConnectionStatus(conn.id, 'error')
    message.error(`连接失败: ${error}`)
  }
}

// 断开连接
async function handleDisconnect(conn: ConnectionConfig) {
  try {
    await connectionStore.disconnectFromDatabase(conn.id)
    connectionStore.updateConnectionStatus(conn.id, 'disconnected')
    
    // 收起连接
    const newExpanded = new Set(expandedConnections.value)
    newExpanded.delete(conn.id)
    expandedConnections.value = newExpanded
    
    message.success(`已断开连接 ${conn.name}`)
  } catch (error: any) {
    message.error(`断开连接失败: ${error}`)
  }
}

// 右键菜单
function handleContextMenu(event: MouseEvent, conn: ConnectionConfig) {
  event.preventDefault()
  selectedConnection.value = conn
  
  // 计算菜单位置，避免超出视口
  const menuWidth = 160 // 估算菜单宽度
  const menuHeight = 120 // 估算菜单高度
  const windowWidth = window.innerWidth
  const windowHeight = window.innerHeight
  
  let x = event.clientX
  let y = event.clientY
  
  // 如果菜单会超出右边界，向左调整
  if (x + menuWidth > windowWidth) {
    x = windowWidth - menuWidth - 10
  }
  
  // 如果菜单会超出下边界，向上调整
  if (y + menuHeight > windowHeight) {
    y = windowHeight - menuHeight - 10
  }
  
  // 确保不会超出左边界和上边界
  x = Math.max(10, x)
  y = Math.max(10, y)
  
  contextMenuX.value = x
  contextMenuY.value = y
  contextMenuVisible.value = true
}

// 处理菜单点击
async function handleMenuClick({ key }: { key: string | number }) {
  if (!selectedConnection.value) return
  
  contextMenuVisible.value = false
  
  const keyStr = String(key)
  switch (keyStr) {
    case 'connect':
      await handleConnectToDatabase(selectedConnection.value)
      break
    case 'disconnect':
      await handleDisconnect(selectedConnection.value)
      break
    case 'create-database':
      showCreateDatabaseDialog.value = true
      break
    case 'edit':
      // 触发编辑连接事件
      emit('edit-connection', selectedConnection.value)
      break
    case 'delete':
      Modal.confirm({
        title: '确认删除',
        content: `确定要删除连接 "${selectedConnection.value.name}" 吗？`,
        okText: '删除',
        okType: 'danger',
        cancelText: '取消',
        async onOk() {
          try {
            await connectionStore.deleteConnection(selectedConnection.value!.id)
            message.success('连接已删除')
          } catch (error: any) {
            message.error(`删除失败: ${error}`)
          }
        },
      })
      break
  }
}

// 数据库创建成功回调
function handleDatabaseCreated() {
  // 刷新对应连接的数据库树
  if (selectedConnection.value) {
    const treeRef = databaseTreeRefs.get(selectedConnection.value.id)
    if (treeRef && treeRef.refresh) {
      treeRef.refresh()
    }
  }
}

// 获取连接状态
function getConnectionStatus(id: string) {
  return connectionStore.getConnectionStatus(id)
}

// 获取状态徽章
function getStatusBadge(id: string) {
  const status = connectionStore.getConnectionStatus(id)
  switch (status) {
    case 'connected':
      return 'success'
    case 'connecting':
      return 'processing'
    case 'error':
      return 'error'
    default:
      return 'default'
  }
}

// 监听键盘事件，ESC 键关闭菜单
function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    contextMenuVisible.value = false
  }
}

// 初始化加载连接列表
onMounted(() => {
  connectionStore.fetchConnections()
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})
</script>

<style scoped>
.connection-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid #e8e8e8;
}

.dark-mode .panel-header {
  border-bottom-color: #303030;
}

.panel-title {
  font-size: 14px;
  font-weight: 600;
}

.panel-content {
  flex: 1;
  overflow: auto;
  padding: 12px 0;
}

.connection-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.connection-group {
  border-radius: 6px;
  overflow: hidden;
}

.connection-item {
  display: flex;
  align-items: center;
  padding: 8px 12px;
  border-radius: 6px;
  cursor: pointer;
  transition: background-color 0.2s;
}

.connection-item:hover {
  background-color: #f5f5f5;
}

.dark-mode .connection-item:hover {
  background-color: #262626;
}

.connection-item.active {
  background-color: #e6f7ff;
}

.dark-mode .connection-item.active {
  background-color: #111b26;
}

.connection-expand-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  margin-right: 4px;
  flex-shrink: 0;
}

.expand-icon {
  font-size: 12px;
  color: #8c8c8c;
  transition: transform 0.2s;
  cursor: pointer;
}

.expand-icon:hover {
  color: #1890ff;
}

.expand-icon-placeholder {
  display: inline-block;
  width: 12px;
  height: 12px;
}

.connection-icon {
  font-size: 20px;
  margin-right: 12px;
  color: #1890ff;
  flex-shrink: 0;
}

.connection-info {
  flex: 1;
  overflow: hidden;
}

.connection-name {
  font-size: 14px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.connection-detail {
  font-size: 12px;
  color: #8c8c8c;
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.connection-status {
  margin-left: 8px;
}

.database-objects {
  background: #f8f9fa;
  border-top: 1px solid #e8e8e8;
  margin-top: 4px;
  padding: 8px 0 8px 12px;
  border-radius: 0 0 6px 6px;
}

.dark-mode .database-objects {
  background: #1a1a1a;
  border-top: 1px solid #303030;
}

/* 右键菜单样式 */
.context-menu-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 9999;
  background: transparent;
}

.context-menu {
  position: absolute;
  background: #fff;
  border-radius: 6px;
  box-shadow: 0 6px 16px 0 rgba(0, 0, 0, 0.08), 0 3px 6px -4px rgba(0, 0, 0, 0.12), 0 9px 28px 8px rgba(0, 0, 0, 0.05);
  z-index: 10000;
}

.dark-mode .context-menu {
  background: #1f1f1f;
  border: 1px solid #303030;
}
</style>

