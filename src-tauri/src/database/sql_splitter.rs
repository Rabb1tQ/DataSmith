/// SQL语句分割器
/// 完全基于DBeaver的SQLScriptParser实现
/// 
/// DBeaver核心逻辑翻译自:
/// org.jkiss.dbeaver.model.sql.parser.SQLScriptParser


/// Token类型 - 对应DBeaver的SQLTokenType
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlTokenType {
    T_UNKNOWN,
    T_KEYWORD,
    T_STRING,
    T_QUOTED,
    T_COMMENT,
    T_DELIMITER,
    T_OTHER,
    T_BLOCK_HEADER,
    T_BLOCK_BEGIN,
    T_BLOCK_END,
    T_BLOCK_TOGGLE,
    T_SET_DELIMITER,
}

/// SQL方言配置 - 对应DBeaver的SQLDialect
#[derive(Debug, Clone)]
pub struct SqlDialect {
    pub statement_delimiters: Vec<String>,
    pub string_quote_strings: Vec<(String, String)>,
    pub identifier_quote_strings: Vec<(String, String)>,
    pub single_line_comments: Vec<String>,
    pub multi_line_comment: Option<(String, String)>,
    pub supports_nested_comments: bool,
    pub script_delimiter_redefiner: Option<String>,
    pub escape_char: char,
    pub block_bound_strings: Vec<(String, String)>,
    pub block_header_strings: Vec<String>,
    pub inner_block_prefixes: Vec<String>,
    pub delimiter_after_block: bool,
}

impl Default for SqlDialect {
    fn default() -> Self { Self::mysql() }
}

impl SqlDialect {
    pub fn mysql() -> Self {
        Self {
            statement_delimiters: vec![";".to_string()],
            string_quote_strings: vec![("'".to_string(), "'".to_string())],
            identifier_quote_strings: vec![
                ("`".to_string(), "`".to_string()),
                ("\"".to_string(), "\"".to_string()),
            ],
            single_line_comments: vec!["-- ".to_string(), "#".to_string()],
            multi_line_comment: Some(("/*".to_string(), "*/".to_string())),
            supports_nested_comments: false,
            script_delimiter_redefiner: Some("DELIMITER".to_string()),
            escape_char: '\\',
            // MySQL存储过程使用BEGIN/END块
            block_bound_strings: vec![
                ("BEGIN".to_string(), "END".to_string()),
            ],
            block_header_strings: vec![
                "DECLARE".to_string(), "FUNCTION".to_string(), "PROCEDURE".to_string(),
                "TRIGGER".to_string(),
            ],
            inner_block_prefixes: vec!["AS".to_string(), "IS".to_string()],
            delimiter_after_block: true,
        }
    }

    pub fn postgresql() -> Self {
        let mut config = Self::mysql();
        config.string_quote_strings.push(("$$".to_string(), "$$".to_string()));
        config.single_line_comments = vec!["-- ".to_string()];
        config.identifier_quote_strings = vec![("\"".to_string(), "\"".to_string())];
        config.script_delimiter_redefiner = None;
        config.block_bound_strings = vec![
            ("BEGIN".to_string(), "END".to_string()),
            ("CASE".to_string(), "END".to_string()),
        ];
        config.supports_nested_comments = true;
        config
    }

    pub fn sqlite() -> Self {
        let mut config = Self::mysql();
        config.single_line_comments = vec!["-- ".to_string()];
        config.script_delimiter_redefiner = None;
        config
    }
}

/// 脚本块信息 - 对应DBeaver的ScriptBlockInfo
#[derive(Debug)]
struct ScriptBlockInfo {
    parent: Option<Box<ScriptBlockInfo>>,
    toggle_pattern: Option<String>,
    is_header: bool,
}

impl Clone for ScriptBlockInfo {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.as_ref().map(|p| Box::new((**p).clone())),
            toggle_pattern: self.toggle_pattern.clone(),
            is_header: self.is_header,
        }
    }
}

impl ScriptBlockInfo {
    fn new(parent: Option<Box<ScriptBlockInfo>>, is_header: bool) -> Self {
        Self { parent, toggle_pattern: None, is_header }
    }

