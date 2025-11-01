<template>
  <div class="database-tree">
    <a-spin :spinning="loading" tip="加载中...">
      <div class="custom-tree">
        <div
          v-for="node in treeData"
          :key="node.key"
          class="tree-node-wrapper"
        >
          <TreeNodeItem
            :node="node"
            :level="0"
            :expanded-keys="expandedKeys"
            :selected-keys="selectedKeys"
            :loading-nodes="loadingNodes"
            @toggle="handleToggle"
            @select="handleSelect"
            @dblclick="handleDoubleClick"
            @contextmenu="onRightClick"
          />
        </div>
      </div>

      <a-empty
        v-if="!loading && treeData.length === 0"
        description="请先选择一个连接"
        :image-style="{ height: '60px' }"
      />
    </a-spin>

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
          <!-- 数据库级别菜单 -->
          <template v-if="selectedNode?.type === 'database'">
            <a-menu-item v-if="isSqlSupported" key="new-query">
              <FileTextOutlined />
              新建查询
            </a-menu-item>
            <a-menu-divider v-if="isSqlSupported" />
            <a-menu-item key="open-database">
              <FolderOpenOutlined />
              打开数据库
            </a-menu-item>
            <a-menu-divider v-if="isSqlSupported" />
            <a-menu-item v-if="isSqlSupported" key="new-table">
              <PlusOutlined />
              新建表
            </a-menu-item>
            <a-menu-item v-if="isSqlSupported" key="new-view">
              <EyeOutlined />
              新建视图
            </a-menu-item>
            <a-menu-divider />
            <a-menu-item key="backup-database">
              <ExportOutlined />
              备份数据库
            </a-menu-item>
            <a-menu-item key="restore-database">
              <ImportOutlined />
              还原数据库
            </a-menu-item>
            <a-menu-divider v-if="canDropDatabase" />
            <a-menu-item v-if="canDropDatabase" key="drop-database" danger>
              <DeleteOutlined />
              删除数据库
            </a-menu-item>
          </template>
          
          <!-- 表级别菜单 -->
          <template v-if="selectedNode?.type === 'table'">
            <a-menu-item key="view-data">
              <TableOutlined />
              查看数据
            </a-menu-item>
            <a-menu-item key="view-structure">
              <ProfileOutlined />
              设计表
            </a-menu-item>
            <a-menu-divider />
            <a-menu-item key="insert-record">
              <PlusOutlined />
              插入记录
            </a-menu-item>
            <a-menu-item key="export-table">
              <ExportOutlined />
              导出表
            </a-menu-item>
            <a-menu-item key="import-table">
              <ImportOutlined />
              导入数据
            </a-menu-item>
            <a-menu-divider />
            <a-menu-item key="truncate-table" danger>
              <ClearOutlined />
              清空表
            </a-menu-item>
            <a-menu-item key="drop-table" danger>
              <DeleteOutlined />
              删除表
            </a-menu-item>
          </template>
          
          <!-- 视图级别菜单 -->
          <template v-if="selectedNode?.type === 'view'">
            <a-menu-item key="view-data">
              <EyeOutlined />
              查看数据
            </a-menu-item>
            <a-menu-item key="view-definition">
              <ProfileOutlined />
              查看定义
            </a-menu-item>
            <a-menu-divider />
            <a-menu-item key="drop-view" danger>
              <DeleteOutlined />
              删除视图
            </a-menu-item>
          </template>
          
          <!-- 存储过程级别菜单 -->
          <template v-if="selectedNode?.type === 'procedure'">
            <a-menu-item key="execute-procedure">
              <CaretRightOutlined />
              执行存储过程
            </a-menu-item>
            <a-menu-item key="view-procedure-definition">
              <ProfileOutlined />
              查看定义
            </a-menu-item>
            <a-menu-divider />
            <a-menu-item key="drop-procedure" danger>
              <DeleteOutlined />
              删除存储过程
            </a-menu-item>
          </template>
          
          <!-- 函数级别菜单 -->
          <template v-if="selectedNode?.type === 'function'">
            <a-menu-item key="view-function-definition">
              <ProfileOutlined />
              查看定义
            </a-menu-item>
            <a-menu-divider />
            <a-menu-item key="drop-function" danger>
              <DeleteOutlined />
              删除函数
            </a-menu-item>
          </template>
          
          <!-- 触发器级别菜单 -->
          <template v-if="selectedNode?.type === 'trigger'">
            <a-menu-item key="view-trigger-definition">
              <ProfileOutlined />
              查看定义
            </a-menu-item>
            <a-menu-divider />
            <a-menu-item key="drop-trigger" danger>
              <DeleteOutlined />
              删除触发器
            </a-menu-item>
          </template>
          
          <!-- 事件级别菜单 -->
          <template v-if="selectedNode?.type === 'event'">
            <a-menu-item key="view-event-definition">
              <ProfileOutlined />
              查看定义
            </a-menu-item>
            <a-menu-divider />
            <a-menu-item key="drop-event" danger>
              <DeleteOutlined />
              删除事件
            </a-menu-item>
          </template>
          
          <!-- 分组节点菜单 -->
          <template v-if="['tables', 'views', 'procedures', 'functions', 'triggers', 'events'].includes(selectedNode?.type || '')">
            <a-menu-item key="new-object">
              <PlusOutlined />
              新建{{ getObjectTypeName(selectedNode?.type) }}
            </a-menu-item>
            <a-menu-divider />
            <a-menu-item key="refresh-group">
              <ReloadOutlined />
              刷新{{ getObjectTypeName(selectedNode?.type) }}
            </a-menu-item>
          </template>
          
          <!-- 通用菜单 -->
          <a-menu-divider />
          <a-menu-item key="refresh">
            <ReloadOutlined />
            刷新
          </a-menu-item>
          <a-menu-item key="copy-name">
            <CopyOutlined />
            复制名称
          </a-menu-item>
        </a-menu>
      </div>
    </div>

    <!-- 各种对话框 -->
    <CreateTableDialog
      v-model="showCreateTableDialog"
      :connection-id="connectionId!"
      :database="currentDatabase"
      @created="handleTableCreated"
    />
    
    <CreateViewDialog
      v-model="showCreateViewDialog"
      :connection-id="connectionId!"
      :database="currentDatabase"
      @created="handleViewCreated"
    />
    
    <InsertRecordDialog
      v-model="showInsertRecordDialog"
      :connection-id="connectionId!"
      :database="currentDatabase"
      :table="currentTable"
      @inserted="handleRecordInserted"
    />
    
    <ExportTableDialog
      v-model="showExportTableDialog"
      :connection-id="connectionId!"
      :database="currentDatabase"
      :table="currentTable"
      @exported="handleTableExported"
    />
    
    <ImportDataDialog
      v-model="showImportDataDialog"
      :connection-id="connectionId!"
      :database="currentDatabase"
      :table="currentTable"
      @imported="handleDataImported"
    />
    
    <BackupDatabaseDialog
      v-model="showBackupDatabaseDialog"
      :connection-id="connectionId!"
      :database="currentDatabase"
      @backed="handleDatabaseBacked"
    />
    
    <RestoreDatabaseDialog
      v-model="showRestoreDatabaseDialog"
      :connection-id="connectionId!"
      :database="currentDatabase"
      @restored="handleDatabaseRestored"
    />
  </div>
