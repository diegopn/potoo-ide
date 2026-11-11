use super::*;

pub(crate) fn localize_learning_problem(
    problem: &crate::features::learning::LearningProblem,
    language: UiLanguage,
) -> (String, Option<String>) {
    use crate::features::learning::{LearningProblemKind, LearningProblemText};

    match &problem.text {
        LearningProblemText::ConstantCondition { always_true } => (
            tr(
                language,
                if *always_true {
                    "A condição é sempre verdadeira; o ramo alternativo pode nunca ser executado."
                } else {
                    "A condição é sempre falsa; o bloco pode nunca ser executado."
                },
            )
            .to_owned(),
            Some(
                tr(
                    language,
                    "Confira se a condição deveria comparar uma variável.",
                )
                .to_owned(),
            ),
        ),
        LearningProblemText::ConstantIndex {
            index,
            name,
            lower_bound,
            upper_bound,
        } => {
            let message = tr(
                language,
                "O índice {value} está fora dos limites de `{name}` ({lower} a {upper}).",
            )
            .replace("{value}", &index.to_string())
            .replace("{name}", name)
            .replace("{lower}", &lower_bound.to_string())
            .replace("{upper}", &upper_bound.to_string());
            (
                message,
                Some(
                    tr(
                        language,
                        "Use um índice entre {lower} e {upper} em `{name}`.",
                    )
                    .replace("{lower}", &lower_bound.to_string())
                    .replace("{upper}", &upper_bound.to_string())
                    .replace("{name}", name),
                ),
            )
        }
        LearningProblemText::InterpreterDiagnostic => {
            if language == UiLanguage::Portuguese {
                let message = match problem.message.as_str() {
                    "division by zero" => "divisão por zero".to_owned(),
                    "integer division by zero" => "divisão inteira por zero".to_owned(),
                    _ => problem.message.clone(),
                };
                return (message, problem.suggestion.clone());
            }
            let suggestion = problem
                .suggestion
                .as_deref()
                .and_then(|suggestion| translate_interpreter_suggestion(suggestion, language));
            if let Some(message) = translate_interpreter_diagnostic(&problem.message, language) {
                return (message, suggestion);
            }

            let summary = match problem.kind {
                LearningProblemKind::Syntax => "Há um problema de sintaxe nesta linha.",
                LearningProblemKind::Type => {
                    "O interpretador encontrou uma incompatibilidade de tipos nesta linha."
                }
                LearningProblemKind::Runtime => "A execução encontrou um problema nesta linha.",
                LearningProblemKind::StaticAnalysis => "A análise encontrou um aviso nesta linha.",
                LearningProblemKind::Analysis => "A análise não foi concluída para esta linha.",
                LearningProblemKind::Logic => "Índice fora dos limites",
                LearningProblemKind::PossibleLogic => "Possível problema lógico",
            };
            let detail = tr(
                language,
                "Detalhe original do interpretador (português): {message}",
            )
            .replace("{message}", &problem.message);
            (
                tr(language, summary).to_owned(),
                suggestion.or(Some(detail)),
            )
        }
        LearningProblemText::AnalysisIncomplete => (
            tr(
                language,
                "A análise foi interrompida antes de recuperar toda a estrutura do programa.",
            )
            .to_owned(),
            None,
        ),
    }
}