    fn with_toggle(parent: Option<Box<ScriptBlockInfo>>, toggle_pattern: String) -> Self {
        Self { parent, toggle_pattern: Some(toggle_pattern), is_header: false }
    }
}

/// Token信息
#[derive(Debug, Clone)]
struct Token {
    token_type: SqlTokenType,
    offset: usize,
    length: usize,
    is_whitespace: bool,
    is_eof: bool,
}

impl Token {
    fn new(token_type: SqlTokenType, offset: usize, length: usize, is_whitespace: bool) -> Self {
        Self { token_type, offset, length, is_whitespace, is_eof: false }
    }

    fn eof(offset: usize) -> Self {
        Self { token_type: SqlTokenType::T_UNKNOWN, offset, length: 0, is_whitespace: false, is_eof: true }
    }
}

/// SQL扫描器 - 对应DBeaver的TPRuleBasedScanner
struct SqlScanner<'a> {
    chars: &'a [char],
    pos: usize,
    dialect: &'a SqlDialect,
    current_delimiter: String,
}

impl<'a> SqlScanner<'a> {
    fn new(chars: &'a [char], dialect: &'a SqlDialect, current_delimiter: String) -> Self {
        Self { chars, pos: 0, dialect, current_delimiter }
    }

    fn set_current_delimiter(&mut self, delimiter: String) {
        self.current_delimiter = delimiter;
    }

    /// 读取下一个Token - 核心扫描逻辑
    fn next_token(&mut self) -> Token {
        if self.pos >= self.chars.len() {
            return Token::eof(self.pos);
        }

        let start = self.pos;

        // 1. 空白字符
        if self.chars[self.pos].is_whitespace() {
            while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
                self.pos += 1;
            }
            return Token::new(SqlTokenType::T_OTHER, start, self.pos - start, true);
        }

        // 2. 多行注释 /* */
        if let Some(token) = self.read_multiline_comment(start) {
            return token;
        }

        // 3. 单行注释 -- 或 #
        if let Some(token) = self.read_single_line_comment(start) {
            return token;
        }

        // 4. 字符串
        if let Some(token) = self.read_string(start) {
            return token;
        }

        // 5. 引号标识符
        if let Some(token) = self.read_quoted_identifier(start) {
            return token;
        }

        // 6. 分隔符
        if let Some(token) = self.read_delimiter(start) {
            return token;
        }

        // 7. 数字
        if let Some(token) = self.read_number(start) {
            return token;
        }

        // 8. 单词（关键字或标识符）
        if self.chars[self.pos].is_alphabetic() || self.chars[self.pos] == '_' {
            return self.read_word(start);
        }

