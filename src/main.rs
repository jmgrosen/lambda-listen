use std::time::Instant;

use string_interner::StringInterner;

use typed_arena::Arena;

mod expr;
mod builtin;
mod parse;
mod interp;
mod typing;

use builtin::make_builtins;
use interp::get_samples;
use parse::Parser;

fn main() {
    let annot_arena = Arena::new();
    let mut interner = StringInterner::new();

    let mut try_typing = |code| {
        let mut parser = Parser::new(&mut interner, &annot_arena);
        let expr = parser.parse(code).unwrap();
        println!("{}", expr.pretty(&interner));
        match typing::synthesize(&typing::Ctx::new(), &expr) {
            Ok(ty) => println!("synthesized type: {:?}", ty),
            Err(err) => println!("type error: {}", err.pretty(&interner, code)),
        }
        println!("");
    };

    try_typing(r"(\x. x) : index -> unit");
    try_typing(r"(\x. x) : sample -> sample");
    try_typing(r"(\x. y) : sample -> sample");
    try_typing(r"((\x. x) : sample -> sample) 3");
    try_typing(r"((\x. x) : sample -> sample) 1.5");
    try_typing(r"1.5 2");
    try_typing(r"1.5");

    /*
    let source_code = r"let pifourth = (div pi 4.0 : sample) in (&s. \x. sin x :: !s (add x pifourth)) 0.0";

    let expr = parser.parse(source_code).unwrap();
    println!("\n{}", expr.pretty(&interner));

    let arena = Arena::new();
    let expr_unannotated = expr.map_ext(&arena, &(|_| ()));

    // println!("{:?}", interp(&Env::new(), &expr_unannotated));

    let builtins = make_builtins(&mut interner);

    let mut samples = [0f32; 48000];
    let before = Instant::now();
    let result = get_samples(&builtins, &expr_unannotated, &mut samples[..]);
    let elapsed = before.elapsed();
    println!("\n{:?}", result);
    // println!("{samples:?}");
    println!("took {}ms", elapsed.as_millis());
    */
}
