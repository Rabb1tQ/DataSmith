<template>
  <a-modal
    v-model:open="visible"
    title="保存查询"
    width="500px"
    @ok="handleSave"
    @cancel="handleCancel"
    :confirm-loading="saving"
  >
    <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
      <a-form-item label="查询名称" required>
        <a-input v-model:value="queryName" placeholder="为查询起一个名称" />
      </a-form-item>

      <a-form-item label="分类">
        <a-select
          v-model:value="category"
          placeholder="选择分类"
          :options="categoryOptions"
          :show-search="true"
          :allow-clear="true"
        >
          <template #dropdownRender="{ menuNode }">
            <div>
              <div>{{ menuNode }}</div>
              <a-divider style="margin: 4px 0" />
              <div style="padding: 4px 8px; cursor: pointer" @click="showAddCategory = true">
                <PlusOutlined /> 添加新分类
              </div>
            </div>
          </template>
        </a-select>
      </a-form-item>

      <a-form-item label="描述">
        <a-textarea v-model:value="description" placeholder="查询说明（可选）" :rows="3" />
      </a-form-item>

      <a-form-item label="SQL预览">
        <div class="sql-preview">
          {{ sqlPreview }}
        </div>
      </a-form-item>
    </a-form>

    <!-- 添加分类对话框 -->
    <a-modal
      v-model:open="showAddCategory"
      title="添加分类"
      width="400px"
      @ok="handleAddCategory"
      @cancel="showAddCategory = false"
    >
      <a-input v-model:value="newCategory" placeholder="输入分类名称" />
    </a-modal>
  </a-modal>
</template>

<script setup lang="ts">
import { PlusOutlined } from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'

interface SavedQuery {
  id: string
  name: string
  sql: string
  category: string
  description: string
  createdAt: number
  updatedAt: number
}

const props = defineProps<{
  modelValue: boolean
  sql: string
}>()

const emit = defineEmits(['update:modelValue', 'saved'])

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
})

const saving = ref(false)
const queryName = ref('')
const category = ref('')
const description = ref('')
const showAddCategory = ref(false)
const newCategory = ref('')

const categories = ref<string[]>([])

const categoryOptions = computed(() => {
  return categories.value.map(cat => ({ label: cat, value: cat }))
})

const sqlPreview = computed(() => {
  if (props.sql.length > 200) {
    return props.sql.substring(0, 200) + '...'
  }
  return props.sql
})

// 加载分类列表
function loadCategories() {
  const stored = localStorage.getItem('query_categories')
  if (stored) {
    try {
      categories.value = JSON.parse(stored)
    } catch (e) {
      categories.value = ['常用查询', '数据分析', '报表查询']
    }
  } else {
    categories.value = ['常用查询', '数据分析', '报表查询']
  }
}

// 添加分类
function handleAddCategory() {
  if (!newCategory.value.trim()) {
    message.error('请输入分类名称')
    return
  }

  if (categories.value.includes(newCategory.value)) {
    message.error('分类已存在')
    return
  }

  categories.value.push(newCategory.value)
  localStorage.setItem('query_categories', JSON.stringify(categories.value))
  category.value = newCategory.value
  newCategory.value = ''
  showAddCategory.value = false
  message.success('分类添加成功')
}

// 保存查询
function handleSave() {
  if (!queryName.value.trim()) {
    message.error('请输入查询名称')
    return
  }

  saving.value = true
  try {
    // 获取已保存的查询
    const stored = localStorage.getItem('saved_queries')
    let queries: SavedQuery[] = []
    
    if (stored) {
      try {
        queries = JSON.parse(stored)
      } catch (e) {
        queries = []
      }
    }

    // 创建新查询
    const query: SavedQuery = {
      id: Date.now().toString(),
      name: queryName.value,
      sql: props.sql,
      category: category.value,
      description: description.value,
      createdAt: Date.now(),
      updatedAt: Date.now(),
    }

    queries.unshift(query)

    // 限制数量
    if (queries.length > 200) {
      queries = queries.slice(0, 200)
    }

    localStorage.setItem('saved_queries', JSON.stringify(queries))
    
    message.success('查询保存成功')
    emit('saved', query)
    handleCancel()
  } catch (error: any) {
    message.error(`保存失败: ${error}`)
  } finally {
    saving.value = false
  }
}

function handleCancel() {
  queryName.value = ''
  category.value = ''
  description.value = ''
  visible.value = false
}

watch(visible, (newVal) => {
  if (newVal) {
    loadCategories()
  }
})
</script>

<style scoped>
.sql-preview {
  padding: 8px;
  background: #f5f5f5;
  border: 1px solid #d9d9d9;
  border-radius: 4px;
  font-family: monospace;
  font-size: 12px;
  max-height: 150px;
  overflow-y: auto;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>

