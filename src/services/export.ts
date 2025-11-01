import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import type { QueryResult } from '@/types/database'

export class ExportService {
  /**
   * 导出为 CSV
   */
  static async exportToCsv(data: QueryResult, defaultName = 'export.csv') {
    const filePath = await save({
      defaultPath: defaultName,
      filters: [
        {
          name: 'CSV Files',
          extensions: ['csv'],
        },
      ],
    })

    if (!filePath) return false

    try {
      await invoke('export_to_csv', {
        data,
        filePath,
      })
      return true
    } catch (error) {
      console.error('导出 CSV 失败:', error)
      throw error
    }
  }

  /**
   * 导出为 JSON
   */
  static async exportToJson(data: QueryResult, defaultName = 'export.json') {
    const filePath = await save({
      defaultPath: defaultName,
      filters: [
        {
          name: 'JSON Files',
          extensions: ['json'],
        },
      ],
    })

    if (!filePath) return false

    try {
      await invoke('export_to_json', {
        data,
        filePath,
      })
      return true
    } catch (error) {
      console.error('导出 JSON 失败:', error)
      throw error
    }
  }

  /**
   * 导出为 SQL
   */
  static async exportToSql(
    data: QueryResult,
    tableName: string,
    defaultName = 'export.sql'
  ) {
    const filePath = await save({
      defaultPath: defaultName,
      filters: [
        {
          name: 'SQL Files',
          extensions: ['sql'],
        },
      ],
    })

    if (!filePath) return false

    try {
      await invoke('export_to_sql', {
        data,
        tableName,
        filePath,
      })
      return true
    } catch (error) {
      console.error('导出 SQL 失败:', error)
      throw error
    }
  }

  /**
   * 复制为文本
   */
  static copyAsText(data: QueryResult): string {
    const header = data.columns.join('\t')
    const rows = data.rows.map((row) =>
      data.columns.map((col) => row[col] ?? '').join('\t')
    )
    return [header, ...rows].join('\n')
  }

  /**
   * 复制为 Markdown 表格
   */
  static copyAsMarkdown(data: QueryResult): string {
    const header = '| ' + data.columns.join(' | ') + ' |'
    const separator = '| ' + data.columns.map(() => '---').join(' | ') + ' |'
    const rows = data.rows.map(
      (row) => '| ' + data.columns.map((col) => row[col] ?? '').join(' | ') + ' |'
    )
    return [header, separator, ...rows].join('\n')
  }

  /**
   * 复制为 HTML 表格
   */
  static copyAsHtml(data: QueryResult): string {
    const header = `<thead><tr>${data.columns.map((col) => `<th>${col}</th>`).join('')}</tr></thead>`
    const body = `<tbody>${data.rows
      .map(
        (row) =>
          `<tr>${data.columns.map((col) => `<td>${row[col] ?? ''}</td>`).join('')}</tr>`
      )
      .join('')}</tbody>`
    return `<table>${header}${body}</table>`
  }
}

