use potoo_gportugol_interpreter::{
    DiagnosticKind, ExecutionOptions,
    ast::{ExprKind, Literal, Statement},
    parse, parse_with_options,
};

#[test]
fn parses_algorithm_globals_main_and_functions() {
    let program = parse(
        "algoritmo exemplo;\n\
         variáveis\n\
           total: inteiro;\n\
         fim-variáveis\n\
         início\n\
           total := soma(2, 3);\n\
           imprima(total);\n\
         fim\n\
         função soma(a: inteiro, b: inteiro): inteiro\n\
         início\n\
           retorne a + b;\n\
         fim",
    )
    .expect("the program should parse");

    assert_eq!(program.name, "exemplo");
    assert_eq!(program.globals.len(), 1);
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "soma");
    assert_eq!(program.main.statements.len(), 2);
}

#[test]
fn parameter_spans_cover_their_declarations() {
    let source = "algoritmo teste; início fim função f(a: inteiro, b: matriz[2] de inteiros): inteiro início retorne 0; fim";
    let program = parse(source).expect("the function parameters should parse");

    for (declaration, parameter) in [
        ("a: inteiro", &program.functions[0].parameters[0]),
        (
            "b: matriz[2] de inteiros",
            &program.functions[0].parameters[1],
        ),
    ] {
        let start = source
            .find(declaration)
            .expect("parameter should be in source");
        assert_eq!(parameter.span.start.byte_offset, start);
        assert_eq!(parameter.span.end.byte_offset, start + declaration.len());
    }
}

#[test]
fn call_statement_spans_include_the_terminating_semicolon() {
    let source = "algoritmo teste; início f(); fim função f(): inteiro início retorne 0; fim";
    let program = parse(source).expect("the call statement should parse");
    let call = source.find("f();").expect("call should be in source");
    let span = program.main.statements[0].span();

    assert_eq!(span.start.byte_offset, call);
    assert_eq!(span.end.byte_offset, call + "f();".len());
}

#[test]
fn binary_and_octal_literals_use_signed_int32_values() {
    let source = "algoritmo teste; variáveis r: real; fim-variáveis início r := 0b11111111111111111111111111111111; r := 0b10000000000000000000000000000000; r := 0c37777777777; r := 0c20000000000; fim";
    let program = parse(source).expect("binary and octal integer literals should parse");

    for (statement, expected) in
        program
            .main
            .statements
            .iter()
            .zip([-1, -2147483648, -1, -2147483648])
    {
        let Statement::Assign { value, .. } = statement else {
            panic!("the main block should contain assignments");
        };
        assert_eq!(&value.kind, &ExprKind::Literal(Literal::Integer(expected)));
    }
}

#[test]
fn parses_matrices_and_control_blocks() {
    let program = parse(
        "algoritmo controles;\n\
         variáveis\n\
           i: inteiro;\n\
           grade: matriz[2][3] de inteiros;\n\
         fim-variáveis\n\
         início\n\
           para i de 0 até 1 faça\n\
             se i = 0 então\n\
               enquanto verdadeiro faça\n\
                 grade[i][0] := i;\n\
               fim-enquanto\n\
             senão\n\
               repita\n\
                 i := i + 1;\n\
               até i > 2;\n\
             fim-se\n\
           fim-para\n\
         fim",
    )
    .expect("the blocks and matrix declaration should parse");

    assert_eq!(program.globals[1].dimensions, [2, 3]);
    assert!(matches!(program.main.statements[0], Statement::For { .. }));
}

#[test]
fn reports_missing_block_terminators_at_source_location() {
    let diagnostics = parse(
        "algoritmo incompleto;\n\
         início\n\
           se verdadeiro então\n\
             imprima(1);\n\
         fim",
    )
    .expect_err("a missing fim-se must be reported");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Syntax);
    assert_eq!(diagnostics[0].span.start.line, 5);
}

#[test]
fn rejects_nesting_beyond_configured_depth() {
    let source = "algoritmo profundo; início se verdadeiro então se verdadeiro então imprima(1); fim-se fim-se fim";
    let options = ExecutionOptions {
        max_parse_depth: 1,
        ..ExecutionOptions::default()
    };

    let diagnostics = parse_with_options(source, options)
        .expect_err("nested statement blocks must respect the parser limit");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Syntax);
    assert!(diagnostics[0].message.contains("nesting limit"));
}

#[test]
fn recursive_array_subscripts_respect_parse_depth() {
    let options = ExecutionOptions {
        max_parse_depth: 2,
        ..ExecutionOptions::default()
    };

    parse_with_options("algoritmo raso; início a[0] := 1; fim", options)
        .expect("a single subscript should fit within the configured depth");
    let diagnostics =
        parse_with_options("algoritmo profundo; início a[a[a[0]]] := 1; fim", options)
            .expect_err("recursive subscripts should count toward the parser limit");

    assert!(diagnostics[0].message.contains("nesting limit"));
}

#[test]
fn parses_multiple_and_empty_control_flow_bodies() {
    for source in [
        "algoritmo teste; início se verdadeiro então imprima(1); imprima(2); fim-se fim",
        "algoritmo teste; variáveis i: inteiro; fim-variáveis início enquanto i < 1 faça imprima(i); i := i + 1; fim-enquanto fim",
        "algoritmo teste; variáveis i: inteiro; fim-variáveis início para i de 1 até 1 faça imprima(i); imprima(2); fim-para fim",
        "algoritmo teste; início se verdadeiro então fim-se fim",
        "algoritmo teste; início repita imprima(1); imprima(2); até verdadeiro; fim",
        "algoritmo teste; início se verdadeiro então se verdadeiro então imprima(1); imprima(2); fim-se senão imprima(3); fim-se fim",
    ] {
        parse(source)
            .expect("control-flow bodies should consume statements through their terminator");
    }
}

#[test]
fn rejects_syntax_extensions_outside_gpt_grammar() {
    for source in [
        "algoritmo teste; início imprima(verdadeiro && falso); fim",
        "algoritmo teste; início imprima(verdadeiro || falso); fim",
        "algoritmo teste; início imprima(~~1); fim",
        "algoritmo teste; variáveis fim-variáveis início fim",
    ] {
        parse(source).expect_err("the original grammar rejects this extension");
    }
}
