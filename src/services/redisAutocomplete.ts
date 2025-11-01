import * as monaco from 'monaco-editor'

/**
 * Redis 命令补全提供器
 */
export class RedisCompletionProvider {
  private commands: RedisCommand[] = []

  constructor() {
    this.initializeCommands()
  }

  private initializeCommands() {
    // Redis 常用命令列表
    this.commands = [
      // 字符串命令
      { command: 'GET', args: ['key'], description: '获取指定键的值', category: 'String' },
      { command: 'SET', args: ['key', 'value'], description: '设置指定键的值', category: 'String' },
      { command: 'SETNX', args: ['key', 'value'], description: '只在键不存在时设置值', category: 'String' },
      { command: 'SETEX', args: ['key', 'seconds', 'value'], description: '设置值并指定过期时间（秒）', category: 'String' },
      { command: 'PSETEX', args: ['key', 'milliseconds', 'value'], description: '设置值并指定过期时间（毫秒）', category: 'String' },
      { command: 'MGET', args: ['key1', 'key2', '...'], description: '获取多个键的值', category: 'String' },
      { command: 'MSET', args: ['key1', 'value1', 'key2', 'value2', '...'], description: '同时设置多个键值对', category: 'String' },
      { command: 'STRLEN', args: ['key'], description: '获取字符串长度', category: 'String' },
      { command: 'APPEND', args: ['key', 'value'], description: '追加字符串到指定键', category: 'String' },
      { command: 'INCR', args: ['key'], description: '将键的整数值加一', category: 'String' },
      { command: 'INCRBY', args: ['key', 'increment'], description: '将键的整数值增加指定数量', category: 'String' },
      { command: 'DECR', args: ['key'], description: '将键的整数值减一', category: 'String' },
      { command: 'DECRBY', args: ['key', 'decrement'], description: '将键的整数值减少指定数量', category: 'String' },
      { command: 'GETRANGE', args: ['key', 'start', 'end'], description: '获取字符串的子串', category: 'String' },
      { command: 'SETRANGE', args: ['key', 'offset', 'value'], description: '覆盖字符串的部分内容', category: 'String' },

      // 哈希命令
      { command: 'HGET', args: ['key', 'field'], description: '获取哈希表中字段的值', category: 'Hash' },
      { command: 'HSET', args: ['key', 'field', 'value'], description: '设置哈希表字段的值', category: 'Hash' },
      { command: 'HSETNX', args: ['key', 'field', 'value'], description: '只在字段不存在时设置值', category: 'Hash' },
      { command: 'HMGET', args: ['key', 'field1', 'field2', '...'], description: '获取多个字段的值', category: 'Hash' },
      { command: 'HMSET', args: ['key', 'field1', 'value1', 'field2', 'value2', '...'], description: '同时设置多个字段', category: 'Hash' },
      { command: 'HGETALL', args: ['key'], description: '获取哈希表所有字段和值', category: 'Hash' },
      { command: 'HKEYS', args: ['key'], description: '获取哈希表所有字段', category: 'Hash' },
      { command: 'HVALS', args: ['key'], description: '获取哈希表所有值', category: 'Hash' },
      { command: 'HLEN', args: ['key'], description: '获取哈希表字段数量', category: 'Hash' },
      { command: 'HDEL', args: ['key', 'field1', 'field2', '...'], description: '删除哈希表字段', category: 'Hash' },
      { command: 'HEXISTS', args: ['key', 'field'], description: '检查字段是否存在', category: 'Hash' },
      { command: 'HINCRBY', args: ['key', 'field', 'increment'], description: '为字段值加上增量', category: 'Hash' },

      // 列表命令
      { command: 'LPUSH', args: ['key', 'value1', 'value2', '...'], description: '从列表左侧插入元素', category: 'List' },
      { command: 'RPUSH', args: ['key', 'value1', 'value2', '...'], description: '从列表右侧插入元素', category: 'List' },
      { command: 'LPOP', args: ['key'], description: '移除并返回列表左侧第一个元素', category: 'List' },
      { command: 'RPOP', args: ['key'], description: '移除并返回列表右侧第一个元素', category: 'List' },
      { command: 'LLEN', args: ['key'], description: '获取列表长度', category: 'List' },
      { command: 'LRANGE', args: ['key', 'start', 'stop'], description: '获取列表指定范围的元素', category: 'List' },
      { command: 'LINDEX', args: ['key', 'index'], description: '通过索引获取列表元素', category: 'List' },
      { command: 'LSET', args: ['key', 'index', 'value'], description: '通过索引设置列表元素的值', category: 'List' },
      { command: 'LREM', args: ['key', 'count', 'value'], description: '移除列表中与参数值相等的元素', category: 'List' },
      { command: 'LTRIM', args: ['key', 'start', 'stop'], description: '保留列表指定范围的元素', category: 'List' },

      // 集合命令
      { command: 'SADD', args: ['key', 'member1', 'member2', '...'], description: '向集合添加成员', category: 'Set' },
      { command: 'SREM', args: ['key', 'member1', 'member2', '...'], description: '移除集合成员', category: 'Set' },
      { command: 'SMEMBERS', args: ['key'], description: '返回集合所有成员', category: 'Set' },
      { command: 'SISMEMBER', args: ['key', 'member'], description: '判断成员是否在集合中', category: 'Set' },
      { command: 'SCARD', args: ['key'], description: '获取集合成员数量', category: 'Set' },
      { command: 'SPOP', args: ['key', '[count]'], description: '随机移除并返回集合成员', category: 'Set' },
      { command: 'SRANDMEMBER', args: ['key', '[count]'], description: '随机返回集合成员', category: 'Set' },
      { command: 'SUNION', args: ['key1', 'key2', '...'], description: '返回多个集合的并集', category: 'Set' },
      { command: 'SINTER', args: ['key1', 'key2', '...'], description: '返回多个集合的交集', category: 'Set' },
      { command: 'SDIFF', args: ['key1', 'key2', '...'], description: '返回多个集合的差集', category: 'Set' },

      // 有序集合命令
      { command: 'ZADD', args: ['key', 'score1', 'member1', 'score2', 'member2', '...'], description: '向有序集合添加成员', category: 'Sorted Set' },
      { command: 'ZREM', args: ['key', 'member1', 'member2', '...'], description: '移除有序集合成员', category: 'Sorted Set' },
      { command: 'ZSCORE', args: ['key', 'member'], description: '获取成员的分数', category: 'Sorted Set' },
      { command: 'ZRANGE', args: ['key', 'start', 'stop', '[WITHSCORES]'], description: '返回指定范围的成员', category: 'Sorted Set' },
      { command: 'ZREVRANGE', args: ['key', 'start', 'stop', '[WITHSCORES]'], description: '返回指定范围的成员（倒序）', category: 'Sorted Set' },
      { command: 'ZRANK', args: ['key', 'member'], description: '返回成员的排名', category: 'Sorted Set' },
      { command: 'ZREVRANK', args: ['key', 'member'], description: '返回成员的排名（倒序）', category: 'Sorted Set' },
      { command: 'ZCARD', args: ['key'], description: '获取有序集合成员数量', category: 'Sorted Set' },
      { command: 'ZCOUNT', args: ['key', 'min', 'max'], description: '计算指定分数范围的成员数量', category: 'Sorted Set' },
      { command: 'ZINCRBY', args: ['key', 'increment', 'member'], description: '为成员的分数加上增量', category: 'Sorted Set' },

      // 键命令
      { command: 'DEL', args: ['key1', 'key2', '...'], description: '删除指定键', category: 'Key' },
      { command: 'EXISTS', args: ['key'], description: '检查键是否存在', category: 'Key' },
      { command: 'EXPIRE', args: ['key', 'seconds'], description: '设置键的过期时间（秒）', category: 'Key' },
      { command: 'EXPIREAT', args: ['key', 'timestamp'], description: '设置键的过期时间戳', category: 'Key' },
      { command: 'PEXPIRE', args: ['key', 'milliseconds'], description: '设置键的过期时间（毫秒）', category: 'Key' },
      { command: 'TTL', args: ['key'], description: '获取键的剩余生存时间（秒）', category: 'Key' },
      { command: 'PTTL', args: ['key'], description: '获取键的剩余生存时间（毫秒）', category: 'Key' },
      { command: 'PERSIST', args: ['key'], description: '移除键的过期时间', category: 'Key' },
      { command: 'KEYS', args: ['pattern'], description: '查找所有符合模式的键', category: 'Key' },
      { command: 'SCAN', args: ['cursor', '[MATCH pattern]', '[COUNT count]'], description: '迭代数据库中的键', category: 'Key' },
      { command: 'RENAME', args: ['key', 'newkey'], description: '重命名键', category: 'Key' },
      { command: 'RENAMENX', args: ['key', 'newkey'], description: '只在新键不存在时重命名', category: 'Key' },
      { command: 'TYPE', args: ['key'], description: '返回键的数据类型', category: 'Key' },
      { command: 'DUMP', args: ['key'], description: '序列化给定键', category: 'Key' },
      { command: 'RESTORE', args: ['key', 'ttl', 'serialized-value'], description: '反序列化并创建键', category: 'Key' },

      // 服务器命令
      { command: 'PING', args: ['[message]'], description: '测试服务器是否运行', category: 'Server' },
      { command: 'ECHO', args: ['message'], description: '打印字符串', category: 'Server' },
      { command: 'SELECT', args: ['index'], description: '切换到指定数据库', category: 'Server' },
      { command: 'QUIT', args: [], description: '关闭连接', category: 'Server' },
      { command: 'DBSIZE', args: [], description: '返回当前数据库键的数量', category: 'Server' },
      { command: 'INFO', args: ['[section]'], description: '获取服务器信息', category: 'Server' },
      { command: 'CONFIG GET', args: ['parameter'], description: '获取配置参数', category: 'Server' },
      { command: 'CONFIG SET', args: ['parameter', 'value'], description: '设置配置参数', category: 'Server' },
      { command: 'FLUSHDB', args: [], description: '清空当前数据库', category: 'Server' },
      { command: 'FLUSHALL', args: [], description: '清空所有数据库', category: 'Server' },
      { command: 'SAVE', args: [], description: '同步保存数据到磁盘', category: 'Server' },
      { command: 'BGSAVE', args: [], description: '异步保存数据到磁盘', category: 'Server' },
      { command: 'LASTSAVE', args: [], description: '返回最近一次成功保存的时间戳', category: 'Server' },
      { command: 'SHUTDOWN', args: ['[NOSAVE|SAVE]'], description: '关闭服务器', category: 'Server' },
      { command: 'TIME', args: [], description: '返回当前服务器时间', category: 'Server' },
      { command: 'CLIENT LIST', args: [], description: '获取客户端连接列表', category: 'Server' },
      { command: 'CLIENT SETNAME', args: ['connection-name'], description: '设置当前连接名称', category: 'Server' },
      { command: 'CLIENT GETNAME', args: [], description: '获取当前连接名称', category: 'Server' },
      { command: 'MONITOR', args: [], description: '实时监控服务器接收的命令', category: 'Server' },

      // 发布订阅命令
      { command: 'PUBLISH', args: ['channel', 'message'], description: '发送消息到指定频道', category: 'Pub/Sub' },
      { command: 'SUBSCRIBE', args: ['channel1', 'channel2', '...'], description: '订阅频道', category: 'Pub/Sub' },
      { command: 'UNSUBSCRIBE', args: ['[channel1]', '[channel2]', '...'], description: '退订频道', category: 'Pub/Sub' },
      { command: 'PSUBSCRIBE', args: ['pattern1', 'pattern2', '...'], description: '订阅匹配模式的频道', category: 'Pub/Sub' },
      { command: 'PUNSUBSCRIBE', args: ['[pattern1]', '[pattern2]', '...'], description: '退订匹配模式的频道', category: 'Pub/Sub' },

      // 事务命令
      { command: 'MULTI', args: [], description: '开始事务', category: 'Transaction' },
      { command: 'EXEC', args: [], description: '执行事务', category: 'Transaction' },
      { command: 'DISCARD', args: [], description: '取消事务', category: 'Transaction' },
      { command: 'WATCH', args: ['key1', 'key2', '...'], description: '监视键', category: 'Transaction' },
      { command: 'UNWATCH', args: [], description: '取消监视所有键', category: 'Transaction' },

      // Lua 脚本命令
      { command: 'EVAL', args: ['script', 'numkeys', 'key1', '...', 'arg1', '...'], description: '执行 Lua 脚本', category: 'Script' },
      { command: 'EVALSHA', args: ['sha1', 'numkeys', 'key1', '...', 'arg1', '...'], description: '执行缓存的 Lua 脚本', category: 'Script' },
      { command: 'SCRIPT LOAD', args: ['script'], description: '加载脚本到缓存', category: 'Script' },
      { command: 'SCRIPT EXISTS', args: ['sha1', '...'], description: '检查脚本是否存在', category: 'Script' },
      { command: 'SCRIPT FLUSH', args: [], description: '清空脚本缓存', category: 'Script' },
      { command: 'SCRIPT KILL', args: [], description: '杀死当前运行的脚本', category: 'Script' },
    ]
  }