fn translate_interpreter_diagnostic(message: &str, language: UiLanguage) -> Option<String> {
    let translated = match message {
        "divisão por zero" => ("Division by zero", "División por cero"),
        "divisão inteira por zero" => ("Integer division by zero", "División entera por cero"),
        "division by zero" => ("Division by zero", "División por cero"),
        "integer division by zero" => ("Integer division by zero", "División entera por cero"),
        "Comentário de bloco sem fechamento" => {
            ("Unclosed block comment", "Comentario de bloque sin cerrar")
        }
        "resto de divisão por zero" => ("Remainder by zero", "Resto de una división por cero"),
        "estouro na operação inteira" => (
            "Integer operation overflow.",
            "Desbordamiento en una operación entera.",
        ),
        "estouro em operação com inteiros" => (
            "Integer operation overflow.",
            "Desbordamiento en una operación entera.",
        ),
        "estouro em divisão inteira" => (
            "Integer division overflow.",
            "Desbordamiento en una división entera.",
        ),
        "estouro ao negar inteiro" => (
            "Integer negation overflow.",
            "Desbordamiento al negar un entero.",
        ),
        "resultado real fora do limite" => (
            "Real result is out of range.",
            "El resultado real está fuera de rango.",
        ),
        "índice de vetor negativo" => (
            "Array index cannot be negative",
            "El índice no puede ser negativo",
        ),
        "número fora do limite de inteiro" => (
            "Number is outside the integer range",
            "El número está fuera del rango de enteros",
        ),
        "A chamada pertence à biblioteca Jerimum, mas ela não foi importada. Adicione `use \"jerimum\"` no início do arquivo." => {
            (
                "This call belongs to the Jerimum library, but it has not been imported. Add `use \"jerimum\"` near the start of the file.",
                "Esta llamada pertenece a la biblioteca Jerimum, pero no se ha importado. Añada `use \"jerimum\"` al inicio del archivo.",
            )
        }
        "não é possível comparar estes valores" => (
            "These values cannot be compared",
            "No se pueden comparar estos valores",
        ),
        "índice fora dos limites" | "índice de vetor fora do limite" => (
            "Array index out of bounds",
            "Índice de vector fuera de los límites",
        ),
        "variável não declarada" => ("Variable is not declared", "La variable no está declarada"),
        "esperada uma expressão" | "esperava uma expressão" => {
            ("Expected an expression", "Se esperaba una expresión")
        }
        "esperado um identificador" => ("Expected an identifier", "Se esperaba un identificador"),
        "esperado o nome da função" => (
            "Expected the function name",
            "Se esperaba el nombre de la función",
        ),
        "esperado o nome do parâmetro" => (
            "Expected the parameter name",
            "Se esperaba el nombre del parámetro",
        ),
        "esperado o nome da variável" => (
            "Expected the variable name",
            "Se esperaba el nombre de la variable",
        ),
        "esperado o nome da biblioteca" => (
            "Expected the library name",
            "Se esperaba el nombre de la biblioteca",
        ),
        "esperado o nome do algoritmo entre aspas" => (
            "Expected the algorithm name in quotation marks",
            "Se esperaba el nombre del algoritmo entre comillas",
        ),
        "esperado o nome do arquivo entre aspas" => (
            "Expected the file name in quotation marks",
            "Se esperaba el nombre del archivo entre comillas",
        ),
        "esperado um apelido para a biblioteca" => (
            "Expected a library alias",
            "Se esperaba un alias para la biblioteca",
        ),
        "esperado o tipo da variável" => (
            "Expected the variable type",
            "Se esperaba el tipo de la variable",
        ),
        "tipo de variável desconhecido" => {
            ("Unknown variable type", "Tipo de variable desconocido")
        }
        "esperado um tipo: inteiro, real, caractere ou logico" => (
            "Expected a type: integer, real, character, or logical",
            "Se esperaba un tipo: entero, real, carácter o lógico",
        ),
        "tipo VisuAlg desconhecido" => ("Unknown VisuAlg type", "Tipo de VisuAlg desconocido"),
        "esperado um limite inteiro constante para o vetor" => (
            "Expected a constant integer bound for the array",
            "Se esperaba un límite entero constante para el vector",
        ),
        "o programa deve começar com 'programa'" => (
            "The program must start with 'programa'",
            "El programa debe comenzar con 'programa'",
        ),
        "esperado ':' ou bloco após o rótulo do caso" => (
            "Expected ':' or a block after the case label",
            "Se esperaba ':' o un bloque después de la etiqueta del caso",
        ),
        "esperado 'caso', 'outrocaso' ou 'fimescolha'" => (
            "Expected 'caso', 'outrocaso', or 'fimescolha'",
            "Se esperaba 'caso', 'outrocaso' o 'fimescolha'",
        ),
        "esperado 'var', 'arquivo', uma rotina ou 'inicio'" => (
            "Expected 'var', 'arquivo', a routine, or 'inicio'",
            "Se esperaba 'var', 'arquivo', una rutina o 'inicio'",
        ),
        "esperado o fim da linha" => (
            "Expected the end of the line",
            "Se esperaba el fin de la línea",
        ),
        "esperada uma nova linha ou ';' entre instruções" => (
            "Expected a new line or ';' between statements",
            "Se esperaba una nueva línea o ';' entre instrucciones",
        ),
        "há código depois do fim do programa" => (
            "There is code after the end of the program",
            "Hay código después del final del programa",
        ),
        "programa excede o limite de blocos aninhados" => (
            "The program exceeds the nested block limit",
            "El programa supera el límite de bloques anidados",
        ),
        "expressão excede o limite de aninhamento" => (
            "The expression exceeds the nesting limit",
            "La expresión supera el límite de anidamiento",
        ),
        "palavra reservada não pode ser usada como identificador" => (
            "A reserved word cannot be used as an identifier",
            "No se puede usar una palabra reservada como identificador",
        ),
        "vetores podem ter no máximo duas dimensões" => (
            "Arrays can have at most two dimensions",
            "Los vectores pueden tener como máximo dos dimensiones",
        ),
        "a função precisa declarar o tipo do resultado" => (
            "The function must declare its result type",
            "La función debe declarar el tipo de resultado",
        ),
        "um procedimento não declara tipo de retorno" => (
            "A procedure does not declare a return type",
            "Un procedimiento no declara tipo de retorno",
        ),
        "o valor não é um vetor" => ("The value is not an array", "El valor no es un vector"),
        "esperada uma matriz" => ("Expected a matrix", "Se esperaba una matriz"),
        "texto não terminado" => ("Unterminated string", "Cadena sin terminar"),
        "texto não terminado antes do fim da linha" => (
            "String not terminated before the end of the line",
            "Cadena sin terminar antes del final de la línea",
        ),
        "caractere não terminado" => ("Unterminated character literal", "Carácter sin terminar"),
        "literal de caracter deve conter um caractere" => (
            "A character literal must contain one character",
            "Un literal de carácter debe contener un carácter",
        ),
        "sequência de escape incompleta" => (
            "Incomplete escape sequence",
            "Secuencia de escape incompleta",
        ),
        "a declaração 'arquivo' só pode aparecer uma vez" => (
            "The 'arquivo' declaration can appear only once",
            "La declaración 'arquivo' solo puede aparecer una vez",
        ),
        "'outrocaso' deve ser o último bloco de 'escolha'" => (
            "'outrocaso' must be the last block in 'escolha'",
            "'outrocaso' debe ser el último bloque de 'escolha'",
        ),
        _ => {
            if let Some(name) = message
                .strip_prefix("variável '")
                .and_then(|rest| rest.strip_suffix("' não declarada"))
            {
                return Some(match language {
                    UiLanguage::English => format!("Variable `{name}` is not declared"),
                    UiLanguage::Spanish => format!("La variable `{name}` no está declarada"),
                    UiLanguage::Portuguese => message.to_owned(),
                });
            }
            if let Some(translated) = translate_expected_token(message, language) {
                return Some(translated);
            }
            return None;
        }
    };
    Some(match language {
        UiLanguage::English => translated.0.to_owned(),
        UiLanguage::Spanish => translated.1.to_owned(),
        UiLanguage::Portuguese => message.to_owned(),
    })
}

