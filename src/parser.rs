//! Pascal parser sederhana yang menghasilkan AST dan symbol table.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Keyword(String),
    Identifier(String),
    Integer(String),
    StringLit(String),
    Symbol(String),
    Eof,
}

#[derive(Debug, Clone)]
pub struct TokenSpan {
    pub token: Token,
    pub line: usize,
    pub col: usize,
    pub len: usize,
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> char {
        self.chars.get(self.pos).copied().unwrap_or('\0')
    }

    fn next(&mut self) -> char {
        let ch = self.peek();
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        self.pos += 1;
        ch
    }

    fn skip_ws(&mut self) {
        loop {
            match self.peek() {
                ' ' | '\t' | '\r' | '\n' => { self.next(); }
                '{' => {
                    while self.peek() != '}' && self.peek() != '\0' {
                        self.next();
                    }
                    if self.peek() == '}' { self.next(); }
                }
                '(' => {
                    if self.chars.get(self.pos + 1) == Some(&'*') {
                        self.next(); self.next();
                        while !(self.peek() == '*' && self.chars.get(self.pos + 1) == Some(&')')) && self.peek() != '\0' {
                            self.next();
                        }
                        if self.peek() == '*' { self.next(); }
                        if self.peek() == ')' { self.next(); }
                    } else {
                        return;
                    }
                }
                '/' => {
                    if self.chars.get(self.pos + 1) == Some(&'/') {
                        while self.peek() != '\n' && self.peek() != '\0' {
                            self.next();
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            }
        }
    }

    fn next_token(&mut self) -> TokenSpan {
        self.skip_ws();
        let line = self.line;
        let col = self.col;
        let start = self.pos;

        let ch = self.peek();
        if ch == '\0' {
            self.pos += 1;
            return TokenSpan { token: Token::Eof, line, col, len: 0 };
        }

        if ch.is_alphabetic() || ch == '_' {
            let mut s = String::new();
            while self.peek().is_alphanumeric() || self.peek() == '_' {
                s.push(self.next());
            }
            let len = self.pos - start;
            let token = if is_keyword(&s) {
                Token::Keyword(s.clone())
            } else {
                Token::Identifier(s.clone())
            };
            return TokenSpan { token, line, col, len };
        }

        if ch.is_ascii_digit() {
            let mut s = String::new();
            while self.peek().is_ascii_digit() {
                s.push(self.next());
            }
            let len = self.pos - start;
            return TokenSpan { token: Token::Integer(s), line, col, len };
        }

        if ch == '\'' {
            self.next();
            let mut s = String::new();
            s.push('\'');
            while self.peek() != '\'' && self.peek() != '\0' {
                if self.peek() == '\\' { self.next(); }
                s.push(self.next());
            }
            if self.peek() == '\'' { s.push(self.next()); }
            let len = self.pos - start;
            return TokenSpan { token: Token::StringLit(s), line, col, len };
        }

        let two: String = self.chars.get(self.pos..self.pos + 2).map(|s| s.iter().collect()).unwrap_or_default();
        match two.as_str() {
            ":=" | "<>" | "<=" | ">=" | ".." => {
                self.next(); self.next();
                let len = self.pos - start;
                return TokenSpan { token: Token::Symbol(two), line, col, len };
            }
            _ => {}
        }

        let s = ch.to_string();
        self.next();
        let len = self.pos - start;
        TokenSpan { token: Token::Symbol(s), line, col, len }
    }
}

const KEYWORDS: &[&str] = &[
    "absolute", "and", "array", "as", "asm", "case", "const", "constructor", "destructor",
    "div", "do", "downto", "else", "end", "except", "exports", "file", "finalization", "finally",
    "for", "function", "goto", "if", "implementation", "in", "inherited", "inline", "interface",
    "is", "label", "library", "mod", "nil", "not", "object", "of", "only", "operator", "or",
    "otherwise", "packed", "procedure", "program", "record", "reintroduce", "repeat", "resident",
    "self", "set", "shl", "shr", "string", "then", "to", "type", "unit", "until", "uses", "var",
    "virtual", "while", "with", "xor", "begin",
];

fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Var(String, usize, usize),
    Int(String, usize, usize),
    Str(String, usize, usize),
    BinOp(Box<Expr>, String, Box<Expr>, usize, usize),
    Call(String, Vec<Expr>, usize, usize),
}

#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    pub vars: HashMap<String, (String, usize, usize)>,
    pub procs: HashMap<String, (usize, usize)>,
}

impl SymbolTable {
    pub fn new() -> Self { Self::default() }
}

pub struct Parser {
    tokens: Vec<TokenSpan>,
    pos: usize,
    pub symbols: SymbolTable,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::new();
        loop {
            let t = lexer.next_token();
            let is_eof = matches!(t.token, Token::Eof);
            tokens.push(t);
            if is_eof { break; }
        }
        Self { tokens, pos: 0, symbols: SymbolTable::new() }
    }

