use super::*;

fn e(kind: ExprKind, ty: Type) -> Expr {
    Expr { kind, ty, line: 1 }
}

/// `let p = P(1, 2); let q = p; q.x = 7; keep = q.x + p.y` splits
/// both records away: no `Record`, `GetField` or `SetField` remains.
#[test]
fn unshared_records_become_locals() {
    let shapes = vec![Shape::Record {
        drop: false,
        name: "P".into(),
        fields: vec![
            FieldShape {
                name: "x".into(),
                ty: Type::I64,
                secret: false,
            },
            FieldShape {
                name: "y".into(),
                ty: Type::I64,
                secret: false,
            },
        ],
    }];
    let rec = Type::Ref(0);
    let record = |a: i128, b: i128| {
        e(
            ExprKind::Record {
                variant: 0,
                fields: vec![
                    e(ExprKind::Int(a), Type::I64),
                    e(ExprKind::Int(b), Type::I64),
                ],
            },
            rec,
        )
    };
    let body = Block {
        stmts: vec![
            Stmt::Let(0, record(1, 2)),
            Stmt::Let(1, e(ExprKind::Local(0), rec)),
            Stmt::Expr(e(
                ExprKind::Assign(
                    1,
                    Box::new(e(
                        ExprKind::SetField(
                            Box::new(e(ExprKind::Local(1), rec)),
                            0,
                            Box::new(e(ExprKind::Int(7), Type::I64)),
                        ),
                        rec,
                    )),
                ),
                Type::Unit,
            )),
            Stmt::Let(
                2,
                e(
                    ExprKind::Binary(
                        BinOp::Add,
                        Box::new(e(
                            ExprKind::GetField(Box::new(e(ExprKind::Local(1), rec)), 0),
                            Type::I64,
                        )),
                        Box::new(e(
                            ExprKind::GetField(Box::new(e(ExprKind::Local(0), rec)), 1),
                            Type::I64,
                        )),
                    ),
                    Type::I64,
                ),
            ),
        ],
        value: None,
    };
    let mut f = Function {
        name: "t".into(),
        params: Vec::new(),
        ret: Type::Unit,
        locals: vec![rec, rec, Type::I64],
        body,
        line: 1,
        fails: false,
        external: None,
    };
    scalarize(&mut f, &shapes, &std::collections::HashSet::new());
    // 3 originals, 2 field locals per record, and a temporary per
    // stored field (two building p, one for the `SetField` value).
    assert_eq!(f.locals.len(), 3 + 4 + 3);
    let heapy = crate::walk::any_in_block(&f.body, &mut |x| {
        matches!(
            x.kind,
            ExprKind::Record { .. } | ExprKind::GetField(..) | ExprKind::SetField(..)
        )
    });
    assert!(!heapy, "no record operations stay behind");
}

/// A record passed to a call is kept whole, and a copy of it rebuilt
/// where needed.
#[test]
fn a_shared_record_stays_whole() {
    let shapes = vec![Shape::Record {
        drop: false,
        name: "P".into(),
        fields: vec![FieldShape {
            name: "x".into(),
            ty: Type::I64,
            secret: false,
        }],
    }];
    let rec = Type::Ref(0);
    let body = Block {
        stmts: vec![
            Stmt::Let(
                0,
                e(
                    ExprKind::Record {
                        variant: 0,
                        fields: vec![e(ExprKind::Int(1), Type::I64)],
                    },
                    rec,
                ),
            ),
            Stmt::Expr(e(
                ExprKind::Call(0, vec![e(ExprKind::Local(0), rec)]),
                Type::Unit,
            )),
        ],
        value: None,
    };
    let mut f = Function {
        name: "t".into(),
        params: Vec::new(),
        ret: Type::Unit,
        locals: vec![rec],
        body,
        line: 1,
        fails: false,
        external: None,
    };
    scalarize(&mut f, &shapes, &std::collections::HashSet::new());
    assert_eq!(f.locals.len(), 1, "nothing split");
}

/// Locals 0 `total`; 1 the list; the loop writes `total += xs[0] + xs[2]`.
#[test]
fn a_fixed_list_read_at_constant_indexes_becomes_locals() {
    let lst = Type::Ref(0);
    let items = |line: u32| {
        (0..3)
            .map(|k| Expr {
                kind: ExprKind::Int(k),
                ty: Type::I64,
                line,
            })
            .collect::<Vec<_>>()
    };
    let read = |k: i128| Expr {
        kind: ExprKind::Index(
            Box::new(Expr {
                kind: ExprKind::Local(1),
                ty: lst,
                line: 1,
            }),
            Box::new(Expr {
                kind: ExprKind::Int(k),
                ty: Type::I64,
                line: 1,
            }),
        ),
        ty: Type::I64,
        line: 1,
    };
    let sum = Expr {
        kind: ExprKind::Binary(BinOp::Add, Box::new(read(0)), Box::new(read(2))),
        ty: Type::I64,
        line: 1,
    };
    let mut f = Function {
        name: "main".into(),
        params: vec![],
        ret: Type::Unit,
        locals: vec![Type::I64, lst],
        body: Block {
            stmts: vec![
                Stmt::Let(
                    1,
                    Expr {
                        kind: ExprKind::List(items(1)),
                        ty: lst,
                        line: 1,
                    },
                ),
                Stmt::Expr(Expr {
                    kind: ExprKind::Assign(0, Box::new(sum)),
                    ty: Type::Unit,
                    line: 1,
                }),
            ],
            value: None,
        },
        line: 1,
        fails: false,
        external: None,
    };
    scalarize(
        &mut f,
        &[Shape::List(Type::I64)],
        &std::collections::HashSet::new(),
    );
    // 2 + 3 elements + 3 store temporaries.
    assert_eq!(f.locals.len(), 8);
    assert!(!crate::walk::any_in_block(&f.body, &mut |e| {
        matches!(e.kind, ExprKind::List(_) | ExprKind::Index(..))
    }));
}