        // 9. 其他字符
        self.pos += 1;
        Token::new(SqlTokenType::T_OTHER, start, 1, false)
    }

    fn read_multiline_comment(&mut self, start: usize) -> Option<Token> {
        if let Some((begin, end)) = &self.dialect.multi_line_comment {
            let begin_chars: Vec<char> = begin.chars().collect();
            let end_chars: Vec<char> = end.chars().collect();

            if self.pos + begin_chars.len() <= self.chars.len()
                && self.chars[self.pos..self.pos + begin_chars.len()].iter().eq(begin_chars.iter())
            {
                self.pos += begin_chars.len();
                let mut depth = 1;

                while self.pos < self.chars.len() && depth > 0 {
                    // 嵌套注释支持
                    if self.dialect.supports_nested_comments
                        && self.pos + begin_chars.len() <= self.chars.len()
                        && self.chars[self.pos..self.pos + begin_chars.len()].iter().eq(begin_chars.iter())
                    {
                        depth += 1;
                        self.pos += begin_chars.len();
                        continue;
                    }

                    if self.pos + end_chars.len() <= self.chars.len()
                        && self.chars[self.pos..self.pos + end_chars.len()].iter().eq(end_chars.iter())
                    {
                        depth -= 1;
                        self.pos += end_chars.len();
                        continue;
                    }
                    self.pos += 1;
                }

                return Some(Token::new(SqlTokenType::T_COMMENT, start, self.pos - start, false));
            }
        }
        None
    }

    fn read_single_line_comment(&mut self, start: usize) -> Option<Token> {
        for comment_prefix in &self.dialect.single_line_comments {
            let prefix_chars: Vec<char> = comment_prefix.chars().collect();

            if self.pos + prefix_chars.len() <= self.chars.len()
                && self.chars[self.pos..self.pos + prefix_chars.len()].iter().eq(prefix_chars.iter())
            {
                self.pos += prefix_chars.len();

                while self.pos < self.chars.len() && self.chars[self.pos] != '\n' && self.chars[self.pos] != '\r' {
                    self.pos += 1;
                }

                if self.pos < self.chars.len() && self.chars[self.pos] == '\r' { self.pos += 1; }
                if self.pos < self.chars.len() && self.chars[self.pos] == '\n' { self.pos += 1; }

                return Some(Token::new(SqlTokenType::T_COMMENT, start, self.pos - start, false));
            }
        }
        None
    }

    fn read_string(&mut self, start: usize) -> Option<Token> {
        for (begin, end) in &self.dialect.string_quote_strings {
            let begin_chars: Vec<char> = begin.chars().collect();
            let end_chars: Vec<char> = end.chars().collect();

            if self.pos + begin_chars.len() > self.chars.len() { continue; }
            if !self.chars[self.pos..self.pos + begin_chars.len()].iter().eq(begin_chars.iter()) { continue; }

            self.pos += begin_chars.len();

            while self.pos < self.chars.len() {
                // 转义字符
                if self.chars[self.pos] == self.dialect.escape_char {
                    self.pos += 1;
                    if self.pos < self.chars.len() { self.pos += 1; }
                    continue;
                }

                // 结束引号
                if self.pos + end_chars.len() <= self.chars.len()
                    && self.chars[self.pos..self.pos + end_chars.len()].iter().eq(end_chars.iter())
                {
                    self.pos += end_chars.len();
                    return Some(Token::new(SqlTokenType::T_STRING, start, self.pos - start, false));
                }
                self.pos += 1;
            }

            return Some(Token::new(SqlTokenType::T_STRING, start, self.pos - start, false));
        }
        None
    }

    fn read_quoted_identifier(&mut self, start: usize) -> Option<Token> {
        for (begin, end) in &self.dialect.identifier_quote_strings {
            let begin_chars: Vec<char> = begin.chars().collect();
            let end_chars: Vec<char> = end.chars().collect();

            if self.pos + begin_chars.len() > self.chars.len() { continue; }
            if !self.chars[self.pos..self.pos + begin_chars.len()].iter().eq(begin_chars.iter()) { continue; }

            self.pos += begin_chars.len();

            while self.pos + end_chars.len() <= self.chars.len() {
                if self.chars[self.pos..self.pos + end_chars.len()].iter().eq(end_chars.iter()) {
                    self.pos += end_chars.len();
                    return Some(Token::new(SqlTokenType::T_QUOTED, start, self.pos - start, false));
                }
                self.pos += 1;
            }

            return Some(Token::new(SqlTokenType::T_QUOTED, start, self.pos - start, false));
        }
        None
    }

    fn read_delimiter(&mut self, start: usize) -> Option<Token> {
        let delim_chars: Vec<char> = self.current_delimiter.chars().collect();
        
        if self.pos + delim_chars.len() <= self.chars.len()
            && self.chars[self.pos..self.pos + delim_chars.len()].iter().eq(delim_chars.iter())
        {
            self.pos += delim_chars.len();
            return Some(Token::new(SqlTokenType::T_DELIMITER, start, self.pos - start, false));
        }
        None
    }

    fn read_number(&mut self, start: usize) -> Option<Token> {
        if !self.chars[self.pos].is_ascii_digit() {
            return None;
        }

        while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
            self.pos += 1;
        }

        // 小数部分
        if self.pos < self.chars.len() && self.chars[self.pos] == '.' {
            self.pos += 1;
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        // 指数部分
        if self.pos < self.chars.len() && (self.chars[self.pos] == 'e' || self.chars[self.pos] == 'E') {
            self.pos += 1;
            if self.pos < self.chars.len() && (self.chars[self.pos] == '+' || self.chars[self.pos] == '-') {
                self.pos += 1;
            }
            while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        Some(Token::new(SqlTokenType::T_OTHER, start, self.pos - start, false))
    }

    fn read_word(&mut self, start: usize) -> Token {
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if c.is_alphanumeric() || c == '_' || c == '$' {
                self.pos += 1;
            } else {
                break;
            }
        }

        let word: String = self.chars[start..self.pos].iter().collect();
        let word_upper = word.to_uppercase();

        // 判断token类型
        let token_type = if self.dialect.block_header_strings.contains(&word_upper) {
            SqlTokenType::T_BLOCK_HEADER
        } else if self.dialect.block_bound_strings.iter().any(|(begin, _)| begin == &word_upper) {
            SqlTokenType::T_BLOCK_BEGIN
        } else if self.dialect.block_bound_strings.iter().any(|(_, end)| end == &word_upper) {
            SqlTokenType::T_BLOCK_END
        } else if word_upper == "BEGIN" {
            SqlTokenType::T_BLOCK_BEGIN
        } else if word_upper == "END" {
            SqlTokenType::T_BLOCK_END
        } else {
            SqlTokenType::T_KEYWORD
        };

        Token::new(token_type, start, self.pos - start, false)
    }
}