fn translate_interpreter_suggestion(suggestion: &str, language: UiLanguage) -> Option<String> {
    match suggestion {
        "Confira se o divisor pode ser zero antes de fazer a divisão." => Some(
            tr(
                language,
                "Confira se o divisor pode ser zero antes de fazer a divisão.",
            )
            .to_owned(),
        ),
        "Adicione `*/` para fechar o comentário de bloco." => {
            Some(tr(language, "Adicione `*/` para fechar o comentário de bloco.").to_owned())
        }
        "Informe o valor ou a expressão que será exibida após `escreva`." => Some(
            tr(
                language,
                "Informe o valor ou a expressão que será exibida após `escreva`.",
            )
            .to_owned(),
        ),
        "Informe o valor ou a expressão que será exibida após `imprima`." => Some(
            tr(
                language,
                "Informe o valor ou a expressão que será exibida após `imprima`.",
            )
            .to_owned(),
        ),
        _ => None,
    }
}

fn translate_expected_token(message: &str, language: UiLanguage) -> Option<String> {
    let expected = message.strip_prefix("esperado '")?;
    let (token, context) = expected.split_once('\'')?;
    let context_translation = match context {
        "" => ("", ""),
        " após 'programa'" => (" after 'programa'", " después de 'programa'"),
        " após o nome da função" => (
            " after the function name",
            " después del nombre de la función",
        ),
        " após a dimensão do parâmetro" => (
            " after the parameter dimension",
            " después de la dimensión del parámetro",
        ),
        " após os parâmetros" => (" after the parameters", " después de los parámetros"),
        " após 'inclua'" => (" after 'inclua'", " después de 'inclua'"),
        " após o bloco 'faca'" => (" after the 'faca' block", " después del bloque 'faca'"),
        " após 'para'" => (" after 'para'", " después de 'para'"),
        " após o início do 'para'" => (
            " after the start of 'para'",
            " después del inicio de 'para'",
        ),
        " após a condição do 'para'" => (
            " after the 'para' condition",
            " después de la condición de 'para'",
        ),
        " após o cabeçalho do 'para'" => (
            " after the 'para' header",
            " después del encabezado de 'para'",
        ),
        " após a expressão de 'escolha'" => (
            " after the 'escolha' expression",
            " después de la expresión de 'escolha'",
        ),
        " após o tamanho do vetor" => (" after the array size", " después del tamaño del vector"),
        " após a condição" => (" after the condition", " después de la condición"),
        " após os argumentos" => (" after the arguments", " después de los argumentos"),
        " após o índice" => (" after the index", " después del índice"),
        " após a expressão" => (" after the expression", " después de la expresión"),
        " após os valores" => (" after the values", " después de los valores"),
        " antes do fim do arquivo" => (
            " before the end of the file",
            " antes del final del archivo",
        ),
        " antes da lista de argumentos" => (
            " before the argument list",
            " antes de la lista de argumentos",
        ),
        " para fechar o programa" => (" to close the program", " para cerrar el programa"),
        " para fechar o bloco" => (" to close the block", " para cerrar el bloque"),
        " para fechar 'escolha'" => (" to close 'escolha'", " para cerrar 'escolha'"),
        " para fechar o caso" => (" to close the case", " para cerrar el caso"),
        _ => return None,
    };
    let (context, punctuation) = match language {
        UiLanguage::English => (context_translation.0, "."),
        UiLanguage::Spanish => (context_translation.1, "."),
        UiLanguage::Portuguese => return Some(message.to_owned()),
    };
    let subject = match language {
        UiLanguage::English => "Expected",
        UiLanguage::Spanish => "Se esperaba",
        UiLanguage::Portuguese => unreachable!(),
    };
    Some(format!("{subject} '{token}'{context}{punctuation}"))
}
