<template>
  <a-modal
    v-model:open="visible"
    :title="`备份数据库 - ${database}`"
    width="600px"
    @ok="handleBackup"
    @cancel="handleCancel"
    :confirm-loading="backing"
  >
    <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
      <a-form-item label="备份内容">
        <a-checkbox-group v-model:value="backupOptions">
          <a-checkbox value="structure">表结构</a-checkbox>
          <a-checkbox value="data">表数据</a-checkbox>
          <a-checkbox value="views">视图</a-checkbox>
          <a-checkbox value="procedures">存储过程</a-checkbox>
          <a-checkbox value="functions">函数</a-checkbox>
          <a-checkbox value="triggers">触发器</a-checkbox>
        </a-checkbox-group>
      </a-form-item>

      <a-form-item label="保存位置" required>
        <a-input
          v-model:value="savePath"
          placeholder="点击选择保存位置"
          readonly
          @click="selectSavePath"
        >
          <template #suffix>
            <FolderOpenOutlined style="cursor: pointer" @click="selectSavePath" />
          </template>
        </a-input>
      </a-form-item>

      <a-form-item label="压缩">
        <a-switch v-model:checked="compress" />
        <span style="margin-left: 8px; color: #999; font-size: 12px;">
          压缩后文件更小，但需要更长时间
        </span>
      </a-form-item>
    </a-form>
  </a-modal>
</template>

<script setup lang="ts">
import { computed, ref, watch} from 'vue'
import { FolderOpenOutlined } from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { downloadDir } from '@tauri-apps/api/path'

const props = defineProps<{
  modelValue: boolean
  connectionId: string
  database: string
}>()

const emit = defineEmits(['update:modelValue', 'backed'])

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
})

const backing = ref(false)
const backupOptions = ref(['structure', 'data'])
const savePath = ref('')
const compress = ref(false)

// 生成默认文件名
function getDefaultFileName(): string {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, -5)
  const extension = compress.value ? '.sql.gz' : '.sql'
  return `${props.database}_backup_${timestamp}${extension}`
}

// 当对话框打开时，设置默认保存路径（Downloads目录）
watch(() => props.modelValue, async (newVal) => {
  if (newVal && !savePath.value) {
    try {
      const downloadsPath = await downloadDir()
      const fileName = getDefaultFileName()
      savePath.value = `${downloadsPath}\\${fileName}`
    } catch (error) {
      console.error('获取下载目录失败:', error)
      savePath.value = getDefaultFileName()
    }
  }
})

// 当压缩选项改变时，更新文件扩展名
watch(compress, async () => {
  try {
    const downloadsPath = await downloadDir()
    const fileName = getDefaultFileName()
    savePath.value = `${downloadsPath}\\${fileName}`
  } catch (error) {
    savePath.value = getDefaultFileName()
  }
})

async function selectSavePath() {
  const defaultPath = getDefaultFileName()

  const path = await save({
    defaultPath,
    filters: [{
      name: 'SQL文件',
      extensions: compress.value ? ['sql.gz'] : ['sql'],
    }],
  })

  if (path) {
    savePath.value = path
  }
}

async function handleBackup() {
  if (!savePath.value) {
    message.error('请选择保存位置')
    return
  }

  if (backupOptions.value.length === 0) {
    message.error('请至少选择一项备份内容')
    return
  }

  backing.value = true
  try {
    let backupSql = `-- 数据库备份: ${props.database}\n`
    backupSql += `-- 备份时间: ${new Date().toLocaleString()}\n\n`

    // 备份表结构和数据
    if (backupOptions.value.includes('structure') || backupOptions.value.includes('data')) {
      const tables = await invoke<any[]>('get_tables', {
        connectionId: props.connectionId,
        database: props.database,
      })

      for (const table of tables) {
        // 导出表结构
        if (backupOptions.value.includes('structure')) {
          const ddl = await invoke<string>('export_table_ddl', {
            connectionId: props.connectionId,
            database: props.database,
            table: table.name,
          })
          backupSql += `\n-- 表结构: ${table.name}\n`
          backupSql += `DROP TABLE IF EXISTS \`${table.name}\`;\n`
          backupSql += `${ddl};\n\n`
        }

        // 导出表数据
        if (backupOptions.value.includes('data')) {
          const result = await invoke<any>('execute_query', {
            connectionId: props.connectionId,
            sql: `SELECT * FROM \`${table.name}\``,
            database: props.database,
          })

          if (result.rows && result.rows.length > 0) {
            backupSql += `-- 表数据: ${table.name}\n`
            
            for (const row of result.rows) {
              const columns = Object.keys(row)
              const values = columns.map(col => {
                const val = row[col]
                if (val === null) return 'NULL'
                if (typeof val === 'string') return `'${val.replace(/'/g, "''")}'`
                return val
              })

              backupSql += `INSERT INTO \`${table.name}\` (\`${columns.join('`, `')}\`) VALUES (${values.join(', ')});\n`
            }
            backupSql += '\n'
          }
        }
      }
    }

    // 备份视图
    if (backupOptions.value.includes('views')) {
      const views = await invoke<any[]>('get_views', {
        connectionId: props.connectionId,
        database: props.database,
      })

      for (const view of views) {
        const definition = await invoke<string>('get_view_definition', {
          connectionId: props.connectionId,
          database: props.database,
          view: view.name,
        })
        backupSql += `\n-- 视图: ${view.name}\n`
        backupSql += `DROP VIEW IF EXISTS \`${view.name}\`;\n`
        backupSql += `${definition};\n\n`
      }
    }

    // 保存到文件
    await invoke('write_file', {
      path: savePath.value,
      content: backupSql,
    })

    // 显示备份成功提示
    Modal.success({
      title: '备份成功',
      content: `数据库 "${props.database}" 已成功备份到：\n${savePath.value}`,
      okText: '确定',
    })

    emit('backed')
    handleCancel()
  } catch (error: any) {
    message.error(`备份失败: ${error}`)
  } finally {
    backing.value = false
  }
}

function handleCancel() {
  backupOptions.value = ['structure', 'data']
  savePath.value = ''
  compress.value = false
  visible.value = false
}
</script>

