<template>
  <a-layout class="main-layout">
    <!-- 顶部工具栏 -->
    <a-layout-header class="header">
      <div class="header-content">
        <div class="logo">
          <DatabaseOutlined style="font-size: 24px; margin-right: 8px" />
          <span class="title">DataSmith</span>
        </div>
        <div class="header-menu">
          <a-menu mode="horizontal" :selected-keys="[]" class="top-menu">
            <a-sub-menu key="file">
              <template #title>文件</template>
              <a-menu-item key="new-connection" @click="showConnectionDialog = true">
                <PlusOutlined />
                新建连接
              </a-menu-item>
              <a-menu-divider />
              <a-menu-item key="settings" @click="showSettings = true">
                <SettingOutlined />
                设置
              </a-menu-item>
              <a-menu-divider />
              <a-menu-item key="exit">
                <LogoutOutlined />
                退出
              </a-menu-item>
            </a-sub-menu>
            <a-sub-menu key="view">
              <template #title>视图</template>
              <a-menu-item key="toggle-sidebar" @click="appStore.toggleSidebar()">
                <MenuOutlined />
                {{ appStore.sidebarCollapsed ? '显示' : '隐藏' }}侧边栏
              </a-menu-item>
              <a-menu-divider />
              <a-menu-item key="theme" @click="appStore.toggleTheme()">
                <BulbOutlined />
                {{ appStore.theme === 'light' ? '暗色' : '明亮' }}主题
              </a-menu-item>
            </a-sub-menu>
            <a-sub-menu key="help">
              <template #title>帮助</template>
              <a-menu-item key="docs">
                <FileTextOutlined />
                文档
              </a-menu-item>
              <a-menu-item key="about" @click="showAbout = true">
                <InfoCircleOutlined />
                关于
              </a-menu-item>
            </a-sub-menu>
          </a-menu>
        </div>
        <div class="header-actions">
          <a-button
            type="text"
            :icon="h(SearchOutlined)"
            @click="showGlobalSearch = true"
            :disabled="!connectionStore.activeConnectionId"
          >
            全局搜索
          </a-button>
          <a-button type="text" :icon="h(BulbOutlined)" @click="appStore.toggleTheme()">
            {{ appStore.theme === 'light' ? '暗色' : '明亮' }}
          </a-button>
        </div>
      </div>
    </a-layout-header>

    <a-layout class="content-layout">
      <!-- 左侧连接和对象浏览器 -->
      <div 
        v-if="!appStore.sidebarCollapsed"
        class="sidebar"
        :style="{ width: sidebarWidth + 'px' }"
      >
        <div class="sidebar-content">
          <ConnectionPanel 
            @add-connection="showConnectionDialog = true"
            @edit-connection="handleEditConnection"
            @table-selected="handleTableSelected"
            @database-selected="handleDatabaseSelected"
            @new-query="handleNewQuery"
            @design-table="handleDesignTable"
          />
        </div>
      </div>

      <!-- 拖拽分隔条 -->
      <div 
        v-if="!appStore.sidebarCollapsed"
        class="sidebar-resizer"
        @mousedown="startResize"
      ></div>

      <!-- 中间主工作区 -->
      <a-layout-content class="main-content">
        <a-tabs 
          v-model:activeKey="mainTabKey" 
          type="editable-card" 
          @edit="onTabEdit"
          class="main-tabs"
        >
          <a-tab-pane v-if="isSqlSupported" key="sql" :closable="false">
            <template #tab>
              <span @contextmenu.prevent="handleTabContextMenu($event, 'sql', false)">
                SQL 查询
              </span>
            </template>
            <SqlEditor ref="sqlEditorRef" />
          </a-tab-pane>
          <a-tab-pane v-if="!isSqlSupported" key="redis" :closable="false">
            <template #tab>
              <span @contextmenu.prevent="handleTabContextMenu($event, 'redis', false)">
                Redis 命令行
              </span>
            </template>
            <RedisEditor ref="redisEditorRef" />
          </a-tab-pane>
          <a-tab-pane
            v-for="tab in dataTabs"
            :key="tab.key"
            :closable="true"
          >
            <template #tab>
              <span @contextmenu.prevent="handleTabContextMenu($event, tab.key, true)">
                {{ tab.title }}
              </span>
            </template>
            <TableDataGrid
              v-if="tab.type === 'data'"
              :connection-id="tab.connectionId"
              :database="tab.database"
              :table="tab.table"
            />
            <TableDesigner
              v-else-if="tab.type === 'design'"
              :connection-id="tab.connectionId"
              :database="tab.database"
              :table="tab.table"
            />
          </a-tab-pane>
        </a-tabs>

        <!-- Tab 右键菜单 -->
        <a-dropdown
          v-model:open="contextMenuVisible"
          :trigger="['contextmenu']"
          :overlayStyle="{ position: 'fixed', left: contextMenuPosition.x + 'px', top: contextMenuPosition.y + 'px' }"
        >
          <div style="position: fixed; left: 0; top: 0; width: 0; height: 0;"></div>
          <template #overlay>
            <a-menu @click="handleContextMenuClick">
              <a-menu-item key="close" :disabled="!currentContextTab.closable">
                <CloseOutlined />
                关闭当前标签
              </a-menu-item>
              <a-menu-item key="closeOthers" :disabled="dataTabs.length === 0">
                <CloseCircleOutlined />
                关闭其他标签
              </a-menu-item>
              <a-menu-item key="closeAll" :disabled="dataTabs.length === 0">
                <CloseSquareOutlined />
                关闭所有标签
              </a-menu-item>
              <a-menu-divider />
              <a-menu-item key="closeLeft" :disabled="!hasTabsOnLeft">
                <VerticalRightOutlined />
                关闭左侧标签
              </a-menu-item>
              <a-menu-item key="closeRight" :disabled="!hasTabsOnRight">
                <VerticalLeftOutlined />
                关闭右侧标签
              </a-menu-item>
              <a-menu-divider />
              <a-menu-item key="closeAllExceptSql" :disabled="dataTabs.length === 0">
                <DeleteOutlined />
                关闭除 SQL 外所有标签
              </a-menu-item>
            </a-menu>
          </template>
        </a-dropdown>

        <a-empty
          v-if="!connectionStore.activeConnectionId"
          description="请选择一个数据库连接开始使用"
          style="margin-top: 100px"
        >
          <a-button type="primary" @click="showConnectionDialog = true">
            创建连接
          </a-button>
        </a-empty>
        
        <!-- 非 SQL 数据库提示 -->
        <a-empty
          v-else-if="!isSqlSupported && dataTabs.length === 0 && mainTabKey !== 'redis'"
          description="当前数据库不支持 SQL 查询，请使用 Redis 命令行或左侧数据库树浏览数据"
          style="margin-top: 100px"
        >
          <a-tag color="blue">{{ connectionStore.getActiveConnection()?.db_type?.toUpperCase() }}</a-tag>
        </a-empty>
      </a-layout-content>
    </a-layout>

    <!-- 连接配置对话框 -->
    <ConnectionDialog 
      v-model:visible="showConnectionDialog" 
      :editing-connection="editingConnection"
      @close="editingConnection = null"
    />

    <!-- 设置对话框 -->
    <a-modal
      v-model:open="showSettings"
      title="设置"
      :width="600"
      @ok="handleSaveSettings"
    >
      <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
        <a-form-item label="默认主题">
          <a-radio-group v-model:value="settingsForm.theme">
            <a-radio value="light">明亮</a-radio>
            <a-radio value="dark">暗色</a-radio>
          </a-radio-group>
        </a-form-item>
        <a-form-item label="字体大小">
          <a-slider v-model:value="settingsForm.fontSize" :min="12" :max="20" />
        </a-form-item>
        <a-form-item label="自动保存">
          <a-switch v-model:checked="settingsForm.autoSave" />
        </a-form-item>
        <a-form-item label="查询超时(秒)">
          <a-input-number v-model:value="settingsForm.queryTimeout" :min="5" :max="300" />
        </a-form-item>
      </a-form>
    </a-modal>

    <!-- 关于对话框 -->
    <a-modal
      v-model:open="showAbout"
      title="关于 DataSmith"
      :footer="null"
      :width="500"
    >
      <div style="text-align: center; padding: 20px;">
        <DatabaseOutlined style="font-size: 64px; color: #1890ff; margin-bottom: 16px;" />
        <h2>DataSmith</h2>
        <p>版本: 1.0.0</p>
        <p>一个现代化的数据库管理工具</p>
        <a-divider />
        <p style="color: #8c8c8c;">
          支持 MySQL, PostgreSQL, SQLite, MongoDB, Redis
        </p>
      </div>
    </a-modal>

    <!-- 全局搜索 -->
    <GlobalSearch
      v-model:visible="showGlobalSearch"
      :connection-id="connectionStore.activeConnectionId"
      @view-data="handleTableSelected"
    />
  </a-layout>
