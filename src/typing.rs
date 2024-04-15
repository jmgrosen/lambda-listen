use core::fmt;
use std::collections::HashMap;
use std::cmp::Ordering;
use std::rc::Rc;

use num::{One, rational::Ratio};
use string_interner::DefaultStringInterner;

use crate::expr::{Symbol, Expr, Value, PrettyExpr};

// eventually this will get more complicated...
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ArraySize(usize);

impl ArraySize {
    pub fn from_const(n: usize) -> ArraySize {
        ArraySize(n)
    }

    pub fn as_const(&self) -> Option<usize> {
        Some(self.0)
    }
}

impl fmt::Display for ArraySize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Clock {
    pub coeff: Ratio<u32>,
    pub var: Symbol,
}

impl PartialOrd for Clock {
    fn partial_cmp(&self, other: &Clock) -> Option<Ordering> {
        if self.var == other.var {
            Some(self.coeff.cmp(&other.coeff))
        } else {
            None
        }
    }
}

impl Clock {
    pub fn pretty<'a>(&'a self, interner: &'a DefaultStringInterner) -> PrettyClock<'a> {
        PrettyClock { interner, clock: self }
    }

    /*
    // TODO: figure out better name for this
    pub fn compose(&self, other: &Clock) -> Option<Clock> {
        if self.var == other.var {
            Some(Clock { coeff: (self.coeff.recip() + other.coeff.recip()).recip(), var: self.var })
        } else {
            None
        }
    }
    */

    // TODO: figure out better name for this
    pub fn uncompose(&self, other: &Clock) -> Option<Clock> {
        if self.var == other.var {
            Some(Clock { coeff: (self.coeff.recip() - other.coeff.recip()).recip(), var: self.var })
        } else {
            None
        }
    }
}

pub struct PrettyClock<'a> {
    interner: &'a DefaultStringInterner,
    clock: &'a Clock,
}

impl<'a> fmt::Display for PrettyClock<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.clock.coeff.is_one() {
            write!(f, "{} ", self.clock.coeff)?;
        }
        write!(f, "{}", self.interner.resolve(self.clock.var).unwrap())
    }
}

pub struct PrettyTiming<'a> {
    interner: &'a DefaultStringInterner,
    timing: &'a [Clock],
}

impl<'a> fmt::Display for PrettyTiming<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let num_clocks = self.timing.len();
        for (i, clock) in self.timing.into_iter().enumerate() {
            write!(f, "{}", PrettyClock { interner: self.interner, clock })?;
            if i + 1 < num_clocks {
                write!(f, ", ")?;
            }
        }
        write!(f, "]")
    }
}


#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Type {
    Unit,
    Sample,
    Index,
    Stream(Clock, Box<Type>),
    Function(Box<Type>, Box<Type>),
    Product(Box<Type>, Box<Type>),
    Sum(Box<Type>, Box<Type>),
    Later(Clock, Box<Type>),
    Array(Box<Type>, ArraySize),
    Box(Box<Type>),
}

impl Type {
    pub fn pretty<'a>(&'a self, interner: &'a DefaultStringInterner) -> PrettyType<'a> {
        PrettyType { interner, ty: self }
    }

    fn is_stable(&self) -> bool {
        match *self {
            Type::Unit => true,
            Type::Sample => true,
            Type::Index => true,
            Type::Stream(_, _) => false,
            Type::Function(_, _) => false,
            Type::Product(ref ty1, ref ty2) => ty1.is_stable() && ty2.is_stable(),
            Type::Sum(ref ty1, ref ty2) => ty1.is_stable() && ty2.is_stable(),
            Type::Later(_, _) => false,
            Type::Array(ref ty, _) => ty.is_stable(),
            Type::Box(_) => true,
        }
    }

    /*
    fn later(clock: Clock, ty: Type) -> Type {
        if clock.coeff.is_zero() {
            ty
        } else {
            Type::Later(clock, Box::new(ty))
        }
    }
    */
}

fn parenthesize(f: &mut fmt::Formatter<'_>, p: bool, inner: impl FnOnce(&mut fmt::Formatter<'_>) -> fmt::Result) -> fmt::Result {
    if p {
        write!(f, "(")?;
        inner(f)?;
        write!(f, ")")
    } else {
        inner(f)
    }
}