  /**
   * 提供补全建议
   */
  provideCompletionItems(
    model: monaco.editor.ITextModel,
    position: monaco.Position
  ): monaco.languages.CompletionList {
    const textUntilPosition = model.getValueInRange({
      startLineNumber: position.lineNumber,
      startColumn: 1,
      endLineNumber: position.lineNumber,
      endColumn: position.column,
    })

    // 获取当前行的文本
    const line = textUntilPosition.trim()
    
    // 如果是注释行，不提供补全
    if (line.startsWith('#')) {
      return { suggestions: [] }
    }

    const words = line.split(/\s+/)
    const currentWord = words[words.length - 1]?.toUpperCase() || ''

    const suggestions: monaco.languages.CompletionItem[] = []

    // 如果是第一个单词或当前单词是空的，提供命令补全
    if (words.length <= 1 || !line.includes(' ')) {
      this.commands.forEach((cmd) => {
        if (cmd.command.startsWith(currentWord)) {
          suggestions.push({
            label: cmd.command,
            kind: monaco.languages.CompletionItemKind.Function,
            detail: cmd.category,
            documentation: {
              value: `**${cmd.command}** ${cmd.args.join(' ')}\n\n${cmd.description}`,
            },
            insertText: cmd.command,
            range: {
              startLineNumber: position.lineNumber,
              startColumn: position.column - currentWord.length,
              endLineNumber: position.lineNumber,
              endColumn: position.column,
            },
          })
        }
      })
    } else {
      // 提供参数提示
      const command = words[0].toUpperCase()
      const cmdInfo = this.commands.find((c) => c.command === command)
      
      if (cmdInfo && cmdInfo.args.length > 0) {
        const argIndex = words.length - 2
        if (argIndex < cmdInfo.args.length) {
          suggestions.push({
            label: cmdInfo.args[argIndex],
            kind: monaco.languages.CompletionItemKind.Variable,
            detail: `参数 ${argIndex + 1}`,
            documentation: `${cmdInfo.command} 的第 ${argIndex + 1} 个参数`,
            insertText: cmdInfo.args[argIndex],
            range: {
              startLineNumber: position.lineNumber,
              startColumn: position.column - currentWord.length,
              endLineNumber: position.lineNumber,
              endColumn: position.column,
            },
          })
        }
      }
    }

    return { suggestions }
  }
}

interface RedisCommand {
  command: string
  args: string[]
  description: string
  category: string
}

/**
 * 注册 Redis 补全提供器
 */
export function registerRedisCompletionProvider(): RedisCompletionProvider {
  const provider = new RedisCompletionProvider()

  monaco.languages.registerCompletionItemProvider('shell', {
    provideCompletionItems: (model, position) => {
      return provider.provideCompletionItems(model, position)
    },
  })

  return provider
}

