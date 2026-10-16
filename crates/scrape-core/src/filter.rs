use crate::RoleType;
use regex::Regex;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FilterField {
    Title,
    Location,
    Description,
    Company,
    RoleType,
}

impl FilterField {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "title" => Some(Self::Title),
            "location" | "loc" => Some(Self::Location),
            "description" | "desc" => Some(Self::Description),
            "company" => Some(Self::Company),
            "role_type" | "role" | "roletype" => Some(Self::RoleType),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Like,
    NotLike,
    Contains,
    NotContains,
    Equals,
    NotEquals,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum FilterExpr {
    FieldMatch {
        field: FilterField,
        op: FilterOp,
        value: String,
    },
    FieldIn {
        field: FilterField,
        values: Vec<String>,
        negated: bool,
    },
    TextMatch {
        pattern: String,
        negated: bool,
    },
    And(Vec<FilterExpr>),
    Or(Vec<FilterExpr>),
    Not(Box<FilterExpr>),
    AlwaysTrue,
    AlwaysFalse,
}

pub struct JobFilterContext<'a> {
    pub title: &'a str,
    pub location: &'a str,
    pub description: &'a str,
    pub company: &'a str,
    pub role_type: RoleType,
}

impl<'a> JobFilterContext<'a> {
    pub fn get_field_str(&self, field: FilterField) -> String {
        match field {
            FilterField::Title => self.title.to_string(),
            FilterField::Location => self.location.to_string(),
            FilterField::Description => self.description.to_string(),
            FilterField::Company => self.company.to_string(),
            FilterField::RoleType => match self.role_type {
                RoleType::FullTime => "full_time".to_string(),
                RoleType::PartTime => "part_time".to_string(),
                RoleType::Internship => "internship".to_string(),
                RoleType::Contract => "contract".to_string(),
                RoleType::Seasonal => "seasonal".to_string(),
                RoleType::Unknown => "unknown".to_string(),
            },
        }
    }
}

/// Matches a string against a SQL LIKE pattern (% = any sequence, _ = any single char)
/// Defaults to case-insensitive matching.
pub fn sql_like_matches(haystack: &str, pattern: &str) -> bool {
    let h_lower = haystack.to_lowercase();
    let p_lower = pattern.to_lowercase();

    // Fast paths
    if !p_lower.contains('%') && !p_lower.contains('_') {
        return h_lower == p_lower;
    }

    if p_lower.starts_with('%') && p_lower.ends_with('%') && p_lower.len() >= 2 {
        let inner = &p_lower[1..p_lower.len() - 1];
        if !inner.contains('%') && !inner.contains('_') {
            return h_lower.contains(inner);
        }
    }

    if p_lower.ends_with('%') && !p_lower.starts_with('%') {
        let prefix = &p_lower[..p_lower.len() - 1];
        if !prefix.contains('%') && !prefix.contains('_') {
            return h_lower.starts_with(prefix);
        }
    }

    if p_lower.starts_with('%') && !p_lower.ends_with('%') {
        let suffix = &p_lower[1..];
        if !suffix.contains('%') && !suffix.contains('_') {
            return h_lower.ends_with(suffix);
        }
    }

    // Generalized Regex translation for SQL LIKE
    let mut regex_str = String::with_capacity(p_lower.len() * 2 + 6);
    regex_str.push_str("(?i)^");
    for ch in p_lower.chars() {
        match ch {
            '%' => regex_str.push_str(".*"),
            '_' => regex_str.push('.'),
            '.' | '+' | '?' | '*' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '\\' | '|' => {
                regex_str.push('\\');
                regex_str.push(ch);
            }
            _ => regex_str.push(ch),
        }
    }
    regex_str.push('$');

    if let Ok(re) = Regex::new(&regex_str) {
        re.is_match(haystack)
    } else {
        false
    }
}

pub(crate) fn known_all(items: impl Iterator<Item = Option<bool>>) -> Option<bool> {
    let mut unknown = false;
    for item in items {
        match item {
            Some(false) => return Some(false),
            None => unknown = true,
            _ => {}
        }
    }
    if unknown {
        None
    } else {
        Some(true)
    }
}
pub(crate) fn known_any(items: impl Iterator<Item = Option<bool>>) -> Option<bool> {
    let mut unknown = false;
    for item in items {
        match item {
            Some(true) => return Some(true),
            None => unknown = true,
            _ => {}
        }
    }
    if unknown {
        None
    } else {
        Some(false)
    }
}