    fn peek(&self) -> &TokenSpan {
        self.tokens.get(self.pos).unwrap_or(&TokenSpan { token: Token::Eof, line: 0, col: 0, len: 0 })
    }

    fn advance(&mut self) -> TokenSpan {
        let t = self.peek().clone();
        if !matches!(t.token, Token::Eof) {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, s: &str) -> Result<TokenSpan, String> {
        let t = self.advance();
        match &t.token {
            Token::Symbol(sym) if sym == s => Ok(t),
            Token::Keyword(kw) if kw == s => Ok(t),
            _ => Err(format!("expected '{}' at line {} col {}", s, t.line, t.col)),
        }
    }

    fn expect_identifier(&mut self) -> Result<String, String> {
        let t = self.advance();
        match &t.token {
            Token::Identifier(name) => Ok(name.clone()),
            Token::Keyword(kw) if is_keyword(kw) => Ok(kw.clone()),
            _ => Err(format!("expected identifier at line {} col {}", t.line, t.col)),
        }
    }

    pub fn parse_program(&mut self) -> Result<(), String> {
        if let Token::Keyword(kw) = &self.peek().token {
            if kw == "program" {
                self.advance();
                let _name = self.expect_identifier()?;
                self.expect(";")?;
            }
        }

        loop {
            match &self.peek().token {
                Token::Keyword(kw) if kw == "uses" => {
                    self.advance();
                    loop {
                        let _ = self.expect_identifier()?;
                        match &self.peek().token {
                            Token::Symbol(sym) if sym == "," => { self.advance(); continue; }
                            Token::Symbol(sym) if sym == ";" => { self.advance(); break; }
                            _ => break,
                        }
                    }
                }
                Token::Keyword(kw) if kw == "type" => {
                    self.advance();
                    self.parse_type_section()?;
                }
                Token::Keyword(kw) if kw == "var" => {
                    self.advance();
                    self.parse_var_section()?;
                }
                Token::Keyword(kw) if kw == "procedure" => {
                    self.advance();
                    self.parse_proc_or_function(false)?;
                }
                Token::Keyword(kw) if kw == "function" => {
                    self.advance();
                    self.parse_proc_or_function(true)?;
                }
                Token::Keyword(kw) if kw == "begin" => {
                    self.advance();
                    self.parse_statement_list()?;
                    self.expect("end")?;
                    self.expect(".")?;
                    break;
                }
                Token::Eof => break,
                _ => {
                    self.advance();
                }
            }
        }
        Ok(())
    }

    fn parse_type_section(&mut self) -> Result<(), String> {
        loop {
            let _name = self.expect_identifier()?;
            self.expect("=")?;
            self.parse_type()?;
            self.expect(";")?;
            match &self.peek().token {
                Token::Keyword(kw) if kw == "var" || kw == "procedure" || kw == "function" || kw == "begin" => break,
                Token::Eof => break,
                _ => {}
            }
        }
        Ok(())
    }

    fn parse_type(&mut self) -> Result<(), String> {
        let t = self.advance();
        match &t.token {
            Token::Keyword(kw) if matches!(kw.as_str(), "integer" | "string" | "char" | "boolean" | "real" | "record" | "array") => Ok(()),
            Token::Identifier(_) => Ok(()),
            Token::Symbol(sym) if sym == "[" => {
                while self.peek().token != Token::Symbol("]".to_string()) && !matches!(self.peek().token, Token::Eof) {
                    self.advance();
                }
                self.expect("]")?;
                self.expect("of")?;
                self.parse_type()
            }
            _ => Ok(()),
        }
    }

    fn parse_var_section(&mut self) -> Result<(), String> {
        loop {
            let mut names: Vec<(String, usize, usize)> = Vec::new();
            loop {
                let t = self.peek();
                let name_line = t.line;
                let name_col = t.col;
                names.push((self.expect_identifier()?, name_line, name_col));
                match &self.peek().token {
                    Token::Symbol(sym) if sym == "," => { self.advance(); continue; }
                    _ => break,
                }
            }
            self.expect(":")?;
            let type_name = match self.advance().token {
                Token::Identifier(n) => n,
                Token::Keyword(kw) => kw,
                t => return Err(format!("expected type name, got {:?} at line {}", t, self.peek().line)),
            };
            self.expect(";")?;
            for (name, line, col) in names {
                self.symbols.vars.insert(name, (type_name.clone(), line, col));
            }
            match &self.peek().token {
                Token::Keyword(kw) if kw == "var" || kw == "procedure" || kw == "function" || kw == "begin" => break,
                Token::Eof => break,
                _ => {}
            }
        }
        Ok(())
    }

    fn parse_proc_or_function(&mut self, is_function: bool) -> Result<(), String> {
        let name = self.expect_identifier()?;
        let mut params = Vec::new();

        if self.peek().token == Token::Symbol("(".to_string()) {
            self.advance();
            while self.peek().token != Token::Symbol(")".to_string()) && !matches!(self.peek().token, Token::Eof) {
                let mut param_names = Vec::new();
                loop {
                    param_names.push(self.expect_identifier()?);
                    match &self.peek().token {
                        Token::Symbol(sym) if sym == "," => { self.advance(); continue; }
                        Token::Symbol(sym) if sym == ":" => { self.advance(); break; }
                        _ => break,
                    }
                }
                let ptype = match self.advance().token {
                    Token::Identifier(n) => n,
                    Token::Keyword(kw) => kw,
                    t => return Err(format!("expected param type, got {:?}", t)),
                };
                for n in param_names {
                    params.push((n, ptype.clone()));
                }
                match &self.peek().token {
                    Token::Symbol(sym) if sym == ";" => { self.advance(); continue; }
                    _ => break,
                }
            }
            self.expect(")")?;
        }

        let mut return_type = None;
        if is_function {
            self.expect(":")?;
            return_type = Some(match self.advance().token {
                Token::Identifier(n) => n,
                Token::Keyword(kw) => kw,
                t => return Err(format!("expected return type, got {:?}", t)),
            });
        }

        self.expect(";")?;
        let proc_line = self.peek().line;
        let proc_col = self.peek().col;
        self.symbols.procs.insert(name.clone(), (proc_line, proc_col));

        let mut depth = 0i32;
        loop {
            match &self.peek().token {
                Token::Keyword(kw) if kw == "begin" => { depth += 1; self.advance(); }
                Token::Keyword(kw) if kw == "end" => {
                    if depth <= 1 {
                        self.advance();
                        break;
                    } else {
                        depth -= 1;
                        self.advance();
                    }
                }
                Token::Eof => break,
                _ => { self.advance(); }
            }
        }

        let _ = params;
        let _ = return_type;
        Ok(())
    }

    fn parse_statement_list(&mut self) -> Result<(), String> {
        loop {
            self.parse_statement()?;
            match &self.peek().token {
                Token::Symbol(sym) if sym == ";" => { self.advance(); }
                Token::Keyword(kw) if kw == "end" => break,
                Token::Eof => break,
                _ => break,
            }
        }
        Ok(())
    }

    fn parse_statement(&mut self) -> Result<(), String> {
        match &self.peek().token {
            Token::Keyword(kw) if kw == "if" => {
                self.advance();
                self.parse_expr()?;
                self.expect("then")?;
                self.parse_statement()?;
                if self.peek().token == Token::Keyword("else".to_string()) {
                    self.advance();
                    self.parse_statement()?;
                }
            }
            Token::Keyword(kw) if kw == "while" => {
                self.advance();
                self.parse_expr()?;
                self.expect("do")?;
                self.parse_statement()?;
            }
            Token::Keyword(kw) if kw == "for" => {
                self.advance();
                self.expect_identifier()?;
                self.expect(":=")?;
                self.parse_expr()?;
                self.expect("to")?;
                self.parse_expr()?;
                self.expect("do")?;
                self.parse_statement()?;
            }
            Token::Keyword(kw) if kw == "repeat" => {
                self.advance();
                self.parse_statement_list()?;
                self.expect("until")?;
                self.parse_expr()?;
            }
            Token::Keyword(kw) if kw == "case" => {
                self.advance();
                self.parse_expr()?;
                self.expect("of")?;
                while self.peek().token != Token::Keyword("end".to_string()) && !matches!(self.peek().token, Token::Eof) {
                    self.parse_expr()?;
                    self.expect(":")?;
                    self.parse_statement()?;
                    match &self.peek().token {
                        Token::Symbol(sym) if sym == ";" => { self.advance(); }
                        _ => break,
                    }
                }
                self.expect("end")?;
            }
            Token::Keyword(kw) if kw == "begin" => {
                self.advance();
                self.parse_statement_list()?;
                self.expect("end")?;
            }
            Token::Identifier(_) | Token::Keyword(_) => {
                // Jangan konsumsi 'end'; biar parse_statement_list yang tangani
                if let Token::Keyword(kw) = &self.peek().token {
                    if kw == "end" {
                        return Ok(());
                    }
                }
                let _name = self.expect_identifier()?;
                let nxt = self.peek().token.clone();
                match nxt {
                    Token::Symbol(sym) if sym == ":=" => { self.advance(); self.parse_expr()?; }
                    Token::Symbol(sym) if sym == "(" => {
                        self.advance();
                        while self.peek().token != Token::Symbol(")".to_string()) && !matches!(self.peek().token, Token::Eof) {
                            self.parse_expr()?;
                            match &self.peek().token {
                                Token::Symbol(sym) if sym == "," => { self.advance(); }
                                _ => break,
                            }
                        }
                        self.expect(")")?;
                    }
                    Token::Symbol(sym) if sym == "." => {
                        self.advance();
                        self.expect_identifier()?;
                    }
                    _ => {}
                }
            }
            _ => { self.advance(); }
        }
        Ok(())
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_simple_expr()
    }

    fn parse_simple_expr(&mut self) -> Result<Expr, String> {
        let t = self.advance();
        match &t.token {
            Token::Identifier(name) => {
                if self.peek().token == Token::Symbol("(".to_string()) {
                    self.advance();
                    let mut args = Vec::new();
                    while self.peek().token != Token::Symbol(")".to_string()) && !matches!(self.peek().token, Token::Eof) {
                        args.push(self.parse_expr()?);
                        match &self.peek().token {
                            Token::Symbol(sym) if sym == "," => { self.advance(); }
                            _ => break,
                        }
                    }
                    self.expect(")")?;
                    Ok(Expr::Call(name.clone(), args, t.line, t.col))
                } else {
                    Ok(Expr::Var(name.clone(), t.line, t.col))
                }
            }
            Token::Integer(n) => Ok(Expr::Int(n.clone(), t.line, t.col)),
            Token::StringLit(s) => Ok(Expr::Str(s.clone(), t.line, t.col)),
            _ => Err(format!("unexpected token in expr at line {} col {}", t.line, t.col)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_program() {
        let src = "program test;\nvar x: integer;\nbegin\nx := 1;\nend.\n";
        let mut p = Parser::new(src);
        assert!(p.parse_program().is_ok());
        assert!(p.symbols.vars.contains_key("x"));
    }

    #[test]
    fn parse_proc_and_func() {
        let src = "program test;\nprocedure p(a: integer);\nbegin end;\nfunction f: integer;\nbegin end;\nbegin end.\n";
        let mut p = Parser::new(src);
        assert!(p.parse_program().is_ok());
        assert!(p.symbols.procs.contains_key("p"));
        assert!(p.symbols.procs.contains_key("f"));
    }

    #[test]
    fn parse_full_test_program() {
        let src = "program test;\nvar x, y: integer;\nprocedure add(a, b: integer);\nbegin x := a + b; end;\nfunction double(n: integer): integer;\nbegin double := n * 2; end;\nbegin x := 10; y := 20; add(x, y); end.\n";
        let mut p = Parser::new(src);
        assert!(p.parse_program().is_ok(), "parse failed");
        assert!(p.symbols.vars.contains_key("x"), "x not in vars");
        assert!(p.symbols.vars.contains_key("y"), "y not in vars");
        assert!(p.symbols.procs.contains_key("add"), "add not in procs");
        assert!(p.symbols.procs.contains_key("double"), "double not in procs");
        let (t, l, _c) = p.symbols.vars.get("x").unwrap();
        assert_eq!(t.as_str(), "integer");
        assert_eq!(*l, 2);
    }
}