pub struct PrettyType<'a> {
    interner: &'a DefaultStringInterner,
    ty: &'a Type,
}

impl<'a> PrettyType<'a> {
    fn for_type(&self, ty: &'a Type) -> PrettyType<'a> {
        PrettyType { ty, ..*self }
    }

    fn for_clock(&self, clock: &'a Clock) -> PrettyClock<'a> {
        PrettyClock { clock, interner: self.interner }
    }
}

impl<'a> PrettyType<'a> {
    fn fmt_prec(&self, f: &mut fmt::Formatter<'_>, prec: u8) -> fmt::Result {
        match *self.ty {
            Type::Unit =>
                write!(f, "unit"),
            Type::Sample =>
                write!(f, "sample"),
            Type::Index =>
                write!(f, "index"),
            Type::Stream(ref clock, ref ty) =>
                parenthesize(f, prec > 3, |f| {
                    // oh GOD this syntax will be noisy
                    write!(f, "~^({}) ", self.for_clock(clock))?;
                    self.for_type(ty).fmt_prec(f, 3)
                }),
            Type::Function(ref ty1, ref ty2) =>
                parenthesize(f, prec > 0, |f| {
                    self.for_type(ty1).fmt_prec(f, 1)?;
                    write!(f, " -> ")?;
                    self.for_type(ty2).fmt_prec(f, 0)
                }),
            Type::Product(ref ty1, ref ty2) =>
                parenthesize(f, prec > 2, |f| {
                    self.for_type(ty1).fmt_prec(f, 3)?;
                    write!(f, " * ")?;
                    self.for_type(ty2).fmt_prec(f, 2)
                }),
            Type::Sum(ref ty1, ref ty2) =>
                parenthesize(f, prec > 1, |f| {
                    self.for_type(ty1).fmt_prec(f, 2)?;
                    write!(f, " * ")?;
                    self.for_type(ty2).fmt_prec(f, 1)
                }),
            Type::Later(ref clock, ref ty) =>
                parenthesize(f, prec > 3, |f| {
                    write!(f, "|>^({})", self.for_clock(clock))?;
                    self.for_type(ty).fmt_prec(f, 3)
                }),
            Type::Array(ref ty, ref size) => {
                write!(f, "[")?;
                self.for_type(ty).fmt_prec(f, 0)?;
                write!(f, "; {}]", size)
            },
            Type::Box(ref ty) =>
                parenthesize(f, prec > 3, |f| {
                    write!(f, "[]")?;
                    self.for_type(ty).fmt_prec(f, 3)
                }),
        }
    }
}

impl<'a> fmt::Display for PrettyType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_prec(f, 0)
    }
}

// these should implicitly have boxes on them, i think? or, at least,
// they can always be used
pub type Globals = HashMap<Symbol, Type>;

// TODO: should probably find a more efficient representation of this,
// but it'll work for now
//
// TODO: want to keep variables around that are in lexical scope but
// removed due to tick-stripping, so that type errors can make more
// sense. but can't immediately figure out a nice way of doing that,
// so using this representation in the meantime.
#[derive(Clone)]
pub enum Ctx {
    Empty,
    Tick(Clock, Rc<Ctx>),
    Var(Symbol, Type, Rc<Ctx>),
}

impl Ctx {
    fn lookup(&self, x: Symbol) -> Option<(Vec<Clock>, &Type)> {
        match *self {
            Ctx::Empty => None,
            Ctx::Tick(c, ref next) =>
                next.lookup(x).map(|(mut cs, ty)| {
                    cs.push(c);
                    (cs, ty)
                }),
            Ctx::Var(y, ref ty, ref next) =>
                if x == y {
                    Some((Vec::new(), ty))
                } else {
                    next.lookup(x)
                },
        }
    }

    fn with_var(self, x: Symbol, ty: Type) -> Ctx {
        Ctx::Var(x, ty, Rc::new(self))
    }

