use super::{
  simplify, Cond, Expr, Field, Instr, Lit, Op, Packet, RType, Type, Value, VarBlock, VarKind,
};

fn cond(expr: Expr) -> Cond { Cond::Bool(expr) }
fn field(name: &str) -> Expr { Expr::new(Value::Field(name.into())) }
fn lit(lit: impl Into<Lit>) -> Expr { Expr::new(Value::Lit(lit.into())) }
fn local(id: usize) -> Expr { Expr::new(Value::Var(id)) }
fn as_(name: &str) -> Op { Op::As(name.into()) }
fn and(val: Expr) -> Op { Op::BitAnd(val) }
fn packet() -> Expr { Expr::new(Value::packet_var()) }
fn block(block: Vec<Instr>, locals: usize) -> VarBlock {
  let mut vars = vec![VarKind::This, VarKind::Arg];
  for _ in 0..locals {
    vars.push(VarKind::Local);
  }
  VarBlock { vars, block }
}

macro_rules! call {
  ( $name:ident [ $($arg:expr),* ] ) => {
    Op::Call("tcp::Packet".into(), stringify!($name).into(), vec![$($arg),*])
  };
  ( ::$name:ident [ $($arg:expr),* ] ) => {
    Op::Call("".into(), stringify!($name).into(), vec![$($arg),*])
  };
  ( $class:ident::$name:ident [ $($arg:expr),* ] ) => {
    Op::Call(stringify!($class).into(), stringify!($name).into(), vec![$($arg),*])
  }
}

macro_rules! fields {
  [ $($name:ident: $ty:ident),* ] => {
    vec![ $( Field {
      name: stringify!($name).into(),
      ty: Type::$ty,
      reader_type: None,
      initialized: false,
      option: false,
    } ),* ]
  }
}

fn generate(p: &mut Packet) {
  simplify::finish(p);
  p.find_reader_types_gen_writer();

  let mut gen = crate::gen::CodeGen::new();
  gen.write_line("READER:");
  super::gen::write_from_tcp(&mut gen, &p, crate::VERSIONS[0]);
  gen.write_line("");
  gen.write_line("WRITER:");
  super::gen::write_to_tcp(&mut gen, &p, crate::VERSIONS[0]);

  println!("{}", gen.into_output());
}

#[test]
fn simple_writer_test() {
  let reader = vec![
    Instr::Set("foo".into(), packet().op(call!(read_i8[]))),
    Instr::Set("bar".into(), packet().op(call!(read_i16[]))),
    Instr::Set("baz".into(), packet().op(call!(read_i32[]))),
  ];
  let writer = vec![
    Instr::Expr(packet().op(call!(write_i8[field("foo").op(Op::Deref).op(as_("i8"))]))),
    Instr::Expr(packet().op(call!(write_i16[field("bar").op(Op::Deref).op(as_("i16"))]))),
    Instr::Expr(packet().op(call!(write_i32[field("baz").op(Op::Deref)]))),
  ];
  let fields = vec![
    Field {
      name:        "foo".into(),
      ty:          Type::Int,
      reader_type: Some(RType::new("i8")),
      initialized: true,
      option:      false,
    },
    Field {
      name:        "bar".into(),
      ty:          Type::Int,
      reader_type: Some(RType::new("i16")),
      initialized: true,
      option:      false,
    },
    Field {
      name:        "baz".into(),
      ty:          Type::Int,
      reader_type: Some(RType::new("i32")),
      initialized: true,
      option:      false,
    },
  ];
  let mut p = Packet {
    extends: "".into(),
    class:   "".into(),
    name:    "Bar".into(),
    fields:  fields![foo: Int, bar: Int, baz: Int],
    reader:  block(reader, 0),
    writer:  block(vec![], 0),
  };
  generate(&mut p);

  assert_eq!(p.fields, fields);
  assert_eq!(p.writer.block, writer);
}

