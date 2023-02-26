use std::collections::HashMap;

use crate::token::{Literal, Token};
use crate::error::RuntimeError;

pub struct Environment {
    pub variables: HashMap<String, Literal>,
}

impl Environment {
    pub fn new() -> Self {
        Environment { variables: HashMap::new() }
    }

    pub fn define(&mut self, name: &str, value: Literal) {
        self.variables.insert(name.to_owned(), value);
    }

    pub fn get(&self, name: &Token) -> Result<Literal, RuntimeError> {
        if let Some(variable) = self.variables.get(&name.lexeme) {
            return Ok(variable.clone());
        }

        let message = format!("Undefined variable '{}'", name.lexeme);
        Err(RuntimeError { token: name.clone(), message })
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Type;

    #[test]
    fn test_define() {
        let mut env = Environment::new();
        env.define("a", Literal::Number(1.0));
        env.define("b", Literal::Number(2.0));

        assert_eq!(env.variables.get("a").unwrap(), &Literal::Number(1.0));
        assert_eq!(env.variables.get("b").unwrap(), &Literal::Number(2.0));
    }


    #[test]
    fn test_get() {
        let mut env = Environment::new();
        env.define("a", Literal::Number(1.0));
        env.define("b", Literal::Number(2.0));

        assert_eq!(env.get(&Token::new(Type::Identifier, "a".to_string(), None, 1)).unwrap(), Literal::Number(1.0));
        assert_eq!(env.get(&Token::new(Type::Identifier, "b".to_string(), None, 1)).unwrap(), Literal::Number(2.0));
    }
}
