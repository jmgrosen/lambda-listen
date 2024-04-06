use string_interner::DefaultStringInterner;
use typed_arena::Arena;

use crate::{expr::{Expr, Value, Symbol}, typing::{Type, ArraySize}};

macro_rules! make_node_enum {
    ($enum_name:ident { $($rust_name:ident : $ts_name:ident),* } with matcher $matcher_name:ident) => {
        #[derive(PartialEq, Eq, Debug, Copy, Clone)]
        enum $enum_name {
            $( $rust_name ),*
        }

        pub struct $matcher_name {
            node_id_table: Vec<Option<$enum_name>>,
        }

        impl $matcher_name {
            fn new(lang: &tree_sitter::Language) -> $matcher_name {
                let mut table = [None].repeat(lang.node_kind_count());
                $( table[lang.id_for_node_kind(stringify!($ts_name), true) as usize] = Some($enum_name::$rust_name); )*
                $matcher_name {
                    node_id_table: table
                }
            }

            fn lookup(&self, id: u16) -> Option<$enum_name> {
                self.node_id_table.get(id as usize).copied().flatten()
            }
        }
    };
}

// why isn't this information in the generated bindings...?
make_node_enum!(ConcreteNode {
    SourceFile: source_file,
    Expression: expression,
    WrapExpression: wrap_expression,
    Identifier: identifier,
    Literal: literal,
    Sample: sample,
    ApplicationExpression: application_expression,
    LambdaExpression: lambda_expression,
    LobExpression: lob_expression,
    ForceExpression: force_expression,
    GenExpression: gen_expression,
    LetExpression: let_expression,
    AnnotateExpression: annotate_expression,
    PairExpression: pair_expression,
    UnPairExpression: unpair_expression,
    InLExpression: inl_expression,
    InRExpression: inr_expression,
    CaseExpression: case_expression,
    ArrayExpression: array_expression,
    ArrayInner: array_inner,
    Type: type,
    WrapType: wrap_type,
    BaseType: base_type,
    FunctionType: function_type,
    StreamType: stream_type,
    ProductType: product_type,
    SumType: sum_type,
    ArrayType: array_type
} with matcher ConcreteNodeMatcher);

pub struct Parser<'a, 'b> {
    parser: tree_sitter::Parser,
    node_matcher: ConcreteNodeMatcher,
    interner: &'a mut DefaultStringInterner,
    arena: &'b Arena<Expr<'b, tree_sitter::Range>>,
}

impl<'a, 'b> Parser<'a, 'b> {
    pub fn new(interner: &'a mut DefaultStringInterner, arena: &'b Arena<Expr<'b, tree_sitter::Range>>) -> Parser<'a, 'b> {
        let mut parser = tree_sitter::Parser::new();
        let lang = tree_sitter_lambdalisten::language();
        let node_matcher = ConcreteNodeMatcher::new(&lang);
        parser.set_language(lang).expect("Error loading lambda listen grammar");

        Parser {
            parser,
            node_matcher,
            interner,
            arena,
        }
    }

    pub fn parse(&mut self, text: &str) -> Result<Expr<'b, tree_sitter::Range>, String> {
        // this unwrap should be safe because we make sure to set the language and don't set a timeout or cancellation flag
        let tree = self.parser.parse(text, None).unwrap();
        let root_node = tree.root_node();
        AbstractionContext { parser: self, original_text: text }.parse_expr(root_node).map_err(|e| format!("{:?}", e))
    }
}