    // TODO: optimize this for the case that it's kept the same?
    fn box_strengthen(&self) -> Ctx {
        match *self {
            Ctx::Empty => Ctx::Empty,
            Ctx::Tick(_, ref next) => next.box_strengthen(),
            Ctx::Var(x, ref ty, ref next) =>
                if ty.is_stable() {
                    Ctx::Var(x, ty.clone(), Rc::new(next.box_strengthen()))
                } else {
                    next.box_strengthen()
                },
        }
    }

    fn strip_tick(&self) -> Option<(Clock, Ctx)> {
        match *self {
            Ctx::Empty => None,
            Ctx::Tick(clock, ref next) => Some((clock, (**next).clone())),
            Ctx::Var(_, _, ref next) => next.strip_tick(),
        }
    }
}

#[derive(Debug)]
pub enum TypeError<'a, R> {
    MismatchingTypes { expr: &'a Expr<'a, R>, synth: Type, expected: Type },
    VariableNotFound { range: R, var: Symbol },
    BadArgument { range: R, arg_type: Type, fun: &'a Expr<'a, R>, arg: &'a Expr<'a, R>, arg_err: Box<TypeError<'a, R>> },
    NonFunctionApplication { range: R, purported_fun: &'a Expr<'a, R>, actual_type: Type },
    SynthesisUnsupported { expr: &'a Expr<'a, R> },
    BadAnnotation { range: R, expr: &'a Expr<'a, R>, purported_type: Type, err: Box<TypeError<'a, R>> },
    LetSynthFailure { range: R, var: Symbol, expr: &'a Expr<'a, R>, err: Box<TypeError<'a, R>> },
    LetCheckFailure { range: R, var: Symbol, expected_type: Type, expr: &'a Expr<'a, R>, err: Box<TypeError<'a, R>> },
    ForcingNonThunk { range: R, expr: &'a Expr<'a, R>, actual_type: Type },
    UnPairingNonProduct { range: R, expr: &'a Expr<'a, R>, actual_type: Type },
    CasingNonSum { range: R, expr: &'a Expr<'a, R>, actual_type: Type },
    CouldNotUnify { type1: Type, type2: Type },
    MismatchingArraySize { range: R, expected_size: ArraySize, found_size: usize },
    UnGenningNonStream { range: R, expr: &'a Expr<'a, R>, actual_type: Type },
    VariableTimingBad { range: R, var: Symbol, timing: Vec<Clock>, var_type: Type },
    ForcingWithNoTick { range: R, expr: &'a Expr<'a, R> },
    ForcingMismatchingClock { range: R, expr: &'a Expr<'a, R>, stripped_clock: Clock, synthesized_clock: Clock, remaining_type: Type },
    UnboxingNonBox { range: R, expr: &'a Expr<'a, R>, actual_type: Type },
}

impl<'a, R> TypeError<'a, R> {
    fn mismatching(expr: &'a Expr<'a, R>, synth: Type, expected: Type) -> TypeError<'a, R> {
        TypeError::MismatchingTypes { expr, synth, expected }
    }

    fn var_not_found(range: R, var: Symbol) -> TypeError<'a, R> {
        TypeError::VariableNotFound { range, var }
    }

    fn bad_argument(range: R, arg_type: Type, fun: &'a Expr<'a, R>, arg: &'a Expr<'a, R>, arg_err: TypeError<'a, R>) -> TypeError<'a, R> {
        TypeError::BadArgument { range, arg_type, fun, arg, arg_err: Box::new(arg_err) }
    }

    fn non_function_application(range: R, purported_fun: &'a Expr<'a, R>, actual_type: Type) -> TypeError<'a, R> {
        TypeError::NonFunctionApplication { range, purported_fun, actual_type }
    }

    fn synthesis_unsupported(expr: &'a Expr<'a, R>) -> TypeError<'a, R> {
        TypeError::SynthesisUnsupported { expr }
    }

    fn bad_annotation(range: R, expr: &'a Expr<'a, R>, purported_type: Type, err: TypeError<'a, R>) -> TypeError<'a, R> {
        TypeError::BadAnnotation { range, expr, purported_type, err: Box::new(err) }
    }

    fn let_failure(range: R, var: Symbol, expr: &'a Expr<'a, R>, err: TypeError<'a, R>) -> TypeError<'a, R> {
        TypeError::LetSynthFailure { range, var, expr, err: Box::new(err) }
    }

    fn forcing_non_thunk(range: R, expr: &'a Expr<'a, R>, actual_type: Type) -> TypeError<'a, R> {
        TypeError::ForcingNonThunk { range, expr, actual_type }
    }

    fn unpairing_non_product(range: R, expr: &'a Expr<'a, R>, actual_type: Type) -> TypeError<'a, R> {
        TypeError::UnPairingNonProduct { range, expr, actual_type }
    }

    fn casing_non_sum(range: R, expr: &'a Expr<'a, R>, actual_type: Type) -> TypeError<'a, R> {
        TypeError::CasingNonSum { range, expr, actual_type }
    }

    fn could_not_unify(type1: Type, type2: Type) -> TypeError<'a, R> {
        TypeError::CouldNotUnify { type1, type2 }
    }

    pub fn pretty(&'a self, interner: &'a DefaultStringInterner, program_text: &'a str) -> PrettyTypeError<'a, R> {
        PrettyTypeError { interner, program_text, error: self }
    }
}

