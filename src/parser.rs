use crate::error::{rloxError, ParseError};
use crate::token::{Token, Type};
use crate::literal::Literal;
use crate::expr::{Expr, BinaryData, UnaryData, GroupingData, VariableData, AssignData, LogicalData, CallData};
use crate::stmt::{Stmt, PrintData, ExpressionData, VarData, WhileData, BlockData, IfData, ReturnData, FunctionData};

type ParseResult<T> = Result<T, ParseError>;

/// Returns if the next token is any of the given types.
macro_rules! matches {
    ( $self:ident, $( $type:expr ),+ ) => {
        {
            if $( $self.check($type) ) ||* {
                $self.advance();
                true
            } else {
                false
            }
        }
    }
}

/// Parses the tokens and returns the resulting expression.
///
/// - Program     -> Decleration* EOF ;
/// - Decleration -> FunDecl | VarDecl | Statement ;
/// - Statement   -> ExprStmt | ForStmt | IfStmt | PrintStmt | ReturnStmt | WhileStmt | Block ;
/// - ForStmt     -> "for" "(" ( Decleration | ExprStmt | ";" ) Expression? ";" Expression? ")" Statement ;
/// - ReturnStmt  -> "return" Expression? ";" ;
/// - WhileStmt   -> "while" "(" Expression ")" Statement ;
/// - IfStmt      -> "if" "(" Expression ")" Statement ( "else" Statement )? ;
/// - Block       -> "{" Decleration* "}" ;
/// - FunDecl     -> "fun" Function ;
/// - Function    -> IDENTIFIER "(" Parameters? ")" Block ;
/// - Parameters  -> IDENTIFIER ( "," IDENTIFIER )* ;
/// - VarDecl     -> "var" IDENTIFIER ( "=" Expression )? ";" ;
/// - ExprStmt    -> Expression ";" ;
/// - PrintStmt   -> "print" Expression ";" ;
/// - Expression  -> Assignment ;
/// - Assignment  -> IDENTIFIER "=" Assignment | LogicOr ;
/// - LogicOr     -> LogicAnd ( "or" LogicAnd )* ;
/// - LogicAnd    -> Equality ( "and" Equality )* ;
/// - Equality    -> Comparison ( ( "!=" | "==" ) Comparison )* ;
/// - Comparison  -> Term ( ( ">" | ">=" | "<" | "<=" ) Term )* ;
/// - Term        -> Factor ( ( "+" | "-" ) Factor )* ;
/// - Factor      -> Unary ( ( "*" | "/" ) Unary )* ;
/// - Unary       -> ( "!" | "-" ) Unary | Primary ;
/// - Arguments   -> Expression ( "," Expression )* ;
/// - Call        -> Primary ( "(" Arguments? ")" )* ;
/// - Primary     -> NUMBER | STRING | false | true | null | "(" Expression ")" | IDENTIFIER ;
pub struct Parser {
    tokens: Vec<Token>,
    current: u32,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current: 0,
        }
    }

    /// Parses the tokens and returns the resulting expression.
    pub fn parse(&mut self) -> Vec<Stmt> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            if let Some(stmt) = self.decleration() {
                statements.push(stmt);
            }
        }

        statements
    }

    /// Returns the next token without consuming it.
    fn peek(&mut self) -> &Token {
        &self.tokens[self.current as usize]
    }

    /// Returns the previous token without consuming it.
    fn previous(&mut self) -> &Token {
        &self.tokens[(self.current - 1) as usize]
    }

    /// Returns if the parser has reached the end of the file.
    fn is_at_end(&mut self) -> bool {
        self.peek().r#type == Type::EOF
    }

    /// Returns if the next token is of the given type.
    fn check(&mut self, r#type: Type) -> bool {
        if self.is_at_end() {
            return false
        }

        self.peek().r#type == r#type
    }

    /// Consumes the next token and returns it.
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }

        self.previous()
    }

    /// Consumes the next token if it is of the given type.
    fn consume(&mut self, r#type: Type, message: &str) -> ParseResult<&Token> {
        if self.check(r#type) {
            return Ok(self.advance());
        }

        Err(ParseError {
            token: self.previous().clone(),
            message: message.to_string(),
        }) 
    }

    /// Parses a decleration.
    fn decleration(&mut self) -> Option<Stmt> {
        let statement = if matches!(self, Type::Fun) {
            self.function("function")
        } else if matches!(self, Type::Var) {
            self.var_decleration()
        } else {
            self.statement()
        };

        match statement {
            Ok(stmt) => Some(stmt),
            Err(error) => {
                error.throw();
                self.synchronize();
                None
            }
        }
    }

    /// Parses a variable decleration.
    fn var_decleration(&mut self) -> ParseResult<Stmt> {
        let name = self.consume(Type::Identifier, "Expect variable name")?.clone();

        let mut initializer: Option<Expr> = None;
        if matches!(self, Type::Equal) {
            match self.expression() {
                Ok(expr) => initializer = Some(expr),
                Err(error) => return Err(error),
            };
        }

        self.consume(Type::Semicolon, "Expect ';' after variable decleration")?;
        Ok(Stmt::Var(VarData { name, initializer }))
    }

    /// Parses a while statement.
    fn while_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(Type::LeftParen, "Expect '(' after while.")?;
        let condition = self.expression()?;
        self.consume(Type::RightParen, "Expect ')' after condition.")?;
        let body = self.statement()?;

        Ok(Stmt::While(WhileData {
            condition,
            body: Box::new(body),
        }))
    }

    /// Parses an expression.
    fn expression(&mut self) -> ParseResult<Expr> {
        self.assignment()
    }

    /// Parses a statement.
    fn statement(&mut self) -> ParseResult<Stmt> {
        if matches!(self, Type::For) {
            return self.for_statement();
        }

        if matches!(self, Type::If) {
            return self.if_statement();
        }

        if matches!(self, Type::Print) {
            return self.print_statement();
        }

        if matches!(self, Type::Return) {
            return self.return_statement();
        }

        if matches!(self, Type::While) {
            return self.while_statement();
        }

        if matches!(self, Type::LeftBrace) {
            return Ok(Stmt::Block(BlockData { statements: self.block()? }));
        }

        self.expression_statement()
    }

    /// Parses a for statement.
    fn for_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(Type::LeftParen, "Expect '(' after 'for'")?;

        let initializer: Option<Stmt>;
        if matches!(self, Type::Semicolon) {
            initializer = None;
        } else if matches!(self, Type::Var) {
            initializer = Some(self.var_decleration()?);
        } else {
            initializer = Some(self.expression_statement()?);
        }

        let condition = match !self.check(Type::Semicolon) {
            true => Some(self.expression()?),
            false => None,
        };
        self.consume(Type::Semicolon, "Expect ';' after loop condition")?;

        let increment = match !self.check(Type::RightParen) {
            true => Some(self.expression()?),
            false => None,
        };
        self.consume(Type::RightParen, "Expect ')' after loop clauses")?;

        let mut body = self.statement()?;

        // Execute the increment after the body.
        if let Some(increment) = increment {
            body = Stmt::Block(BlockData {
                statements: vec![
                    body,
                    Stmt::Expression(ExpressionData {
                        expr: increment
                    }),
                ],
            });
        }

        // Wrap the body into a while loop.
        // If there is no condition, use true.
        body = Stmt::While(WhileData {
            condition: condition.unwrap_or(Expr::Literal(Literal::Bool(true))),
            body: Box::new(body),
        });

        // Add the initializer before the loop if there is one.
        if let Some(initializer) = initializer {
            body = Stmt::Block(BlockData {
                statements: vec![
                    initializer,
                    body,
                ],
            });
        }

        Ok(body)
    }

    /// Parses an if statement.
    fn if_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(Type::LeftParen, "Expect '(' after 'if'")?;
        let condition = self.expression()?;
        self.consume(Type::RightParen, "Expect ')' after if condition")?;

        let then_branch = Box::new(self.statement()?);
        let mut else_branch: Option<Box<Stmt>> = None;
        if matches!(self, Type::Else) {
            else_branch = Some(Box::new(self.statement()?));
        }

        Ok(Stmt::If(IfData { condition, then_branch, else_branch }))
    }

    /// Parses a print statement.
    fn print_statement(&mut self) -> ParseResult<Stmt> {
        let expr = match self.expression() {
            Ok(expr) => expr,
            Err(error) => return Err(error),
        };

        self.consume(Type::Semicolon, "Expect ';' after value")?;

        Ok(Stmt::Print(PrintData { expr }))
    }

    /// Parses a return statement.
    fn return_statement(&mut self) -> ParseResult<Stmt> {
        let keyword = self.previous().to_owned();

        let value = match self.check(Type::Semicolon) {
            true => None,
            false => Some(self.expression()?),
        };

        self.consume(Type::Semicolon, "Expect ';' after return value")?;
        Ok(Stmt::Return(ReturnData { keyword, value }))
    }

    /// Parses an expression statement.
    fn expression_statement(&mut self) -> ParseResult<Stmt> {
        let expr = match self.expression() {
            Ok(expr) => expr,
            Err(error) => return Err(error),
        };

        self.consume(Type::Semicolon, "Expect ';' after expression")?;

        Ok(Stmt::Expression(ExpressionData { expr }))
    }

    /// Parses a function decleration.
    fn function(&mut self, kind: &str) -> ParseResult<Stmt> {
        let name = self.consume(Type::Identifier, &format!("Expect {kind} name"))?.to_owned();

        self.consume(Type::LeftParen, &format!("Expect '(' after {kind} name"))?;

        let mut params = vec![];

        if !self.check(Type::RightParen) {
            loop {
                if params.len() >= 255 {
                    return Err(ParseError {
                        token: self.peek().to_owned(),
                        message: "Can't have more than 255 parameters".to_string(),
                    });
                }

                params.push(self.consume(Type::Identifier, "Expect parameter name")?.to_owned());

                if !matches!(self, Type::Comma) {
                    break;
                }
            }
        }

        self.consume(Type::RightParen, "Expect ')' after parameters")?;

        self.consume(Type::LeftBrace, &format!("Expect '{{' before {kind} body"))?;

        let body = self.block()?;

        Ok(Stmt::Function(FunctionData { name, params, body }))
    }

    /// Parses a block statement.
    fn block(&mut self) -> ParseResult<Vec<Stmt>> {
        let mut statements = Vec::new();

        while !self.check(Type::RightBrace) && !self.is_at_end() {
            if let Some(stmt) = self.decleration() {
                statements.push(stmt);
            }
        }

        self.consume(Type::RightBrace, "Expect '}' after block")?;

        Ok(statements)
    }

    /// Parses an assignment expression.
    fn assignment(&mut self) -> ParseResult<Expr> {
        let expr = self.or()?;

        if matches!(self, Type::Equal) {
            let equals = self.previous().to_owned();
            let value = self.assignment()?;

            if let Expr::Variable(data) = expr {
                let name = data.name;

                return Ok(Expr::Assign(AssignData { name, value: Box::new(value) }))
            }

            ParseError {
                token: equals,
                message: "Invalid assignment target".to_string()
            }.throw();
        }

        Ok(expr)
    }

    /// Parses an or expression.
    fn or(&mut self) -> ParseResult<Expr> {
        let mut expr = self.and()?;

        while matches!(self, Type::Or) {
            let operator = self.previous().clone();
            let right = self.and()?;
            expr = Expr::Logical(LogicalData {
                left: Box::new(expr),
                operator,
                right: Box::new(right)
            });
        }

        Ok(expr)
    }

    /// Parses and and expression.
    fn and(&mut self) -> ParseResult<Expr> {
        let mut expr = self.equality()?;

        while matches!(self, Type::And) {
            let operator = self.previous().clone();
            let right = self.equality()?;
            expr = Expr::Logical(LogicalData {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            });
        }

        Ok(expr)
    }

    /// Parses an equality expression.
    fn equality(&mut self) -> ParseResult<Expr> {
        let mut expr = match self.comparison() {
            Ok(expr) => expr,
            Err(error) => return Err(error),
        };

        while matches!(self, Type::BangEqual, Type::EqualEqual) {
            let operator = self.previous().clone();
            let right = match self.comparison() {
                Ok(expr) => expr,
                Err(error) => return Err(error),
            };

            expr = Expr::Binary(BinaryData {
                left: Box::new(expr),
                operator,
                right: Box::new(right)
            });
        }

        Ok(expr)
    }

    /// Parses a comparison expression.
    fn comparison(&mut self) -> ParseResult<Expr> {
        let mut expr = match self.term() {
            Ok(expr) => expr,
            Err(error) => return Err(error),
        };

        while matches!(self, Type::Greater, Type::GreaterEqual, Type::Less, Type::LessEqual) {
            let operator = self.previous().clone();
            let right = match self.term() {
                Ok(expr) => expr,
                Err(error) => return Err(error),
            };

            expr = Expr::Binary(BinaryData {
                left: Box::new(expr),
                operator,
                right: Box::new(right)
            });
        }

        Ok(expr)
    }

    /// Parses a term expression.
    fn term(&mut self) -> ParseResult<Expr> {
        let mut expr = match self.factor() {
            Ok(expr) => expr,
            Err(error) => return Err(error),
        };

        while matches!(self, Type::Minus, Type::Plus) {
            let operator = self.previous().clone();
            let right = match self.factor() {
                Ok(expr) => expr,
                Err(error) => return Err(error),
            };

            expr = Expr::Binary(BinaryData {
                left: Box::new(expr),
                operator,
                right: Box::new(right)
            });
        }

        Ok(expr)
    }

    /// Parses a factor expression.
    fn factor(&mut self) -> ParseResult<Expr> {
        let mut expr = match self.unary() {
            Ok(expr) => expr,
            Err(error) => return Err(error),
        };

        while matches!(self, Type::Slash, Type::Star) {
            let operator = self.previous().clone();
            let right = match self.unary() {
                Ok(expr) => expr,
                Err(error) => return Err(error),
            };

            expr = Expr::Binary(BinaryData {
                left: Box::new(expr),
                operator,
                right: Box::new(right)
            });
        }

        Ok(expr)
    }

    /// Parses a unary expression.
    fn unary(&mut self) -> ParseResult<Expr> {
        if matches!(self, Type::Bang, Type::Minus) {
            let operator = self.previous().clone();
            let right = match self.unary() {
                Ok(expr) => expr,
                Err(error) => return Err(error),
            };

            return Ok(Expr::Unary(UnaryData {
                operator,
                expr: Box::new(right)
            }));
        }

        self.call()
    }

    /// Parses a call arguments.
    fn finish_call(&mut self, callee: &Expr) -> ParseResult<Expr> {
        let mut arguments = vec![];

        if !self.check(Type::RightParen) {
            while { 
                if arguments.len() >= 255 {
                    ParseError {
                        token: self.peek().to_owned(),
                        message: "Can't have more than 255 arguments".to_string(),
                    }.throw();
                }

                arguments.push(self.expression()?);
                matches!(self, Type::Comma)
            } {}
        }

        let paren = self.consume(Type::RightParen, "Expect ')' after arguments")?;

        Ok(Expr::Call(CallData {
            callee: Box::new(callee.to_owned()),
            paren: paren.to_owned(),
            arguments,
        }))
    }

    /// Parses a call expression.
    fn call(&mut self) -> ParseResult<Expr> {
        let mut expr = self.primary()?;

        loop {
            if matches!(self, Type::LeftParen) {
                expr = self.finish_call(&expr)?;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Parses a primary expression.
    fn primary(&mut self) -> ParseResult<Expr> {
        if matches!(self, Type::False) {
            return Ok(Expr::Literal(Literal::Bool(false)));
        }

        if matches!(self, Type::True) {
            return Ok(Expr::Literal(Literal::Bool(true)));
        }

        if matches!(self, Type::Null) {
            return Ok(Expr::Literal(Literal::Null));
        }

        if matches!(self, Type::Number, Type::String) {
            return Ok(Expr::Literal(self.previous().clone().literal
                .expect("number or string to have a literal value")));
        }

        if matches!(self, Type::Identifier) {
            return Ok(Expr::Variable(VariableData {
                name: self.previous().clone()
            }))
        }

        if matches!(self, Type::LeftParen) {
            let expr = match self.expression() {
                Ok(expr) => expr,
                Err(error) => return Err(error),
            };

            match self.consume(Type::RightParen, "Expected ')' after expression") {
                Ok(_) => (),
                Err(error) => return Err(error),
            };

            return Ok(Expr::Grouping(GroupingData { expr: Box::new(expr) }));
        }

        Err(ParseError {
            token: self.peek().clone(),
            message: "Expected expression".to_string()
        })
    }

    /// Tries to recover from a parse error.
    fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            if self.previous().r#type == Type::Semicolon {
                return;
            }

            match self.peek().r#type {
                Type::Class => return,
                Type::Fun => return,
                Type::Var => return,
                Type::For => return,
                Type::If => return,
                Type::While => return,
                Type::Print => return,
                Type::Return => return,
                _ => self.advance()
            };
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::token::Type;

    #[test]
    fn test_matches() {
        let mut parser = Parser::new(vec![
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::Plus, "+".to_string(), None, 1),
            Token::new(Type::Number, "456".to_string(), Some(Literal::Number(456.0)), 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        assert!(matches!(parser, Type::Number));
        assert!(matches!(parser, Type::Plus));
        assert!(matches!(parser, Type::Number));
    }

    #[test]
    fn test_or() {
        let mut parser = Parser::new(vec![
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::Or, "or".to_string(), None, 1),
            Token::new(Type::Number, "456".to_string(), Some(Literal::Number(456.0)), 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let expr = parser.expression().unwrap();

        assert_eq!(expr, Expr::Logical(LogicalData {
            left: Box::new(Expr::Literal(Literal::Number(123.0))),
            operator: Token::new(Type::Or, "or".to_string(), None, 1),
            right: Box::new(Expr::Literal(Literal::Number(456.0)))
        }));
    }

    #[test]
    fn test_and() {
        let mut parser = Parser::new(vec![
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::And, "and".to_string(), None, 1),
            Token::new(Type::Number, "456".to_string(), Some(Literal::Number(456.0)), 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let expr = parser.expression().unwrap();

        assert_eq!(expr, Expr::Logical(LogicalData {
            left: Box::new(Expr::Literal(Literal::Number(123.0))),
            operator: Token::new(Type::And, "and".to_string(), None, 1),
            right: Box::new(Expr::Literal(Literal::Number(456.0)))
        }));
    }

    #[test]
    fn test_nested_logic() {
        let mut parser = Parser::new(vec![
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::Or, "or".to_string(), None, 1),
            Token::new(Type::Number, "456".to_string(), Some(Literal::Number(456.0)), 1),
            Token::new(Type::And, "and".to_string(), None, 1),
            Token::new(Type::Number, "789".to_string(), Some(Literal::Number(789.0)), 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let expr = parser.expression().unwrap();

        assert_eq!(expr, Expr::Logical(LogicalData {
            left: Box::new(Expr::Literal(Literal::Number(123.0))),
            operator: Token::new(Type::Or, "or".to_string(), None, 1),
            right: Box::new(Expr::Logical(LogicalData {
                left: Box::new(Expr::Literal(Literal::Number(456.0))),
                operator: Token::new(Type::And, "and".to_string(), None, 1),
                right: Box::new(Expr::Literal(Literal::Number(789.0)))
            }))
        }));
    }

    #[test]
    fn test_binary() {
        let mut parser = Parser::new(vec![
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::Plus, "+".to_string(), None, 1),
            Token::new(Type::Number, "456".to_string(), Some(Literal::Number(456.0)), 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let expr = parser.expression().unwrap();

        assert_eq!(expr, Expr::Binary(BinaryData {
            left: Box::new(Expr::Literal(Literal::Number(123.0))),
            operator: Token::new(Type::Plus, "+".to_string(), None, 1),
            right: Box::new(Expr::Literal(Literal::Number(456.0)))
        }));
    }

    #[test]
    fn test_unary() {
        let mut parser = Parser::new(vec![
            Token::new(Type::Minus, "-".to_string(), None, 1),
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let expr = parser.expression().unwrap();

        assert_eq!(expr, Expr::Unary(UnaryData {
            operator: Token::new(Type::Minus, "-".to_string(), None, 1),
            expr: Box::new(Expr::Literal(Literal::Number(123.0)))
        }));
    }

    #[test]
    fn test_grouping() {
        let mut parser = Parser::new(vec![
            Token::new(Type::LeftParen, "(".to_string(), None, 1),
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::RightParen, ")".to_string(), None, 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let expr = parser.expression().unwrap();

        assert_eq!(expr, Expr::Grouping(GroupingData {
            expr: Box::new(Expr::Literal(Literal::Number(123.0)))
        }));
    }

    #[test]
    fn test_precedence() {
        let mut parser = Parser::new(vec![
            Token::new(Type::Number, "1".to_string(), Some(Literal::Number(1.0)), 1),
            Token::new(Type::Minus, "-".to_string(), None, 1),
            Token::new(Type::Number, "2".to_string(), Some(Literal::Number(2.0)), 1),
            Token::new(Type::Star, "*".to_string(), None, 1),
            Token::new(Type::Number, "3".to_string(), Some(Literal::Number(3.0)), 1),
            Token::new(Type::Plus, "+".to_string(), None, 1),
            Token::new(Type::Number, "4".to_string(), Some(Literal::Number(4.0)), 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let expr = parser.expression().unwrap();

        assert_eq!(expr, Expr::Binary(BinaryData {
            left: Box::new(Expr::Binary(BinaryData {
                left: Box::new(Expr::Literal(Literal::Number(1.0))),
                operator: Token::new(Type::Minus, "-".to_string(), None, 1),
                right: Box::new(Expr::Binary(BinaryData {
                    left: Box::new(Expr::Literal(Literal::Number(2.0))),
                    operator: Token::new(Type::Star, "*".to_string(), None, 1),
                    right: Box::new(Expr::Literal(Literal::Number(3.0)))
                }))
            })),
            operator: Token::new(Type::Plus, "+".to_string(), None, 1),
            right: Box::new(Expr::Literal(Literal::Number(4.0)))
        }));
    }

    #[test]
    fn test_equality() {
        let mut parser = Parser::new(vec![
            Token::new(Type::Number, "1".to_string(), Some(Literal::Number(1.0)), 1),
            Token::new(Type::BangEqual, "!=".to_string(), None, 1),
            Token::new(Type::Number, "2".to_string(), Some(Literal::Number(2.0)), 1),
            Token::new(Type::EqualEqual, "==".to_string(), None, 1),
            Token::new(Type::Number, "3".to_string(), Some(Literal::Number(3.0)), 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let expr = parser.equality().unwrap();

        assert_eq!(expr, Expr::Binary(BinaryData {
            left: Box::new(Expr::Binary(BinaryData {
                left: Box::new(Expr::Literal(Literal::Number(1.0))),
                operator: Token::new(Type::BangEqual, "!=".to_string(), None, 1),
                right: Box::new(Expr::Literal(Literal::Number(2.0)))
            })),
            operator: Token::new(Type::EqualEqual, "==".to_string(), None, 1),
            right: Box::new(Expr::Literal(Literal::Number(3.0)))
        }));
    }

    #[test]
    fn test_comparison() {
        let mut parser = Parser::new(vec![
            Token::new(Type::Number, "1".to_string(), Some(Literal::Number(1.0)), 1),
            Token::new(Type::Greater, ">".to_string(), None, 1),
            Token::new(Type::Number, "2".to_string(), Some(Literal::Number(2.0)), 1),
            Token::new(Type::Less, "<".to_string(), None, 1),
            Token::new(Type::Number, "3".to_string(), Some(Literal::Number(3.0)), 1),
            Token::new(Type::GreaterEqual, ">=".to_string(), None, 1),
            Token::new(Type::Number, "4".to_string(), Some(Literal::Number(4.0)), 1),
            Token::new(Type::LessEqual, "<=".to_string(), None, 1),
            Token::new(Type::Number, "5".to_string(), Some(Literal::Number(5.0)), 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let expr = parser.comparison().unwrap();

        assert_eq!(expr, Expr::Binary(BinaryData {
            left: Box::new(Expr::Binary(BinaryData {
                left: Box::new(Expr::Binary(BinaryData {
                    left: Box::new(Expr::Binary(BinaryData {
                        left: Box::new(Expr::Literal(Literal::Number(1.0))),
                        operator: Token::new(Type::Greater, ">".to_string(), None, 1),
                        right: Box::new(Expr::Literal(Literal::Number(2.0)))
                    })),
                    operator: Token::new(Type::Less, "<".to_string(), None, 1),
                    right: Box::new(Expr::Literal(Literal::Number(3.0)))
                })),
                operator: Token::new(Type::GreaterEqual, ">=".to_string(), None, 1),
                right: Box::new(Expr::Literal(Literal::Number(4.0)))
            })),
            operator: Token::new(Type::LessEqual, "<=".to_string(), None, 1),
            right: Box::new(Expr::Literal(Literal::Number(5.0))),
        }));
    }

    #[test]
    fn test_print_stmt() {
        let mut parser = Parser::new(vec![
            Token::new(Type::Print, "print".to_string(), None, 1),
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::Semicolon, ";".to_string(), None, 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let stmt = parser.statement().unwrap();

        assert_eq!(stmt, Stmt::Print(PrintData {
            expr: Expr::Literal(Literal::Number(123.0))
        }));
    }

    #[test]
    fn test_expression_stmt() {
        let mut parser = Parser::new(vec![
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::Semicolon, ";".to_string(), None, 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let stmt = parser.statement().unwrap();

        assert_eq!(stmt, Stmt::Expression(ExpressionData {
            expr: Expr::Literal(Literal::Number(123.0))
        }));
    }

    #[test]
    fn test_if_stmt() {
        let mut parser = Parser::new(vec![
            Token::new(Type::If, "if".to_string(), None, 1),
            Token::new(Type::LeftParen, "(".to_string(), None, 1),
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::RightParen, ")".to_string(), None, 1),
            Token::new(Type::LeftBrace, "{".to_string(), None, 1),
            Token::new(Type::Print, "print".to_string(), None, 1),
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::Semicolon, ";".to_string(), None, 1),
            Token::new(Type::RightBrace, "}".to_string(), None, 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let stmt = parser.statement().unwrap();

        assert_eq!(stmt, Stmt::If(IfData {
            condition: Expr::Literal(Literal::Number(123.0)),
            then_branch: Box::new(Stmt::Block(BlockData {
                statements: vec![Stmt::Print(PrintData {
                    expr: Expr::Literal(Literal::Number(123.0))
                })],
            })),
            else_branch: None
        }));
    }

    #[test]
    fn test_if_stmt_with_else() {
        let mut parser = Parser::new(vec![
            Token::new(Type::If, "if".to_string(), None, 1),
            Token::new(Type::LeftParen, "(".to_string(), None, 1),
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::RightParen, ")".to_string(), None, 1),
            Token::new(Type::LeftBrace, "{".to_string(), None, 1),
            Token::new(Type::Print, "print".to_string(), None, 1),
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::Semicolon, ";".to_string(), None, 1),
            Token::new(Type::RightBrace, "}".to_string(), None, 1),
            Token::new(Type::Else, "else".to_string(), None, 1),
            Token::new(Type::LeftBrace, "{".to_string(), None, 1),
            Token::new(Type::Print, "print".to_string(), None, 1),
            Token::new(Type::Number, "456".to_string(), Some(Literal::Number(456.0)), 1),
            Token::new(Type::Semicolon, ";".to_string(), None, 1),
            Token::new(Type::RightBrace, "}".to_string(), None, 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let stmt = parser.statement().unwrap();

        assert_eq!(stmt, Stmt::If(IfData {
            condition: Expr::Literal(Literal::Number(123.0)),
            then_branch: Box::new(Stmt::Block(BlockData {
                statements: vec![Stmt::Print(PrintData {
                    expr: Expr::Literal(Literal::Number(123.0))
                })],
            })),
            else_branch: Some(Box::new(Stmt::Block(BlockData {
                statements: vec![Stmt::Print(PrintData {
                    expr: Expr::Literal(Literal::Number(456.0))
                })],
            })))
        }));
    }

    #[test]
    fn test_var_stmt() {
        let mut parser = Parser::new(vec![
            Token::new(Type::Var, "var".to_string(), None, 1),
            Token::new(Type::Identifier, "a".to_string(), None, 1),
            Token::new(Type::Equal, "=".to_string(), None, 1),
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::Semicolon, ";".to_string(), None, 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        parser.advance();
        let stmt = parser.var_decleration().unwrap();

        assert_eq!(stmt, Stmt::Var(VarData {
            name: Token::new(Type::Identifier, "a".to_string(), None, 1),
            initializer: Some(Expr::Literal(Literal::Number(123.0)))
        }));
    }

    #[test]
    fn test_decleration() {
        let mut parser = Parser::new(vec![
            Token::new(Type::Var, "var".to_string(), None, 1),
            Token::new(Type::Identifier, "a".to_string(), None, 1),
            Token::new(Type::Equal, "=".to_string(), None, 1),
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::Semicolon, ";".to_string(), None, 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let stmt = parser.decleration().unwrap();

        assert_eq!(stmt, Stmt::Var(VarData {
            name: Token::new(Type::Identifier, "a".to_string(), None, 1),
            initializer: Some(Expr::Literal(Literal::Number(123.0)))
        }));
    }

    #[test]
    fn test_assignment() {
        let mut parser = Parser::new(vec![
            Token::new(Type::Identifier, "a".to_string(), None, 1),
            Token::new(Type::Equal, "=".to_string(), None, 1),
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::Semicolon, ";".to_string(), None, 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let stmt = parser.assignment().unwrap();

        assert_eq!(
            stmt,
            Expr::Assign(AssignData {
                name: Token::new(Type::Identifier, "a".to_string(), None, 1),
                value: Box::new(Expr::Literal(Literal::Number(123.0)))
            })
        );
    }

    #[test]
    fn test_while_stmt() {
        let mut parser = Parser::new(vec![
            Token::new(Type::While, "while".to_string(), None, 1),
            Token::new(Type::LeftParen, "(".to_string(), None, 1),
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::RightParen, ")".to_string(), None, 1),
            Token::new(Type::LeftBrace, "{".to_string(), None, 1),
            Token::new(Type::Print, "print".to_string(), None, 1),
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::Semicolon, ";".to_string(), None, 1),
            Token::new(Type::RightBrace, "}".to_string(), None, 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        let stmt = parser.statement().unwrap();

        assert_eq!(stmt, Stmt::While(WhileData {
            condition: Expr::Literal(Literal::Number(123.0)),
            body: Box::new(Stmt::Block(BlockData {
                statements: vec![Stmt::Print(PrintData {
                    expr: Expr::Literal(Literal::Number(123.0))
                })],
            }))
        }));
    }

    #[test]
    fn test_block_stmt() {
        let mut parser = Parser::new(vec![
            Token::new(Type::LeftBrace, "{".to_string(), None, 1),
            Token::new(Type::Var, "var".to_string(), None, 1),
            Token::new(Type::Identifier, "a".to_string(), None, 1),
            Token::new(Type::Equal, "=".to_string(), None, 1),
            Token::new(Type::Number, "123".to_string(), Some(Literal::Number(123.0)), 1),
            Token::new(Type::Semicolon, ";".to_string(), None, 1),
            Token::new(Type::RightBrace, "}".to_string(), None, 1),
            Token::new(Type::EOF, "".to_string(), None, 1)
        ]);

        parser.advance();
        let stmt = parser.block().unwrap();

        assert_eq!(
            stmt,
            vec![Stmt::Var(VarData {
                name: Token::new(Type::Identifier, "a".to_string(), None, 1),
                initializer: Some(Expr::Literal(Literal::Number(123.0)))
            })]
        );
    }
}