impl FilterExpr {
    pub fn eval(&self, ctx: &JobFilterContext) -> bool {
        match self {
            Self::AlwaysTrue => true,
            Self::AlwaysFalse => false,
            Self::FieldMatch { field, op, value } => {
                let actual = ctx.get_field_str(*field);
                match op {
                    FilterOp::Like => sql_like_matches(&actual, value),
                    FilterOp::NotLike => !sql_like_matches(&actual, value),
                    FilterOp::Contains => actual.to_lowercase().contains(&value.to_lowercase()),
                    FilterOp::NotContains => !actual.to_lowercase().contains(&value.to_lowercase()),
                    FilterOp::Equals => actual.eq_ignore_ascii_case(value),
                    FilterOp::NotEquals => !actual.eq_ignore_ascii_case(value),
                }
            }
            Self::FieldIn {
                field,
                values,
                negated,
            } => {
                let actual = ctx.get_field_str(*field);
                let matched = values.iter().any(|v| actual.eq_ignore_ascii_case(v));
                if *negated {
                    !matched
                } else {
                    matched
                }
            }
            Self::TextMatch { pattern, negated } => {
                let in_title = sql_like_matches(ctx.title, pattern);
                let in_desc = sql_like_matches(ctx.description, pattern);
                let in_loc = sql_like_matches(ctx.location, pattern);
                let matched = in_title || in_desc || in_loc;
                if *negated {
                    !matched
                } else {
                    matched
                }
            }
            Self::And(exprs) => {
                if exprs.is_empty() {
                    true
                } else {
                    exprs.iter().all(|e| e.eval(ctx))
                }
            }
            Self::Or(exprs) => {
                if exprs.is_empty() {
                    true
                } else {
                    exprs.iter().any(|e| e.eval(ctx))
                }
            }
            Self::Not(expr) => !expr.eval(ctx),
        }
    }

    /// Three-valued evaluation: absent evidence is unresolved, including under NOT.
    pub fn eval_known(&self, ctx: &JobFilterContext, known: &[FilterField]) -> Option<bool> {
        match self {
            Self::FieldMatch { field, .. } | Self::FieldIn { field, .. } => {
                known.contains(field).then(|| self.eval(ctx))
            }
            Self::TextMatch { pattern, negated } => {
                let fields = [
                    FilterField::Title,
                    FilterField::Description,
                    FilterField::Location,
                ];
                if fields
                    .iter()
                    .filter(|field| known.contains(field))
                    .any(|field| sql_like_matches(&ctx.get_field_str(*field), pattern))
                {
                    Some(!negated)
                } else if fields.iter().all(|field| known.contains(field)) {
                    Some(*negated)
                } else {
                    None
                }
            }
            Self::And(items) => known_all(items.iter().map(|item| item.eval_known(ctx, known))),
            Self::Or(items) if items.is_empty() => Some(true), // Preserve legacy empty-OR semantics.
            Self::Or(items) => known_any(items.iter().map(|item| item.eval_known(ctx, known))),
            Self::Not(item) => item.eval_known(ctx, known).map(|value| !value),
            Self::AlwaysTrue => Some(true),
            Self::AlwaysFalse => Some(false),
        }
    }

    /// Parse a SQL-like boolean expression into a FilterExpr
    pub fn parse_query(query_str: &str) -> Result<Self, String> {
        let tokens = tokenize(query_str)?;
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expr()?;
        if !parser.is_eof() {
            return Err(format!(
                "Unexpected trailing tokens after expression: {:?}",
                parser.peek()
            ));
        }
        Ok(expr)
    }

