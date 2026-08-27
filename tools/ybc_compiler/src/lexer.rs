#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    Int(i64),
    Str(String),
    KwClass, KwExtends, KwFn, KwLet, KwIf, KwElse, KwWhile,
    KwReturn, KwImport, KwAs, KwTrue, KwFalse,
    Plus, Minus, Star, Slash, Lt, Gt, Eq, EqEq, Not,
    LParen, RParen, LBrace, RBrace,
    Comma, Dot, Semi,
    Eof,
}

pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self { src: src.as_bytes(), pos: 0 }
    }

    fn peek(&self) -> u8 {
        if self.pos < self.src.len() { self.src[self.pos] } else { 0 }
    }

    fn advance(&mut self) -> u8 {
        let c = self.peek();
        self.pos += 1;
        c
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while self.peek().is_ascii_whitespace() { self.advance(); }
            if self.peek() == b'/' && self.pos + 1 < self.src.len() && self.src[self.pos + 1] == b'/' {
                while self.peek() != b'\n' && self.peek() != 0 { self.advance(); }
                continue;
            }
            break;
        }
    }

    pub fn tokenize(mut self) -> Vec<Token> {
        let mut toks = Vec::new();
        loop {
            self.skip_ws_and_comments();
            let c = self.peek();
            if c == 0 { toks.push(Token::Eof); break; }

            if c.is_ascii_alphabetic() || c == b'_' {
                let start = self.pos;
                while self.peek().is_ascii_alphanumeric() || self.peek() == b'_' { self.advance(); }
                let word = std::str::from_utf8(&self.src[start..self.pos]).unwrap().to_string();
                toks.push(match word.as_str() {
                    "class" => Token::KwClass,
                    "extends" => Token::KwExtends,
                    "fn" => Token::KwFn,
                    "let" => Token::KwLet,
                    "if" => Token::KwIf,
                    "else" => Token::KwElse,
                    "while" => Token::KwWhile,
                    "return" => Token::KwReturn,
                    "import" => Token::KwImport,
                    "as" => Token::KwAs,
                    "true" => Token::KwTrue,
                    "false" => Token::KwFalse,
                    _ => Token::Ident(word),
                });
                continue;
            }

            if c.is_ascii_digit() {
                let start = self.pos;
                while self.peek().is_ascii_digit() { self.advance(); }
                let num_str = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
                toks.push(Token::Int(num_str.parse().unwrap()));
                continue;
            }

            if c == b'"' {
                self.advance();
                let mut s = String::new();
                while self.peek() != b'"' && self.peek() != 0 {
                    let ch = self.advance();
                    if ch == b'\\' && self.peek() == b'n' { self.advance(); s.push('\n'); }
                    else { s.push(ch as char); }
                }
                self.advance(); // closing quote
                toks.push(Token::Str(s));
                continue;
            }

            self.advance();
            toks.push(match c {
                b'+' => Token::Plus,
                b'-' => Token::Minus,
                b'*' => Token::Star,
                b'/' => Token::Slash,
                b'<' => Token::Lt,
                b'>' => Token::Gt,
                b'=' => {
                    if self.peek() == b'=' { self.advance(); Token::EqEq } else { Token::Eq }
                }
                b'!' => Token::Not,
                b'(' => Token::LParen,
                b')' => Token::RParen,
                b'{' => Token::LBrace,
                b'}' => Token::RBrace,
                b',' => Token::Comma,
                b'.' => Token::Dot,
                b';' => Token::Semi,
                _ => continue,
            });
        }
        toks
    }
}