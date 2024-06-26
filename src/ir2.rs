use crate::ir1::{DebruijnIndex, Expr as HExpr, Op, Global, Con, Value};
use crate::util::ArenaPlus;

#[derive(Debug)]
pub enum Expr<'a> {
    Var(DebruijnIndex),
    If(&'a Expr<'a>, &'a Expr<'a>, &'a Expr<'a>),
    Let(&'a [&'a Expr<'a>], &'a Expr<'a>),
    Op(Op, &'a [&'a Expr<'a>]),
    #[allow(unused)]
    CallDirect(Global, &'a [&'a Expr<'a>]),
    CallIndirect(&'a Expr<'a>, &'a [&'a Expr<'a>]),
}

#[derive(Debug)]
pub enum GlobalDef<'a> {
    Func {
        rec: bool,
        arity: u32,
        env_size: u32, // do we need this here?
        body: &'a Expr<'a>,
    },
    ClosedExpr {
        body: &'a Expr<'a>,
    },
}

impl<'a> GlobalDef<'a> {
    pub fn arity(&self) -> Option<u32> {
        match *self {
            GlobalDef::Func { arity, .. } =>
                Some(arity),
            GlobalDef::ClosedExpr { .. } =>
                None,
        }
    }
}

pub struct Translator<'a> {
    pub arena: &'a ArenaPlus<'a, Expr<'a>>,
    pub globals: Vec<GlobalDef<'a>>,
}

impl<'a> Translator<'a> {
    pub fn translate<'b>(&mut self, expr: &'b HExpr<'b>) -> &'a Expr<'a> {
        let arena = self.arena;
        match *expr {
            HExpr::Var(i) =>
                arena.alloc(Expr::Var(i)),
            HExpr::Val(v) =>
                arena.alloc(Expr::Op(Op::Const(v), &[])),
            HExpr::Glob(g) =>
                // TODO: treat funcs and values/closedexprs differently?
                self.arena.alloc(Expr::Op(Op::LoadGlobal(g), &[])),
            HExpr::Lam(Some(ref used), arity, body) =>
                self.build_new_closure(false, arity, used, body),
            HExpr::App(e1, es) => {
                let e1t = self.translate(e1);
                arena.alloc(Expr::CallIndirect(
                    e1t,
                    arena.alloc_slice_r(es.iter().map(|e| self.translate(e)))
                ))
            },
            HExpr::Unbox(e) =>
                arena.alloc(Expr::CallIndirect(self.translate(e), &[])),
            HExpr::Box(Some(ref used), e) =>
                self.build_new_closure(false, 0, used, e),
            HExpr::Lob(Some(ref used), e) => {
                // this is real weird
                let inner_used: Vec<DebruijnIndex> = (0..=used.len() as u32).map(DebruijnIndex).collect();
                let inner_closure = self.build_new_closure(false, 0, &inner_used, e);
                let closure_builder = self.build_new_closure_inner(true, 0, used, inner_closure);
                arena.alloc(Expr::Let(
                    // ... what.
                    arena.alloc_slice([closure_builder]),
                    arena.alloc(Expr::CallIndirect(
                        arena.alloc(Expr::CallIndirect(
                            arena.alloc(Expr::Var(DebruijnIndex(0))),
                            &[]
                        )),
                        &[]
                    ))
                ))
            },
            HExpr::LetIn(e1, e2) => {
                let e1t = self.translate(e1);
                let e2t = self.translate(e2);
                arena.alloc(Expr::Let(arena.alloc_slice([e1t]), e2t))
            },
            HExpr::If(e0, e1, e2) => {
                let e0t = self.translate(e0);
                let e1t = self.translate(e1);
                let e2t = self.translate(e2);
                arena.alloc(Expr::If(e0t, e1t, e2t))
            },
            HExpr::Con(con, es) => {
                let constructed = self.build_constructor(con, es);
                arena.alloc(Expr::Op(Op::AllocAndFill, constructed))
            },
            HExpr::Op(op, es) => {
                let args = arena.alloc_slice_r(es.iter().map(|e| self.translate(e)));
                arena.alloc(Expr::Op(op, args))
            },
            HExpr::Delay(Some(ref used), e) =>
                self.build_new_closure(false, 0, used, e),
            HExpr::Adv(e) =>
                arena.alloc(Expr::Op(Op::Advance, arena.alloc_slice([self.translate(e)]))),
            HExpr::Lam(None, _, _) |
            HExpr::Box(None, _) |
            HExpr::Lob(None, _) |
            HExpr::Delay(None, _) =>
                panic!("you really must do the annotating and shifting first"),
        }
    }

    fn build_new_closure<'b>(&mut self, rec: bool, arity: u32, used: &[DebruijnIndex], body: &'b HExpr<'b>) -> &'a Expr<'a> {
        let bodyp = self.translate(body);
        self.build_new_closure_inner(rec, arity, used, bodyp)
    }

    fn build_new_closure_inner<'b>(&mut self, rec: bool, arity: u32, used: &[DebruijnIndex], body: &'a Expr<'a>) -> &'a Expr<'a> {
        let func_idx = self.globals.len();
        self.globals.push(GlobalDef::Func {
            rec,
            arity,
            env_size: used.len() as u32,
            body,
        });
        let closure_env_args = self.arena.alloc_slice_r(used.iter().map(|&i| self.arena.alloc(Expr::Var(i))));
        self.arena.alloc(Expr::Op(Op::BuildClosure(Global(func_idx as u32)), closure_env_args))
    }

    fn build_constructor<'b>(&mut self, con: Con, args: &'b [&'b HExpr<'b>]) -> &'a [&'a Expr<'a>] {
        let arena = self.arena;
        match con {
            // this will do for now, i think
            Con::Stream |
            Con::Array | // right now we have no array size polymorphism so all array sizes are static
            Con::Pair |
            Con::ClockEx =>
                arena.alloc_slice_r(args.iter().map(|e| self.translate(e))),
            Con::InL => {
                assert!(args.len() == 1);
                let one = arena.alloc(Expr::Op(Op::Const(Value::Index(0)), &[]));
                arena.alloc_slice([one, self.translate(args[0])])
            },
            Con::InR => {
                assert!(args.len() == 1);
                let one = arena.alloc(Expr::Op(Op::Const(Value::Index(1)), &[]));
                arena.alloc_slice([one, self.translate(args[0])])
            },
        }
    }
}