    /// Parse a structured policy object (e.g. from YAML `policy: { all: [...], any: [...] }`)
    pub fn parse_structured_policy(val: &serde_json::Value) -> Result<Self, String> {
        match val {
            serde_json::Value::Object(map) => {
                let mut and_exprs = Vec::new();
                for (k, v) in map {
                    match k.to_lowercase().as_str() {
                        "all" | "and" => {
                            if let serde_json::Value::Array(items) = v {
                                for item in items {
                                    and_exprs.push(Self::parse_structured_policy(item)?);
                                }
                            } else {
                                return Err("'all'/'and' block must be an array".to_string());
                            }
                        }
                        "any" | "or" => {
                            if let serde_json::Value::Array(items) = v {
                                let mut or_exprs = Vec::new();
                                for item in items {
                                    or_exprs.push(Self::parse_structured_policy(item)?);
                                }
                                and_exprs.push(Self::Or(or_exprs));
                            } else {
                                return Err("'any'/'or' block must be an array".to_string());
                            }
                        }
                        "none" | "not" => {
                            if let serde_json::Value::Array(items) = v {
                                let mut not_or = Vec::new();
                                for item in items {
                                    not_or.push(Self::parse_structured_policy(item)?);
                                }
                                and_exprs.push(Self::Not(Box::new(Self::Or(not_or))));
                            } else if let serde_json::Value::Object(_) = v {
                                and_exprs
                                    .push(Self::Not(Box::new(Self::parse_structured_policy(v)?)));
                            } else {
                                return Err(
                                    "'none'/'not' block must be an array or object".to_string()
                                );
                            }
                        }
                        field_name => {
                            if let Some(field) = FilterField::parse(field_name) {
                                if let serde_json::Value::String(s) = v {
                                    and_exprs.push(Self::FieldMatch {
                                        field,
                                        op: FilterOp::Like,
                                        value: s.clone(),
                                    });
                                } else if let serde_json::Value::Array(arr) = v {
                                    let mut strs = Vec::new();
                                    for item in arr {
                                        if let serde_json::Value::String(s) = item {
                                            strs.push(s.clone());
                                        }
                                    }
                                    and_exprs.push(Self::FieldIn {
                                        field,
                                        values: strs,
                                        negated: false,
                                    });
                                }
                            } else {
                                return Err(format!("Unrecognized policy field: '{}'", field_name));
                            }
                        }
                    }
                }
                if and_exprs.is_empty() {
                    Ok(Self::AlwaysTrue)
                } else if and_exprs.len() == 1 {
                    Ok(and_exprs.remove(0))
                } else {
                    Ok(Self::And(and_exprs))
                }
            }
            serde_json::Value::Array(arr) => {
                let mut exprs = Vec::new();
                for item in arr {
                    exprs.push(Self::parse_structured_policy(item)?);
                }
                Ok(Self::And(exprs))
            }
            serde_json::Value::String(s) => Self::parse_query(s),
            _ => Ok(Self::AlwaysTrue),
        }
    }
}