</template>

<script setup lang="ts">
import { h } from 'vue'
import {
  TableOutlined,
  ReloadOutlined,
  ProfileOutlined,
  CopyOutlined,
  FolderOpenOutlined,
  PlusOutlined,
  EyeOutlined,
  ExportOutlined,
  ImportOutlined,
  DeleteOutlined,
  ClearOutlined,
  CaretRightOutlined,
  FileTextOutlined,
} from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import type { DatabaseInfo, TableInfo } from '@/types/database'
import TreeNodeItem from './TreeNodeItem.vue'
import CreateTableDialog from './CreateTableDialog.vue'
import CreateViewDialog from './CreateViewDialog.vue'
import InsertRecordDialog from './InsertRecordDialog.vue'
import ExportTableDialog from './ExportTableDialog.vue'
import ImportDataDialog from './ImportDataDialog.vue'
import BackupDatabaseDialog from './BackupDatabaseDialog.vue'
import RestoreDatabaseDialog from './RestoreDatabaseDialog.vue'

interface TreeNode {
  key: string
  title: string
  type: string
  icon?: any
  children?: TreeNode[]
  isLeaf?: boolean
  metadata?: any
}

const props = defineProps<{
  connectionId: string | null
  dbType?: string
}>()

const emit = defineEmits(['table-selected', 'database-selected', 'new-query', 'design-table'])

// 判断当前数据库是否支持 SQL
const isSqlSupported = computed(() => {
  if (!props.dbType) return true
  const nonSqlTypes = ['redis', 'mongodb', 'elasticsearch']
  return !nonSqlTypes.includes(props.dbType.toLowerCase())
})

// 判断是否可以删除数据库（SQLite 不支持 DROP DATABASE）
const canDropDatabase = computed(() => {
  if (!props.dbType) return true
  return props.dbType.toLowerCase() !== 'sqlite'
})

const loading = ref(false)
const treeData = ref<TreeNode[]>([])
const expandedKeys = ref<string[]>([])
const selectedKeys = ref<string[]>([])
const loadingNodes = ref<Set<string>>(new Set()) // 正在加载的节点

// 右键菜单
const contextMenuVisible = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const selectedNode = ref<TreeNode | null>(null)

// 对话框状态
const showCreateTableDialog = ref(false)
const showCreateViewDialog = ref(false)
const showInsertRecordDialog = ref(false)
const showExportTableDialog = ref(false)
const showImportDataDialog = ref(false)
const showBackupDatabaseDialog = ref(false)
const showRestoreDatabaseDialog = ref(false)

// 当前操作的数据库和表
const currentDatabase = ref('')
const currentTable = ref('')

// 获取图标
// function getIcon(type: string) {
//   const iconMap: Record<string, any> = {
//     connection: DatabaseOutlined,
//     database: DatabaseOutlined,
//     tables: FolderOutlined,
//     table: TableOutlined,
//     views: EyeOutlined,
//     view: EyeOutlined,
//     procedures: FolderOutlined,
//     procedure: FileOutlined,
//     functions: FolderOutlined,
//     function: FileOutlined,
//     triggers: FolderOutlined,
//     trigger: FileOutlined,
//     events: FolderOutlined,
//     event: FileOutlined,
//   }
//   return iconMap[type] || FileOutlined
// }

// 获取对象类型名称
function getObjectTypeName(type?: string) {
  const nameMap: Record<string, string> = {
    tables: '表',
    views: '视图',
    procedures: '存储过程',
    functions: '函数',
    triggers: '触发器',
    events: '事件',
  }
  return nameMap[type || ''] || '对象'
}

// 根据key查找节点
function findNodeByKey(nodes: TreeNode[], key: string): TreeNode | null {
  for (const node of nodes) {
    if (node.key === key) {
      return node
    }
    if (node.children) {
      const found = findNodeByKey(node.children, key)
      if (found) return found
    }
  }
  return null
}

// 加载数据库列表
async function loadDatabases() {
  if (!props.connectionId) return

  loading.value = true
  try {
    // SQLite 是单文件数据库，直接显示表、视图等分组，不显示数据库层级
    if (props.dbType === 'sqlite') {
      treeData.value = [
        {
          key: 'tables',
          title: '表',
          type: 'tables',
          isLeaf: false,
          metadata: { database: 'main' },
        },
        {
          key: 'views',
          title: '视图',
          type: 'views',
          isLeaf: false,
          metadata: { database: 'main' },
        },
      ]
    } else {
      // 其他数据库类型显示数据库列表
      const databases = await invoke<DatabaseInfo[]>('get_databases', {
        connectionId: props.connectionId,
      })

      treeData.value = databases.map((db) => ({
        key: `db-${db.name}`,
        title: db.name,
        type: 'database',
        isLeaf: false,
        metadata: db,
      }))
    }
  } catch (error: any) {
    message.error(`加载数据库列表失败: ${error}`)
  } finally {
    loading.value = false
  }
}