</template>

<script setup lang="ts">
import { h, reactive, ref, computed, nextTick, watch } from 'vue'
import { 
  DatabaseOutlined, 
  BulbOutlined,
  PlusOutlined,
  SettingOutlined,
  LogoutOutlined,
  MenuOutlined,
  FileTextOutlined,
  InfoCircleOutlined,
  SearchOutlined,
  CloseOutlined,
  CloseCircleOutlined,
  CloseSquareOutlined,
  VerticalRightOutlined,
  VerticalLeftOutlined,
  DeleteOutlined,
} from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { useAppStore } from '@/stores/app'
import { useConnectionStore } from '@/stores/connection'
import ConnectionPanel from '@/components/connection/ConnectionPanel.vue'
import ConnectionDialog from '@/components/connection/ConnectionDialog.vue'
import SqlEditor from '@/components/editor/SqlEditor.vue'
import RedisEditor from '@/components/editor/RedisEditor.vue'
import TableDataGrid from '@/components/data/TableDataGrid.vue'
import TableDesigner from '@/components/database/TableDesigner.vue'
import GlobalSearch from '@/components/search/GlobalSearch.vue'

const appStore = useAppStore()
const connectionStore = useConnectionStore()
const showConnectionDialog = ref(false)
const showSettings = ref(false)
const showAbout = ref(false)
const showGlobalSearch = ref(false)
const mainTabKey = ref('sql')
const editingConnection = ref<any>(null)
const sqlEditorRef = ref<any>(null)
const redisEditorRef = ref<any>(null)