#[derive(Debug)]
pub enum ParseError<'a> {
    BadLiteral(tree_sitter::Node<'a>),
    ExpectedExpression(tree_sitter::Node<'a>),
    ExpectedType(tree_sitter::Node<'a>),
    UnknownNodeType(tree_sitter::Node<'a>),
}

struct AbstractionContext<'a, 'b, 'c> {
    parser: &'c mut Parser<'a, 'b>,
    original_text: &'c str,
}

impl<'a, 'b, 'c> AbstractionContext<'a, 'b, 'c> {
    fn node_text<'d>(&self, node: tree_sitter::Node<'d>) -> &'c str {
        // utf8_text must return Ok because it is fetching from a &str, which must be utf8
        node.utf8_text(self.original_text.as_bytes()).unwrap()
    }

    fn alloc(&self, expr: Expr<'b, tree_sitter::Range>) -> &'b Expr<'b, tree_sitter::Range> {
        self.parser.arena.alloc(expr)
    }

    fn identifier<'d>(&mut self, node: tree_sitter::Node<'d>) -> Symbol {
        self.parser.interner.get_or_intern(self.node_text(node))
    }

    fn parse_expr<'d>(&mut self, node: tree_sitter::Node<'d>) -> Result<Expr<'b, tree_sitter::Range>, ParseError<'d>> {
        // TODO: use a TreeCursor instead
        match self.parser.node_matcher.lookup(node.kind_id()) {
            Some(ConcreteNode::SourceFile) =>
                self.parse_expr(node.child(0).unwrap()),
            Some(ConcreteNode::Expression) =>
                self.parse_expr(node.child(0).unwrap()),
            Some(ConcreteNode::WrapExpression) =>
                // the literals are included in the children indices
                self.parse_expr(node.child(1).unwrap()),
            Some(ConcreteNode::Identifier) => {
                let interned_ident = self.parser.interner.get_or_intern(self.node_text(node));
                Ok(Expr::Var(node.range(), interned_ident))
            },
            Some(ConcreteNode::Literal) => {
                let int_lit = self.node_text(node).parse().map_err(|_| ParseError::BadLiteral(node))?;
                Ok(Expr::Val(node.range(), Value::Index(int_lit)))
            },
            Some(ConcreteNode::Sample) => {
                let sample_text = self.node_text(node);
                let sample = sample_text.parse().map_err(|_| ParseError::BadLiteral(node))?;
                Ok(Expr::Val(node.range(), Value::Sample(sample)))
            },
            Some(ConcreteNode::ApplicationExpression) => {
                let e1 = self.parse_expr(node.child(0).unwrap())?;
                let e2 = self.parse_expr(node.child(1).unwrap())?;
                Ok(Expr::App(node.range(), self.parser.arena.alloc(e1), self.parser.arena.alloc(e2)))
            },
            Some(ConcreteNode::LambdaExpression) => {
                let x = self.parser.interner.get_or_intern(self.node_text(node.child(1).unwrap()));
                let e = self.parse_expr(node.child(3).unwrap())?;
                Ok(Expr::Lam(node.range(), x, self.parser.arena.alloc(e)))
            },
            Some(ConcreteNode::LobExpression) => {
                let x = self.parser.interner.get_or_intern(self.node_text(node.child(1).unwrap()));
                let e = self.parse_expr(node.child(3).unwrap())?;
                Ok(Expr::Lob(node.range(), x, self.parser.arena.alloc(e)))
            },
            Some(ConcreteNode::ForceExpression) => {
                let e = self.parse_expr(node.child(1).unwrap())?;
                Ok(Expr::Force(node.range(), self.parser.arena.alloc(e)))
            },
            Some(ConcreteNode::GenExpression) => {
                let e1 = self.parse_expr(node.child(0).unwrap())?;
                let e2 = self.parse_expr(node.child(2).unwrap())?;
                Ok(Expr::Gen(node.range(), self.parser.arena.alloc(e1), self.parser.arena.alloc(e2)))
            },
            Some(ConcreteNode::LetExpression) => {
                let x = self.parser.interner.get_or_intern(self.node_text(node.child(1).unwrap()));
                let e1 = self.parse_expr(node.child(3).unwrap())?;
                let e2 = self.parse_expr(node.child(5).unwrap())?;
                Ok(Expr::LetIn(node.range(), x, self.parser.arena.alloc(e1), self.parser.arena.alloc(e2)))
            },
            Some(ConcreteNode::AnnotateExpression) => {
                let e = self.parse_expr(node.child(0).unwrap())?;
                let ty = self.parse_type(node.child(2).unwrap())?;
                Ok(Expr::Annotate(node.range(), self.parser.arena.alloc(e), ty))
            },
            Some(ConcreteNode::PairExpression) => {
                let e1 = self.parse_expr(node.child(1).unwrap())?;
                let e2 = self.parse_expr(node.child(3).unwrap())?;
                Ok(Expr::Pair(node.range(), self.alloc(e1), self.alloc(e2)))
            },
            Some(ConcreteNode::UnPairExpression) => {
                let x1 = self.identifier(node.child(2).unwrap());
                let x2 = self.identifier(node.child(4).unwrap());
                let e0 = self.parse_expr(node.child(7).unwrap())?;
                let e = self.parse_expr(node.child(9).unwrap())?;
                Ok(Expr::UnPair(node.range(), x1, x2, self.alloc(e0), self.alloc(e)))
            },
            Some(ConcreteNode::InLExpression) => {
                let e = self.parse_expr(node.child(1).unwrap())?;
                Ok(Expr::InL(node.range(), self.alloc(e)))
            },
            Some(ConcreteNode::InRExpression) => {
                let e = self.parse_expr(node.child(1).unwrap())?;
                Ok(Expr::InR(node.range(), self.alloc(e)))
            },
            Some(ConcreteNode::CaseExpression) => {
                let e0 = self.parse_expr(node.child(1).unwrap())?;
                let x1 = self.identifier(node.child(4).unwrap());
                let e1 = self.parse_expr(node.child(6).unwrap())?;
                let x2 = self.identifier(node.child(9).unwrap());
                let e2 = self.parse_expr(node.child(11).unwrap())?;
                Ok(Expr::Case(node.range(), self.alloc(e0), x1, self.alloc(e1), x2, self.alloc(e2)))
            },
            Some(ConcreteNode::ArrayExpression) => {
                Ok(Expr::Array(node.range(),
                    if node.child_count() == 2 {
                        [].into()
                    } else {
                        let array_inner = node.child(1).unwrap();
                        let mut es = Vec::with_capacity((array_inner.child_count() + 1) / 2);
                        let mut cur = array_inner.walk();
                        cur.goto_first_child();
                        loop {
                            let e = self.parse_expr(cur.node())?;
                            es.push(self.alloc(e));
                            cur.goto_next_sibling();
                            if !cur.goto_next_sibling() {
                                break;
                            }
                        }
                        es.into()
                    }))
            },
            Some(ConcreteNode::Type) |
            Some(ConcreteNode::BaseType) |
            Some(ConcreteNode::StreamType) |
            Some(ConcreteNode::WrapType) |
            Some(ConcreteNode::ProductType) |
            Some(ConcreteNode::SumType) |
            Some(ConcreteNode::ArrayType) |
            Some(ConcreteNode::ArrayInner) |
            Some(ConcreteNode::FunctionType) =>
                Err(ParseError::ExpectedExpression(node)),
            None => 
                Err(ParseError::UnknownNodeType(node)),
        }
    }

    // TODO: add range information to Type?
    fn parse_type<'d>(&mut self, node: tree_sitter::Node<'d>) -> Result<Type, ParseError<'d>> {
        match self.parser.node_matcher.lookup(node.kind_id()) {
            Some(ConcreteNode::Type) =>
                self.parse_type(node.child(0).unwrap()),
            Some(ConcreteNode::WrapType) =>
                self.parse_type(node.child(1).unwrap()),
            Some(ConcreteNode::BaseType) =>
                Ok(match self.node_text(node) {
                    "sample" => Type::Sample,
                    "index" => Type::Index,
                    "unit" => Type::Unit,
                    base => panic!("unknown base type {base}"),
                }),
            Some(ConcreteNode::FunctionType) => {
                let ty1 = self.parse_type(node.child(0).unwrap())?;
                let ty2 = self.parse_type(node.child(2).unwrap())?;
                Ok(Type::Function(Box::new(ty1), Box::new(ty2)))
            },
            Some(ConcreteNode::StreamType) => {
                let ty = self.parse_type(node.child(1).unwrap())?;
                Ok(Type::Stream(Box::new(ty)))
            },
            Some(ConcreteNode::ProductType) => {
                let ty1 = self.parse_type(node.child(0).unwrap())?;
                let ty2 = self.parse_type(node.child(2).unwrap())?;
                Ok(Type::Product(Box::new(ty1), Box::new(ty2)))
            },
            Some(ConcreteNode::SumType) => {
                let ty1 = self.parse_type(node.child(0).unwrap())?;
                let ty2 = self.parse_type(node.child(2).unwrap())?;
                Ok(Type::Sum(Box::new(ty1), Box::new(ty2)))
            },
            Some(ConcreteNode::ArrayType) => {
                let ty = self.parse_type(node.child(1).unwrap())?;
                let size = self.parse_size(node.child(3).unwrap())?;
                Ok(Type::Array(Box::new(ty), size))
            },
            Some(_) =>
                Err(ParseError::ExpectedType(node)),
            None =>
                Err(ParseError::UnknownNodeType(node)),
        }
    }

    fn parse_size<'d>(&mut self, node: tree_sitter::Node<'d>) -> Result<ArraySize, ParseError<'d>> {
        let text = self.node_text(node);
        match text.parse() {
            Ok(n) => Ok(ArraySize::from_const(n)),
            Err(_) => Err(ParseError::BadLiteral(node)),
        }
    }
}