// 根据 key 在 treeData 中查找并更新节点
function updateNodeInTree(nodes: TreeNode[], targetKey: string, updater: (node: TreeNode) => void): boolean {
  for (const node of nodes) {
    if (node.key === targetKey) {
      updater(node)
      return true
    }
    if (node.children && node.children.length > 0) {
      if (updateNodeInTree(node.children, targetKey, updater)) {
        return true
      }
    }
  }
  return false
}

// 懒加载子节点
async function onLoadData(treeNode: TreeNode): Promise<void> {
  console.log('onLoadData 被调用，节点:', treeNode.key, treeNode.type)
  
  if (treeNode.children && treeNode.children.length > 0) {
    console.log('节点已有子节点，跳过加载')
    return
  }

  // 加载数据库下的对象分组
  if (treeNode.type === 'database') {
    console.log('加载数据库对象分组, 数据库类型:', props.dbType)
    
    let children: TreeNode[] = []
    
    // 根据数据库类型显示不同的树结构
    if (props.dbType === 'redis') {
      // Redis: 显示键空间 (Keys)
      children = [
        {
          key: `${treeNode.key}-keys`,
          title: '键 (Keys)',
          type: 'keys',
          isLeaf: false,
          metadata: { database: treeNode.metadata.name },
        },
      ]
    } else if (props.dbType === 'mongodb') {
      // MongoDB: 显示集合 (Collections)
      children = [
        {
          key: `${treeNode.key}-collections`,
          title: '集合 (Collections)',
          type: 'collections',
          isLeaf: false,
          metadata: { database: treeNode.metadata.name },
        },
      ]
    } else {
      // MySQL, PostgreSQL, SQLite: 显示传统的表、视图等
      children = [
        {
          key: `${treeNode.key}-tables`,
          title: '表',
          type: 'tables',
          isLeaf: false,
          metadata: { database: treeNode.metadata.name },
        },
        {
          key: `${treeNode.key}-views`,
          title: '视图',
          type: 'views',
          isLeaf: false,
          metadata: { database: treeNode.metadata.name },
        },
      ]
      
      // MySQL 和 PostgreSQL 支持存储过程等
      if (props.dbType === 'mysql' || props.dbType === 'postgresql') {
        children.push(
          {
            key: `${treeNode.key}-procedures`,
            title: '存储过程',
            type: 'procedures',
            isLeaf: false,
            metadata: { database: treeNode.metadata.name },
          },
          {
            key: `${treeNode.key}-functions`,
            title: '函数',
            type: 'functions',
            isLeaf: false,
            metadata: { database: treeNode.metadata.name },
          },
          {
            key: `${treeNode.key}-triggers`,
            title: '触发器',
            type: 'triggers',
            isLeaf: false,
            metadata: { database: treeNode.metadata.name },
          }
        )
      }
      
      // MySQL 特有的事件
      if (props.dbType === 'mysql') {
        children.push({
          key: `${treeNode.key}-events`,
          title: '事件',
          type: 'events',
          isLeaf: false,
          metadata: { database: treeNode.metadata.name },
        })
      }
    }
    
    // 更新 treeData 触发响应式更新
    updateNodeInTree(treeData.value, treeNode.key, (node) => {
      node.children = children
    })
    // 强制触发响应式
    treeData.value = [...treeData.value]
    console.log('数据库对象分组加载完成')
    return
  }
  // 加载集合列表 (MongoDB)
  if (treeNode.type === 'collections') {
    console.log('=== 开始加载集合列表 (MongoDB) ===')
    try {
      const tables = await invoke<TableInfo[]>('get_tables', {
        connectionId: props.connectionId,
        database: treeNode.metadata.database,
      })

      const children: TreeNode[] = tables.map((table) => ({
        key: `${treeNode.key}-${table.name}`,
        title: table.name,
        type: 'collection',
        isLeaf: true,
        metadata: { ...table, database: treeNode.metadata.database },
      }))
      
      if (tables.length === 0) {
        children.push({
          key: `${treeNode.key}-empty`,
          title: '(无集合)',
          type: 'empty',
          isLeaf: true,
          metadata: {},
        })
      }
      
      updateNodeInTree(treeData.value, treeNode.key, (node) => {
        node.children = children
      })
      treeData.value = [...treeData.value]
      console.log('=== 集合列表加载完成 ===')
    } catch (error: any) {
      console.error('加载集合列表失败:', error)
      message.error(`加载集合列表失败: ${error}`)
    }
    return
  }
  
  // 加载键列表 (Redis)
  if (treeNode.type === 'keys') {
    console.log('=== 开始加载 Redis 键列表 ===')
    try {
      // 调用 get_tables 获取 Redis 键信息
      const keys = await invoke<TableInfo[]>('get_tables', {
        connectionId: props.connectionId,
        database: treeNode.metadata.database,
      })

      const children: TreeNode[] = keys.map((key) => ({
        key: `${treeNode.key}-${key.name}`,
        title: key.name,
        type: 'redis-key',
        isLeaf: true,
        metadata: { ...key, database: treeNode.metadata.database },
      }))
      
      if (keys.length === 0) {
        children.push({
          key: `${treeNode.key}-empty`,
          title: '(无键)',
          type: 'empty',
          isLeaf: true,
          metadata: {},
        })
      }
      
      updateNodeInTree(treeData.value, treeNode.key, (node) => {
        node.children = children
      })
      treeData.value = [...treeData.value]
      console.log('=== Redis 键列表加载完成 ===')
    } catch (error: any) {
      console.error('加载 Redis 键列表失败:', error)
      message.error(`加载键列表失败: ${error}`)
    }
    return
  }
  
  // 加载表列表
  if (treeNode.type === 'tables') {
    console.log('=== 开始加载表列表 ===')
    console.log('数据库:', treeNode.metadata.database)
    console.log('父节点 key:', treeNode.key)
    
    try {
      const tables = await invoke<TableInfo[]>('get_tables', {
        connectionId: props.connectionId,
        database: treeNode.metadata.database,
      })
      
      console.log('获取到的表数量:', tables.length)
      console.log('表列表:', tables.map(t => t.name))

      const children: TreeNode[] = tables.map((table) => ({
        key: `${treeNode.key}-${table.name}`,
        title: table.name,
        type: 'table',
        isLeaf: true,
        metadata: { ...table, database: treeNode.metadata.database },
      }))
      
      console.log('生成的子节点数量:', children.length)
      
      if (tables.length === 0) {
        console.log('没有表，设置空节点')
        children.push({
          key: `${treeNode.key}-empty`,
          title: '(无表)',
          type: 'empty',
          isLeaf: true,
          metadata: {},
        })
      }
      
      // 更新 treeData 触发响应式更新
      console.log('开始更新 treeData...')
      const updated = updateNodeInTree(treeData.value, treeNode.key, (node) => {
        node.children = children
      })
      console.log('updateNodeInTree 返回:', updated)
      
      // 强制触发响应式
      treeData.value = [...treeData.value]
      console.log('treeData 已更新，新长度:', treeData.value.length)
      console.log('=== 表列表加载完成 ===')
    } catch (error: any) {
      console.error('加载表列表失败:', error)
      message.error(`加载表列表失败: ${error}`)
    }
    return
  }
  // 加载视图列表
  if (treeNode.type === 'views') {
    try {
      const views = await invoke<TableInfo[]>('get_views', {
        connectionId: props.connectionId,
        database: treeNode.metadata.database,
      })

      const children: TreeNode[] = views.map((view) => ({
        key: `${treeNode.key}-${view.name}`,
        title: view.name,
        type: 'view',
        isLeaf: true,
        metadata: { ...view, database: treeNode.metadata.database },
      }))
      
      if (views.length === 0) {
        children.push({
          key: `${treeNode.key}-empty`,
          title: '(无视图)',
          type: 'empty',
          isLeaf: true,
          metadata: {},
        })
      }
      
      // 更新 treeData 触发响应式更新
      updateNodeInTree(treeData.value, treeNode.key, (node) => {
        node.children = children
      })
      treeData.value = [...treeData.value]
    } catch (error: any) {
      message.error(`加载视图列表失败: ${error}`)
    }
    return
  }
  // 加载存储过程列表
  if (treeNode.type === 'procedures') {
    try {
      const procedures = await invoke<any[]>('get_procedures', {
        connectionId: props.connectionId,
        database: treeNode.metadata.database,
      })

      treeNode.children = procedures.map((proc) => ({
        key: `${treeNode.key}-${proc.ROUTINE_NAME}`,
        title: proc.ROUTINE_NAME,
        type: 'procedure',
        isLeaf: true,
        metadata: { ...proc, database: treeNode.metadata.database },
      }))
      
      if (procedures.length === 0) {
        treeNode.children = [{
          key: `${treeNode.key}-empty`,
          title: '(无存储过程)',
          type: 'empty',
          isLeaf: true,
          metadata: {},
        }]
      }
    } catch (error: any) {
      message.error(`加载存储过程列表失败: ${error}`)
      treeNode.children = []
    }
    return
  }
  // 加载函数列表
  if (treeNode.type === 'functions') {
    try {
      const functions = await invoke<any[]>('get_functions', {
        connectionId: props.connectionId,
        database: treeNode.metadata.database,
      })

      treeNode.children = functions.map((func) => ({
        key: `${treeNode.key}-${func.ROUTINE_NAME}`,
        title: func.ROUTINE_NAME,
        type: 'function',
        isLeaf: true,
        metadata: { ...func, database: treeNode.metadata.database },
      }))
      
      if (functions.length === 0) {
        treeNode.children = [{
          key: `${treeNode.key}-empty`,
          title: '(无函数)',
          type: 'empty',
          isLeaf: true,
          metadata: {},
        }]
      }
    } catch (error: any) {
      message.error(`加载函数列表失败: ${error}`)
      treeNode.children = []
    }
    return
  }
  // 加载触发器列表
  if (treeNode.type === 'triggers') {
    try {
      const triggers = await invoke<any[]>('get_triggers', {
        connectionId: props.connectionId,
        database: treeNode.metadata.database,
      })

      treeNode.children = triggers.map((trigger) => ({
        key: `${treeNode.key}-${trigger.TRIGGER_NAME}`,
        title: trigger.TRIGGER_NAME,
        type: 'trigger',
        isLeaf: true,
        metadata: { ...trigger, database: treeNode.metadata.database },
      }))
      
      if (triggers.length === 0) {
        treeNode.children = [{
          key: `${treeNode.key}-empty`,
          title: '(无触发器)',
          type: 'empty',
          isLeaf: true,
          metadata: {},
        }]
      }
    } catch (error: any) {
      message.error(`加载触发器列表失败: ${error}`)
      treeNode.children = []
    }
    return
  }
  // 加载事件列表
  if (treeNode.type === 'events') {
    try {
      const events = await invoke<any[]>('get_events', {
        connectionId: props.connectionId,
        database: treeNode.metadata.database,
      })

      treeNode.children = events.map((event) => ({
        key: `${treeNode.key}-${event.EVENT_NAME}`,
        title: event.EVENT_NAME,
        type: 'event',
        isLeaf: true,
        metadata: { ...event, database: treeNode.metadata.database },
      }))
      
      if (events.length === 0) {
        treeNode.children = [{
          key: `${treeNode.key}-empty`,
          title: '(无事件)',
          type: 'empty',
          isLeaf: true,
          metadata: {},
        }]
      }
    } catch (error: any) {
      message.error(`加载事件列表失败: ${error}`)
      treeNode.children = []
    }
    return
  }
}

