use std::fmt;

use string_interner::{DefaultStringInterner, DefaultSymbol};
use typed_arena::Arena;

use crate::typing::{Clock, Type, PrettyClock, PrettyType};

pub type Symbol = DefaultSymbol;

#[derive(Debug, Clone)]
pub enum Value {
    Unit,
    Sample(f32),
    Index(usize),
}

#[derive(Debug, Clone, Copy)]
pub enum Binop {
    FMul,
    FDiv,
    FAdd,
    FSub,
    FGt,
    FGe,
    FLt,
    FLe,
    FEq,
    FNe,
    Shl,
    Shr,
    And,
    Xor,
    Or,
    IMul,
    IDiv,
    IAdd,
    ISub,
    IGt,
    IGe,
    ILt,
    ILe,
    IEq,
    INe,
}

impl Binop {
    #[allow(unused)]
    pub fn is_cmp(&self) -> bool {
        match *self {
            Binop::FGt => true,
            Binop::FGe => true,
            Binop::FLt => true,
            Binop::FLe => true,
            Binop::FEq => true,
            Binop::FNe => true,
            Binop::IGt => true,
            Binop::IGe => true,
            Binop::ILt => true,
            Binop::ILe => true,
            Binop::IEq => true,
            Binop::INe => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expr<'a, R> {
    Var(R, Symbol),
    Val(R, Value),
    Annotate(R, &'a Expr<'a, R>, Type),
    Lam(R, Symbol, &'a Expr<'a, R>),
    App(R, &'a Expr<'a, R>, &'a Expr<'a, R>),
    Adv(R, &'a Expr<'a, R>),
    Lob(R, Clock, Symbol, &'a Expr<'a, R>),
    Gen(R, &'a Expr<'a, R>, &'a Expr<'a, R>),
    LetIn(R, Symbol, Option<Type>, &'a Expr<'a, R>, &'a Expr<'a, R>),
    Pair(R, &'a Expr<'a, R>, &'a Expr<'a, R>),
    UnPair(R, Symbol, Symbol, &'a Expr<'a, R>, &'a Expr<'a, R>),
    InL(R, &'a Expr<'a, R>),
    InR(R, &'a Expr<'a, R>),
    Case(R, &'a Expr<'a, R>, Symbol, &'a Expr<'a, R>, Symbol, &'a Expr<'a, R>),
    Array(R, Box<[&'a Expr<'a, R>]>),
    UnGen(R, &'a Expr<'a, R>),
    Delay(R, &'a Expr<'a, R>),
    Box(R, &'a Expr<'a, R>),
    Unbox(R, &'a Expr<'a, R>),
    ClockApp(R, &'a Expr<'a, R>, Clock),
    TypeApp(R, &'a Expr<'a, R>, Type),
    Binop(R, Binop, &'a Expr<'a, R>, &'a Expr<'a, R>),
    ExIntro(R, Clock, &'a Expr<'a, R>),
    ExElim(R, Symbol, Symbol, &'a Expr<'a, R>, &'a Expr<'a, R>),
    // below this are expressions that are not exposed in the surface syntax

    // TODO: this Symbol is in the typevar namespace. we should have
    // newtypes for typevar vs termvar namespaces
    ClockLam(R, Symbol, &'a Expr<'a, R>),
}

impl<'a, R> Expr<'a, R> {
    pub fn map_ext<'b, U>(&self, arena: &'b Arena<Expr<'b, U>>, f: &dyn Fn(&R) -> U) -> Expr<'b, U> where 'a: 'b {
        match *self {
            Expr::Var(ref r, s) => Expr::Var(f(r), s),
            Expr::Val(ref r, ref v) => Expr::Val(f(r), v.clone()),
            Expr::Annotate(ref r, e, ref ty) => Expr::Annotate(f(r), arena.alloc(e.map_ext(arena, f)), ty.clone()),
            Expr::Lam(ref r, s, ref e) => Expr::Lam(f(r), s, arena.alloc(e.map_ext(arena, f))),
            Expr::App(ref r, ref e1, ref e2) => Expr::App(f(r), arena.alloc(e1.map_ext(arena, f)), arena.alloc(e2.map_ext(arena, f))),
            Expr::Adv(ref r, ref e) => Expr::Adv(f(r), arena.alloc(e.map_ext(arena, f))),
            Expr::Lob(ref r, clock, s, ref e) => Expr::Lob(f(r), clock, s, arena.alloc(e.map_ext(arena, f))),
            Expr::Gen(ref r, ref e1, ref e2) => Expr::Gen(f(r), arena.alloc(e1.map_ext(arena, f)), arena.alloc(e2.map_ext(arena, f))),
            Expr::LetIn(ref r, s, ref ty, ref e1, ref e2) => Expr::LetIn(f(r), s, ty.clone(), arena.alloc(e1.map_ext(arena, f)), arena.alloc(e2.map_ext(arena, f))),
            Expr::Pair(ref r, e1, e2) => Expr::Pair(f(r), arena.alloc(e1.map_ext(arena, f)), arena.alloc(e2.map_ext(arena, f))),
            Expr::UnPair(ref r, s1, s2, e1, e2) => Expr::UnPair(f(r), s1, s2, arena.alloc(e1.map_ext(arena, f)), arena.alloc(e2.map_ext(arena, f))),
            Expr::InL(ref r, e) => Expr::InL(f(r), arena.alloc(e.map_ext(arena, f))),
            Expr::InR(ref r, e) => Expr::InR(f(r), arena.alloc(e.map_ext(arena, f))),
            Expr::Case(ref r, e0, s1, e1, s2, e2) => Expr::Case(f(r), arena.alloc(e0.map_ext(arena, f)), s1, arena.alloc(e1.map_ext(arena, f)), s2, arena.alloc(e2.map_ext(arena, f))),
            Expr::Array(ref r, ref es) => Expr::Array(f(r), es.iter().map(|e| &*arena.alloc(e.map_ext(arena, f))).collect::<Vec<_>>().into()),
            Expr::UnGen(ref r, ref e) => Expr::UnGen(f(r), arena.alloc(e.map_ext(arena, f))),
            Expr::Delay(ref r, ref e) => Expr::Delay(f(r), arena.alloc(e.map_ext(arena, f))),
            Expr::Box(ref r, ref e) => Expr::Box(f(r), arena.alloc(e.map_ext(arena, f))),
            Expr::Unbox(ref r, ref e) => Expr::Unbox(f(r), arena.alloc(e.map_ext(arena, f))),
            Expr::ClockApp(ref r, ref e, c) => Expr::ClockApp(f(r), arena.alloc(e.map_ext(arena, f)), c),
            Expr::TypeApp(ref r, ref e, ref ty) => Expr::TypeApp(f(r), arena.alloc(e.map_ext(arena, f)), ty.clone()),
            Expr::Binop(ref r, op, ref e1, ref e2) => Expr::Binop(f(r), op, arena.alloc(e1.map_ext(arena, f)), arena.alloc(e2.map_ext(arena, f))),
            Expr::ExIntro(ref r, c, ref e) => Expr::ExIntro(f(r), c, arena.alloc(e.map_ext(arena, f))),
            Expr::ExElim(ref r, x1, x2, ref e1, ref e2) => Expr::ExElim(f(r), x1, x2, arena.alloc(e1.map_ext(arena, f)), arena.alloc(e2.map_ext(arena, f))),
            Expr::ClockLam(ref r, x, e) => Expr::ClockLam(f(r), x, arena.alloc(e.map_ext(arena, f))),
        }
    }

    pub fn pretty<'b>(&'b self, interner: &'b DefaultStringInterner) -> PrettyExpr<'b, 'a, R> {
        PrettyExpr { interner, expr: self }
    }

    pub fn range(&self) -> &R {
        match *self {
            Expr::Var(ref r, _) => r,
            Expr::Val(ref r, _) => r,
            Expr::Annotate(ref r, _, _) => r,
            Expr::Lam(ref r, _, _) => r,
            Expr::App(ref r, _, _) => r,
            Expr::Adv(ref r, _) => r,
            Expr::Lob(ref r, _, _, _) => r,
            Expr::Gen(ref r, _, _) => r,
            Expr::LetIn(ref r, _, _, _, _) => r,
            Expr::Pair(ref r, _, _) => r,
            Expr::UnPair(ref r, _, _, _, _) => r,
            Expr::InL(ref r, _) => r,
            Expr::InR(ref r, _) => r,
            Expr::Case(ref r, _, _, _, _, _) => r,
            Expr::Array(ref r, _) => r,
            Expr::UnGen(ref r, _) => r,
            Expr::Delay(ref r, _) => r,
            Expr::Box(ref r, _) => r,
            Expr::Unbox(ref r, _) => r,
            Expr::ClockApp(ref r, _, _) => r,
            Expr::TypeApp(ref r, _, _) => r,
            Expr::Binop(ref r, _, _, _) => r,
            Expr::ExIntro(ref r, _, _) => r,
            Expr::ExElim(ref r, _, _, _, _) => r,
            Expr::ClockLam(ref r, _, _) => r,
        }
    }
}

pub struct PrettyExpr<'a, 'b, R> {
    interner: &'a DefaultStringInterner,
    expr: &'a Expr<'b, R>,
}

impl<'a, 'b, R> PrettyExpr<'a, 'b, R> {
    fn for_expr(&self, other_expr: &'a Expr<'b, R>) -> PrettyExpr<'a, 'b, R> {
        PrettyExpr { interner: self.interner, expr: other_expr }
    }