#[test]
fn conditional_writer_test() {
  let reader = vec![
    Instr::Set("foo".into(), packet().op(call!(read_i32[]))),
    Instr::If(
      Cond::Greater(field("foo"), lit(0)),
      vec![Instr::Set("baz".into(), packet().op(call!(read_i32[])))],
      vec![],
    ),
  ];
  let writer = vec![
    Instr::Expr(packet().op(call!(write_i32[field("foo").op(Op::Deref)]))),
    Instr::If(
      Cond::Greater(field("foo"), lit(0)),
      vec![Instr::Expr(packet().op(call!(write_i32[field("baz").op(Op::Deref)])))],
      vec![],
    ),
  ];
  let fields = vec![
    Field {
      name:        "foo".into(),
      ty:          Type::Int,
      reader_type: Some(RType::new("i32")),
      initialized: true,
      option:      false,
    },
    Field {
      name:        "baz".into(),
      ty:          Type::Int,
      reader_type: Some(RType::new("i32")),
      initialized: false,
      option:      true,
    },
  ];
  let mut p = Packet {
    extends: "".into(),
    class:   "".into(),
    name:    "Bar".into(),
    fields:  fields![foo: Int, bar: Int, baz: Int],
    reader:  block(reader, 0),
    writer:  block(vec![], 0),
  };
  generate(&mut p);

  assert_eq!(p.fields, fields);
  assert_eq!(p.writer.block, writer);

  let reader = vec![
    Instr::Let(2, packet().op(call!(read_i32[]))),
    Instr::If(
      Cond::Greater(local(2), lit(0)),
      vec![Instr::Set("baz".into(), packet().op(call!(read_i32[])))],
      vec![],
    ),
  ];
  let writer = vec![
    Instr::Expr(packet().op(call!(
      write_i32[field("baz").op(call!(Option::is_some[])).op(as_("u8")).op(call!(::into[]))]
    ))),
    Instr::If(
      cond(field("baz").op(call!(Option::is_some[]))),
      vec![Instr::Expr(packet().op(call!(write_i32[field("baz").op(Op::Deref)])))],
      vec![],
    ),
  ];
  let fields = vec![Field {
    name:        "baz".into(),
    ty:          Type::Int,
    reader_type: Some(RType::new("i32")),
    initialized: false,
    option:      true,
  }];
  let mut p = Packet {
    extends: "".into(),
    class:   "".into(),
    name:    "Bar".into(),
    fields:  fields![foo: Int, bar: Int, baz: Int],
    reader:  block(reader, 1),
    writer:  block(vec![], 0),
  };
  generate(&mut p);

  assert_eq!(p.writer.block, writer);
  assert_eq!(p.fields, fields);
}

#[test]
fn multi_conditional_writer_test() {
  let reader = vec![
    Instr::Let(2, packet().op(call!(read_u8[]))),
    Instr::If(
      Cond::Neq(local(2).op(and(lit(1))), lit(0)),
      vec![Instr::Set("foo".into(), packet().op(call!(read_i32[])))],
      vec![],
    ),
    Instr::If(
      Cond::Neq(local(2).op(and(lit(2))), lit(0)),
      vec![Instr::Set("bar".into(), packet().op(call!(read_i32[])))],
      vec![],
    ),
    Instr::If(
      Cond::Neq(local(2).op(and(lit(4))), lit(0)),
      vec![Instr::Set("baz".into(), packet().op(call!(read_i32[])))],
      vec![],
    ),
  ];
  let writer = vec![
    Instr::Let(2, lit(0)),
    Instr::Expr(packet().op(call!(
      write_i32[field("baz").op(call!(Option::is_some[])).op(as_("u8")).op(call!(::into[]))]
    ))),
    Instr::If(
      cond(field("baz").op(call!(Option::is_some[]))),
      vec![Instr::Expr(packet().op(call!(write_i32[field("baz").op(Op::Deref)])))],
      vec![],
    ),
  ];
  let fields = vec![Field {
    name:        "baz".into(),
    ty:          Type::Int,
    reader_type: Some(RType::new("i32")),
    initialized: false,
    option:      true,
  }];
  let mut p = Packet {
    extends: "".into(),
    class:   "".into(),
    name:    "Bar".into(),
    fields:  fields![foo: Int, bar: Int, baz: Int],
    reader:  block(reader, 1),
    writer:  block(vec![], 0),
  };
  generate(&mut p);

  assert_eq!(p.writer.block, writer);
  assert_eq!(p.fields, fields);
}