// 处理切换展开/收缩
async function handleToggle(node: TreeNode) {
  console.log('handleToggle:', node.key, node.type)
  const key = node.key
  const isCurrentlyExpanded = expandedKeys.value.includes(key)
  
  if (!isCurrentlyExpanded) {
    // 展开节点
    expandedKeys.value = [...expandedKeys.value, key]
    
    // 如果没有子节点，加载数据
    if (!node.children || node.children.length === 0) {
      loadingNodes.value.add(key)
      loadingNodes.value = new Set(loadingNodes.value)
      
      try {
        await onLoadData(node)
      } finally {
        loadingNodes.value.delete(key)
        loadingNodes.value = new Set(loadingNodes.value)
      }
    }
  } else {
    // 收起节点
    expandedKeys.value = expandedKeys.value.filter(k => k !== key)
  }
}

// 处理选择节点
function handleSelect(node: TreeNode) {
  console.log('handleSelect:', node.key, node.type)
  selectedKeys.value = [node.key]
  
  // 单击数据库节点时，触发选择事件（用于切换到 SQL 编辑器）
  if (node.type === 'database') {
    emit('database-selected', node.metadata)
  }
}

// 双击处理
async function handleDoubleClick(node: TreeNode) {
  console.log('=== handleDoubleClick 被调用 ===')
  console.log('节点类型:', node.type)
  console.log('节点标题:', node.title)
  console.log('节点 key:', node.key)
  console.log('节点元数据:', node.metadata)
  
  if (node.type === 'database') {
    console.log('处理数据库双击')
    // 双击数据库时展开/收缩并加载表
    const key = node.key
    const isCurrentlyExpanded = expandedKeys.value.includes(key)
    console.log('当前展开状态:', isCurrentlyExpanded)
    
    if (!isCurrentlyExpanded) {
      console.log('准备展开数据库')
      
      // 先添加到展开列表,立即更新UI
      expandedKeys.value = [...expandedKeys.value, key]
      
      // 添加加载状态
      loadingNodes.value.add(key)
      loadingNodes.value = new Set(loadingNodes.value)
      
      try {
        // 检查是否已有子节点
        if (!node.children || node.children.length === 0) {
          console.log('数据库无子节点,开始加载')
          await onLoadData(node)
          console.log('数据库数据加载完成')
        } else {
          console.log('数据库已有子节点,跳过加载')
        }
      } catch (error) {
        console.error('加载数据失败:', error)
        // 加载失败时移除展开状态
        expandedKeys.value = expandedKeys.value.filter(k => k !== key)
      } finally {
        loadingNodes.value.delete(key)
        loadingNodes.value = new Set(loadingNodes.value)
      }
    } else {
      console.log('收起数据库')
      expandedKeys.value = expandedKeys.value.filter(k => k !== key)
    }
    // 不再在双击时触发 database-selected，只负责展开/收缩
  } else if (node.type === 'table' || node.type === 'view' || node.type === 'collection' || node.type === 'redis-key') {
    console.log('=== 处理表/视图/集合/Redis键双击 ===')
    console.log('名称:', node.metadata?.name || node.title)
    console.log('数据库:', node.metadata?.database)
    
    // 双击时查看数据
    const key = node.key
    loadingNodes.value.add(key)
    // 强制触发响应式更新
    loadingNodes.value = new Set(loadingNodes.value)
    
    try {
      const eventData = {
        database: node.metadata.database,
        table: node.metadata.name || node.title,
        metadata: node.metadata,
      }
      
      console.log('准备触发 table-selected 事件，数据:', eventData)
      emit('table-selected', eventData)
      console.log('table-selected 事件已触发')
      
      // 给一点延迟，让用户看到加载动画
      await new Promise(resolve => setTimeout(resolve, 300))
    } finally {
      loadingNodes.value.delete(key)
      // 强制触发响应式更新
      loadingNodes.value = new Set(loadingNodes.value)
      console.log('=== 表/视图双击处理完成 ===')
    }
  } else if (['tables', 'views', 'procedures', 'functions', 'triggers', 'events', 'collections', 'keys'].includes(node.type)) {
    console.log('处理分组节点双击:', node.type)
    // 双击分组节点时展开/收缩
    const key = node.key
    
    // 先检查是否已经在展开状态中
    const isCurrentlyExpanded = expandedKeys.value.includes(key)
    console.log('当前展开状态:', isCurrentlyExpanded)
    
    if (!isCurrentlyExpanded) {
      console.log('准备展开分组节点')
      
      // 先添加到展开列表,立即更新UI
      expandedKeys.value = [...expandedKeys.value, key]
      
      // 添加加载状态
      loadingNodes.value.add(key)
      loadingNodes.value = new Set(loadingNodes.value)
      
      // 异步加载数据
      try {
        // 检查是否已有子节点
        if (!node.children || node.children.length === 0) {
          console.log('节点无子节点,开始加载数据')
          await onLoadData(node)
          console.log('分组数据加载完成')
        } else {
          console.log('节点已有子节点,跳过加载')
        }
      } catch (error) {
        console.error('加载数据失败:', error)
        // 加载失败时移除展开状态
        expandedKeys.value = expandedKeys.value.filter(k => k !== key)
      } finally {
        loadingNodes.value.delete(key)
        loadingNodes.value = new Set(loadingNodes.value)
      }
    } else {
      console.log('收起分组节点')
      expandedKeys.value = expandedKeys.value.filter(k => k !== key)
    }
  } else {
    console.log('未处理的节点类型:', node.type)
  }
}


