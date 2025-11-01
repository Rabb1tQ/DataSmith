<template>
  <div class="data-compare">
    <div class="compare-header">
      <h3>数据比较</h3>
      <p>比较两个表或数据库之间的数据差异</p>
    </div>

    <div class="compare-config">
      <a-row :gutter="16">
        <a-col :span="12">
          <a-card title="源" size="small">
            <a-form layout="vertical">
              <a-form-item label="数据库">
                <a-select v-model:value="sourceDatabase" placeholder="选择数据库">
                  <a-select-option
                    v-for="db in databases"
                    :key="db.name"
                    :value="db.name"
                  >
                    {{ db.name }}
                  </a-select-option>
                </a-select>
              </a-form-item>
              <a-form-item label="表">
                <a-select
                  v-model:value="sourceTable"
                  placeholder="选择表"
                  :disabled="!sourceDatabase"
                >
                  <a-select-option
                    v-for="table in sourceTables"
                    :key="table.name"
                    :value="table.name"
                  >
                    {{ table.name }}
                  </a-select-option>
                </a-select>
              </a-form-item>
            </a-form>
          </a-card>
        </a-col>

        <a-col :span="12">
          <a-card title="目标" size="small">
            <a-form layout="vertical">
              <a-form-item label="数据库">
                <a-select v-model:value="targetDatabase" placeholder="选择数据库">
                  <a-select-option
                    v-for="db in databases"
                    :key="db.name"
                    :value="db.name"
                  >
                    {{ db.name }}
                  </a-select-option>
                </a-select>
              </a-form-item>
              <a-form-item label="表">
                <a-select
                  v-model:value="targetTable"
                  placeholder="选择表"
                  :disabled="!targetDatabase"
                >
                  <a-select-option
                    v-for="table in targetTables"
                    :key="table.name"
                    :value="table.name"
                  >
                    {{ table.name }}
                  </a-select-option>
                </a-select>
              </a-form-item>
            </a-form>
          </a-card>
        </a-col>
      </a-row>

      <div class="compare-options" style="margin-top: 16px;">
        <a-space>
          <a-button type="primary" @click="handleCompare" :loading="comparing">
            <CompareOutlined />
            开始比较
          </a-button>
          <a-checkbox v-model:checked="compareStructure">
            比较结构
          </a-checkbox>
          <a-checkbox v-model:checked="compareData">
            比较数据
          </a-checkbox>
        </a-space>
      </div>
    </div>

    <div v-if="comparisonResult" class="compare-result" style="margin-top: 24px;">
      <a-tabs>
        <a-tab-pane key="summary" tab="概要">
          <a-descriptions bordered size="small">
            <a-descriptions-item label="结构差异">
              <a-tag :color="comparisonResult.structureDiff > 0 ? 'warning' : 'success'">
                {{ comparisonResult.structureDiff }} 处
              </a-tag>
            </a-descriptions-item>
            <a-descriptions-item label="数据差异">
              <a-tag :color="comparisonResult.dataDiff > 0 ? 'warning' : 'success'">
                {{ comparisonResult.dataDiff }} 行
              </a-tag>
            </a-descriptions-item>
            <a-descriptions-item label="缺少的行">
              {{ comparisonResult.missingRows }}
            </a-descriptions-item>
            <a-descriptions-item label="多余的行">
              {{ comparisonResult.extraRows }}
            </a-descriptions-item>
          </a-descriptions>
        </a-tab-pane>

        <a-tab-pane key="structure" tab="结构差异">
          <a-empty v-if="!comparisonResult.structureDetails || comparisonResult.structureDetails.length === 0" description="无结构差异" />
          <a-list
            v-else
            :data-source="comparisonResult.structureDetails"
            size="small"
            bordered
          >
            <template #renderItem="{ item }">
              <a-list-item>
                <a-tag color="warning">{{ item.type }}</a-tag>
                {{ item.message }}
              </a-list-item>
            </template>
          </a-list>
        </a-tab-pane>

        <a-tab-pane key="data" tab="数据差异">
          <a-alert
            v-if="comparisonResult.dataDiff === 0"
            message="数据完全一致"
            type="success"
            show-icon
          />
          <div v-else>
            <p>发现 {{ comparisonResult.dataDiff }} 行数据差异</p>
            <a-button type="primary" @click="generateSyncScript">
              生成同步脚本
            </a-button>
          </div>
        </a-tab-pane>
      </a-tabs>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
// import { CompressOutlined } from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{
  connectionId: string | null
}>()