pub struct PrettyTypeError<'a, R> {
    interner: &'a DefaultStringInterner,
    program_text: &'a str,
    error: &'a TypeError<'a, R>,
}

impl<'a, R> PrettyTypeError<'a, R> {
    fn for_error(&self, error: &'a TypeError<'a, R>) -> PrettyTypeError<'a, R> {
        PrettyTypeError { interner: self.interner, program_text: self.program_text, error }
    }

    fn for_expr(&self, expr: &'a Expr<'a, R>) -> PrettyExpr<'a, 'a, R> {
        expr.pretty(self.interner)
    }

    fn for_type(&self, ty: &'a Type) -> PrettyType<'a> {
        ty.pretty(self.interner)
    }

    fn for_timing(&self, timing: &'a [Clock]) -> PrettyTiming<'a> {
        PrettyTiming { interner: self.interner, timing }
    }

    fn for_clock(&self, clock: &'a Clock) -> PrettyClock<'a> {
        PrettyClock { interner: self.interner, clock }
    }
}

impl<'a, R> fmt::Display for PrettyTypeError<'a, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.error {
            TypeError::MismatchingTypes { expr, ref synth, ref expected } =>
                write!(f, "found {} to have type {} but expected {}", self.for_expr(expr), self.for_type(synth), self.for_type(expected)),
            TypeError::VariableNotFound { var, .. } =>
                write!(f, "variable \"{}\" not found", self.interner.resolve(var).unwrap()),
            TypeError::BadArgument { ref arg_type, fun, arg, ref arg_err, .. } =>
                write!(f, "found {} to take argument type {}, but argument {} does not have that type: {}",
                       self.for_expr(fun), self.for_type(arg_type), self.for_expr(arg), self.for_error(arg_err)),
            TypeError::NonFunctionApplication { purported_fun, ref actual_type, .. } =>
                write!(f, "trying to call {}, but found it to have type {}, which is not a function type",
                       self.for_expr(purported_fun), self.for_type(actual_type)),
            TypeError::SynthesisUnsupported { expr } =>
                write!(f, "don't know how to implement synthesis for {} yet", self.for_expr(expr)),
            TypeError::BadAnnotation { expr, ref purported_type, ref err, .. } =>
                write!(f, "bad annotation of expression {} as type {}: {}", self.for_expr(expr), self.for_type(purported_type), self.for_error(err)),
            TypeError::LetCheckFailure { var, expr, ref expected_type, ref err, .. } =>
                write!(f, "couldn't check variable {} to have type {} from definition {}: {}",
                       self.interner.resolve(var).unwrap(), self.for_type(expected_type), self.for_expr(expr), self.for_error(err)),
            TypeError::LetSynthFailure { var, expr, ref err, .. } =>
                write!(f, "couldn't infer the type of variable {} from definition {}: {}",
                       self.interner.resolve(var).unwrap(), self.for_expr(expr), self.for_error(err)),
            TypeError::ForcingNonThunk { expr, ref actual_type, .. } =>
                write!(f, "tried to force expression {} of type {}, which is not a thunk", self.for_expr(expr), self.for_type(actual_type)),
            TypeError::UnPairingNonProduct { expr, ref actual_type, .. } =>
                write!(f, "tried to unpair expression {} of type {}, which is not a product", self.for_expr(expr), self.for_type(actual_type)),
            TypeError::CasingNonSum { expr, ref actual_type, .. } =>
                write!(f, "tried to case on expression {} of type {}, which is not a sum", self.for_expr(expr), self.for_type(actual_type)),
            TypeError::CouldNotUnify { ref type1, ref type2 } =>
                write!(f, "could not unify types {} and {}", self.for_type(type1), self.for_type(type2)),
            TypeError::MismatchingArraySize { ref expected_size, found_size, .. } =>
                write!(f, "expected array of size {} but found size {}", expected_size, found_size),
            TypeError::UnGenningNonStream { expr, ref actual_type, .. } =>
                write!(f, "expected stream to ungen, but found {} of type {}", self.for_expr(expr), self.for_type(actual_type)),
            TypeError::VariableTimingBad { var, ref timing, ref var_type, .. } =>
                write!(f, "found use of variable {}, but it has timing {} and non-stable type {}",
                       self.interner.resolve(var).unwrap(), self.for_timing(timing), self.for_type(var_type)),
            TypeError::ForcingWithNoTick { expr, .. } =>
                write!(f, "trying to force expression {}, but there is no tick in the context!", self.for_expr(expr)),
            TypeError::ForcingMismatchingClock { expr, stripped_clock, synthesized_clock, ref remaining_type, .. } =>
                write!(f, "trying to force expression {} of type {}, but the most recent tick in the context has clock {}",
                       self.for_expr(expr), self.for_type(&Type::Later(synthesized_clock, Box::new(remaining_type.clone()))), self.for_clock(&stripped_clock)),
            TypeError::UnboxingNonBox { expr, ref actual_type, .. } =>
                write!(f, "trying to unbox expression {}, but found it has type {}, which is not a box",
                       self.for_expr(expr), self.for_type(actual_type)),
        }
    }
}