// 右键菜单
function onRightClick({ event, node }: { event: MouseEvent; node: TreeNode }) {
  event.preventDefault()
  selectedNode.value = node
  
  // 先设置一个临时位置显示菜单（用于获取实际尺寸）
  contextMenuX.value = event.clientX
  contextMenuY.value = event.clientY
  contextMenuVisible.value = true
  
  // 在下一帧获取菜单实际尺寸并调整位置
  nextTick(() => {
    const menuElement = document.querySelector('.context-menu') as HTMLElement
    if (!menuElement) return
    
    const menuRect = menuElement.getBoundingClientRect()
    const windowWidth = window.innerWidth
    const windowHeight = window.innerHeight
    const padding = 10 // 距离边界的最小间距
    
    let x = event.clientX
    let y = event.clientY
    
    // 如果菜单会超出右边界，向左调整
    if (x + menuRect.width > windowWidth - padding) {
      x = windowWidth - menuRect.width - padding
    }
    
    // 如果菜单会超出下边界，向上调整
    if (y + menuRect.height > windowHeight - padding) {
      y = windowHeight - menuRect.height - padding
    }
    
    // 确保不会超出左边界和上边界
    x = Math.max(padding, x)
    y = Math.max(padding, y)
    
    // 更新菜单位置
    contextMenuX.value = x
    contextMenuY.value = y
  })
}