// 侧边栏拖拽调整宽度相关
const sidebarWidth = ref(280) // 默认宽度 280px
const isResizing = ref(false)

// 判断当前连接是否支持 SQL
const isSqlSupported = computed(() => {
  const activeConnection = connectionStore.getActiveConnection()
  if (!activeConnection) return true // 默认显示 SQL 编辑器
  
  // 非 SQL 数据库类型
  const nonSqlTypes = ['redis', 'mongodb', 'elasticsearch']
  return !nonSqlTypes.includes(activeConnection.db_type)
})

// 设置表单
const settingsForm = reactive({
  theme: appStore.theme,
  fontSize: 14,
  autoSave: true,
  queryTimeout: 30,
})

interface DataTab {
  key: string
  title: string
  type: 'data' | 'design'
  connectionId: string
  database: string
  table: string
}

const dataTabs = ref<DataTab[]>([])

// 右键菜单相关状态
const contextMenuVisible = ref(false)
const contextMenuPosition = reactive({ x: 0, y: 0 })
const currentContextTab = reactive({ key: '', closable: false })

// 计算属性：是否有左侧标签
const hasTabsOnLeft = computed(() => {
  if (currentContextTab.key === 'sql') return false
  const currentIndex = dataTabs.value.findIndex(tab => tab.key === currentContextTab.key)
  return currentIndex > 0
})

// 计算属性：是否有右侧标签
const hasTabsOnRight = computed(() => {
  if (currentContextTab.key === 'sql') return dataTabs.value.length > 0
  const currentIndex = dataTabs.value.findIndex(tab => tab.key === currentContextTab.key)
  return currentIndex >= 0 && currentIndex < dataTabs.value.length - 1
})

// 处理 Tab 右键菜单
function handleTabContextMenu(event: MouseEvent, tabKey: string, closable: boolean) {
  event.preventDefault()
  event.stopPropagation()
  
  currentContextTab.key = tabKey
  currentContextTab.closable = closable
  contextMenuPosition.x = event.clientX
  contextMenuPosition.y = event.clientY
  contextMenuVisible.value = true
}