    fn for_clock(&self, clock: &'a Clock) -> PrettyClock<'a> {
        clock.pretty(self.interner)
    }

    fn for_type(&self, ty: &'a Type) -> PrettyType<'a> {
        ty.pretty(self.interner)
    }

    fn name(&self, s: Symbol) -> &'a str {
        self.interner.resolve(s).expect("encountered an symbol not corresponding to an identifier while pretty printing an expression")
    }
}

impl<'a, 'b, R> fmt::Display for PrettyExpr<'a, 'b, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.expr {
            Expr::Var(_, x) =>
                write!(f, "Var({})", self.name(x)),
            Expr::Val(_, ref v) =>
                write!(f, "{:?}", v),
            Expr::Annotate(_, e, ref ty) =>
                write!(f, "Annotate({}, {:?})", self.for_expr(e), ty),
            Expr::App(_, ref e1, ref e2) =>
                write!(f, "App({}, {})", self.for_expr(e1), self.for_expr(e2)),
            Expr::Lam(_, x, ref e) =>
                write!(f, "Lam({}, {})", self.name(x), self.for_expr(e)),
            Expr::Adv(_, ref e) =>
                write!(f, "Force({})", self.for_expr(e)),
            Expr::Lob(_, ref clock, x, ref e) =>
                write!(f, "Lob({}, {}, {})", self.for_clock(clock), self.name(x), self.for_expr(e)),
            Expr::Gen(_, ref eh, ref et) =>
                write!(f, "Gen({}, {})", self.for_expr(eh), self.for_expr(et)),
            Expr::LetIn(_, x, ref ty, e1, e2) =>
                if let Some(ref ty) = *ty {
                    write!(f, "Let({}, Some({}), {}, {})", self.name(x), self.for_type(ty), self.for_expr(e1), self.for_expr(e2))
                } else {
                    write!(f, "Let({}, None, {}, {})", self.name(x), self.for_expr(e1), self.for_expr(e2))
                },
            Expr::Pair(_, e1, e2) =>
                write!(f, "Pair({}, {})", self.for_expr(e1), self.for_expr(e2)),
            Expr::UnPair(_, x1, x2, e1, e2) =>
                write!(f, "UnPair({}, {}, {}, {})", self.name(x1), self.name(x2), self.for_expr(e1), self.for_expr(e2)),
            Expr::InL(_, e) =>
                write!(f, "InL({})", self.for_expr(e)),
            Expr::InR(_, e) =>
                write!(f, "InR({})", self.for_expr(e)),
            Expr::Case(_, e0, x1, e1, x2, e2) =>
                write!(f, "Case({}, {}, {}, {}, {})", self.for_expr(e0), self.name(x1), self.for_expr(e1), self.name(x2), self.for_expr(e2)),
            Expr::Array(_, ref es) => {
                write!(f, "Array(")?;
                for (i, e) in es.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", self.for_expr(e))?;
                }
                write!(f, ")")
            },
            Expr::UnGen(_, ref e) =>
                write!(f, "UnGen({})", self.for_expr(e)),
            Expr::Delay(_, ref e) =>
                write!(f, "Delay({})", self.for_expr(e)),
            Expr::Box(_, ref e) =>
                write!(f, "Box({})", self.for_expr(e)),
            Expr::Unbox(_, ref e) =>
                write!(f, "Unbox({})", self.for_expr(e)),
            Expr::ClockApp(_, ref e, ref c) =>
                write!(f, "ClockApp({}, {})", self.for_expr(e), self.for_clock(c)),
            Expr::TypeApp(_, ref e, ref ty) =>
                write!(f, "TypeApp({}, {})", self.for_expr(e), self.for_type(ty)),
            Expr::Binop(_, op, ref e1, ref e2) =>
                write!(f, "Binop({:?}, {}, {})", op, self.for_expr(e1), self.for_expr(e2)),
            Expr::ExIntro(_, c, ref e) =>
                write!(f, "ExIntro({}, {})", self.for_clock(&c), self.for_expr(e)),
            Expr::ExElim(_, x1, x2, ref e1, ref e2) =>
                write!(f, "ExElim({}, {}, {}, {})", self.name(x1), self.name(x2), self.for_expr(e1), self.for_expr(e2)),
            Expr::ClockLam(_, x, e) =>
                write!(f, "ClockLam({}, {})", self.name(x), self.for_expr(e)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopLevelDefKind {
    Let,
    Def,
}

impl fmt::Display for TopLevelDefKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TopLevelDefKind::Let => write!(f, "let"),
            TopLevelDefKind::Def => write!(f, "def"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum TopLevelDefBody<'a, R> {
    Def { kind: TopLevelDefKind, type_: Type, expr: &'a Expr<'a, R> },
    Clock { freq: f32 },
}

impl<'a, R> TopLevelDefBody<'a, R> {
    pub fn get_expr(&self) -> Option<&'a Expr<'a, R>> {
        match *self {
            TopLevelDefBody::Def { expr, .. } => Some(expr),
            _ => None,
        }
    }

    pub fn get_type(&self) -> Option<&Type> {
        match *self {
            TopLevelDefBody::Def { ref type_, .. } => Some(type_),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TopLevelDef<'a, R> {
    pub name: Symbol,
    pub range: R,
    pub body: TopLevelDefBody<'a, R>,
}

pub struct PrettyTopLevelLet<'a, R> {
    interner: &'a DefaultStringInterner,
    def: &'a TopLevelDef<'a, R>,
}

impl<'a, R> fmt::Display for PrettyTopLevelLet<'a, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.interner.resolve(self.def.name).unwrap();
        match self.def.body {
            TopLevelDefBody::Def { kind, ref type_, expr: body } =>
                write!(f, "{} {}: {} = {};;",
                       kind,
                       name,
                       type_.pretty(self.interner),
                       body.pretty(self.interner)),
            TopLevelDefBody::Clock { freq } =>
                write!(f, "clock {} of frequency {} Hz;;",
                       name,
                       freq),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SourceFile<'a, R> {
    pub defs: Vec<TopLevelDef<'a, R>>,
}

impl<'a, R> SourceFile<'a, R> {
    pub fn pretty(&'a self, interner: &'a DefaultStringInterner) -> PrettySourceFile<'a, R> {
        PrettySourceFile { interner, file: self }
    }
}

pub struct PrettySourceFile<'a, R> {
    interner: &'a DefaultStringInterner,
    file: &'a SourceFile<'a, R>,
}

impl<'a, R> fmt::Display for PrettySourceFile<'a, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for def in self.file.defs.iter() {
            write!(f, "{}\n", PrettyTopLevelLet { interner: self.interner, def })?;
        }
        Ok(())
    }
}
