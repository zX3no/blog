#![feature(iter_advance_by)]
#![allow(unused)]
use std::{fs::File, io::Read, iter::Peekable, slice::Iter};

/*
Tables:

This is probably the hardest thing to convert.

<table>
  <tr>
    <td>Emil</td>
    <td>Tobias</td>
    <td>Linus</td>
  </tr>
  <tr>
    <td>16</td>
    <td>14</td>
    <td>10</td>
  </tr>
</table>

| Emil | Tobias | Linus |
| ---- | ------ | ----- |
|  16  |   14   |   10  |
 */

#[derive(Debug)]
enum Token {
    Hash,
    Italic,
    Bold,
    Strikethrough,
    BackTick,
    CodeBlock,
    BlockQuote,
    OpenBracket,
    CloseBracket,
    OpenParentheses,
    CloseParentheses,
    NewLine,
    CarriageReturn,
    String(String),
    Tab,
}

#[derive(Debug)]
enum Expr {
    ///Level, Text
    Heading(u8, String),
    ///Level, Text
    BlockQuote(u8, String),
    Bold(String),
    Italic(String),
    ///Number, Content
    NumberedList(u16, String),
    List(Vec<Expr>),
    ///Title, Reference/Link
    Link(String, String),
    Text(String),
    Strikethrough(String),
    CodeBlock(Vec<String>),
    Code(String),
    HorizontalRule,
}

//https://alajmovic.com/posts/writing-a-markdown-ish-parser/

//This markdown parser won't support proper markdown.
//No __text__ or _text_ or maybe no **text** or *text*.
//No links with titles [test](http://link.net "title")
//No * or ~ for lists.
fn main() {
    let mut file = File::open("test.md").unwrap();
    let mut string = String::new();
    file.read_to_string(&mut string).unwrap();

    let mut tokens: Vec<Token> = Vec::new();
    let mut iter = string.chars().peekable();

    let mut string = String::new();
    let mut string_changed = false;

    while let Some(char) = iter.next() {
        match char {
            '#' => tokens.push(Token::Hash),
            '>' => tokens.push(Token::BlockQuote),
            '`' => {
                let mut i = iter.clone();
                if let Some('`') = i.next() {
                    if let Some('`') = i.next() {
                        iter.advance_by(2);
                        tokens.push(Token::CodeBlock);
                        continue;
                    }
                }

                tokens.push(Token::BackTick);
            }
            '*' => {
                if iter.peek() == Some(&'*') {
                    iter.next();
                    tokens.push(Token::Bold);
                } else {
                    tokens.push(Token::Italic);
                }
            }
            '\n' => tokens.push(Token::NewLine),
            '\r' => tokens.push(Token::CarriageReturn),
            '\t' => tokens.push(Token::Tab),
            '[' => {
                tokens.push(Token::OpenBracket);
            }
            ']' => tokens.push(Token::CloseBracket),
            '(' => tokens.push(Token::OpenParentheses),
            ')' => tokens.push(Token::CloseParentheses),
            '~' if iter.peek() == Some(&'~') => {
                iter.next();
                tokens.push(Token::Strikethrough);
            }
            ' ' if !string.is_empty() => {
                string.push(char);
                string_changed = true;
            }
            ' ' => (),
            _ => {
                string.push(char);
                string_changed = true;
            }
        }

        if string_changed {
            string_changed = false;
        } else if !string.is_empty() {
            tokens.insert(tokens.len() - 1, Token::String(string));
            string = String::new();
        }
    }

    //Insert the final string.
    if !string.is_empty() {
        tokens.insert(tokens.len(), Token::String(string));
    }

    // dbg!(&tokens);

    let ast = parse(&tokens);
    let mut html = convert(ast);

    println!("{}", html);
    std::fs::write("test.html", html).unwrap();
}

fn parse(tokens: &[Token]) -> Vec<Expr> {
    let mut iter = tokens.iter().peekable();

    let mut ast: Vec<Expr> = Vec::new();

    while let Some(token) = iter.next() {
        if let Some(expr) = expression(token, &mut iter) {
            // dbg!(&expr);
            ast.push(expr);
        }
    }

    ast
}