// 处理右键菜单点击事件
function handleContextMenuClick({ key }: { key: string | number }) {
  contextMenuVisible.value = false
  
  const keyStr = String(key)
  switch (keyStr) {
    case 'close':
      // 关闭当前标签
      if (currentContextTab.closable && currentContextTab.key !== 'sql') {
        closeTab(currentContextTab.key)
      }
      break
      
    case 'closeOthers':
      // 关闭其他标签（保留当前和 SQL）
      if (currentContextTab.key === 'sql') {
        dataTabs.value = []
        mainTabKey.value = 'sql'
      } else {
        dataTabs.value = dataTabs.value.filter(tab => tab.key === currentContextTab.key)
        mainTabKey.value = currentContextTab.key
      }
      message.success('已关闭其他标签')
      break
      
    case 'closeAll':
      // 关闭所有标签（除了 SQL）
      dataTabs.value = []
      mainTabKey.value = 'sql'
      message.success('已关闭所有标签')
      break
      
    case 'closeLeft':
      // 关闭左侧所有标签
      const currentIndex = dataTabs.value.findIndex(tab => tab.key === currentContextTab.key)
      if (currentIndex > 0) {
        const tabsToClose = dataTabs.value.slice(0, currentIndex)
        dataTabs.value = dataTabs.value.slice(currentIndex)
        
        // 如果当前活动标签被关闭了，切换到当前上下文标签
        if (tabsToClose.some(tab => tab.key === mainTabKey.value)) {
          mainTabKey.value = currentContextTab.key
        }
        message.success(`已关闭左侧 ${tabsToClose.length} 个标签`)
      }
      break
      
    case 'closeRight':
      // 关闭右侧所有标签
      if (currentContextTab.key === 'sql') {
        dataTabs.value = []
        mainTabKey.value = 'sql'
        message.success('已关闭所有数据标签')
      } else {
        const currentIdx = dataTabs.value.findIndex(tab => tab.key === currentContextTab.key)
        if (currentIdx >= 0 && currentIdx < dataTabs.value.length - 1) {
          const tabsToClose = dataTabs.value.slice(currentIdx + 1)
          dataTabs.value = dataTabs.value.slice(0, currentIdx + 1)
          
          // 如果当前活动标签被关闭了，切换到当前上下文标签
          if (tabsToClose.some(tab => tab.key === mainTabKey.value)) {
            mainTabKey.value = currentContextTab.key
          }
          message.success(`已关闭右侧 ${tabsToClose.length} 个标签`)
        }
      }
      break
      
    case 'closeAllExceptSql':
      // 关闭除 SQL 外的所有标签
      dataTabs.value = []
      mainTabKey.value = 'sql'
      message.success('已关闭所有数据标签')
      break
  }
}

// 关闭单个标签
function closeTab(tabKey: string) {
  const index = dataTabs.value.findIndex(tab => tab.key === tabKey)
  if (index >= 0) {
    dataTabs.value.splice(index, 1)
    
    // 如果关闭的是当前活动标签，切换到相邻标签
    if (mainTabKey.value === tabKey) {
      if (dataTabs.value.length > 0) {
        // 优先切换到右侧标签，如果没有则切换到左侧
        const newIndex = Math.min(index, dataTabs.value.length - 1)
        mainTabKey.value = dataTabs.value[newIndex].key
      } else {
        mainTabKey.value = 'sql'
      }
    }
    message.success('已关闭标签')
  }
}

// 处理表选择
function handleTableSelected(data: any) {
  console.log('=== handleTableSelected 被调用 ===')
  console.log('接收到的数据:', data)
  console.log('数据库:', data.database)
  console.log('表名:', data.table)
  console.log('connectionId:', data.connectionId)
  console.log('当前活动连接:', connectionStore.activeConnectionId)
  
  const connectionId = data.connectionId || connectionStore.activeConnectionId
  console.log('使用的连接ID:', connectionId)
  
  const tabKey = `table-${connectionId}-${data.database}-${data.table}`
  console.log('生成的标签 key:', tabKey)
  
  // 检查是否已经打开
  const existingTab = dataTabs.value.some((tab) => tab.key === tabKey)
  console.log('标签是否已存在:', existingTab)
  
  if (existingTab) {
    console.log('标签已存在，切换到该标签')
    mainTabKey.value = tabKey
    return
  }

  // 添加新标签
  const newTab = {
    key: tabKey,
    title: `${data.table}`,
    type: 'data' as const,
    connectionId: connectionId!,
    database: data.database,
    table: data.table,
  }
  
  console.log('添加新标签:', newTab)
  dataTabs.value.push(newTab)
  
  console.log('切换到新标签')
  mainTabKey.value = tabKey
  
  console.log('当前所有标签:', dataTabs.value.map(t => ({ key: t.key, title: t.title })))
  console.log('当前活动标签:', mainTabKey.value)
  console.log('=== handleTableSelected 完成 ===')
}