/// SQL分割器 - 对应DBeaver的SQLScriptParser
pub struct SqlSplitter {
    dialect: SqlDialect,
    current_delimiter: String,
}

impl SqlSplitter {
    pub fn new(dialect: SqlDialect) -> Self {
        let default_delimiter = dialect.statement_delimiters.first()
            .cloned()
            .unwrap_or_else(|| ";".to_string());
        Self {
            dialect,
            current_delimiter: default_delimiter,
        }
    }

    /// 分割SQL脚本 - 对应DBeaver的extractScriptQueries方法
    /// 核心逻辑翻译自SQLScriptParser.parseQueryImpl
    pub fn split(&mut self, sql: &str) -> Vec<String> {
        let mut statements = Vec::new();
        let chars: Vec<char> = sql.chars().collect();
        
        if chars.is_empty() {
            return statements;
        }

        let mut scanner = SqlScanner::new(&chars, &self.dialect, self.current_delimiter.clone());
        let mut statement_start = 0;
        let mut has_valuable_tokens = false;
        let mut cur_block: Option<Box<ScriptBlockInfo>> = None;
        let mut bracket_depth = 0i32;
        let mut prev_not_empty_token_type = SqlTokenType::T_UNKNOWN;
        let mut first_keyword: Option<String> = None;
        let mut last_keyword: Option<String> = None;

        loop {
            let token = scanner.next_token();
            
            if token.is_eof {
                // 处理最后一个语句
                if has_valuable_tokens && token.offset > statement_start {
                    let stmt = self.extract_statement(&chars, statement_start, token.offset);
                    if !stmt.trim().is_empty() {
                        statements.push(stmt);
                    }
                }
                break;
            }

            let token_type = token.token_type;
            let token_offset = token.offset;
            let token_length = token.length;

            // 处理DELIMITER命令
            if token_type == SqlTokenType::T_KEYWORD && self.dialect.script_delimiter_redefiner.is_some() {
                let token_text: String = chars[token_offset..token_offset + token_length].iter().collect();
                let token_upper = token_text.to_uppercase();
                
                if let Some(ref delim_cmd) = self.dialect.script_delimiter_redefiner {
                    if token_upper == delim_cmd.to_uppercase() {
                        // 解析新的分隔符
                        if let Some((new_delim, end_pos)) = self.parse_delimiter_command(&chars, token_offset + token_length) {
                            // 保存之前的语句
                            if has_valuable_tokens && token_offset > statement_start {
                                let stmt = self.extract_statement(&chars, statement_start, token_offset);
                                if !stmt.trim().is_empty() {
                                    statements.push(stmt);
                                }
                            }
                            
                            self.current_delimiter = new_delim;
                            scanner.set_current_delimiter(self.current_delimiter.clone());
                            
                            // 跳过到新位置
                            statement_start = end_pos;
                            has_valuable_tokens = false;
                            bracket_depth = 0;
                            cur_block = None;
                            prev_not_empty_token_type = SqlTokenType::T_UNKNOWN;
                            first_keyword = None;
                            last_keyword = None;
                            continue;
                        }
                    }
                }
            }

            // 跳过空白
            if token.is_whitespace {
                continue;
            }

            // 跳过注释
            if token_type == SqlTokenType::T_COMMENT {
                continue;
            }

            // 跳过字符串和引号标识符
            if token_type == SqlTokenType::T_STRING || token_type == SqlTokenType::T_QUOTED {
                has_valuable_tokens = true;
                prev_not_empty_token_type = token_type;
                continue;
            }

            // 处理括号
            if token_length == 1 && token_offset < chars.len() {
                let ch = chars[token_offset];
                if ch == '(' || ch == '{' || ch == '[' {
                    bracket_depth += 1;
                    has_valuable_tokens = true;
                    prev_not_empty_token_type = token_type;
                    continue;
                }
                if ch == ')' || ch == '}' || ch == ']' {
                    if bracket_depth > 0 {
                        bracket_depth -= 1;
                    }
                    has_valuable_tokens = true;
                    prev_not_empty_token_type = token_type;
                    continue;
                }
            }

            // 处理块开始/结束
            match token_type {
                SqlTokenType::T_BLOCK_HEADER => {
                    cur_block = Some(Box::new(ScriptBlockInfo::new(cur_block, true)));
                    has_valuable_tokens = true;
                }
                SqlTokenType::T_BLOCK_BEGIN => {
                    // 如果BEGIN紧跟分隔符，则不是块
                    if prev_not_empty_token_type == SqlTokenType::T_DELIMITER {
                        // 不是块
                    } else {
                        // 如果前一个非空token是块头，则丢弃头块
                        if let Some(ref block) = cur_block {
                            if block.is_header {
                                let parent = block.parent.clone();
                                cur_block = parent;
                            }
                        }
                        cur_block = Some(Box::new(ScriptBlockInfo::new(cur_block, false)));
                    }
                    has_valuable_tokens = true;
                }
                SqlTokenType::T_BLOCK_END => {
                    if let Some(ref block) = cur_block {
                        cur_block = block.parent.clone();
                    }
                    has_valuable_tokens = true;
                }
                SqlTokenType::T_BLOCK_TOGGLE => {
                    let toggle_text: String = chars[token_offset..token_offset + token_length].iter().collect();
                    if let Some(ref block) = cur_block {
                        if block.toggle_pattern.as_ref() == Some(&toggle_text) {
                            cur_block = block.parent.clone();
                        } else {
                            cur_block = Some(Box::new(ScriptBlockInfo::with_toggle(cur_block, toggle_text)));
                        }
                    } else {
                        cur_block = Some(Box::new(ScriptBlockInfo::with_toggle(None, toggle_text)));
                    }
                    has_valuable_tokens = true;
                }
                _ => {}
            }

            // 记录关键字
            if token_type == SqlTokenType::T_KEYWORD ||
               token_type == SqlTokenType::T_BLOCK_BEGIN ||
               token_type == SqlTokenType::T_BLOCK_END ||
               token_type == SqlTokenType::T_BLOCK_HEADER {
                let word: String = chars[token_offset..token_offset + token_length].iter().collect();
                last_keyword = Some(word.clone());
                if first_keyword.is_none() {
                    first_keyword = Some(word);
                }
            }

            // 处理分隔符
            if token_type == SqlTokenType::T_DELIMITER {
                // 如果在括号或块内，忽略分隔符
                if bracket_depth > 0 || cur_block.is_some() {
                    prev_not_empty_token_type = token_type;
                    continue;
                }

                // 提取语句
                if has_valuable_tokens {
                    let stmt = self.extract_statement(&chars, statement_start, token_offset);
                    if !stmt.trim().is_empty() {
                        statements.push(stmt);
                    }
                }

                statement_start = token_offset + token_length;
                has_valuable_tokens = false;
                bracket_depth = 0;
                cur_block = None;
                prev_not_empty_token_type = SqlTokenType::T_UNKNOWN;
                first_keyword = None;
                last_keyword = None;
                continue;
            }

            has_valuable_tokens = true;
            prev_not_empty_token_type = token_type;
        }

        statements
    }