pub struct Typechecker {
    pub globals: Globals,
}


// rules to port/verify:
// - [X] delay
// - [X] adv
// - [X] unbox
// - [X] box
// - [X] gen
// - [X] ungen
// - [ ] proj?
// - [X] fix

impl Typechecker {
    pub fn check<'a, R: Clone>(&self, ctx: &Ctx, expr: &'a Expr<'a, R>, ty: &Type) -> Result<(), TypeError<'a, R>> {
        match (ty, expr) {
            (&Type::Unit, &Expr::Val(_, Value::Unit)) =>
                Ok(()),
            (&Type::Function(ref ty1, ref ty2), &Expr::Lam(_, x, e)) => {
                let new_ctx = ctx.clone().with_var(x, (**ty1).clone());
                self.check(&new_ctx, e, ty2)
            },
            (_, &Expr::Lob(_, clock, x, e)) => {
                let rec_ty = Type::Box(Box::new(Type::Later(clock, Box::new(ty.clone()))));
                let new_ctx = ctx.box_strengthen().with_var(x, rec_ty);
                self.check(&new_ctx, e, ty)
            },
            // if we think of streams as infinitary products, it makes sense to *check* their introduction, right?
            (&Type::Stream(clock, ref ty1), &Expr::Gen(_, eh, et)) => {
                // TODO: probably change once we figure out the stream semantics we actually want
                self.check(&ctx, eh, ty1)?;
                self.check(&ctx, et, &Type::Later(clock, Box::new(ty.clone())))
            },
            (_, &Expr::LetIn(ref r, x, None, e1, e2)) =>
                match self.synthesize(ctx, e1) {
                    Ok(ty_x) => {
                        let new_ctx = ctx.clone().with_var(x, ty_x);
                        self.check(&new_ctx, e2, ty)
                    },
                    Err(err) =>
                        Err(TypeError::let_failure(r.clone(), x, e1, err)),
                },
            (_, &Expr::LetIn(ref r, x, Some(ref ty), e1, e2)) => {
                if let Err(err) = self.check(ctx, e1, ty) {
                    return Err(TypeError::LetCheckFailure {
                        range: r.clone(),
                        var: x,
                        expected_type: ty.clone(),
                        expr: e1,
                        err: Box::new(err)
                    });
                }
                let new_ctx = ctx.clone().with_var(x, ty.clone());
                self.check(&new_ctx, e2, ty)
            },
            (&Type::Product(ref ty1, ref ty2), &Expr::Pair(_, e1, e2)) => {
                self.check(ctx, e1, ty1)?;
                self.check(ctx, e2, ty2)
            },
            (_, &Expr::UnPair(ref r, x1, x2, e0, e)) =>
                match self.synthesize(ctx, e0)? {
                    Type::Product(ty1, ty2) => {
                        let new_ctx = ctx.clone().with_var(x1, *ty1).with_var(x2, *ty2);
                        self.check(&new_ctx, e, ty)
                    },
                    ty =>
                        Err(TypeError::unpairing_non_product(r.clone(), e0, ty)),
                },
            (&Type::Sum(ref ty1, _), &Expr::InL(_, e)) =>
                self.check(ctx, e, ty1),
            (&Type::Sum(_, ref ty2), &Expr::InR(_, e)) =>
                self.check(ctx, e, ty2),
            (_, &Expr::Case(ref r, e0, x1, e1, x2, e2)) =>
                match self.synthesize(ctx, e0)? {
                    Type::Sum(ty1, ty2) => {
                        let old_ctx = Rc::new(ctx.clone());
                        let ctx1 = Ctx::Var(x1, *ty1, old_ctx.clone());
                        self.check(&ctx1, e1, ty)?;
                        let ctx2 = Ctx::Var(x2, *ty2, old_ctx);
                        self.check(&ctx2, e2, ty)
                    },
                    ty =>
                        Err(TypeError::casing_non_sum(r.clone(), e0, ty)),
                },
            (&Type::Array(ref ty, ref size), &Expr::Array(ref r, ref es)) =>
                if size.as_const() != Some(es.len()) {
                    Err(TypeError::MismatchingArraySize { range: r.clone(), expected_size: size.clone(), found_size: es.len() })
                } else {
                    for e in es.iter() {
                        self.check(ctx, e, ty)?;
                    }
                    Ok(())
                },
            (&Type::Later(clock, ref ty), &Expr::Delay(_, e)) => {
                let new_ctx = Ctx::Tick(clock, Rc::new(ctx.clone()));
                self.check(&new_ctx, e, ty)
            },
            (&Type::Box(ref ty), &Expr::Box(_, e)) =>
                self.check(&ctx.box_strengthen(), e, ty),
            (_, _) => {
                let synthesized = self.synthesize(ctx, expr)?;
                if subtype(ctx, &synthesized, ty) {
                    Ok(())
                } else {
                    Err(TypeError::mismatching(expr, synthesized, ty.clone()))
                }
            }
        }
    }

