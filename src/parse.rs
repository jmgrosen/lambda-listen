use string_interner::DefaultStringInterner;
use typed_arena::Arena;

use crate::expr::{Expr, Value};

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
    GenExpression: gen_expression
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
        AbstractionContext { parser: self, original_text: text }.parse_concrete(root_node)
    }
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

    fn parse_concrete<'d>(&mut self, node: tree_sitter::Node<'d>) -> Result<Expr<'b, tree_sitter::Range>, String> {
        // TODO: use a TreeCursor instead
        match self.parser.node_matcher.lookup(node.kind_id()) {
            Some(ConcreteNode::SourceFile) =>
                self.parse_concrete(node.child(0).unwrap()),
            Some(ConcreteNode::Expression) =>
                self.parse_concrete(node.child(0).unwrap()),
            Some(ConcreteNode::WrapExpression) =>
                // the literals are included in the children indices
                self.parse_concrete(node.child(1).unwrap()),
            Some(ConcreteNode::Identifier) => {
                let interned_ident = self.parser.interner.get_or_intern(self.node_text(node));
                Ok(Expr::Var(node.range(), interned_ident))
            },
            Some(ConcreteNode::Literal) => {
                let int_lit = self.node_text(node).parse().map_err(|_| "int lit out of bounds".to_string())?;
                Ok(Expr::Val(node.range(), Value::Index(int_lit)))
            },
            Some(ConcreteNode::Sample) => {
                let sample_text = self.node_text(node);
                let sample = sample_text.parse().map_err(|_| format!("couldn't parse sample {:?}", sample_text))?;
                Ok(Expr::Val(node.range(), Value::Sample(sample)))
            },
            Some(ConcreteNode::ApplicationExpression) => {
                let e1 = self.parse_concrete(node.child(0).unwrap())?;
                let e2 = self.parse_concrete(node.child(1).unwrap())?;
                Ok(Expr::App(node.range(), self.parser.arena.alloc(e1), self.parser.arena.alloc(e2)))
            },
            Some(ConcreteNode::LambdaExpression) => {
                let x = self.parser.interner.get_or_intern(self.node_text(node.child(1).unwrap()));
                let e = self.parse_concrete(node.child(3).unwrap())?;
                Ok(Expr::Lam(node.range(), x, self.parser.arena.alloc(e)))
            },
            Some(ConcreteNode::LobExpression) => {
                let x = self.parser.interner.get_or_intern(self.node_text(node.child(1).unwrap()));
                let e = self.parse_concrete(node.child(3).unwrap())?;
                Ok(Expr::Lob(node.range(), x, self.parser.arena.alloc(e)))
            },
            Some(ConcreteNode::ForceExpression) => {
                let e = self.parse_concrete(node.child(1).unwrap())?;
                Ok(Expr::Force(node.range(), self.parser.arena.alloc(e)))
            },
            Some(ConcreteNode::GenExpression) => {
                let e1 = self.parse_concrete(node.child(0).unwrap())?;
                let e2 = self.parse_concrete(node.child(2).unwrap())?;
                Ok(Expr::Gen(node.range(), self.parser.arena.alloc(e1), self.parser.arena.alloc(e2)))
            },
            None => {
                eprintln!("{:?}", node);
                Err("what".to_string())
            },
        }
    }
}