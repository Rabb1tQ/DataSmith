/**
 * SQL 自动补全服务
 * 为 Monaco Editor 提供智能的 SQL 自动补全功能
 */

import * as monaco from 'monaco-editor'
import { invoke } from '@tauri-apps/api/core'

export interface AutoCompleteData {
  databases: string[]
  tables: TableSuggestion[]
  keywords: string[]
}

export interface TableSuggestion {
  name: string
  database: string
  columns: ColumnSuggestion[]
}

export interface ColumnSuggestion {
  name: string
  data_type: string
}

/**
 * SQL 自动补全提供程序
 */
export class SqlCompletionProvider implements monaco.languages.CompletionItemProvider {
  private autoCompleteData: AutoCompleteData | null = null
  private connectionId: string | null = null
  private currentDatabase: string | null = null

  /**
   * 设置连接 ID
   */
  setConnectionId(connectionId: string | null) {
    this.connectionId = connectionId
    // 当连接改变时，清空缓存的数据
    if (connectionId !== this.connectionId) {
      this.autoCompleteData = null
    }
  }

  /**
   * 设置当前数据库
   */
  setCurrentDatabase(database: string | null) {
    // 当数据库改变时，重新加载数据
    if (database !== this.currentDatabase) {
      this.currentDatabase = database
      this.loadAutoCompleteData()
    }
  }

  /**
   * 加载自动补全数据
   */
  async loadAutoCompleteData() {
    if (!this.connectionId) {
      this.autoCompleteData = null
      return
    }

    try {
      this.autoCompleteData = await invoke<AutoCompleteData>('get_autocomplete_data', {
        connectionId: this.connectionId,
        database: this.currentDatabase,
      })
    } catch (error) {
      console.error('加载自动补全数据失败:', error)
      this.autoCompleteData = null
    }
  }

