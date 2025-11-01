/**
 * 数据库类型定义
 */

export type DatabaseType = 'mysql' | 'postgresql' | 'sqlite' | 'mongodb' | 'redis' | 'elasticsearch'

/**
 * 连接配置
 */
export interface ConnectionConfig {
  id: string
  name: string
  db_type: DatabaseType
  host: string
  port: number
  username: string
  password?: string
  database?: string
  ssl: boolean
  connection_timeout: number
  pool_size: number
  group?: string
  color?: string
  tags: string[]
  created_at: number
  updated_at: number
}

/**
 * 连接测试结果
 */
export interface ConnectionTestResult {
  success: boolean
  message: string
  version?: string
  ping_time_ms: number
}

/**
 * 连接状态
 */
export type ConnectionStatus = 'connected' | 'disconnected' | 'connecting' | 'error'

/**
 * 数据库信息
 */
export interface DatabaseInfo {
  name: string
  charset?: string
  collation?: string
}

/**
 * 表信息
 */
export interface TableInfo {
  name: string
  schema?: string
  table_type: string
  engine?: string
  rows?: number
  size_mb?: number
  comment?: string
}

/**
 * 列信息
 */
export interface ColumnInfo {
  name: string
  data_type: string
  nullable: boolean
  default_value?: string
  is_primary_key: boolean
  is_auto_increment: boolean
  comment?: string
  character_maximum_length?: number
  numeric_precision?: number
  numeric_scale?: number
}

/**
 * 索引信息
 */
export interface IndexInfo {
  name: string
  columns: string[]
  is_unique: boolean
  is_primary: boolean
  index_type: string
}

/**
 * 查询结果
 */
export interface QueryResult {
  columns: string[]
  rows: Record<string, any>[]
  affected_rows: number
  execution_time_ms: number
}

/**
 * 数据库对象类型
 */
export type DatabaseObjectType = 
  | 'database'
  | 'table'
  | 'view'
  | 'procedure'
  | 'function'
  | 'trigger'
  | 'sequence'
  | 'collection'
  | 'index'

/**
 * 数据库对象树节点
 */
export interface DatabaseTreeNode {
  key: string
  title: string
  type: DatabaseObjectType
  icon?: string
  children?: DatabaseTreeNode[]
  isLeaf?: boolean
  metadata?: any
}