    /// 解析DELIMITER命令
    fn parse_delimiter_command(&self, chars: &[char], start: usize) -> Option<(String, usize)> {
        let mut pos = start;
        
        // 跳过空白
        while pos < chars.len() && chars[pos].is_whitespace() {
            pos += 1;
        }

        // 读取新分隔符
        let mut new_delim = String::new();
        while pos < chars.len() {
            let c = chars[pos];
            if c == '\n' || c == '\r' {
                break;
            }
            if c.is_whitespace() && !new_delim.is_empty() {
                break;
            }
            new_delim.push(c);
            pos += 1;
        }

        if new_delim.is_empty() {
            return None;
        }

        // 跳过换行符
        while pos < chars.len() && (chars[pos] == '\r' || chars[pos] == '\n') {
            pos += 1;
        }

        Some((new_delim, pos))
    }

    /// 提取语句
    fn extract_statement(&self, chars: &[char], start: usize, end: usize) -> String {
        // 移除前导空白
        let mut stmt_start = start;
        while stmt_start < end && chars[stmt_start].is_whitespace() {
            stmt_start += 1;
        }

        // 移除尾部空白
        let mut stmt_end = end;
        while stmt_end > stmt_start && chars[stmt_end - 1].is_whitespace() {
            stmt_end -= 1;
        }

        chars[stmt_start..stmt_end].iter().collect()
    }
}