    pub fn synthesize<'a, R: Clone>(&self, ctx: &Ctx, expr: &'a Expr<'a, R>) -> Result<Type, TypeError<'a, R>> {
        match expr {
            &Expr::Val(_, ref v) =>
                match *v {
                    Value::Unit => Ok(Type::Unit),
                    Value::Sample(_) => Ok(Type::Sample),
                    Value::Index(_) => Ok(Type::Index),
                    _ => panic!("trying to type {v:?} but that kind of value shouldn't be created yet?"),
                }
            &Expr::Var(ref r, x) =>
                if let Some((timing, ty)) = ctx.lookup(x) {
                    if timing.is_empty() || ty.is_stable() {
                        Ok(ty.clone())
                    } else {
                        Err(TypeError::VariableTimingBad {
                            range: r.clone(),
                            var: x,
                            timing,
                            var_type: ty.clone(),
                        })
                    }
                } else if let Some(ty) = self.globals.get(&x) {
                    Ok(ty.clone())
                } else {
                    Err(TypeError::var_not_found(r.clone(), x))
                },
            &Expr::Annotate(ref r, e, ref ty) =>
                match self.check(ctx, e, ty) {
                    Ok(()) => Ok(ty.clone()),
                    Err(err) => Err(TypeError::bad_annotation(r.clone(), e, ty.clone(), err)),
                },
            &Expr::App(ref r, e1, e2) =>
                match self.synthesize(ctx, e1)? {
                    Type::Function(ty_a, ty_b) => {
                        match self.check(ctx, e2, &ty_a) {
                            Ok(()) => Ok(*ty_b),
                            Err(arg_err) => Err(TypeError::bad_argument(r.clone(), *ty_a, e1, e2, arg_err)),
                        }
                    },
                    ty =>
                        Err(TypeError::non_function_application(r.clone(), e1, ty)),
                },
            &Expr::Force(ref r, e1) => {
                // TODO: something fancier here?
                let Some((stripped_clock, stripped_ctx)) = ctx.strip_tick() else {
                    return Err(TypeError::ForcingWithNoTick { range: r.clone(), expr: e1 });
                };
                match self.synthesize(&stripped_ctx, e1)? {
                    Type::Later(synthesized_clock, ty) =>
                        if stripped_clock == synthesized_clock {
                            Ok(*ty)
                        } else {
                            Err(TypeError::ForcingMismatchingClock {
                                range: r.clone(),
                                expr: e1,
                                stripped_clock,
                                synthesized_clock,
                                remaining_type: *ty,
                            })
                        },
                    ty => Err(TypeError::forcing_non_thunk(r.clone(), e1, ty)),
                }
            },
            &Expr::LetIn(ref r, x, None, e1, e2) =>
                match self.synthesize(ctx, e1) {
                    Ok(ty_x) => {
                        let new_ctx = ctx.clone().with_var(x, ty_x);
                        self.synthesize(&new_ctx, e2)
                    },
                    Err(err) =>
                        Err(TypeError::let_failure(r.clone(), x, e1, err)),
                },
            &Expr::LetIn(ref r, x, Some(ref ty), e1, e2) => {
                if let Err(err) = self.check(ctx, e1, ty) {
                    return Err(TypeError::LetCheckFailure {
                        range: r.clone(),
                        var: x,
                        expected_type: ty.clone(),
                        expr: e1,
                        err: Box::new(err)
                    });
                }
                let new_ctx = ctx.clone().with_var(x, ty.clone());
                self.synthesize(&new_ctx, e2)
            },
            &Expr::UnPair(ref r, x1, x2, e0, e) =>
                match self.synthesize(ctx, e0)? {
                    Type::Product(ty1, ty2) => {
                        let new_ctx = ctx.clone().with_var(x1, *ty1).with_var(x2, *ty2);
                        self.synthesize(&new_ctx, e)
                    },
                    ty =>
                        Err(TypeError::unpairing_non_product(r.clone(), e0, ty)),
                }
            &Expr::Case(ref r, e0, x1, e1, x2, e2) =>
                match self.synthesize(ctx, e0)? {
                    Type::Sum(ty1, ty2) => {
                        let old_ctx = Rc::new(ctx.clone());
    
                        let ctx1 = Ctx::Var(x1, *ty1, old_ctx.clone());
                        let ty_out1 = self.synthesize(&ctx1, e1)?;
    
                        let ctx2 = Ctx::Var(x2, *ty2, old_ctx);
                        let ty_out2 = self.synthesize(&ctx2, e2)?;
    
                        meet(ctx, ty_out1, ty_out2)
                    },
                    ty =>
                        Err(TypeError::casing_non_sum(r.clone(), e0, ty)),
                },
            &Expr::UnGen(ref r, e) =>
                match self.synthesize(ctx, e)? {
                    Type::Stream(clock, ty) =>
                        Ok(Type::Product(ty.clone(), Box::new(Type::Later(clock, Box::new(Type::Stream(clock, ty)))))),
                    ty =>
                        Err(TypeError::UnGenningNonStream { range: r.clone(), expr: e, actual_type: ty }),
                }
            &Expr::Unbox(ref r, e) =>
                match self.synthesize(ctx, e)? {
                    Type::Box(ty) =>
                        Ok(*ty),
                    ty =>
                        Err(TypeError::UnboxingNonBox { range: r.clone(), expr: e, actual_type: ty }),
                },
            _ =>
                Err(TypeError::synthesis_unsupported(expr)),
        }
    }
}

