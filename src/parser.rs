use hash_cons::{Hc, HcTable};
use std::fmt;
use serde::{Serialize, Deserialize, Serializer, Deserializer};

#[derive(Hash, Eq, PartialEq, Debug, Clone, Ord, PartialOrd, serde::Serialize, serde::Deserialize)]
pub enum Expr {
    Assign(String, #[serde(with = "hc_expr_serde")] Hc<Expr>),
    Equal(#[serde(with = "hc_expr_serde")] Hc<Expr>, #[serde(with = "hc_expr_serde")] Hc<Expr>),
    Add(#[serde(with = "hc_expr_serde")] Hc<Expr>, #[serde(with = "hc_expr_serde")] Hc<Expr>),
    Subtract(#[serde(with = "hc_expr_serde")] Hc<Expr>, #[serde(with = "hc_expr_serde")] Hc<Expr>),
    Sequence(#[serde(with = "hc_expr_serde")] Hc<Expr>, #[serde(with = "hc_expr_serde")] Hc<Expr>),
    If(#[serde(with = "hc_expr_serde")] Hc<Expr>, #[serde(with = "hc_expr_serde")] Hc<Expr>, #[serde(with = "hc_expr_serde")] Hc<Expr>),
    While(#[serde(with = "hc_expr_serde")] Hc<Expr>, #[serde(with = "hc_expr_serde")] Hc<Expr>),
    Not(#[serde(with = "hc_expr_serde")] Hc<Expr>),
    And(#[serde(with = "hc_expr_serde")] Hc<Expr>, #[serde(with = "hc_expr_serde")] Hc<Expr>),
    Or(#[serde(with = "hc_expr_serde")] Hc<Expr>, #[serde(with = "hc_expr_serde")] Hc<Expr>),
    Yield,
    Exit,
    Unknown,
    Number(i64),
    Variable(String),
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Ord, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct Program {
    pub requests: Vec<Request>,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Ord, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct Request {
    pub name: String,
    #[serde(with = "hc_expr_serde")]
    pub body: Hc<Expr>,
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Assign(var, expr) => write!(f, "{} := {}", var, expr),
            Expr::Equal(left, right) => write!(f, "{} == {}", left, right),
            Expr::Add(left, right) => write!(f, "{} + {}", left, right),
            Expr::Subtract(left, right) => write!(f, "{} - {}", left, right),
            Expr::Sequence(first, second) => write!(f, "{}; {}", first, second),
            Expr::If(cond, then_branch, else_branch) => {
                write!(f, "if({}){{{}}}else{{{}}}", cond, then_branch, else_branch)
            }
            Expr::While(cond, body) => write!(f, "while({}){{ {} }}", cond, body),
            Expr::Not(expr) => write!(f, "!{}", expr),
            Expr::And(left, right) => write!(f, "{} && {}", left, right),
            Expr::Or(left, right) => write!(f, "{} || {}", left, right),
            Expr::Yield => write!(f, "yield"),
            Expr::Exit => write!(f, "exit"),
            Expr::Unknown => write!(f, "?"),
            Expr::Number(n) => write!(f, "{}", n),
            Expr::Variable(var) => write!(f, "{}", var),
        }
    }
}

// Custom serialization module for Hc<Expr>
pub mod hc_expr_serde {
    use super::*;
    
    // Serialize Hc<Expr> by serializing the inner Expr
    pub fn serialize<S>(hc: &Hc<Expr>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Dereference to get the inner Expr and serialize it
        (**hc).serialize(serializer)
    }
    
    // Deserialize by deserializing an Expr and wrapping in Hc
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Hc<Expr>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // We need access to a hash cons table, which is a problem...
        // For now, we'll create a temporary one - this is not ideal
        // In a real implementation, we'd need to pass the table somehow
        thread_local! {
            static TEMP_TABLE: std::cell::RefCell<HcTable<Expr>> = std::cell::RefCell::new(HcTable::new());
        }
        
        let expr = Expr::deserialize(deserializer)?;
        
        TEMP_TABLE.with(|table| {
            Ok(table.borrow_mut().hashcons(expr))
        })
    }
}

// Now we need to tell serde to use our custom module for Hc<Expr> fields
// We'll need to update the Expr enum to use this

pub struct ExprHc {
    table: HcTable<Expr>,
}

impl ExprHc {
    pub fn new() -> Self {
        Self {
            table: HcTable::new(),
        }
    }
    pub fn assign(&mut self, var: String, expr: Hc<Expr>) -> Hc<Expr> {
        self.table.hashcons(Expr::Assign(var, expr))
    }

    pub fn equal(&mut self, left: Hc<Expr>, right: Hc<Expr>) -> Hc<Expr> {
        // If both are constants, return 1 or 0
        if let Expr::Number(n1) = left.as_ref() {
            if let Expr::Number(n2) = right.as_ref() {
                return self.number(if n1 == n2 { 1 } else { 0 });
            }
        }
        self.table.hashcons(Expr::Equal(left, right))
    }

    pub fn add(&mut self, left: Hc<Expr>, right: Hc<Expr>) -> Hc<Expr> {
        // If both are constants, return the sum
        if let Expr::Number(n1) = left.as_ref() {
            if let Expr::Number(n2) = right.as_ref() {
                return self.number(n1 + n2);
            }
        }
        self.table.hashcons(Expr::Add(left, right))
    }

    pub fn subtract(&mut self, left: Hc<Expr>, right: Hc<Expr>) -> Hc<Expr> {
        // If both are constants, return the difference
        if let Expr::Number(n1) = left.as_ref() {
            if let Expr::Number(n2) = right.as_ref() {
                return self.number(n1 - n2);
            }
        }
        self.table.hashcons(Expr::Subtract(left, right))
    }

    pub fn not(&mut self, expr: Hc<Expr>) -> Hc<Expr> {
        // If expr is a constant, return 1 or 0
        if let Expr::Number(n) = expr.as_ref() {
            return self.number(if *n == 0 { 1 } else { 0 });
        }
        self.table.hashcons(Expr::Not(expr))
    }

    pub fn and(&mut self, left: Hc<Expr>, right: Hc<Expr>) -> Hc<Expr> {
        // Short-circuit: if left is 0, return 0 without evaluating right
        if let Expr::Number(n) = left.as_ref() {
            if *n == 0 {
                return self.number(0);
            }
            // If left is non-zero constant, return right
            // This preserves any effects in right
            return right;
        }

        self.table.hashcons(Expr::And(left, right))
    }

    pub fn or(&mut self, left: Hc<Expr>, right: Hc<Expr>) -> Hc<Expr> {
        // Short-circuit: if left is non-zero, return 1 without evaluating right
        if let Expr::Number(n) = left.as_ref() {
            if *n != 0 {
                return self.number(1);
            }
            // If left is 0, return right
            // This preserves any effects in right
            return right;
        }

        self.table.hashcons(Expr::Or(left, right))
    }

    pub fn sequence(&mut self, first: Hc<Expr>, second: Hc<Expr>) -> Hc<Expr> {
        // If first is a constant, return second
        if let Expr::Number(_) = first.as_ref() {
            return second;
        }
        self.table.hashcons(Expr::Sequence(first, second))
    }

    pub fn if_expr(
        &mut self,
        cond: Hc<Expr>,
        then_branch: Hc<Expr>,
        else_branch: Hc<Expr>,
    ) -> Hc<Expr> {
        // If cond is a constant, return then_branch or else_branch
        if let Expr::Number(_) = cond.as_ref() {
            if cond == self.number(0) {
                return else_branch;
            } else {
                return then_branch;
            }
        }
        self.table
            .hashcons(Expr::If(cond, then_branch, else_branch))
    }

    pub fn while_expr(&mut self, cond: Hc<Expr>, body: Hc<Expr>) -> Hc<Expr> {
        // If cond is a 0 constant, return 0
        if let Expr::Number(_) = cond.as_ref() {
            if cond == self.number(0) {
                return self.number(0);
            }
        }
        self.table.hashcons(Expr::While(cond, body))
    }

    pub fn yield_expr(&mut self) -> Hc<Expr> {
        self.table.hashcons(Expr::Yield)
    }

    pub fn exit(&mut self) -> Hc<Expr> {
        self.table.hashcons(Expr::Exit)
    }

    pub fn unknown(&mut self) -> Hc<Expr> {
        self.table.hashcons(Expr::Unknown)
    }

    pub fn number(&mut self, n: i64) -> Hc<Expr> {
        self.table.hashcons(Expr::Number(n))
    }

    pub fn variable(&mut self, var: String) -> Hc<Expr> {
        self.table.hashcons(Expr::Variable(var))
    }
}

#[derive(Debug)]
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Identifier(String),
    Number(i64),
    Assign,    // :=
    Equal,     // ==
    Plus,      // +
    Minus,     // -
    Semicolon, // ;
    If,        // if
    Else,      // else
    While,     // while
    Yield,     // yield
    Exit,      // exit
    Question,  // ?
    Request,   // request
    Not,       // !
    And,       // &&
    Or,        // ||
    LParen,    // (
    RParen,    // )
    LBrace,    // {
    RBrace,    // }
    Eof,
}

/// Parse a string directly into an expression
pub fn parse(source: &str, table: &mut ExprHc) -> Result<Hc<Expr>, String> {
    let tokens = tokenize(source)?;
    let mut parser = Parser::new(tokens);
    parser.parse(table)
}

/// Parse a string into a program containing multiple requests
pub fn parse_program(source: &str, table: &mut ExprHc) -> Result<Program, String> {
    let tokens = tokenize(source)?;
    let mut parser = Parser::new(tokens);
    parser.parse_program(table)
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0 }
    }

    pub fn parse(&mut self, table: &mut ExprHc) -> Result<Hc<Expr>, String> {
        let expr = self.expression(table)?;

        if self.current < self.tokens.len() && self.tokens[self.current] != Token::Eof {
            return Err(format!(
                "Unexpected token after expression: {:?}",
                self.tokens[self.current]
            ));
        }

        Ok(expr)
    }

    pub fn parse_program(&mut self, table: &mut ExprHc) -> Result<Program, String> {
        let mut requests = Vec::new();

        while !self.is_at_end() {
            if self.check(&Token::Request) {
                let request = self.parse_request(table)?;
                requests.push(request);
            } else if self.is_at_end() {
                break;
            } else {
                return Err(format!(
                    "Expected 'request' keyword, found {:?}",
                    self.tokens[self.current]
                ));
            }
        }

        if requests.is_empty() {
            return Err("No requests found in program".to_string());
        }

        Ok(Program { requests })
    }

    fn parse_request(&mut self, table: &mut ExprHc) -> Result<Request, String> {
        self.consume(Token::Request, "Expected 'request' keyword")?;

        let name = match self.advance() {
            Some(Token::Identifier(name)) => name.clone(),
            _ => return Err("Expected request name".to_string()),
        };

        self.consume(Token::LBrace, "Expected '{' after request name")?;
        let body = self.expression(table)?;
        self.consume(Token::RBrace, "Expected '}' after request body")?;

        Ok(Request { name, body })
    }

    fn expression(&mut self, table: &mut ExprHc) -> Result<Hc<Expr>, String> {
        self.sequence(table)
    }

    fn sequence(&mut self, table: &mut ExprHc) -> Result<Hc<Expr>, String> {
        let expr = self.assignment(table)?;

        if self.match_token(&[Token::Semicolon]) {
            let right = self.expression(table)?;
            return Ok(table.sequence(expr, right));
        }

        Ok(expr)
    }

    fn assignment(&mut self, table: &mut ExprHc) -> Result<Hc<Expr>, String> {
        if let Some(Token::Identifier(name)) = self.peek() {
            let name = name.clone();
            if self.peek_next() == Some(&Token::Assign) {
                self.advance(); // consume the identifier
                self.advance(); // consume the :=
                let value = self.assignment(table)?;
                return Ok(table.assign(name, value));
            }
        }

        self.logical_or(table)
    }

    fn logical_or(&mut self, table: &mut ExprHc) -> Result<Hc<Expr>, String> {
        let mut expr = self.logical_and(table)?;

        while self.match_token(&[Token::Or]) {
            let right = self.logical_and(table)?;
            expr = table.or(expr, right);
        }

        Ok(expr)
    }

    fn logical_and(&mut self, table: &mut ExprHc) -> Result<Hc<Expr>, String> {
        let mut expr = self.equality(table)?;

        while self.match_token(&[Token::And]) {
            let right = self.equality(table)?;
            expr = table.and(expr, right);
        }

        Ok(expr)
    }

    fn equality(&mut self, table: &mut ExprHc) -> Result<Hc<Expr>, String> {
        let mut expr = self.term(table)?;

        if self.match_token(&[Token::Equal]) {
            let right = self.term(table)?;
            expr = table.equal(expr, right);
        }

        Ok(expr)
    }

    fn term(&mut self, table: &mut ExprHc) -> Result<Hc<Expr>, String> {
        let mut expr = self.unary(table)?;

        loop {
            if self.match_token(&[Token::Plus]) {
                let right = self.unary(table)?;
                expr = table.add(expr, right);
            } else if self.match_token(&[Token::Minus]) {
                let right = self.unary(table)?;
                expr = table.subtract(expr, right);
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn unary(&mut self, table: &mut ExprHc) -> Result<Hc<Expr>, String> {
        if self.match_token(&[Token::Not]) {
            let expr = self.unary(table)?;
            return Ok(table.not(expr));
        }

        self.primary(table)
    }

    fn primary(&mut self, table: &mut ExprHc) -> Result<Hc<Expr>, String> {
        let token = self.advance();

        match token {
            Some(Token::Number(n)) => Ok(table.number(*n)),
            Some(Token::Identifier(name)) => Ok(table.variable(name.clone())),
            Some(Token::Question) => Ok(table.unknown()),
            Some(Token::Yield) => Ok(table.yield_expr()),
            Some(Token::Exit) => Ok(table.exit()),
            Some(Token::If) => {
                self.consume(Token::LParen, "Expected '(' after 'if'")?;
                let condition = self.expression(table)?;
                self.consume(Token::RParen, "Expected ')' after condition")?;
                self.consume(Token::LBrace, "Expected '{' after condition")?;
                let then_branch = self.expression(table)?;
                self.consume(Token::RBrace, "Expected '}' after then branch")?;
                self.consume(Token::Else, "Expected 'else' after then branch")?;
                self.consume(Token::LBrace, "Expected '{' after 'else'")?;
                let else_branch = self.expression(table)?;
                self.consume(Token::RBrace, "Expected '}' after else branch")?;

                Ok(table.if_expr(condition, then_branch, else_branch))
            }
            Some(Token::While) => {
                self.consume(Token::LParen, "Expected '(' after 'while'")?;
                let condition = self.expression(table)?;
                self.consume(Token::RParen, "Expected ')' after condition")?;
                self.consume(Token::LBrace, "Expected '{' after condition")?;
                let body = self.expression(table)?;
                self.consume(Token::RBrace, "Expected '}' after body")?;

                Ok(table.while_expr(condition, body))
            }
            Some(Token::LParen) => {
                let expr = self.expression(table)?;
                self.consume(Token::RParen, "Expected ')' after expression")?;
                Ok(expr)
            }
            _ => Err(format!("Unexpected token: {:?}", token)),
        }
    }

    fn match_token(&mut self, types: &[Token]) -> bool {
        for t in types {
            if self.check(t) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn check(&self, token_type: &Token) -> bool {
        if self.is_at_end() {
            return false;
        }
        &self.tokens[self.current] == token_type
    }

    fn advance(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || self.tokens[self.current] == Token::Eof
    }

    fn previous(&self) -> Option<&Token> {
        if self.current > 0 {
            Some(&self.tokens[self.current - 1])
        } else {
            None
        }
    }

    fn consume(&mut self, token_type: Token, message: &str) -> Result<&Token, String> {
        if self.check(&token_type) {
            Ok(self.advance().unwrap())
        } else {
            Err(message.to_string())
        }
    }

    fn peek(&self) -> Option<&Token> {
        if self.is_at_end() {
            None
        } else {
            Some(&self.tokens[self.current])
        }
    }

    fn peek_next(&self) -> Option<&Token> {
        if self.current + 1 >= self.tokens.len() {
            None
        } else {
            Some(&self.tokens[self.current + 1])
        }
    }
}

// Lexer implementation
pub fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }
            '/' => {
                chars.next(); // consume the first '/'
                if let Some(&'/') = chars.peek() {
                    // This is a comment, consume the second '/'
                    chars.next();
                    // Consume all characters until the end of the line
                    while let Some(&c) = chars.peek() {
                        if c == '\n' {
                            break;
                        }
                        chars.next();
                    }
                } else {
                    return Err("Unexpected character: /".to_string());
                }
            }
            '0'..='9' => {
                let mut number = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() {
                        number.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Number(number.parse().unwrap()));
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut identifier = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        identifier.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }

                match identifier.as_str() {
                    "if" => tokens.push(Token::If),
                    "else" => tokens.push(Token::Else),
                    "while" => tokens.push(Token::While),
                    "yield" => tokens.push(Token::Yield),
                    "exit" => tokens.push(Token::Exit),
                    "request" => tokens.push(Token::Request),
                    _ => tokens.push(Token::Identifier(identifier)),
                }
            }
            ':' => {
                chars.next();
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::Assign);
                } else {
                    return Err("Expected '=' after ':'".to_string());
                }
            }
            '=' => {
                chars.next();
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::Equal);
                } else {
                    return Err("Expected '=' after '='".to_string());
                }
            }
            '+' => {
                chars.next();
                tokens.push(Token::Plus);
            }
            '-' => {
                chars.next();
                tokens.push(Token::Minus);
            }
            '!' => {
                chars.next();
                tokens.push(Token::Not);
            }
            '&' => {
                chars.next();
                if let Some(&'&') = chars.peek() {
                    chars.next();
                    tokens.push(Token::And);
                } else {
                    return Err("Expected '&' after '&'".to_string());
                }
            }
            '|' => {
                chars.next();
                if let Some(&'|') = chars.peek() {
                    chars.next();
                    tokens.push(Token::Or);
                } else {
                    return Err("Expected '|' after '|'".to_string());
                }
            }
            ';' => {
                chars.next();
                tokens.push(Token::Semicolon);
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            '{' => {
                chars.next();
                tokens.push(Token::LBrace);
            }
            '}' => {
                chars.next();
                tokens.push(Token::RBrace);
            }
            '?' => {
                chars.next();
                tokens.push(Token::Question);
            }
            _ => {
                return Err(format!("Unexpected character: {}", c));
            }
        }
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tokenizer tests
    #[test]
    fn test_tokenize_assignment() {
        let tokens = tokenize("x := 42").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("x".to_string()),
                Token::Assign,
                Token::Number(42),
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_tokenize_equality() {
        let tokens = tokenize("x == y").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("x".to_string()),
                Token::Equal,
                Token::Identifier("y".to_string()),
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_tokenize_add() {
        let tokens = tokenize("x + y").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("x".to_string()),
                Token::Plus,
                Token::Identifier("y".to_string()),
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_tokenize_subtract() {
        let tokens = tokenize("x - y").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("x".to_string()),
                Token::Minus,
                Token::Identifier("y".to_string()),
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_tokenize_not() {
        let tokens = tokenize("!x").unwrap();
        assert_eq!(
            tokens,
            vec![Token::Not, Token::Identifier("x".to_string()), Token::Eof]
        );
    }

    #[test]
    fn test_tokenize_and() {
        let tokens = tokenize("x && y").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("x".to_string()),
                Token::And,
                Token::Identifier("y".to_string()),
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_tokenize_or() {
        let tokens = tokenize("x || y").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("x".to_string()),
                Token::Or,
                Token::Identifier("y".to_string()),
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_tokenize_with_comments() {
        let source = "x := 10; // This is a comment\ny := 20; // Another comment";
        let tokens = tokenize(source).unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("x".to_string()),
                Token::Assign,
                Token::Number(10),
                Token::Semicolon,
                Token::Identifier("y".to_string()),
                Token::Assign,
                Token::Number(20),
                Token::Semicolon,
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_tokenize_comment_at_end() {
        let source = "x := 10; // This is a comment at the end";
        let tokens = tokenize(source).unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("x".to_string()),
                Token::Assign,
                Token::Number(10),
                Token::Semicolon,
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_tokenize_line_with_only_comment() {
        let source = "// This line has only a comment\nx := 10;";
        let tokens = tokenize(source).unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("x".to_string()),
                Token::Assign,
                Token::Number(10),
                Token::Semicolon,
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_tokenize_sequence() {
        let tokens = tokenize("x := 1; y := 2").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("x".to_string()),
                Token::Assign,
                Token::Number(1),
                Token::Semicolon,
                Token::Identifier("y".to_string()),
                Token::Assign,
                Token::Number(2),
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_tokenize_if_else() {
        let tokens = tokenize("if(x == 1){y := 2}else{z := 3}").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::If,
                Token::LParen,
                Token::Identifier("x".to_string()),
                Token::Equal,
                Token::Number(1),
                Token::RParen,
                Token::LBrace,
                Token::Identifier("y".to_string()),
                Token::Assign,
                Token::Number(2),
                Token::RBrace,
                Token::Else,
                Token::LBrace,
                Token::Identifier("z".to_string()),
                Token::Assign,
                Token::Number(3),
                Token::RBrace,
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_tokenize_while() {
        let tokens = tokenize("while(x == 0){x := x}").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::While,
                Token::LParen,
                Token::Identifier("x".to_string()),
                Token::Equal,
                Token::Number(0),
                Token::RParen,
                Token::LBrace,
                Token::Identifier("x".to_string()),
                Token::Assign,
                Token::Identifier("x".to_string()),
                Token::RBrace,
                Token::Eof
            ]
        );
    }

    #[test]
    fn test_tokenize_unknown() {
        let tokens = tokenize("?").unwrap();
        assert_eq!(tokens, vec![Token::Question, Token::Eof]);
    }

    #[test]
    fn test_tokenize_number() {
        let tokens = tokenize("42").unwrap();
        assert_eq!(tokens, vec![Token::Number(42), Token::Eof]);
    }

    #[test]
    fn test_tokenize_variable() {
        let tokens = tokenize("variable").unwrap();
        assert_eq!(
            tokens,
            vec![Token::Identifier("variable".to_string()), Token::Eof]
        );
    }

    #[test]
    fn test_tokenize_error_incomplete_assign() {
        let result = tokenize("x :");
        assert!(result.is_err());
    }

    #[test]
    fn test_tokenize_error_incomplete_equal() {
        let result = tokenize("x =");
        assert!(result.is_err());
    }

    // Parser tests
    #[test]
    fn test_parse_assignment() {
        let mut table = ExprHc::new();
        let expr = parse("x := 42", &mut table).unwrap();
        let num = table.number(42);
        let expected = table.assign("x".to_string(), num);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_equality() {
        let mut table = ExprHc::new();
        let expr = parse("x == y", &mut table).unwrap();
        let x_var = table.variable("x".to_string());
        let y_var = table.variable("y".to_string());
        let expected = table.equal(x_var, y_var);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_sequence() {
        let mut table = ExprHc::new();
        let expr = parse("x := 1; y := 2", &mut table).unwrap();
        let one = table.number(1);
        let two = table.number(2);
        let assign_x = table.assign("x".to_string(), one);
        let assign_y = table.assign("y".to_string(), two);
        let expected = table.sequence(assign_x, assign_y);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_if_else() {
        let mut table = ExprHc::new();
        let expr = parse("if(x == 1){y := 2}else{z := 3}", &mut table).unwrap();
        let x_var = table.variable("x".to_string());
        let one = table.number(1);
        let two = table.number(2);
        let three = table.number(3);
        let condition = table.equal(x_var, one);
        let then_branch = table.assign("y".to_string(), two);
        let else_branch = table.assign("z".to_string(), three);
        let expected = table.if_expr(condition, then_branch, else_branch);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_while() {
        let mut table = ExprHc::new();
        let expr = parse("while(x == 0){x := x}", &mut table).unwrap();
        let x_var = table.variable("x".to_string());
        let zero = table.number(0);
        let condition = table.equal(x_var, zero);
        let x_var2 = table.variable("x".to_string()); // Need a new instance since each is a different node
        let body = table.assign("x".to_string(), x_var2);
        let expected = table.while_expr(condition, body);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_unknown() {
        let mut table = ExprHc::new();
        let expr = parse("?", &mut table).unwrap();
        let expected = table.unknown();
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_yield() {
        let mut table = ExprHc::new();
        let expr = parse("yield", &mut table).unwrap();
        let expected = table.yield_expr();
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_exit() {
        let mut table = ExprHc::new();
        let expr = parse("exit", &mut table).unwrap();
        let expected = table.exit();
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_number() {
        let mut table = ExprHc::new();
        let expr = parse("42", &mut table).unwrap();
        let expected = table.number(42);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_variable() {
        let mut table = ExprHc::new();
        let expr = parse("variable", &mut table).unwrap();
        let expected = table.variable("variable".to_string());
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_add() {
        let mut table = ExprHc::new();
        let expr = parse("5 + 3", &mut table).unwrap();
        let five = table.number(5);
        let three = table.number(3);
        let expected = table.add(five, three);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_add_constant_folding() {
        let mut table = ExprHc::new();
        let expr = parse("5 + 3", &mut table).unwrap();
        let expected = table.number(8);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_subtract() {
        let mut table = ExprHc::new();
        let expr = parse("10 - 4", &mut table).unwrap();
        let ten = table.number(10);
        let four = table.number(4);
        let expected = table.subtract(ten, four);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_subtract_constant_folding() {
        let mut table = ExprHc::new();
        let expr = parse("10 - 4", &mut table).unwrap();
        let expected = table.number(6);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_mixed_arithmetic() {
        let mut table = ExprHc::new();
        let expr = parse("x + 3 - y", &mut table).unwrap();
        let x = table.variable("x".to_string());
        let three = table.number(3);
        let y = table.variable("y".to_string());
        let add = table.add(x, three);
        let expected = table.subtract(add, y);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_not() {
        let mut table = ExprHc::new();
        let expr = parse("!x", &mut table).unwrap();
        let x = table.variable("x".to_string());
        let expected = table.not(x);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_and() {
        let mut table = ExprHc::new();
        let expr = parse("x && y", &mut table).unwrap();
        let x = table.variable("x".to_string());
        let y = table.variable("y".to_string());
        let expected = table.and(x, y);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_or() {
        let mut table = ExprHc::new();
        let expr = parse("x || y", &mut table).unwrap();
        let x = table.variable("x".to_string());
        let y = table.variable("y".to_string());
        let expected = table.or(x, y);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_complex_boolean() {
        let mut table = ExprHc::new();
        let expr = parse("x && (y || !z)", &mut table).unwrap();
        let x = table.variable("x".to_string());
        let y = table.variable("y".to_string());
        let z = table.variable("z".to_string());

        let not_z = table.not(z);
        let y_or_not_z = table.or(y, not_z);
        let expected = table.and(x, y_or_not_z);

        assert_eq!(expr, expected);
    }

    #[test]
    fn test_not_constant_folding() {
        let mut table = ExprHc::new();
        let expr = parse("!0", &mut table).unwrap();
        let expected = table.number(1);
        assert_eq!(expr, expected);

        let expr = parse("!1", &mut table).unwrap();
        let expected = table.number(0);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_and_constant_folding() {
        let mut table = ExprHc::new();
        let expr = parse("0 && 1", &mut table).unwrap();
        let expected = table.number(0);
        assert_eq!(expr, expected);

        let expr = parse("1 && 1", &mut table).unwrap();
        let expected = table.number(1);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_or_constant_folding() {
        let mut table = ExprHc::new();
        let expr = parse("0 || 0", &mut table).unwrap();
        let expected = table.number(0);
        assert_eq!(expr, expected);

        let expr = parse("1 || 0", &mut table).unwrap();
        let expected = table.number(1);
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_nested_expressions() {
        let mut table = ExprHc::new();
        let expr = parse(
            "if(x == 1){if(y == 2){z := 3}else{z := 4}}else{z := 5}",
            &mut table,
        )
        .unwrap();

        let x_var = table.variable("x".to_string());
        let y_var = table.variable("y".to_string());
        let one = table.number(1);
        let two = table.number(2);
        let three = table.number(3);
        let four = table.number(4);
        let five = table.number(5);

        let inner_cond = table.equal(y_var, two);
        let inner_then = table.assign("z".to_string(), three);
        let inner_else = table.assign("z".to_string(), four);
        let inner_if = table.if_expr(inner_cond, inner_then, inner_else);

        let outer_cond = table.equal(x_var, one);
        let outer_else = table.assign("z".to_string(), five);
        let expected = table.if_expr(outer_cond, inner_if, outer_else);

        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_complex_sequence() {
        let mut table = ExprHc::new();
        let expr = parse("x := 1; y := 2; z := 3", &mut table).unwrap();

        let one = table.number(1);
        let two = table.number(2);
        let three = table.number(3);

        let assign_x = table.assign("x".to_string(), one);
        let assign_y = table.assign("y".to_string(), two);
        let assign_z = table.assign("z".to_string(), three);

        let seq_y_z = table.sequence(assign_y, assign_z);
        let expected = table.sequence(assign_x, seq_y_z);

        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_nested_while() {
        let mut table = ExprHc::new();
        let expr = parse("while(x == 0){while(y == 0){y := 1}; x := 1}", &mut table).unwrap();

        let x_var = table.variable("x".to_string());
        let y_var = table.variable("y".to_string());
        let zero = table.number(0);
        let one = table.number(1);

        let inner_cond = table.equal(y_var.clone(), zero.clone());
        let inner_body = table.assign("y".to_string(), one.clone());
        let inner_while = table.while_expr(inner_cond, inner_body);

        let assign_x = table.assign("x".to_string(), one);
        let body_seq = table.sequence(inner_while, assign_x);

        let outer_cond = table.equal(x_var.clone(), zero);
        let expected = table.while_expr(outer_cond, body_seq);

        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_complex_assignment() {
        let mut table = ExprHc::new();
        let expr = parse("x := y := 42", &mut table).unwrap();

        let num = table.number(42);
        let assign_y = table.assign("y".to_string(), num);
        let expected = table.assign("x".to_string(), assign_y);

        assert_eq!(expr, expected);
    }

    #[test]
    fn test_parse_error_missing_closing_paren() {
        let mut table = ExprHc::new();
        let result = parse("if(x == 1{y := 2}else{z := 3}", &mut table);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_missing_closing_brace() {
        let mut table = ExprHc::new();
        let result = parse("if(x == 1){y := 2 else{z := 3}", &mut table);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_missing_else() {
        let mut table = ExprHc::new();
        let result = parse("if(x == 1){y := 2}{z := 3}", &mut table);
        assert!(result.is_err());
    }

    #[test]
    fn test_tokens_to_string() {
        // Test that tokens can be converted back to strings correctly via Display
        let mut table = ExprHc::new();
        let source = "if(x == 1){y := 2}else{z := 3}";
        let expr = parse(source, &mut table).unwrap();
        assert_eq!(expr.to_string(), source);
    }

    #[test]
    fn test_display_nested_expressions() {
        let mut table = ExprHc::new();
        let source = "if(x == 1){if(y == 2){z := 3}else{z := 4}}else{z := 5}";
        let expr = parse(source, &mut table).unwrap();
        assert_eq!(expr.to_string(), source);
    }

    #[test]
    fn test_roundtrip_parsing() {
        // Test that expressions can be formatted back to strings and reparsed
        let mut table = ExprHc::new();
        let original_source = "while(x == 0){y := 1; z := 2}";
        let expr = parse(original_source, &mut table).unwrap();

        let regenerated_source = expr.to_string();
        let expr2 = parse(&regenerated_source, &mut table).unwrap();

        assert_eq!(expr, expr2);
    }
    
    #[test]
    fn test_expr_serialization() {
        let mut table = ExprHc::new();
        
        // Test simple number
        let num_expr = table.number(42);
        let json = serde_json::to_string(&*num_expr).unwrap();
        println!("Number expr JSON: {}", json);
        let deserialized: Expr = serde_json::from_str(&json).unwrap();
        assert_eq!(*num_expr, deserialized);
        
        // Test variable
        let var_expr = table.variable("x".to_string());
        let json = serde_json::to_string(&*var_expr).unwrap();
        println!("Variable expr JSON: {}", json);
        let deserialized: Expr = serde_json::from_str(&json).unwrap();
        assert_eq!(*var_expr, deserialized);
        
        // Test arithmetic expression
        let x = table.variable("x".to_string());
        let y = table.variable("y".to_string());
        let add_expr = table.add(x, y);
        let json = serde_json::to_string(&*add_expr).unwrap();
        println!("Add expr JSON: {}", json);
        let deserialized: Expr = serde_json::from_str(&json).unwrap();
        assert_eq!(*add_expr, deserialized);
        
        // Test complex expression with nesting
        let x = table.variable("x".to_string());
        let zero = table.number(0);
        let one = table.number(1);
        let cond = table.equal(x.clone(), zero);
        let assign = table.assign("y".to_string(), one);
        let if_expr = table.if_expr(cond, assign, x);
        let json = serde_json::to_string(&*if_expr).unwrap();
        println!("If expr JSON: {}", json);
        let deserialized: Expr = serde_json::from_str(&json).unwrap();
        assert_eq!(*if_expr, deserialized);
    }
    
    #[test]
    fn test_program_serialization() {
        let mut table = ExprHc::new();
        
        // Create a simple program
        let x = table.variable("x".to_string());
        let one = table.number(1);
        let body = table.assign("x".to_string(), one);
        
        let program = Program {
            requests: vec![
                Request {
                    name: "foo".to_string(),
                    body: body.clone(),
                },
                Request {
                    name: "bar".to_string(),
                    body: x.clone(),
                },
            ],
        };
        
        let json = serde_json::to_string_pretty(&program).unwrap();
        println!("Program JSON:\n{}", json);
        
        let deserialized: Program = serde_json::from_str(&json).unwrap();
        assert_eq!(program.requests.len(), deserialized.requests.len());
        assert_eq!(program.requests[0].name, deserialized.requests[0].name);
        assert_eq!(*program.requests[0].body, *deserialized.requests[0].body);
        assert_eq!(program.requests[1].name, deserialized.requests[1].name);
        assert_eq!(*program.requests[1].body, *deserialized.requests[1].body);
    }
}