// 处理设计表
function handleDesignTable(data: any) {
  const connectionId = data.connectionId || connectionStore.activeConnectionId
  const tabKey = `design-${connectionId}-${data.database}-${data.table}`
  
  // 检查是否已经打开
  if (dataTabs.value.some((tab) => tab.key === tabKey)) {
    mainTabKey.value = tabKey
    return
  }

  // 添加新标签
  dataTabs.value.push({
    key: tabKey,
    title: `设计: ${data.table}`,
    type: 'design',
    connectionId: connectionId!,
    database: data.database,
    table: data.table,
  })
  
  mainTabKey.value = tabKey
}

// 处理数据库选择 - 单击数据库时切换到 SQL 编辑器
async function handleDatabaseSelected(data: any) {
  console.log('=== handleDatabaseSelected 被调用 ===')
  console.log('数据库数据:', data)
  console.log('connectionId:', data.connectionId)
  
  // 设置活动连接
  if (data.connectionId) {
    console.log('设置活动连接:', data.connectionId)
    connectionStore.setActiveConnection(data.connectionId)
  }
  
  // 检查当前连接是否支持 SQL
  if (!isSqlSupported.value) {
    console.log('当前数据库不支持 SQL，切换到对应的编辑器')
    // 对于 Redis，切换到 Redis 命令行并切换数据库
    const activeConnection = connectionStore.getActiveConnection()
    if (activeConnection?.db_type === 'redis') {
      mainTabKey.value = 'redis'
      
      // 等待 Redis 编辑器加载
      await nextTick()
      
      // 切换数据库
      setTimeout(async () => {
        if (redisEditorRef.value && redisEditorRef.value.switchDatabase) {
          try {
            await redisEditorRef.value.switchDatabase(data.name)
            message.success(`已切换到 ${data.name}`)
          } catch (error: any) {
            message.error(`切换数据库失败: ${error}`)
          }
        }
      }, 100)
    } else {
      message.info('当前数据库类型不支持 SQL 查询')
    }
    return
  }
  
  // 切换到 SQL 查询标签页
  mainTabKey.value = 'sql'
  console.log('已切换到 SQL 标签页')
  
  // 设置选中的数据库 - 使用更长的延迟确保组件已挂载
  setTimeout(async () => {
    console.log('准备设置数据库...')
    console.log('sqlEditorRef.value:', sqlEditorRef.value)
    
    if (sqlEditorRef.value) {
      console.log('sqlEditorRef 存在')
      if (sqlEditorRef.value.setSelectedDatabase) {
        console.log('setSelectedDatabase 方法存在')
        try {
          await sqlEditorRef.value.setSelectedDatabase(data.name)
          console.log('数据库设置成功')
        } catch (error) {
          console.error('设置数据库失败:', error)
          message.error(`设置数据库失败: ${error}`)
        }
      } else {
        console.error('setSelectedDatabase 方法不存在')
      }
    } else {
      console.error('sqlEditorRef.value 不存在')
      // 如果编辑器还没准备好,再等待一下
      setTimeout(async () => {
        if (sqlEditorRef.value && sqlEditorRef.value.setSelectedDatabase) {
          try {
            await sqlEditorRef.value.setSelectedDatabase(data.name)
          } catch (error) {
            console.error('第二次尝试设置数据库也失败:', error)
          }
        }
      }, 500)
    }
  }, 100)
}

// 处理新建查询
function handleNewQuery(data: any) {
  console.log('handleNewQuery 被调用，数据:', data)
  
  // 检查当前连接是否支持 SQL
  if (!isSqlSupported.value) {
    message.warning('当前数据库不支持 SQL 查询')
    return
  }
  
  // 切换到 SQL 查询标签页
  mainTabKey.value = 'sql'
  
  // 设置选中的数据库
  nextTick(async () => {
    if (sqlEditorRef.value && sqlEditorRef.value.setSelectedDatabase) {
      console.log('调用 setSelectedDatabase，数据库:', data.database)
      try {
        await sqlEditorRef.value.setSelectedDatabase(data.database)
        console.log('setSelectedDatabase 调用成功')
      } catch (error) {
        console.error('setSelectedDatabase 调用失败:', error)
      }
    } else {
      console.warn('sqlEditorRef 或 setSelectedDatabase 方法不可用')
    }
  })
}

// 处理编辑连接
function handleEditConnection(connection: any) {
  editingConnection.value = connection
  showConnectionDialog.value = true
}