// ==============================================================================
// Tokenizer & Recursive Descent Parser for SQL Boolean Expressions
// ==============================================================================

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    StringLit(String),
    And,
    Or,
    Not,
    Like,
    NotLike,
    Contains,
    NotContains,
    In,
    NotIn,
    Eq,
    Neq,
    LParen,
    RParen,
    Comma,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut chars = input.chars().peekable();
    let mut tokens = Vec::new();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        match ch {
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            ',' => {
                chars.next();
                tokens.push(Token::Comma);
            }
            '=' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                }
                tokens.push(Token::Eq);
            }
            '!' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::Neq);
                } else {
                    tokens.push(Token::Not);
                }
            }
            '<' => {
                chars.next();
                if chars.peek() == Some(&'>') {
                    chars.next();
                    tokens.push(Token::Neq);
                } else {
                    return Err(
                        "Unsupported operator '<'. Only '=', '!=', '<>', 'LIKE', 'IN' supported"
                            .to_string(),
                    );
                }
            }
            '\'' | '"' => {
                let quote = ch;
                chars.next();
                let mut lit = String::new();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    if c == '\\' {
                        if let Some(next_c) = chars.next() {
                            lit.push(next_c);
                        }
                    } else if c == quote {
                        closed = true;
                        break;
                    } else {
                        lit.push(c);
                    }
                }
                if !closed {
                    return Err("Unterminated string literal in filter query".to_string());
                }
                tokens.push(Token::StringLit(lit));
            }
            _ if ch.is_alphanumeric() || ch == '_' || ch == '%' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' || c == '%' || c == '-' {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }

                match ident.to_uppercase().as_str() {
                    "AND" | "&&" => tokens.push(Token::And),
                    "OR" | "||" => tokens.push(Token::Or),
                    "NOT" => tokens.push(Token::Not),
                    "LIKE" | "ILIKE" => tokens.push(Token::Like),
                    "CONTAINS" => tokens.push(Token::Contains),
                    "IN" => tokens.push(Token::In),
                    _ => {
                        if ident.starts_with('%') || ident.ends_with('%') {
                            tokens.push(Token::StringLit(ident));
                        } else {
                            tokens.push(Token::Ident(ident));
                        }
                    }
                }
            }
            _ => {
                return Err(format!("Unexpected character '{}' in filter query", ch));
            }
        }
    }

    // Collapse compound tokens: NOT LIKE, NOT CONTAINS, NOT IN
    let mut collapsed = Vec::with_capacity(tokens.len());
    let mut idx = 0;
    while idx < tokens.len() {
        if tokens[idx] == Token::Not && idx + 1 < tokens.len() {
            match tokens[idx + 1] {
                Token::Like => {
                    collapsed.push(Token::NotLike);
                    idx += 2;
                    continue;
                }
                Token::Contains => {
                    collapsed.push(Token::NotContains);
                    idx += 2;
                    continue;
                }
                Token::In => {
                    collapsed.push(Token::NotIn);
                    idx += 2;
                    continue;
                }
                _ => {}
            }
        }
        collapsed.push(tokens[idx].clone());
        idx += 1;
    }

    Ok(collapsed)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn parse_expr(&mut self) -> Result<FilterExpr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<FilterExpr, String> {
        let mut left = self.parse_and()?;

        while let Some(Token::Or) = self.peek() {
            self.next();
            let right = self.parse_and()?;
            left = match left {
                FilterExpr::Or(mut list) => {
                    list.push(right);
                    FilterExpr::Or(list)
                }
                _ => FilterExpr::Or(vec![left, right]),
            };
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<FilterExpr, String> {
        let mut left = self.parse_unary()?;

        while let Some(Token::And) = self.peek() {
            self.next();
            let right = self.parse_unary()?;
            left = match left {
                FilterExpr::And(mut list) => {
                    list.push(right);
                    FilterExpr::And(list)
                }
                _ => FilterExpr::And(vec![left, right]),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<FilterExpr, String> {
        if let Some(Token::Not) = self.peek() {
            self.next();
            let operand = self.parse_unary()?;
            return Ok(FilterExpr::Not(Box::new(operand)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<FilterExpr, String> {
        match self.peek() {
            Some(Token::LParen) => {
                self.next();
                let expr = self.parse_expr()?;
                match self.next() {
                    Some(Token::RParen) => Ok(expr),
                    _ => Err("Expected closing ')'".to_string()),
                }
            }
            Some(Token::Ident(ident_str)) => {
                let ident = ident_str.clone();
                self.next();

                if let Some(field) = FilterField::parse(&ident) {
                    self.parse_field_predicate(field)
                } else {
                    // Shorthand text match if not a known field
                    Ok(FilterExpr::TextMatch {
                        pattern: format!("%{}%", ident),
                        negated: false,
                    })
                }
            }
            Some(Token::StringLit(s)) => {
                let pattern = s.clone();
                self.next();
                Ok(FilterExpr::TextMatch {
                    pattern,
                    negated: false,
                })
            }
            Some(tok) => Err(format!("Unexpected token '{:?}' in filter expression", tok)),
            None => Err("Unexpected end of filter expression".to_string()),
        }
    }

    fn parse_field_predicate(&mut self, field: FilterField) -> Result<FilterExpr, String> {
        match self.peek() {
            Some(Token::Like) => {
                self.next();
                let val = self.parse_value()?;
                Ok(FilterExpr::FieldMatch {
                    field,
                    op: FilterOp::Like,
                    value: val,
                })
            }
            Some(Token::NotLike) => {
                self.next();
                let val = self.parse_value()?;
                Ok(FilterExpr::FieldMatch {
                    field,
                    op: FilterOp::NotLike,
                    value: val,
                })
            }
            Some(Token::Contains) => {
                self.next();
                let val = self.parse_value()?;
                Ok(FilterExpr::FieldMatch {
                    field,
                    op: FilterOp::Contains,
                    value: val,
                })
            }
            Some(Token::NotContains) => {
                self.next();
                let val = self.parse_value()?;
                Ok(FilterExpr::FieldMatch {
                    field,
                    op: FilterOp::NotContains,
                    value: val,
                })
            }
            Some(Token::Eq) => {
                self.next();
                let val = self.parse_value()?;
                Ok(FilterExpr::FieldMatch {
                    field,
                    op: FilterOp::Equals,
                    value: val,
                })
            }
            Some(Token::Neq) => {
                self.next();
                let val = self.parse_value()?;
                Ok(FilterExpr::FieldMatch {
                    field,
                    op: FilterOp::NotEquals,
                    value: val,
                })
            }
            Some(Token::In) => {
                self.next();
                let vals = self.parse_value_list()?;
                Ok(FilterExpr::FieldIn {
                    field,
                    values: vals,
                    negated: false,
                })
            }
            Some(Token::NotIn) => {
                self.next();
                let vals = self.parse_value_list()?;
                Ok(FilterExpr::FieldIn {
                    field,
                    values: vals,
                    negated: true,
                })
            }
            Some(tok) => Err(format!(
                "Expected operator (LIKE, NOT LIKE, CONTAINS, =, !=, IN) after field {:?}, got '{:?}'",
                field, tok
            )),
            None => Err(format!(
                "Expected operator after field {:?}, reached end of input",
                field
            )),
        }
    }

    fn parse_value(&mut self) -> Result<String, String> {
        match self.next() {
            Some(Token::StringLit(s)) => Ok(s),
            Some(Token::Ident(s)) => Ok(s),
            Some(tok) => Err(format!("Expected string or literal value, got '{:?}'", tok)),
            None => Err("Expected value, reached end of expression".to_string()),
        }
    }

    fn parse_value_list(&mut self) -> Result<Vec<String>, String> {
        match self.next() {
            Some(Token::LParen) => {}
            _ => return Err("Expected '(' before list of IN values".to_string()),
        }

        let mut values = Vec::new();
        while let Some(tok) = self.peek() {
            if *tok == Token::RParen {
                self.next();
                return Ok(values);
            }

            let val = self.parse_value()?;
            values.push(val);

            if let Some(Token::Comma) = self.peek() {
                self.next();
            } else if let Some(Token::RParen) = self.peek() {
                self.next();
                return Ok(values);
            } else {
                return Err("Expected ',' or ')' in value list".to_string());
            }
        }

        Err("Unterminated value list in IN clause".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_like_wildcards() {
        assert!(sql_like_matches("Software Engineer", "%Engineer%"));
        assert!(sql_like_matches("Software Engineer", "%software%"));
        assert!(sql_like_matches("Software Engineer", "software%"));
        assert!(!sql_like_matches("Software Engineer", "%intern%"));
        assert!(sql_like_matches("Austin, Texas", "%Texas"));
        assert!(sql_like_matches("Remote", "remote"));
        assert!(sql_like_matches("CO-OP", "CO_OP"));
        assert!(!sql_like_matches("COLOP", "CO-OP"));
    }

    #[test]
    fn test_query_parser_and_evaluation() {
        let query = "title LIKE '%Engineer%' AND (location LIKE '%Remote%' OR location LIKE '%Austin%') AND title NOT LIKE '%Senior%'";
        let expr = FilterExpr::parse_query(query).unwrap();

        let ctx_match = JobFilterContext {
            title: "Staff Software Engineer", // note: not "Senior"
            location: "Austin, Texas",
            description: "Exciting systems work",
            company: "Tech Corp",
            role_type: RoleType::FullTime,
        };
        assert!(expr.eval(&ctx_match));

        let ctx_senior = JobFilterContext {
            title: "Senior Software Engineer",
            location: "Austin, Texas",
            description: "Exciting systems work",
            company: "Tech Corp",
            role_type: RoleType::FullTime,
        };
        assert!(!expr.eval(&ctx_senior));

        let ctx_loc_mismatch = JobFilterContext {
            title: "Software Engineer",
            location: "New York, NY",
            description: "Exciting systems work",
            company: "Tech Corp",
            role_type: RoleType::FullTime,
        };
        assert!(!expr.eval(&ctx_loc_mismatch));
    }

    #[test]
    fn test_query_with_in_clause() {
        let query = "role_type IN ('internship', 'contract') AND title LIKE '%Intern%'";
        let expr = FilterExpr::parse_query(query).unwrap();

        let ctx_intern = JobFilterContext {
            title: "SWE Intern",
            location: "Remote",
            description: "Summer intern program",
            company: "Cloudflare",
            role_type: RoleType::Internship,
        };
        assert!(expr.eval(&ctx_intern));

        let ctx_full_time = JobFilterContext {
            title: "SWE Intern",
            location: "Remote",
            description: "Summer intern program",
            company: "Cloudflare",
            role_type: RoleType::FullTime,
        };
        assert!(!expr.eval(&ctx_full_time));
    }

    #[test]
    fn test_structured_policy_parsing() {
        let policy_json = serde_json::json!({
            "all": [
                {
                    "any": [
                        { "title": "%Engineer%" },
                        { "title": "%Developer%" }
                    ]
                },
                {
                    "none": [
                        { "title": "%Senior%" }
                    ]
                }
            ]
        });

        let expr = FilterExpr::parse_structured_policy(&policy_json).unwrap();
        let ctx = JobFilterContext {
            title: "Software Developer",
            location: "Remote",
            description: "Coding",
            company: "Acme",
            role_type: RoleType::FullTime,
        };
        assert!(expr.eval(&ctx));

        let ctx_senior = JobFilterContext {
            title: "Senior Developer",
            location: "Remote",
            description: "Coding",
            company: "Acme",
            role_type: RoleType::FullTime,
        };
        assert!(!expr.eval(&ctx_senior));
    }
}

#[cfg(test)]
mod partial_evaluation_tests {
    use super::*;
    #[test]
    fn known_decisions_hold_for_every_completion_of_missing_fields() {
        let fields = [
            FilterField::Title,
            FilterField::Location,
            FilterField::Description,
            FilterField::Company,
            FilterField::RoleType,
        ];
        let queries = [
            "title LIKE '%Engineer%'",
            "title LIKE '%Engineer%' OR description LIKE '%Rust%'",
            "title LIKE '%Engineer%' AND NOT (description LIKE '%Senior%')",
            "NOT (company LIKE '%Acme%' OR location LIKE '%Remote%')",
            "role_type = 'internship' OR description LIKE '%Rust%'",
        ];
        let base = JobFilterContext {
            title: "Engineer",
            location: "Remote",
            description: "Rust",
            company: "Acme",
            role_type: RoleType::FullTime,
        };
        for query in queries {
            let expression = FilterExpr::parse_query(query).unwrap();
            for mask in 0..32 {
                let known: Vec<_> = fields
                    .iter()
                    .enumerate()
                    .filter_map(|(i, field)| (mask & (1 << i) != 0).then_some(*field))
                    .collect();
                let decision = expression.eval_known(&base, &known);
                if let Some(expected) = decision {
                    for completion in 0..32 {
                        let alternate = |index, original, other| {
                            if mask & (1 << index) != 0 || completion & (1 << index) == 0 {
                                original
                            } else {
                                other
                            }
                        };
                        let candidate = JobFilterContext {
                            title: alternate(0, "Engineer", "Sales"),
                            location: alternate(1, "Remote", "NYC"),
                            description: alternate(2, "Rust", "Senior"),
                            company: alternate(3, "Acme", "Other"),
                            role_type: if mask & 16 != 0 || completion & 16 == 0 {
                                RoleType::FullTime
                            } else {
                                RoleType::Internship
                            },
                        };
                        assert_eq!(
                            expression.eval(&candidate),
                            expected,
                            "{query}, known mask {mask}, completion {completion}"
                        );
                    }
                }
            }
        }
    }
}
