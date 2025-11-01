import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { ConnectionConfig, ConnectionStatus } from '@/types/database'
import { invoke } from '@tauri-apps/api/core'

export const useConnectionStore = defineStore('connection', () => {
  // 状态
  const connections = ref<ConnectionConfig[]>([])
  const activeConnectionId = ref<string | null>(null)
  const connectionStatuses = ref<Map<string, ConnectionStatus>>(new Map())

  // 获取所有连接
  async function fetchConnections() {
    try {
      connections.value = await invoke<ConnectionConfig[]>('get_connections')
    } catch (error) {
      console.error('获取连接列表失败:', error)
      throw error
    }
  }

  // 保存连接
  async function saveConnection(config: ConnectionConfig, password?: string) {
    try {
      // 创建不包含密码的存储对象
      const storedConnection = {
        id: config.id,
        name: config.name,
        db_type: config.db_type,
        host: config.host,
        port: config.port,
        username: config.username,
        database: config.database,
        ssl: config.ssl,
        connection_timeout: config.connection_timeout,
        pool_size: config.pool_size,
        tags: config.tags || [],
        created_at: config.created_at || Date.now(),
        updated_at: Date.now(),
      }
      
      const saved = await invoke<any>('save_connection', { 
        connection: storedConnection,
        password: password || null
      })
      
      const index = connections.value.findIndex(c => c.id === saved.id)
      if (index >= 0) {
        connections.value[index] = { ...config, ...saved }
      } else {
        connections.value.push({ ...config, ...saved })
      }
      return saved
    } catch (error) {
      console.error('保存连接失败:', error)
      throw error
    }
  }

  // 更新连接
  async function updateConnection(config: ConnectionConfig, password?: string) {
    try {
      // 创建不包含密码的存储对象
      const storedConnection = {
        id: config.id,
        name: config.name,
        db_type: config.db_type,
        host: config.host,
        port: config.port,
        username: config.username,
        database: config.database,
        ssl: config.ssl,
        connection_timeout: config.connection_timeout,
        pool_size: config.pool_size,
        tags: config.tags || [],
        created_at: config.created_at,
        updated_at: Date.now(),
      }
      
      const updated = await invoke<any>('update_connection', { 
        connection: storedConnection,
        password: password || null
      })
      
      const index = connections.value.findIndex(c => c.id === config.id)
      if (index >= 0) {
        connections.value[index] = { ...config, ...updated }
      }
      return updated
    } catch (error) {
      console.error('更新连接失败:', error)
      throw error
    }
  }

  // 删除连接
  async function deleteConnection(id: string) {
    try {
      await invoke('delete_connection', { id })
      connections.value = connections.value.filter(c => c.id !== id)
      if (activeConnectionId.value === id) {
        activeConnectionId.value = null
      }
      connectionStatuses.value.delete(id)
    } catch (error) {
      console.error('删除连接失败:', error)
      throw error
    }
  }

  // 测试连接
  async function testConnection(config: ConnectionConfig) {
    try {
      const result = await invoke<any>('test_connection', { config })
      // 检查连接测试结果
      if (!result.success) {
        throw new Error(result.message || '连接失败')
      }
      return result
    } catch (error) {
      console.error('测试连接失败:', error)
      throw error
    }
  }

  // 连接到数据库
  async function connectToDatabase(id: string) {
    try {
      const conn = connections.value.find(c => c.id === id)
      if (!conn) {
        throw new Error('连接配置不存在')
      }
      // 调用后端创建连接
      await invoke('create_connection', { connectionId: id, config: conn })
    } catch (error) {
      console.error('连接数据库失败:', error)
      throw error
    }
  }

  // 断开数据库连接
  async function disconnectFromDatabase(id: string) {
    try {
      // 调用后端断开连接
      await invoke('disconnect_database', { connectionId: id })
    } catch (error) {
      console.error('断开连接失败:', error)
      throw error
    }
  }

  // 设置活动连接
  function setActiveConnection(id: string | null) {
    activeConnectionId.value = id
  }

  // 更新连接状态
  function updateConnectionStatus(id: string, status: ConnectionStatus) {
    connectionStatuses.value.set(id, status)
  }

  // 获取连接状态
  function getConnectionStatus(id: string): ConnectionStatus {
    return connectionStatuses.value.get(id) || 'disconnected'
  }

  // 获取活动连接
  function getActiveConnection(): ConnectionConfig | null {
    if (!activeConnectionId.value) return null
    return connections.value.find(c => c.id === activeConnectionId.value) || null
  }

  return {
    connections,
    activeConnectionId,
    connectionStatuses,
    fetchConnections,
    saveConnection,
    updateConnection,
    deleteConnection,
    testConnection,
    connectToDatabase,
    disconnectFromDatabase,
    setActiveConnection,
    updateConnectionStatus,
    getConnectionStatus,
    getActiveConnection,
  }
})

