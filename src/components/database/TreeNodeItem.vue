<template>
  <div class="tree-node">
    <div
      :class="['tree-node-content', { selected: isSelected }]"
      @click="handleClick"
      @dblclick="handleDblClick"
      @contextmenu="handleContextMenu"
    >
      <!-- 缩进区域：层级 × 固定步长 -->
      <span class="tree-node-indent" :style="{ width: level * 16 + 'px' }"></span>
      
      <!-- 展开箭头：固定宽度，保证列对齐 -->
      <span class="tree-node-expand" @click="handleToggle">
        <LoadingOutlined v-if="isLoading" spin />
        <template v-else>
          <DownOutlined v-if="hasChildren && isExpanded" />
          <RightOutlined v-else-if="hasChildren" />
          <span v-else class="expand-placeholder"></span>
        </template>
      </span>
      
      <!-- 图标：固定宽度，保证列对齐 -->
      <span class="tree-node-icon" :class="getIconClass(node.type)">
        <!-- 数据库节点：使用 devicon 品牌图标 -->
        <i v-if="node.type === 'database'" :class="getDatabaseIconClass(node)"></i>
        <!-- 其他节点：使用 Tabler Icons -->
        <component v-else :is="getIcon(node.type)" />
      </span>
      
      <!-- 标签文本 -->
      <span class="tree-node-title">{{ node.title }}</span>
    </div>
    <div v-if="isExpanded && node.children && node.children.length > 0" class="tree-node-children">
      <TreeNodeItem
        v-for="child in node.children"
        :key="child.key"
        :node="child"
        :level="level + 1"
        :expanded-keys="expandedKeys"
        :selected-keys="selectedKeys"
        :loading-nodes="loadingNodes"
        @toggle="$emit('toggle', $event)"
        @select="$emit('select', $event)"
        @dblclick="$emit('dblclick', $event)"
        @contextmenu="$emit('contextmenu', $event)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import {
  LoadingOutlined,
  RightOutlined,
  DownOutlined,
} from '@ant-design/icons-vue'
import {
  IconDatabase,
  IconTable,
  IconEye,
  IconCode,
  IconFunction,
  IconBolt,
  IconClock,
  IconFolder,
  IconFile,
  IconServer,
  IconKey,
  IconList,
} from '@tabler/icons-vue'

interface TreeNode {
  key: string
  title: string
  type: string
  isLeaf?: boolean
  children?: TreeNode[]
  metadata?: any
  dbType?: string
}

const props = defineProps<{
  node: TreeNode
  level: number
  expandedKeys: string[]
  selectedKeys: string[]
  loadingNodes: Set<string>
}>()

const emit = defineEmits<{
  toggle: [node: TreeNode]
  select: [node: TreeNode]
  dblclick: [node: TreeNode]
  contextmenu: [payload: { event: MouseEvent; node: TreeNode }]
}>()

const isExpanded = computed(() => props.expandedKeys.includes(props.node.key))
const isSelected = computed(() => props.selectedKeys.includes(props.node.key))
const isLoading = computed(() => props.loadingNodes.has(props.node.key))
const hasChildren = computed(() => !props.node.isLeaf && props.node.type !== 'empty')

const handleToggle = (e: Event) => {
  e.stopPropagation()
  if (hasChildren.value) {
    emit('toggle', props.node)
  }
}

const handleClick = () => {
  emit('select', props.node)
}

const handleDblClick = () => {
  console.log('TreeNodeItem 双击:', props.node.title, props.node.type)
  emit('dblclick', props.node)
}

const handleContextMenu = (e: MouseEvent) => {
  emit('contextmenu', { event: e, node: props.node })
}

// 获取数据库品牌图标类名（使用 devicon）
const getDatabaseIconClass = (node: TreeNode): string => {
  const dbType = node.dbType || node.metadata?.dbType
  const dbTypeLower = dbType?.toLowerCase() || ''
  
  const deviconMap: Record<string, string> = {
    mysql: 'devicon-mysql-plain colored',
    postgresql: 'devicon-postgresql-plain colored',
    sqlite: 'devicon-sqlite-plain colored',
    mongodb: 'devicon-mongodb-plain colored',
    redis: 'devicon-redis-plain colored',
  }
  
  return deviconMap[dbTypeLower] || 'devicon-database-plain colored'
}

// 获取节点图标（Tabler Icons）
const getIcon = (type: string) => {
  const iconMap: Record<string, any> = {
    connection: IconServer,
    database: IconDatabase,
    // 分组节点和具体节点使用相同的语义图标
    tables: IconTable,
    table: IconTable,
    views: IconEye,
    view: IconEye,
    procedures: IconCode,
    procedure: IconCode,
    functions: IconFunction,
    function: IconFunction,
    triggers: IconBolt,
    trigger: IconBolt,
    events: IconClock,
    event: IconClock,
    keys: IconKey,
    key: IconKey,
    values: IconList,
    collections: IconFolder,
    collection: IconTable,
    empty: IconFile,
  }
  return iconMap[type] || IconFile
}