// 查看表数据
async function handleViewData() {
  if (!selectedNode.value || !props.connectionId) return
  
  try {
    const result = await invoke('view_table_data', {
      connectionId: props.connectionId,
      table: selectedNode.value.metadata.name || selectedNode.value.title,
      database: selectedNode.value.metadata.database,
      limit: 1000,
    })
    
    // 发射事件，让父组件处理数据显示
    emit('table-selected', {
      database: selectedNode.value.metadata.database,
      table: selectedNode.value.metadata.name || selectedNode.value.title,
      metadata: selectedNode.value.metadata,
      queryResult: result,
    })
  } catch (error: any) {
    message.error(`查看数据失败: ${error}`)
  }
}

// 查看表结构（打开设计器）
async function handleViewStructure() {
  if (!selectedNode.value || !props.connectionId) return
  
  // 触发设计表事件
  emit('design-table', {
    database: selectedNode.value.metadata.database,
    table: selectedNode.value.metadata.name || selectedNode.value.title,
    connectionId: props.connectionId,
  })
}

// 处理菜单点击
async function handleMenuClick({ key }: { key: string | number }) {
  contextMenuVisible.value = false

  if (!selectedNode.value) return

  const keyStr = String(key)
  switch (keyStr) {
    // 数据库级别操作
    case 'new-query':
      handleNewQuery()
      break
    case 'open-database':
      handleDoubleClick(selectedNode.value)
      break
    case 'new-table':
      handleNewTable()
      break
    case 'new-view':
      handleNewView()
      break
    case 'backup-database':
      handleBackupDatabase()
      break
    case 'restore-database':
      handleRestoreDatabase()
      break
    case 'drop-database':
      handleDropDatabase()
      break
      
    // 表级别操作
    case 'view-data':
      if (selectedNode.value.type === 'table' || selectedNode.value.type === 'view') {
        await handleViewData()
      }
      break
    case 'view-structure':
      await handleViewStructure()
      break
    case 'insert-record':
      handleInsertRecord()
      break
    case 'export-table':
      handleExportTable()
      break
    case 'import-table':
      handleImportData()
      break
    case 'truncate-table':
      handleTruncateTable()
      break
    case 'drop-table':
      handleDropTable()
      break
      
    // 视图操作
    case 'view-definition':
      await handleViewDefinition()
      break
    case 'drop-view':
      handleDropView()
      break
      
    // 存储过程操作
    case 'drop-procedure':
      handleDropProcedure()
      break
      
    // 函数操作
    case 'drop-function':
      handleDropFunction()
      break
      
    // 触发器操作
    case 'drop-trigger':
      handleDropTrigger()
      break
      
    // 事件操作
    case 'drop-event':
      handleDropEvent()
      break
      
    // 分组操作
    case 'new-object':
      handleNewObject()
      break
    case 'refresh-group':
      handleRefreshGroup()
      break
      
    // 通用操作
    case 'refresh':
      loadDatabases()
      break
    case 'copy-name':
      navigator.clipboard.writeText(selectedNode.value.title)
      message.success('已复制到剪贴板')
      break
  }
}

// 新建查询
function handleNewQuery() {
  if (!selectedNode.value || selectedNode.value.type !== 'database') return
  
  emit('new-query', {
    database: selectedNode.value.metadata.name,
    connectionId: props.connectionId,
  })
}

// 新建表
function handleNewTable() {
  if (!selectedNode.value || selectedNode.value.type !== 'database') return
  
  currentDatabase.value = selectedNode.value.metadata.name
  showCreateTableDialog.value = true
}

// 新建视图
function handleNewView() {
  if (!selectedNode.value || selectedNode.value.type !== 'database') return
  
  currentDatabase.value = selectedNode.value.metadata.name
  showCreateViewDialog.value = true
}

// 备份数据库
function handleBackupDatabase() {
  if (!selectedNode.value || selectedNode.value.type !== 'database') return
  
  currentDatabase.value = selectedNode.value.metadata.name
  showBackupDatabaseDialog.value = true
}

// 还原数据库
function handleRestoreDatabase() {
  if (!selectedNode.value || selectedNode.value.type !== 'database') return
  
  currentDatabase.value = selectedNode.value.metadata.name
  showRestoreDatabaseDialog.value = true
}

// 插入记录
function handleInsertRecord() {
  if (!selectedNode.value || selectedNode.value.type !== 'table') return
  
  currentDatabase.value = selectedNode.value.metadata.database
  currentTable.value = selectedNode.value.metadata.name || selectedNode.value.title
  showInsertRecordDialog.value = true
}

// 导出表
function handleExportTable() {
  if (!selectedNode.value || selectedNode.value.type !== 'table') return
  
  currentDatabase.value = selectedNode.value.metadata.database
  currentTable.value = selectedNode.value.metadata.name || selectedNode.value.title
  showExportTableDialog.value = true
}