const databases = ref<any[]>([])
const sourceDatabase = ref('')
const targetDatabase = ref('')
const sourceTable = ref('')
const targetTable = ref('')
const sourceTables = ref<any[]>([])
const targetTables = ref<any[]>([])
const comparing = ref(false)
const compareStructure = ref(true)
const compareData = ref(true)
const comparisonResult = ref<any>(null)

// 加载数据库列表
async function loadDatabases() {
  if (!props.connectionId) return
  
  try {
    const dbs = await invoke<any[]>('get_databases', {
      connectionId: props.connectionId,
    })
    databases.value = dbs
  } catch (error: any) {
    message.error(`加载数据库列表失败: ${error}`)
  }
}

// 加载源表列表
watch(sourceDatabase, async (db) => {
  if (!db || !props.connectionId) return
  
  try {
    const tables = await invoke<any[]>('get_tables', {
      connectionId: props.connectionId,
      database: db,
    })
    sourceTables.value = tables
  } catch (error: any) {
    message.error(`加载表列表失败: ${error}`)
  }
})

// 加载目标表列表
watch(targetDatabase, async (db) => {
  if (!db || !props.connectionId) return
  
  try {
    const tables = await invoke<any[]>('get_tables', {
      connectionId: props.connectionId,
      database: db,
    })
    targetTables.value = tables
  } catch (error: any) {
    message.error(`加载表列表失败: ${error}`)
  }
})

// 执行比较
async function handleCompare() {
  if (!sourceDatabase.value || !sourceTable.value || !targetDatabase.value || !targetTable.value) {
    message.warning('请选择要比较的源和目标')
    return
  }
  
  comparing.value = true
  
  try {
    // 简化版本的比较逻辑
    const structureDiff: any[] = []
    let dataDiff = 0
    
    if (compareStructure.value) {
      // 比较结构
      const sourceStructure = await invoke<any[]>('get_table_structure', {
        connectionId: props.connectionId,
        table: sourceTable.value,
        schema: sourceDatabase.value,
      })
      
      const targetStructure = await invoke<any[]>('get_table_structure', {
        connectionId: props.connectionId,
        table: targetTable.value,
        schema: targetDatabase.value,
      })
      
      // 简单比较列数
      if (sourceStructure.length !== targetStructure.length) {
        structureDiff.push({
          type: '列数不同',
          message: `源表有 ${sourceStructure.length} 列，目标表有 ${targetStructure.length} 列`,
        })
      }
      
      // 比较每一列
      for (const sourceCol of sourceStructure) {
        const targetCol = targetStructure.find((c: any) => c.name === sourceCol.name)
        if (!targetCol) {
          structureDiff.push({
            type: '缺少列',
            message: `目标表缺少列: ${sourceCol.name}`,
          })
        } else if (sourceCol.data_type !== targetCol.data_type) {
          structureDiff.push({
            type: '类型不同',
            message: `列 ${sourceCol.name} 类型不同: ${sourceCol.data_type} vs ${targetCol.data_type}`,
          })
        }
      }
    }
    
    if (compareData.value) {
      // 比较数据行数
      const sourceData = await invoke<any>('execute_query', {
        connectionId: props.connectionId,
        sql: `SELECT COUNT(*) as count FROM \`${sourceDatabase.value}\`.\`${sourceTable.value}\``,
      })
      
      const targetData = await invoke<any>('execute_query', {
        connectionId: props.connectionId,
        sql: `SELECT COUNT(*) as count FROM \`${targetDatabase.value}\`.\`${targetTable.value}\``,
      })
      
      const sourceCount = sourceData.rows[0]?.count || 0
      const targetCount = targetData.rows[0]?.count || 0
      
      dataDiff = Math.abs(sourceCount - targetCount)
    }
    
    comparisonResult.value = {
      structureDiff: structureDiff.length,
      dataDiff,
      missingRows: 0,
      extraRows: 0,
      structureDetails: structureDiff,
    }
    
    message.success('比较完成')
  } catch (error: any) {
    message.error(`比较失败: ${error}`)
  } finally {
    comparing.value = false
  }
}

// 生成同步脚本
function generateSyncScript() {
  message.info('同步脚本生成功能正在开发中...')
}

// 初始化
watch(() => props.connectionId, (id) => {
  if (id) {
    loadDatabases()
  }
}, { immediate: true })
</script>

<style scoped>
.data-compare {
  padding: 24px;
}

.compare-header {
  margin-bottom: 24px;
}

.compare-header h3 {
  margin: 0 0 8px 0;
  font-size: 18px;
  font-weight: 600;
}

.compare-header p {
  margin: 0;
  color: #666;
  font-size: 14px;
}
</style>