// at the moment this implements an equivalence relation, but we'll
// probably want it to be proper subtyping at some point, so let's
// just call it that
//
// terminating: the sum of the sizes of the types decreases
fn subtype(ctx: &Ctx, ty1: &Type, ty2: &Type) -> bool {
    match (ty1, ty2) {
        (&Type::Unit, &Type::Unit) =>
            true,
        (&Type::Sample, &Type::Sample) =>
            true,
        (&Type::Index, &Type::Index) =>
            true,
        (&Type::Stream(ref c1, ref ty1p), &Type::Stream(ref c2, ref ty2p)) =>
            c1 == c2 && subtype(ctx, ty1p, ty2p),
        (&Type::Function(ref ty1a, ref ty1b), &Type::Function(ref ty2a, ref ty2b)) =>
            subtype(ctx, ty2a, ty1a) && subtype(ctx, ty1b, ty2b),
        (&Type::Product(ref ty1a, ref ty1b), &Type::Product(ref ty2a, ref ty2b)) =>
            subtype(ctx, ty1a, ty2a) && subtype(ctx, ty1b, ty2b),
        (&Type::Sum(ref ty1a, ref ty1b), &Type::Sum(ref ty2a, ref ty2b)) =>
            subtype(ctx, ty1a, ty2a) && subtype(ctx, ty1b, ty2b),
        (&Type::Later(ref c1, ref ty1p), &Type::Later(ref c2, ref ty2p)) =>
            match c1.partial_cmp(c2) {
                Some(Ordering::Less) => {
                    // unwrap safety: we've already verified that c1 < c2
                    let rem = c1.uncompose(c2).unwrap();
                    subtype(ctx, &Type::Later(rem, ty1p.clone()), ty2p)
                },
                Some(Ordering::Equal) =>
                    subtype(ctx, ty1p, ty2p),
                Some(Ordering::Greater) => {
                    // unwrap safety: we've already verified that c2 < c1
                    let rem = c2.uncompose(c1).unwrap();
                    subtype(ctx, ty1p, &Type::Later(rem, ty2p.clone()))
                },
                None =>
                    false,
            }
        (&Type::Array(ref ty1p, ref n1), &Type::Array(ref ty2p, ref n2)) =>
            subtype(ctx, ty1p, ty2p) && n1 == n2,
        (&Type::Box(ref ty1p), &Type::Box(ref ty2p)) =>
            subtype(ctx, ty1p, ty2p),
        (_, _) =>
            false,
    }
}