  /**
   * 提供补全建议
   */
  async provideCompletionItems(
    model: monaco.editor.ITextModel,
    position: monaco.Position
  ): Promise<monaco.languages.CompletionList> {
    // 如果没有数据，先加载
    if (!this.autoCompleteData && this.connectionId) {
      await this.loadAutoCompleteData()
    }

    if (!this.autoCompleteData) {
      return { suggestions: [] }
    }

    const suggestions: monaco.languages.CompletionItem[] = []
    
    // 获取当前行的文本和光标前的单词
    const textUntilPosition = model.getValueInRange({
      startLineNumber: position.lineNumber,
      startColumn: 1,
      endLineNumber: position.lineNumber,
      endColumn: position.column,
    })
    
    const word = model.getWordUntilPosition(position)
    const range = {
      startLineNumber: position.lineNumber,
      endLineNumber: position.lineNumber,
      startColumn: word.startColumn,
      endColumn: word.endColumn,
    }

    // 分析上下文
    const upperText = textUntilPosition.toUpperCase()
    const tokens = textUntilPosition.trim().split(/\s+/)
    const lastToken = tokens[tokens.length - 1]?.toUpperCase() || ''
    const secondLastToken = tokens[tokens.length - 2]?.toUpperCase() || ''

    // 1. SQL 关键字补全
    for (const keyword of this.autoCompleteData.keywords) {
      suggestions.push({
        label: keyword,
        kind: monaco.languages.CompletionItemKind.Keyword,
        detail: 'SQL 关键字',
        insertText: keyword,
        range,
        sortText: `0_${keyword}`, // 关键字优先级较高
      })
    }

    // 2. 数据库名补全 (在 FROM, USE, DATABASE 等关键字后)
    if (
      lastToken === 'FROM' ||
      lastToken === 'USE' ||
      lastToken === 'DATABASE' ||
      secondLastToken === 'USE' ||
      upperText.includes('FROM') ||
      upperText.includes('USE')
    ) {
      for (const db of this.autoCompleteData.databases) {
        suggestions.push({
          label: db,
          kind: monaco.languages.CompletionItemKind.Module,
          detail: '数据库',
          insertText: db,
          range,
          sortText: `1_${db}`,
        })
      }
    }

    // 3. 表名补全
    // 在 FROM, JOIN, UPDATE, INTO, TABLE 等关键字后
    const shouldShowTables =
      lastToken === 'FROM' ||
      lastToken === 'JOIN' ||
      lastToken === 'UPDATE' ||
      lastToken === 'INTO' ||
      lastToken === 'TABLE' ||
      secondLastToken === 'FROM' ||
      secondLastToken === 'JOIN' ||
      secondLastToken === 'UPDATE' ||
      secondLastToken === 'INTO' ||
      secondLastToken === 'TABLE' ||
      upperText.includes('FROM') ||
      upperText.includes('JOIN')

    if (shouldShowTables) {
      for (const table of this.autoCompleteData.tables) {
        const label = this.currentDatabase && table.database !== this.currentDatabase
          ? `${table.database}.${table.name}`
          : table.name

        suggestions.push({
          label,
          kind: monaco.languages.CompletionItemKind.Class,
          detail: `表 (${table.database})`,
          documentation: `包含 ${table.columns.length} 列`,
          insertText: label,
          range,
          sortText: `2_${label}`,
        })
      }
    }

    // 4. 列名补全
    // 在 SELECT, WHERE, SET, ON, GROUP BY, ORDER BY 等关键字后
    const shouldShowColumns =
      lastToken === 'SELECT' ||
      lastToken === 'WHERE' ||
      lastToken === 'SET' ||
      lastToken === 'ON' ||
      lastToken === 'BY' ||
      lastToken === ',' ||
      upperText.includes('SELECT') ||
      upperText.includes('WHERE') ||
      upperText.includes('SET') ||
      upperText.includes('ORDER BY') ||
      upperText.includes('GROUP BY')

    if (shouldShowColumns) {
      // 尝试找出当前正在操作的表
      const tablesInQuery = this.extractTablesFromQuery(model.getValue())
      
      for (const table of this.autoCompleteData.tables) {
        // 如果能识别出查询中的表，只显示这些表的列
        if (tablesInQuery.length > 0 && !tablesInQuery.includes(table.name)) {
          continue
        }

        for (const column of table.columns) {
          // 如果有多个表，显示 表.列 的格式
          const label = tablesInQuery.length > 1 || this.autoCompleteData.tables.length > 5
            ? `${table.name}.${column.name}`
            : column.name

          suggestions.push({
            label,
            kind: monaco.languages.CompletionItemKind.Field,
            detail: `${column.data_type} (${table.name})`,
            documentation: `表: ${table.database}.${table.name}`,
            insertText: label,
            range,
            sortText: `3_${label}`,
          })
        }
      }
    }

    // 5. 函数补全
    const functions = [
      { name: 'COUNT', detail: '计数函数' },
      { name: 'SUM', detail: '求和函数' },
      { name: 'AVG', detail: '平均值函数' },
      { name: 'MAX', detail: '最大值函数' },
      { name: 'MIN', detail: '最小值函数' },
      { name: 'CONCAT', detail: '字符串连接' },
      { name: 'SUBSTRING', detail: '字符串截取' },
      { name: 'UPPER', detail: '转大写' },
      { name: 'LOWER', detail: '转小写' },
      { name: 'TRIM', detail: '去除空格' },
      { name: 'NOW', detail: '当前时间' },
      { name: 'DATE', detail: '日期函数' },
      { name: 'YEAR', detail: '获取年份' },
      { name: 'MONTH', detail: '获取月份' },
      { name: 'DAY', detail: '获取日' },
    ]

    for (const func of functions) {
      suggestions.push({
        label: func.name,
        kind: monaco.languages.CompletionItemKind.Function,
        detail: func.detail,
        insertText: `${func.name}($0)`,
        insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
        range,
        sortText: `4_${func.name}`,
      })
    }

    return { suggestions }
  }

  /**
   * 从 SQL 查询中提取表名
   */
  private extractTablesFromQuery(sql: string): string[] {
    const tables: string[] = []
    const upperSql = sql.toUpperCase()

    // 简单的表名提取逻辑
    // 匹配 FROM tablename 和 JOIN tablename
    const fromMatch = upperSql.match(/FROM\s+([a-zA-Z0-9_]+)/g)
    const joinMatch = upperSql.match(/JOIN\s+([a-zA-Z0-9_]+)/g)

    if (fromMatch) {
      fromMatch.forEach((match) => {
        const tableName = match.replace(/FROM\s+/i, '').trim()
        if (tableName) {
          tables.push(tableName.toLowerCase())
        }
      })
    }

    if (joinMatch) {
      joinMatch.forEach((match) => {
        const tableName = match.replace(/JOIN\s+/i, '').trim()
        if (tableName) {
          tables.push(tableName.toLowerCase())
        }
      })
    }

    return tables
  }

  /**
   * 手动刷新自动补全数据
   */
  async refresh() {
    await this.loadAutoCompleteData()
  }
}

/**
 * 创建并注册 SQL 自动补全提供程序
 */
export function registerSqlCompletionProvider(): SqlCompletionProvider {
  const provider = new SqlCompletionProvider()

  // 注册到 Monaco Editor
  monaco.languages.registerCompletionItemProvider('sql', provider)

  return provider
}

