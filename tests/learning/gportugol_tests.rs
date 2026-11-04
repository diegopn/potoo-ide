use super::analyze_source_line_cancellable;
use crate::languages::model::SourceConstruct;

#[test]
fn analyzes_statements_inside_multiple_statement_control_flow_bodies() {
    let source =
        "algoritmo teste;\ninício\nse verdadeiro então\nimprima(1); imprima(2);\nfim-se\nfim";

    let analysis = analyze_source_line_cancellable(source, 4, || false)
        .expect("valid G-Portugol source")
        .expect("the selected statement should be analyzed");

    assert_eq!(analysis.construct, SourceConstruct::Output);
}