fn meet<'a, R>(ctx: &Ctx, ty1: Type, ty2: Type) -> Result<Type, TypeError<'a, R>> {
    if subtype(ctx, &ty1, &ty2) {
        Ok(ty2)
    } else if subtype(ctx, &ty2, &ty1) {
        Ok(ty1)
    } else {
        Err(TypeError::could_not_unify(ty1, ty2))
    }
}

#[cfg(test)]
mod test {
    use crate::expr::Value;
    use super::*;
    
    fn s(i: usize) -> Symbol { string_interner::Symbol::try_from_usize(i).unwrap() }

    #[test]
    fn try_out() {
        let ev = Expr::Val((), Value::Unit);
        let e = Expr::Annotate((), &ev, Type::Unit);
        let checker = Typechecker { globals: HashMap::new() };

        assert_eq!(checker.synthesize(&Ctx::Empty, &e).unwrap(), Type::Unit);
    }

    #[test]
    fn test_fn() {
        let e_x = Expr::Var((), s(0));
        let e = Expr::Lam((), s(0), &e_x);
        let checker = Typechecker { globals: HashMap::new() };

        assert!(checker.check(&Ctx::Empty, &e, &Type::Function(Box::new(Type::Unit), Box::new(Type::Unit))).is_ok());
        assert!(checker.check(&Ctx::Empty, &e, &Type::Function(Box::new(Type::Index), Box::new(Type::Unit))).is_err());
    }
}
