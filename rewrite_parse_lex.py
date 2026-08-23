with open("src/parse.rs", "r") as f:
    text = f.read()

text = text.replace("use crate::lex::{Lexer, Token};", "use crate::lex::{lex, Token};")

text = text.replace("lexer: Lexer::new(input),", "tokens: lex(input),")
text = text.replace("lexer: Lexer<'a>,", "tokens: Vec<Token>,")

text = text.replace("fn peek(&mut self) -> Option<&Token> {\n        self.lexer.tokens().get(self.pos)\n    }", 
                    "fn peek(&mut self) -> Option<&Token> {\n        self.tokens.get(self.pos)\n    }")

# Fix borrow checker error in advance()
text = text.replace("""fn advance(&mut self) -> Option<&Token> {
        let t = self.peek()?;
        self.pos += 1;
        Some(t)
    }""", 
"""fn advance(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned()?;
        self.pos += 1;
        Some(t)
    }""")

text = text.replace("let t = self.advance().ok_or(())?;", "let t = self.advance().ok_or(())?;")

# In predicate(), we might need to adjust some `let t` usage
text = text.replace("if let Token::Colon = t", "if let Token::Colon = &t")
text = text.replace("if let Token::Eq = t", "if let Token::Eq = &t")
text = text.replace("if let Token::Tilde = t", "if let Token::Tilde = &t")

with open("src/parse.rs", "w") as f:
    f.write(text)
