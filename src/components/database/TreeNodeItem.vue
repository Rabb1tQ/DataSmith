<template>
  <div class="tree-node">
    <div
      :class="['tree-node-content', { selected: isSelected }]"
      :style="{ paddingLeft: level * 20 + 'px' }"
      @click="handleClick"
      @dblclick="handleDblClick"
      @contextmenu="handleContextMenu"
    >
      <span class="tree-node-expand" @click="handleToggle">
        <DownOutlined v-if="hasChildren && isExpanded" />
        <RightOutlined v-else-if="hasChildren" />
        <span v-else style="display: inline-block; width: 14px;"></span>
      </span>
      <span class="tree-node-icon">
        <LoadingOutlined v-if="isLoading" spin />
        <component v-else :is="getIcon(node.type)" />
      </span>
      <span class="tree-node-title">{{ node.title }}</span>
      <a-spin v-if="isLoading" size="small" style="margin-left: 8px;" />
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
  DatabaseOutlined,
  TableOutlined,
  FolderOutlined,
  FileOutlined,
  EyeOutlined,
  LoadingOutlined,
  RightOutlined,
  DownOutlined,
} from '@ant-design/icons-vue'

interface TreeNode {
  key: string
  title: string
  type: string
  isLeaf?: boolean
  children?: TreeNode[]
  metadata?: any
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

const getIcon = (type: string) => {
  const iconMap: Record<string, any> = {
    connection: DatabaseOutlined,
    database: DatabaseOutlined,
    tables: FolderOutlined,
    table: TableOutlined,
    views: EyeOutlined,
    view: EyeOutlined,
    procedures: FolderOutlined,
    procedure: FileOutlined,
    functions: FolderOutlined,
    function: FileOutlined,
    triggers: FolderOutlined,
    trigger: FileOutlined,
    events: FolderOutlined,
    event: FileOutlined,
  }
  return iconMap[type] || FileOutlined
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

.tree-node-expand {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  margin-right: 4px;
  font-size: 12px;
  color: #8c8c8c;
  flex-shrink: 0;
}

.tree-node-expand:hover {
  color: #1890ff;
}

.tree-node-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  margin-right: 8px;
  font-size: 16px;
  color: #1890ff;
  flex-shrink: 0;
}

.tree-node-title {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tree-node-children {
  width: 100%;
}
</style>