// 导入数据
function handleImportData() {
  if (!selectedNode.value || selectedNode.value.type !== 'table') return
  
  currentDatabase.value = selectedNode.value.metadata.database
  currentTable.value = selectedNode.value.metadata.name || selectedNode.value.title
  showImportDataDialog.value = true
}

// 删除数据库
function handleDropDatabase() {
  if (!selectedNode.value || selectedNode.value.type !== 'database') return
  
  Modal.confirm({
    title: '确认删除数据库',
    content: `确定要删除数据库 "${selectedNode.value.title}" 吗？此操作不可恢复！`,
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    async onOk() {
      try {
        await invoke('execute_query', {
          connectionId: props.connectionId,
          sql: `DROP DATABASE \`${selectedNode.value!.metadata.name}\``,
          database: null,
        })
        message.success('数据库已删除')
        loadDatabases()
      } catch (error: any) {
        message.error(`删除数据库失败: ${error}`)
      }
    },
  })
}

// 对话框回调 - 刷新对应的列表
function handleTableCreated() {
  // 刷新表列表
  // SQLite 直接使用 'tables'，其他数据库使用 'db-{数据库名}-tables'
  const tablesKey = props.dbType === 'sqlite' ? 'tables' : `db-${currentDatabase.value}-tables`
  const tablesNode = findNodeByKey(treeData.value, tablesKey)
  if (tablesNode) {
    tablesNode.children = []
    if (expandedKeys.value.includes(tablesKey)) {
      onLoadData(tablesNode)
    }
  }
}

function handleViewCreated() {
  // 刷新视图列表
  // SQLite 直接使用 'views'，其他数据库使用 'db-{数据库名}-views'
  const viewsKey = props.dbType === 'sqlite' ? 'views' : `db-${currentDatabase.value}-views`
  const viewsNode = findNodeByKey(treeData.value, viewsKey)
  if (viewsNode) {
    viewsNode.children = []
    if (expandedKeys.value.includes(viewsKey)) {
      onLoadData(viewsNode)
    }
  }
}

function handleRecordInserted() {
  // 可以选择刷新当前表的数据视图
  message.success('记录已插入')
}

function handleTableExported() {
  message.success('表已导出')
}

function handleDataImported() {
  // 刷新表数据
  message.success('数据已导入')
}

function handleDatabaseBacked() {
  message.success('数据库已备份')
}

function handleDatabaseRestored() {
  // 可能需要刷新整个数据库树
  loadDatabases()
}

// 清空表
function handleTruncateTable() {
  if (!selectedNode.value || selectedNode.value.type !== 'table') return
  
  Modal.confirm({
    title: '确认清空表',
    content: `确定要清空表 "${selectedNode.value.title}" 的所有数据吗？此操作不可恢复！`,
    okText: '清空',
    okType: 'danger',
    cancelText: '取消',
    async onOk() {
      try {
        await invoke('truncate_table', {
          connectionId: props.connectionId,
          table: selectedNode.value!.metadata.name || selectedNode.value!.title,
          database: selectedNode.value!.metadata.database,
        })
        message.success('表已清空')
      } catch (error: any) {
        message.error(`清空表失败: ${error}`)
      }
    },
  })
}

// 删除表
function handleDropTable() {
  if (!selectedNode.value || selectedNode.value.type !== 'table') return
  
  Modal.confirm({
    title: '确认删除表',
    content: `确定要删除表 "${selectedNode.value.title}" 吗？此操作不可恢复！`,
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    async onOk() {
      try {
        await invoke('drop_table', {
          connectionId: props.connectionId,
          table: selectedNode.value!.metadata.name || selectedNode.value!.title,
          database: selectedNode.value!.metadata.database,
        })
        message.success('表已删除')
        
        // 刷新表列表
        const parentKey = selectedNode.value!.key.split('-').slice(0, -1).join('-')
        const parentNode = findNodeByKey(treeData.value, parentKey)
        if (parentNode) {
          parentNode.children = []
          if (expandedKeys.value.includes(parentKey)) {
            onLoadData(parentNode)
          }
        }
      } catch (error: any) {
        message.error(`删除表失败: ${error}`)
      }
    },
  })
}

// 查看视图定义
async function handleViewDefinition() {
  if (!selectedNode.value || selectedNode.value.type !== 'view') return
  
  try {
    const definition = await invoke<string>('get_view_definition', {
      connectionId: props.connectionId,
      view: selectedNode.value.metadata.name || selectedNode.value.title,
      database: selectedNode.value.metadata.database,
    })
    
    Modal.info({
      title: `视图定义 - ${selectedNode.value.title}`,
      width: 800,
      content: h('pre', {
        style: {
          backgroundColor: '#f5f5f5',
          padding: '12px',
          borderRadius: '4px',
          fontSize: '12px',
          fontFamily: 'monospace',
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-all',
        }
      }, definition)
    })
  } catch (error: any) {
    message.error(`获取视图定义失败: ${error}`)
  }
}

// 删除视图
function handleDropView() {
  if (!selectedNode.value || selectedNode.value.type !== 'view') return
  
  Modal.confirm({
    title: '确认删除视图',
    content: `确定要删除视图 "${selectedNode.value.title}" 吗？此操作不可恢复！`,
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    async onOk() {
      try {
        await invoke('drop_view', {
          connectionId: props.connectionId,
          view: selectedNode.value!.metadata.name || selectedNode.value!.title,
          database: selectedNode.value!.metadata.database,
        })
        message.success('视图已删除')
        
        // 刷新视图列表
        const parentKey = selectedNode.value!.key.split('-').slice(0, -1).join('-')
        const parentNode = findNodeByKey(treeData.value, parentKey)
        if (parentNode) {
          parentNode.children = []
          if (expandedKeys.value.includes(parentKey)) {
            onLoadData(parentNode)
          }
        }
      } catch (error: any) {
        message.error(`删除视图失败: ${error}`)
      }
    },
  })
}