// 获取图标样式类名
const getIconClass = (type: string): string => {
  const classMap: Record<string, string> = {
    connection: 'connection-icon',
    database: 'database-icon',
    tables: 'table-icon',
    table: 'table-icon',
    views: 'view-icon',
    view: 'view-icon',
    procedures: 'procedure-icon',
    procedure: 'procedure-icon',
    functions: 'function-icon',
    function: 'function-icon',
    triggers: 'trigger-icon',
    trigger: 'trigger-icon',
    events: 'event-icon',
    event: 'event-icon',
    keys: 'key-icon',
    key: 'key-icon',
  }
  return classMap[type] || ''
}
</script>

<style scoped>
.tree-node {
  width: 100%;
}

.tree-node-content {
  display: flex;
  align-items: center;
  padding: 4px 8px;
  cursor: pointer;
  user-select: none;
  transition: background-color 0.2s;
  border-radius: 4px;
}

.tree-node-content:hover {
  background-color: #f5f5f5;
}

.dark-mode .tree-node-content:hover {
  background-color: #262626;
}

.tree-node-content.selected {
  background-color: #e6f7ff;
}

.dark-mode .tree-node-content.selected {
  background-color: #111b26;
}

/* 缩进区域：层级 × 固定步长 */
.tree-node-indent {
  flex-shrink: 0;
  height: 1px;
}

/* 展开箭头：固定宽度 16px，保证列对齐 */
.tree-node-expand {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 20px;
  flex-shrink: 0;
}

.tree-node-expand :deep(.anticon) {
  font-size: 12px;
  color: #8c8c8c;
  transition: transform 0.2s;
}

.tree-node-expand:hover :deep(.anticon) {
  color: #1890ff;
}

/* 占位符：无子节点时保持对齐 */
.expand-placeholder {
  display: inline-block;
  width: 12px;
  height: 12px;
}

/* 图标：固定宽度，保证列对齐 */
.tree-node-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  margin-right: 6px;
  flex-shrink: 0;
  border-radius: 4px;
  transition: background-color 0.2s;
}

.tree-node-icon :deep(svg) {
  width: 16px;
  height: 16px;
  stroke-width: 2;
}

.tree-node-icon :deep(.anticon) {
  font-size: 14px;
}

/* devicon 数据库品牌图标样式 */
.tree-node-icon :deep(i) {
  font-size: 16px;
  line-height: 1;
}

/* ========== 图标颜色主题 ========== */

/* 数据库节点 - 蓝色系 */
.tree-node-icon.database-icon {
  background-color: #e6f4ff;
  color: #1677ff;
}

.dark-mode .tree-node-icon.database-icon {
  background-color: #111d2c;
  color: #3c89e8;
}

/* 表节点 - 绿色系 */
.tree-node-icon.table-icon {
  background-color: #f6ffed;
  color: #52c41a;
}

.dark-mode .tree-node-icon.table-icon {
  background-color: #162312;
  color: #73d13d;
}

/* 视图节点 - 紫色系 */
.tree-node-icon.view-icon {
  background-color: #f9f0ff;
  color: #722ed1;
}

.dark-mode .tree-node-icon.view-icon {
  background-color: #1a1625;
  color: #9254de;
}

/* 存储过程节点 - 橙色系 */
.tree-node-icon.procedure-icon {
  background-color: #fff7e6;
  color: #fa8c16;
}

.dark-mode .tree-node-icon.procedure-icon {
  background-color: #2b2111;
  color: #ffa940;
}

/* 函数节点 - 青色系 */
.tree-node-icon.function-icon {
  background-color: #e6fffb;
  color: #13a8a8;
}

.dark-mode .tree-node-icon.function-icon {
  background-color: #112123;
  color: #36cfc9;
}

/* 触发器节点 - 红色系 */
.tree-node-icon.trigger-icon {
  background-color: #fff1f0;
  color: #f5222d;
}

.dark-mode .tree-node-icon.trigger-icon {
  background-color: #2a1215;
  color: #ff7875;
}

/* 事件节点 - 金色系 */
.tree-node-icon.event-icon {
  background-color: #fffbe6;
  color: #faad14;
}

.dark-mode .tree-node-icon.event-icon {
  background-color: #2b2611;
  color: #ffc53d;
}

/* 键节点 - 灰蓝色系 */
.tree-node-icon.key-icon {
  background-color: #f0f5ff;
  color: #597ef7;
}

.dark-mode .tree-node-icon.key-icon {
  background-color: #1d1f45;
  color: #85a5ff;
}

/* 连接节点 - 深蓝色系 */
.tree-node-icon.connection-icon {
  background-color: #e6f4ff;
  color: #1677ff;
}

.dark-mode .tree-node-icon.connection-icon {
  background-color: #111d2c;
  color: #3c89e8;
}

/* 标题 */
.tree-node-title {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 14px;
  line-height: 20px;
}

.tree-node-children {
  width: 100%;
}
</style>