// 处理标签编辑
function onTabEdit(targetKey: string | number | MouseEvent | KeyboardEvent, action: 'add' | 'remove') {
  if (action === 'remove' && typeof targetKey !== 'object') {
    const key = String(targetKey)
    dataTabs.value = dataTabs.value.filter((tab) => tab.key !== key)
  }
}

// 保存设置
function handleSaveSettings() {
  appStore.setTheme(settingsForm.theme)
  message.success('设置已保存')
  showSettings.value = false
}

// 开始拖拽调整侧边栏宽度
function startResize(e: MouseEvent) {
  isResizing.value = true
  const startX = e.clientX
  const startWidth = sidebarWidth.value

  const doResize = (e: MouseEvent) => {
    if (!isResizing.value) return
    
    const delta = e.clientX - startX
    const newWidth = startWidth + delta
    
    // 限制最小宽度 200px，最大宽度 600px
    if (newWidth >= 200 && newWidth <= 600) {
      sidebarWidth.value = newWidth
    }
  }

  const stopResize = () => {
    isResizing.value = false
    document.removeEventListener('mousemove', doResize)
    document.removeEventListener('mouseup', stopResize)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
  }

  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
  document.addEventListener('mousemove', doResize)
  document.addEventListener('mouseup', stopResize)
}

// 监听对话框关闭，重置编辑状态
watch(showConnectionDialog, (visible) => {
  if (!visible) {
    editingConnection.value = null
  }
})

// 监听连接变化，处理 SQL/Redis 编辑器的显示/隐藏
watch(() => connectionStore.activeConnectionId, (newId, oldId) => {
  if (newId !== oldId) {
    // 如果当前激活的是 SQL 标签，但新连接不支持 SQL
    if (mainTabKey.value === 'sql' && !isSqlSupported.value) {
      // 切换到 Redis 标签页
      mainTabKey.value = 'redis'
    }
    // 如果当前激活的是 Redis 标签，但新连接支持 SQL
    else if (mainTabKey.value === 'redis' && isSqlSupported.value) {
      // 切换到 SQL 标签页
      mainTabKey.value = 'sql'
    }
  }
})
</script>

<style scoped>
.main-layout {
  width: 100%;
  height: 100vh;
}

.header {
  background: #fff;
  padding: 0 16px;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.08);
  z-index: 10;
  line-height: 64px;
  height: 64px;
}

.dark-mode .header {
  background: #1f1f1f;
  border-bottom: 1px solid #303030;
}

.header-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 100%;
}

.logo {
  display: flex;
  align-items: center;
  font-size: 20px;
  font-weight: bold;
  color: #1890ff;
  margin-right: 24px;
}

.header-menu {
  flex: 1;
}

.top-menu {
  border-bottom: none;
  background: transparent;
  line-height: 64px;
}

.dark-mode .top-menu {
  background: transparent;
}

.title {
  margin-left: 8px;
}

.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.content-layout {
  height: calc(100vh - 64px);
  display: flex;
  flex-direction: row;
  position: relative;
}

.sidebar {
  background: #fafafa;
  border-right: 1px solid #e8e8e8;
  flex-shrink: 0;
  overflow: hidden;
}

.dark-mode .sidebar {
  background: #141414;
  border-right: 1px solid #303030;
}

.sidebar-content {
  height: 100%;
  padding: 0 12px;
  overflow: auto;
}

.sidebar-resizer {
  width: 4px;
  cursor: col-resize;
  background: transparent;
  flex-shrink: 0;
  position: relative;
  transition: background-color 0.2s;
}

.sidebar-resizer:hover {
  background: #1890ff;
}

.dark-mode .sidebar-resizer:hover {
  background: #177ddc;
}

.sidebar-resizer::before {
  content: '';
  position: absolute;
  left: -2px;
  right: -2px;
  top: 0;
  bottom: 0;
}

.main-content {
  background: #fff;
  overflow: auto;
  flex: 1;
  min-width: 0;
}

.dark-mode .main-content {
  background: #1f1f1f;
}

.main-tabs :deep(.ant-tabs-tab) {
  user-select: none;
}

.main-tabs :deep(.ant-tabs-tab):hover {
  background-color: rgba(0, 0, 0, 0.03);
}

.dark-mode .main-tabs :deep(.ant-tabs-tab):hover {
  background-color: rgba(255, 255, 255, 0.05);
}
</style>