/// 判断是否为查询语句
pub fn is_query_statement(sql: &str) -> bool {
    let sql_trimmed = sql.trim().to_uppercase();
    sql_trimmed.starts_with("SELECT")
        || sql_trimmed.starts_with("SHOW")
        || sql_trimmed.starts_with("DESCRIBE")
        || sql_trimmed.starts_with("DESC")
        || sql_trimmed.starts_with("EXPLAIN")
        || sql_trimmed.starts_with("WITH")
}

/// 截断SQL用于显示
pub fn truncate_sql(sql: &str, max_len: usize) -> String {
    if sql.len() <= max_len {
        sql.to_string()
    } else {
        sql.chars().take(max_len).collect::<String>() + "..."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_statements() {
        let mut splitter = SqlSplitter::new(SqlDialect::mysql());
        let sql = "SELECT 1; SELECT 2; SELECT 3;";
        let statements = splitter.split(sql);
        assert_eq!(statements.len(), 3);
    }

    #[test]
    fn test_string_with_escape() {
        let mut splitter = SqlSplitter::new(SqlDialect::mysql());
        let sql = r#"INSERT INTO t VALUES ('a;b'); SELECT 1;"#;
        let statements = splitter.split(sql);
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn test_delimiter_command() {
        let mut splitter = SqlSplitter::new(SqlDialect::mysql());
        let sql = "DELIMITER //\nSELECT 1//\nDELIMITER ;\nSELECT 2;";
        let statements = splitter.split(sql);
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn test_create_procedure() {
        let mut splitter = SqlSplitter::new(SqlDialect::mysql());
        let sql = r#"
CREATE PROCEDURE simpleproc (OUT param1 INT)
BEGIN
  SELECT COUNT(*) INTO param1 FROM t;
END;
SELECT 1;
"#;
        let statements = splitter.split(sql);
        println!("Statements: {:?}", statements);
        assert!(statements.len() >= 2);
    }

    #[test]
    fn test_comment_handling() {
        let mut splitter = SqlSplitter::new(SqlDialect::mysql());
        let sql = r#"
-- This is a comment
SELECT 1; -- another comment
/* multi-line
   comment */
SELECT 2;
"#;
        let statements = splitter.split(sql);
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn test_quoted_identifier() {
        let mut splitter = SqlSplitter::new(SqlDialect::mysql());
        let sql = r#"SELECT `col;name` FROM `table`; SELECT 2;"#;
        let statements = splitter.split(sql);
        assert_eq!(statements.len(), 2);
    }
}