// 删除存储过程
function handleDropProcedure() {
  if (!selectedNode.value || selectedNode.value.type !== 'procedure') return
  
  Modal.confirm({
    title: '确认删除存储过程',
    content: `确定要删除存储过程 "${selectedNode.value.title}" 吗？此操作不可恢复！`,
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    async onOk() {
      try {
        await invoke('drop_procedure', {
          connectionId: props.connectionId,
          procedure: selectedNode.value!.metadata.ROUTINE_NAME || selectedNode.value!.title,
          database: selectedNode.value!.metadata.database,
        })
        message.success('存储过程已删除')
        
        // 刷新存储过程列表
        const parentKey = selectedNode.value!.key.split('-').slice(0, -1).join('-')
        const parentNode = findNodeByKey(treeData.value, parentKey)
        if (parentNode) {
          parentNode.children = []
          if (expandedKeys.value.includes(parentKey)) {
            onLoadData(parentNode)
          }
        }
      } catch (error: any) {
        message.error(`删除存储过程失败: ${error}`)
      }
    },
  })
}

// 删除函数
function handleDropFunction() {
  if (!selectedNode.value || selectedNode.value.type !== 'function') return
  
  Modal.confirm({
    title: '确认删除函数',
    content: `确定要删除函数 "${selectedNode.value.title}" 吗？此操作不可恢复！`,
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    async onOk() {
      try {
        await invoke('drop_function', {
          connectionId: props.connectionId,
          function: selectedNode.value!.metadata.ROUTINE_NAME || selectedNode.value!.title,
          database: selectedNode.value!.metadata.database,
        })
        message.success('函数已删除')
        
        // 刷新函数列表
        const parentKey = selectedNode.value!.key.split('-').slice(0, -1).join('-')
        const parentNode = findNodeByKey(treeData.value, parentKey)
        if (parentNode) {
          parentNode.children = []
          if (expandedKeys.value.includes(parentKey)) {
            onLoadData(parentNode)
          }
        }
      } catch (error: any) {
        message.error(`删除函数失败: ${error}`)
      }
    },
  })
}

// 删除触发器
function handleDropTrigger() {
  if (!selectedNode.value || selectedNode.value.type !== 'trigger') return
  
  Modal.confirm({
    title: '确认删除触发器',
    content: `确定要删除触发器 "${selectedNode.value.title}" 吗？此操作不可恢复！`,
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    async onOk() {
      try {
        await invoke('drop_trigger', {
          connectionId: props.connectionId,
          trigger: selectedNode.value!.metadata.TRIGGER_NAME || selectedNode.value!.title,
          database: selectedNode.value!.metadata.database,
        })
        message.success('触发器已删除')
        
        // 刷新触发器列表
        const parentKey = selectedNode.value!.key.split('-').slice(0, -1).join('-')
        const parentNode = findNodeByKey(treeData.value, parentKey)
        if (parentNode) {
          parentNode.children = []
          if (expandedKeys.value.includes(parentKey)) {
            onLoadData(parentNode)
          }
        }
      } catch (error: any) {
        message.error(`删除触发器失败: ${error}`)
      }
    },
  })
}

// 删除事件
function handleDropEvent() {
  if (!selectedNode.value || selectedNode.value.type !== 'event') return
  
  Modal.confirm({
    title: '确认删除事件',
    content: `确定要删除事件 "${selectedNode.value.title}" 吗？此操作不可恢复！`,
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    async onOk() {
      try {
        await invoke('drop_event', {
          connectionId: props.connectionId,
          event: selectedNode.value!.metadata.EVENT_NAME || selectedNode.value!.title,
          database: selectedNode.value!.metadata.database,
        })
        message.success('事件已删除')
        
        // 刷新事件列表
        const parentKey = selectedNode.value!.key.split('-').slice(0, -1).join('-')
        const parentNode = findNodeByKey(treeData.value, parentKey)
        if (parentNode) {
          parentNode.children = []
          if (expandedKeys.value.includes(parentKey)) {
            onLoadData(parentNode)
          }
        }
      } catch (error: any) {
        message.error(`删除事件失败: ${error}`)
      }
    },
  })
}

// 新建对象
function handleNewObject() {
  if (!selectedNode.value) return
  
  const type = selectedNode.value.type
  
  // SQLite 直接使用 metadata 中的 database，其他数据库需要查找父数据库节点
  if (props.dbType === 'sqlite') {
    currentDatabase.value = selectedNode.value.metadata.database || 'main'
  } else {
    const databaseKey = selectedNode.value.key.split('-').slice(0, 2).join('-')
    const databaseNode = findNodeByKey(treeData.value, databaseKey)
    
    if (!databaseNode) return
    
    currentDatabase.value = databaseNode.metadata.name
  }
  
  if (type === 'tables') {
    showCreateTableDialog.value = true
  } else if (type === 'views') {
    showCreateViewDialog.value = true
  } else {
    const objectType = getObjectTypeName(type)
    message.info(`新建${objectType}功能正在开发中...`)
  }
}

// 刷新分组
function handleRefreshGroup() {
  if (!selectedNode.value) return
  
  // 清空子节点，强制重新加载
  selectedNode.value.children = []
  
  // 如果节点已展开，重新加载数据
  if (expandedKeys.value.includes(selectedNode.value.key)) {
    onLoadData(selectedNode.value)
  }
  
  const objectType = getObjectTypeName(selectedNode.value.type)
  message.success(`${objectType}列表已刷新`)
}

// 监听连接变化
watch(
  () => props.connectionId,
  (newId) => {
    if (newId) {
      loadDatabases()
    } else {
      treeData.value = []
    }
  },
  { immediate: true }
)

defineExpose({
  refresh: loadDatabases,
})
</script>

<style scoped>
.database-tree {
  height: 100%;
  overflow: auto;
  padding: 8px 0;
}

.custom-tree {
  width: 100%;
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
  min-width: 180px;
}

.dark-mode .context-menu {
  background: #1f1f1f;
  border: 1px solid #303030;
}
</style>