//TODO: Remove all of the string clones.
fn expression(token: &Token, iter: &mut Peekable<Iter<Token>>) -> Option<Expr> {
    match token {
        Token::OpenBracket => {
            let mut i = iter.clone();

            let mut title = String::new();

            if let Some(Token::String(t)) = i.peek() {
                i.next();
                title = t.clone();
            }

            if let Some(Token::CloseBracket) = i.next() {
                if let Some(Token::OpenParentheses) = i.next() {
                    let mut link = String::new();
                    if let Some(Token::String(l)) = i.peek() {
                        i.next();
                        link = l.clone();
                    }

                    if let Some(Token::CloseParentheses) = i.next() {
                        iter.advance_by(3);

                        if !title.is_empty() {
                            iter.advance_by(1);
                        }

                        if !title.is_empty() {
                            iter.advance_by(3);
                        }

                        return Some(Expr::Link(title, link));
                    }
                }
            }

            return None;
        }
        Token::Hash => {
            let mut level = 1;
            loop {
                match iter.next() {
                    Some(Token::Hash) => {
                        if level > 6 {
                            unreachable!()
                        }
                        level += 1;
                    }
                    Some(Token::String(string)) => {
                        return Some(Expr::Heading(level, string.clone()));
                    }
                    _ => unreachable!(),
                }
            }
        }
        Token::BlockQuote => {
            let mut level = 1;
            loop {
                match iter.next() {
                    Some(Token::BlockQuote) => {
                        level += 1;
                    }
                    Some(Token::String(string)) => {
                        return Some(Expr::BlockQuote(level, string.clone()));
                    }
                    _ => unreachable!("No string after block quote?"),
                }
            }
        }
        Token::String(string) if string.starts_with('-') => {
            if string.len() > 2 && string.chars().all(|c| c == '-') {
                return Some(Expr::HorizontalRule);
            }
        }
        Token::String(string) if string == "!" => {
            let mut i = iter.clone();
            if let Some(Token::OpenBracket) = i.next() {
                if let Some(Token::String(title)) = i.next() {
                    if let Some(Token::CloseBracket) = i.next() {
                        if let Some(Token::OpenParentheses) = i.next() {
                            if let Some(Token::String(link)) = i.next() {
                                if let Some(Token::CloseParentheses) = i.next() {
                                    iter.advance_by(6);
                                    return Some(Expr::Link(title.clone(), link.clone()));
                                }
                            }
                        }
                    }
                }
            }

            return Some(Expr::Text(String::from("!")));
        }
        Token::String(string) => {
            let mut chars = string.chars();
            let mut number = String::new();

            //Match lists:
            //1. First item
            //2. Second item
            //Currently the order doesn't matter and lists are not grouped.
            //Weird orders like 3. 2. 1. are technically valid.
            while let Some(char) = chars.next() {
                if char.is_numeric() {
                    number.push(char);
                } else if char == '.' && !number.is_empty() && chars.next() == Some(' ') {
                    if let Ok(number) = number.parse() {
                        return Some(Expr::NumberedList(number, chars.collect()));
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            return Some(Expr::Text(string.clone()));
        }
        // **This is some bold text**
        Token::Bold => {
            let mut i = iter.clone();
            if let Some(Token::String(string)) = i.next() {
                if let Some(Token::Bold) = i.next() {
                    iter.advance_by(2);
                    return Some(Expr::Bold(string.clone()));
                }
            }
        }
        // *This is some italic text*
        Token::Italic => {
            let mut i = iter.clone();
            if let Some(Token::String(string)) = i.next() {
                if let Some(Token::Bold) = i.next() {
                    iter.advance_by(2);
                    return Some(Expr::Bold(string.clone()));
                }
            }
        }
        Token::Strikethrough => {
            let mut i = iter.clone();
            if let Some(Token::String(string)) = i.next() {
                if let Some(Token::Strikethrough) = i.next() {
                    iter.advance_by(2);
                    return Some(Expr::Strikethrough(string.clone()));
                }
            }
        }
        Token::BackTick => {
            let mut i = iter.clone();
            if let Some(Token::String(string)) = i.next() {
                if let Some(Token::BackTick) = i.next() {
                    iter.advance_by(2);
                    return Some(Expr::Code(string.clone()));
                }
            }
        }
        Token::CodeBlock => {
            let mut content = Vec::new();
            for token in iter.by_ref() {
                match token {
                    Token::CodeBlock => {
                        return Some(Expr::CodeBlock(content));
                    }
                    Token::String(string) => content.push(string.clone()),
                    _ => (),
                }
            }
        }
        _ => (),
    }
    None
}

fn convert(ast: Vec<Expr>) -> String {
    let mut html = String::new();
    for expr in ast {
        match expr {
            Expr::Heading(level, content) => {
                let h = match level {
                    1 => ("<h1>", r"</h1>"),
                    2 => ("<h2>", r"</h2>"),
                    3 => ("<h3>", r"</h3>"),
                    4 => ("<h4>", r"</h4>"),
                    5 => ("<h5>", r"</h5>"),
                    6 => ("<h6>", r"</h6>"),
                    _ => unreachable!(),
                };

                html.push_str(h.0);
                html.push_str(&content);
                html.push_str(h.1);
            }
            Expr::BlockQuote(level, text) => {
                for _ in 0..level {
                    html.push_str("<blockquote>");
                }
                html.push_str(&text);

                for _ in 0..level {
                    html.push_str("</blockquote>");
                }
            }
            Expr::Bold(text) => html.push_str(&format!("<b>{}</b>", text)),
            Expr::Italic(text) => html.push_str(&format!("<i>{}</i>", text)),
            Expr::NumberedList(_, _) => todo!(),
            Expr::List(_) => todo!(),
            Expr::Link(title, link) => {
                html.push_str(&format!("<a href=\"{}\">{}</a>", link, title))
            }
            Expr::Text(text) => html.push_str(&format!("<p>{}</p>", text)),
            Expr::Strikethrough(text) => html.push_str(&format!("<i>{}</i>", text)),
            //Fenced code blocks don't exist in html so this is kina of dumb.
            Expr::CodeBlock(lines) => {
                html.push_str("<code>\n");
                for line in lines {
                    html.push_str(&format!("{}\n", line));
                }
                html.push_str("</code>");
            }
            Expr::Code(code) => html.push_str(&format!("<code>{}</code>", code)),
            Expr::HorizontalRule => html.push_str("<hr>"),
        }
        html.push('\n');
    }
    html
}
