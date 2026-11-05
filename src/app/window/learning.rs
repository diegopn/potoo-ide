use super::*;

pub(super) fn format_diagnostics(
    diagnostics: &[execution::ExecutionDiagnostic],
    source: Option<(Language, &str)>,
) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let (category, diagnostic_category) = match diagnostic.kind {
                execution::DiagnosticKind::Lexical => (
                    "Erro léxico",
                    crate::features::diagnostics::DiagnosticCategory::Lexical,
                ),
                execution::DiagnosticKind::Syntax => (
                    "Erro de sintaxe",
                    crate::features::diagnostics::DiagnosticCategory::Syntax,
                ),
                execution::DiagnosticKind::Type => (
                    "Erro de tipo",
                    crate::features::diagnostics::DiagnosticCategory::Type,
                ),
                execution::DiagnosticKind::Runtime => (
                    "Erro de execução",
                    crate::features::diagnostics::DiagnosticCategory::Runtime,
                ),
            };
            let message = source.map_or_else(
                || diagnostic.message.clone(),
                |(language, source)| {
                    crate::features::diagnostics::adapt_diagnostic_message(
                        language,
                        source,
                        diagnostic_category,
                        &diagnostic.message,
                        diagnostic.line,
                        diagnostic.column,
                    )
                },
            );
            format!(
                "{category} · Linha {}, coluna {}: {message}",
                diagnostic.line, diagnostic.column
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn learning_topic_label(topic: LearningTopic, language: UiLanguage) -> &'static str {
    tr(
        language,
        match topic {
            LearningTopic::Function => "função",
            LearningTopic::Procedure => "procedimento",
            LearningTopic::Decision => "decisão",
            LearningTopic::Loop => "laço",
            LearningTopic::Input => "entrada",
            LearningTopic::Output => "saída",
            LearningTopic::Variable => "variável",
            LearningTopic::Assignment => "atribuição",
            LearningTopic::Call => "chamada",
            LearningTopic::LibraryCall => "chamada de biblioteca",
            LearningTopic::Import => "importação",
            LearningTopic::Return => "retorno",
            LearningTopic::Program => "programa",
            LearningTopic::Unrecognized => "não identificada",
        },
    )
}

pub(super) fn assignment_operator(language: Language) -> Option<&'static str> {
    match language {
        Language::Portugol | Language::Calango => Some("="),
        Language::Visualg | Language::Mapler => Some("<-"),
        Language::Potigol | Language::GPortugol => Some(":="),
    }
}

pub(super) fn output_line_break(hint: &LearningHint) -> Option<bool> {
    match (hint.language, hint.name.as_deref()) {
        (Language::Potigol, Some("escreva"))
        | (Language::Visualg | Language::Calango, Some("escreval"))
        | (Language::GPortugol, Some("imprima"))
        | (Language::Mapler, Some("escrever")) => Some(true),
        (Language::Potigol, Some("imprima"))
        | (Language::Visualg | Language::Calango, Some("escreva"))
        | (Language::Portugol, Some("escreva")) => Some(false),
        _ => None,
    }
}

pub(super) fn potigol_input_description(command: &str) -> Option<&'static str> {
    match command {
        "leia_inteiro" => Some("lê um inteiro"),
        "leia_real" | "leia_numero" | "leia_número" => Some("lê um número real"),
        "leia_texto" => Some("lê um texto"),
        "leia_caractere" => Some("lê um caractere"),
        "leia_logico" => Some("lê um valor lógico"),
        "leia_inteiros" => Some("lê uma lista de inteiros"),
        "leia_reais" | "leia_numeros" | "leia_números" => Some("lê uma lista de números reais"),
        "leia_textos" => Some("lê uma lista de textos"),
        _ => None,
    }
}

pub(super) fn learning_copy(hint: &LearningHint, language: UiLanguage) -> (String, String, String) {
    let text = |portuguese| tr(language, portuguese).to_owned();
    let has_syntax_error = hint
        .problems
        .iter()
        .any(|problem| problem.kind == crate::features::learning::LearningProblemKind::Syntax);
    let parameters = hint
        .parameters
        .iter()
        .map(|symbol| format_learning_symbol(symbol, hint.language, language, true))
        .collect::<Vec<_>>();
    let name = hint.name.as_deref().unwrap_or("?");
    if hint.topic == LearningTopic::LibraryCall
        && let Some(knowledge) = hint
            .call
            .as_ref()
            .and_then(|call| catalog::lookup_library_call(hint.language, call))
    {
        return (
            text(knowledge.explanation),
            text(knowledge.concept),
            text(knowledge.experiment),
        );
    }
    let call_name = hint.call.as_ref().map_or_else(
        || name.to_owned(),
        |call| {
            let callee = call.receiver.as_ref().map_or_else(
                || call.member.clone(),
                |receiver| format!("{receiver}.{}", call.member),
            );
            format!("{callee}({})", call.arguments.join(", "))
        },
    );
    let explanation = match hint.topic {
        LearningTopic::Function => {
            if has_syntax_error && hint.name.is_none() {
                text(
                    "Esta declaração parece ser de uma função; complete a assinatura para identificar nome, parâmetros e retorno.",
                )
            } else {
                let params = if parameters.is_empty() {
                    text("nenhum parâmetro")
                } else {
                    parameters.join(", ")
                };
                let declaration =
                    format!("{} `{name}` {} {params}", text("A função"), text("recebe"));
                match hint.return_type.as_deref() {
                    Some("vazio") => format!(
                        "{declaration}. {}",
                        text("O tipo `vazio` indica que a função não devolve um valor.")
                    ),
                    Some(return_type @ "literal") if hint.language == Language::GPortugol => {
                        format!(
                            "{declaration}. {} `{return_type}`. {}",
                            text("Ela devolve um valor do tipo"),
                            text("Esse tipo representa texto no G-Portugol.")
                        )
                    }
                    Some(return_type) => format!(
                        "{declaration}. {} `{return_type}`.{}",
                        text("Ela devolve um valor do tipo"),
                        if return_type == "cadeia" {
                            format!(" {}", text("Esse tipo representa texto."))
                        } else {
                            String::new()
                        }
                    ),
                    None if hint.language == Language::GPortugol => format!(
                        "{declaration}. {}",
                        text("Esta função não declara um tipo de retorno.")
                    ),
                    None => format!("{declaration}. {}", text("O retorno não foi identificado.")),
                }
            }
        }
        LearningTopic::Procedure => {
            if has_syntax_error && hint.name.is_none() {
                text(
                    "Esta declaração parece ser de um procedimento; complete a assinatura para identificar nome e parâmetros.",
                )
            } else {
                let params = if parameters.is_empty() {
                    text("nenhum parâmetro")
                } else {
                    parameters.join(", ")
                };
                format!(
                    "{} `{name}` {} {params}. {}",
                    text("O procedimento"),
                    text("recebe"),
                    text("Ele executa ações e não devolve um valor como resultado.")
                )
            }
        }
        LearningTopic::Variable => {
            if hint.variables.is_empty() {
                text("A declaração define o nome e o tipo de um dado usado pelo algoritmo.")
            } else {
                hint.variables
                    .iter()
                    .map(|symbol| format_learning_symbol(symbol, hint.language, language, false))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        LearningTopic::Decision if hint.name.as_deref() == Some("escolha") => format!(
            "{} `{name}` {}.",
            text("O comando"),
            text("compara o valor selecionado com cada caso e executa o bloco correspondente")
        ),
        LearningTopic::Decision => match (hint.name.as_deref(), hint.expression.as_deref()) {
            (Some(command), Some(expression)) => format!(
                "{} “{expression}” {} `{command}` {}",
                text("A condição"),
                text("é testada por"),
                text("e determina qual bloco será executado.")
            ),
            (_, Some(expression)) => format!(
                "{} “{expression}” {}",
                text("A condição"),
                text(
                    "determina qual bloco será executado conforme resulte em verdadeiro ou falso."
                )
            ),
            _ => text("Uma condição escolhe qual bloco executar com base em verdadeiro ou falso."),
        },
        LearningTopic::Loop => match hint
            .name
            .as_deref()
            .and_then(|name| name.split_whitespace().next())
        {
            Some("enquanto") => format!(
                "{} `{name}` {}.",
                text("O laço"),
                text("testa a condição antes de cada repetição")
            ),
            Some("repita" | "faca") => format!(
                "{} `{name}` {}.",
                text("O laço"),
                text("verifica a condição depois de executar o bloco")
            ),
            Some("para") => format!(
                "{} `{name}` {}.",
                text("O laço"),
                text("percorre os valores definidos para a variável de controle")
            ),
            _ => text("Um laço repete instruções de acordo com uma condição ou faixa de valores."),
        },
        LearningTopic::Input => {
            let command = hint
                .call
                .as_ref()
                .map(|call| call.member.as_str())
                .or(hint.name.as_deref());
            if hint.language == Language::Potigol
                && let Some(description) = command.and_then(potigol_input_description)
            {
                format!(
                    "{} `{}` {}.",
                    text("A chamada"),
                    command.unwrap_or("leia"),
                    text(description)
                )
            } else if hint.language == Language::GPortugol {
                format!(
                    "{} `{}` {}.",
                    text("A chamada"),
                    command.unwrap_or("leia"),
                    text("fornece um valor para a expressão que a utiliza")
                )
            } else if let Some(command) = command {
                format!(
                    "{} `{command}` {}.",
                    text("A instrução"),
                    text("armazena o valor informado na variável de destino")
                )
            } else {
                text("A instrução de entrada fornece valores para o programa usar.")
            }
        }
        LearningTopic::Output => match output_line_break(hint) {
            Some(line_break) => format!(
                "{} `{name}` {}.",
                text("O comando"),
                text(if line_break {
                    "exibe os valores e encerra a linha"
                } else {
                    "exibe os valores sem encerrar a linha"
                })
            ),
            None => text("Esta instrução apresenta os dados informados na saída do programa."),
        },
        LearningTopic::Assignment => {
            if let (Some(operator), Some(target)) =
                (assignment_operator(hint.language), hint.name.as_deref())
            {
                format!(
                    "{} `{operator}` {} `{target}`.",
                    text("O operador"),
                    text("atribui o valor à variável")
                )
            } else {
                text(
                    "A atribuição calcula o lado direito e atualiza o valor da variável à esquerda.",
                )
            }
        }
        LearningTopic::Call => format!(
            "{} `{call_name}` {}.",
            text("A chamada"),
            text("executa a função ou operação identificada nesta linha")
        ),
        LearningTopic::LibraryCall => format!(
            "{} `{call_name}` {}.",
            text("A chamada"),
            text("executa uma operação disponível na biblioteca importada")
        ),
        LearningTopic::Import => match hint.name.as_deref() {
            Some(library) => format!(
                "{} `{library}` {}.",
                text("A importação"),
                text("disponibiliza os elementos dessa biblioteca ao programa")
            ),
            None => text("Esta linha importa elementos para uso no programa."),
        },
        LearningTopic::Return => match hint.return_type.as_deref() {
            Some("vazio") => text("Esta função termina sem devolver um valor."),
            Some(data_type) => format!(
                "{} `{data_type}` {}.",
                text("O retorno entrega um valor do tipo"),
                text("ao ponto que chamou a função")
            ),
            None => text("O retorno devolve o controle ao ponto que chamou a rotina."),
        },
        LearningTopic::Program => text(
            "A linha selecionada participa do fluxo do algoritmo. Selecione outra linha para ver seu papel.",
        ),
        LearningTopic::Unrecognized if has_syntax_error => text(
            "A análise não reconheceu esta linha; consulte a mensagem em Problemas nesta linha.",
        ),
        LearningTopic::Unrecognized => text(
            "Esta linha não contém uma instrução reconhecida; comentários e linhas vazias não executam código.",
        ),
    };

    let concept = match hint.topic {
        LearningTopic::Function => match hint.return_type.as_deref() {
            Some("vazio") => text(
                "Parâmetros são os dados recebidos pela função. `vazio` informa que nenhum resultado é devolvido.",
            ),
            Some("cadeia") => text(
                "A função produz um valor de texto, representado pelo tipo `cadeia` em Portugol Studio 2.0.",
            ),
            Some("literal") if hint.language == Language::GPortugol => {
                text("A função produz texto, representado pelo tipo `literal` no G-Portugol.")
            }
            Some(return_type) => format!(
                "{} `{return_type}`. {}",
                text("A função devolve um resultado do tipo"),
                text("Seus parâmetros recebem os dados usados no cálculo.")
            ),
            None if hint.language == Language::GPortugol => text(
                "Os parâmetros recebem os dados usados pela função; a ausência de tipo indica que ela não devolve resultado.",
            ),
            None => text(
                "Parâmetros são os dados recebidos pela função; o tipo de retorno descreve o resultado produzido.",
            ),
        },
        LearningTopic::Procedure => text(
            "Uma rotina executa ações sem devolver um resultado; cada linguagem define como seus parâmetros são passados.",
        ),
        LearningTopic::Variable => text(
            "O tipo define quais valores podem ser guardados; o formato indica se há um valor, um vetor ou uma matriz.",
        ),
        LearningTopic::Assignment => {
            text("A expressão à direita do operador produz o valor armazenado.")
        }
        LearningTopic::Call => text(
            "Uma chamada transfere a execução para uma função ou operação e depois continua no ponto seguinte.",
        ),
        LearningTopic::LibraryCall => {
            text("Uma biblioteca fornece operações que o programa pode reutilizar.")
        }
        LearningTopic::Import => {
            text("A importação torna disponíveis nomes e operações definidos em outra biblioteca.")
        }
        LearningTopic::Return => {
            text("O retorno leva o resultado da função de volta ao ponto que a chamou.")
        }
        LearningTopic::Decision if hint.name.as_deref() == Some("escolha") => {
            text("A seleção compara um valor com alternativas explícitas.")
        }
        LearningTopic::Decision => text("A condição é avaliada como verdadeira ou falsa."),
        LearningTopic::Loop
            if hint
                .name
                .as_deref()
                .and_then(|name| name.split_whitespace().next())
                .is_some_and(|name| matches!(name, "repita" | "faca")) =>
        {
            text("O bloco é executado antes da verificação da condição.")
        }
        LearningTopic::Loop => text("Cada repetição executa novamente as instruções do bloco."),
        LearningTopic::Input => {
            text("O tipo solicitado define como o valor de entrada é interpretado.")
        }
        LearningTopic::Output => text("A quebra de linha determina onde a próxima saída começa."),
        LearningTopic::Program => {
            text("Uma instrução é executada na ordem definida pelo algoritmo.")
        }
        LearningTopic::Unrecognized => String::new(),
    };

    let experiment = match hint.topic {
        LearningTopic::Function
            if hint.return_type.as_deref() == Some("vazio")
                || (hint.language == Language::GPortugol && hint.return_type.is_none()) =>
        {
            text("Chame a função e observe as ações que ela executa.")
        }
        LearningTopic::Function if !hint.parameters.is_empty() => format!(
            "{} `{name}` {}.",
            text("Mude um argumento na chamada de"),
            text("e compare o resultado devolvido")
        ),
        LearningTopic::Procedure => format!(
            "{} `{name}` {}.",
            text("Altere um parâmetro usado ao chamar"),
            text("e observe quais valores mudam")
        ),
        LearningTopic::Function => {
            text("Chame a função em outro ponto do programa e compare o valor que ela devolve.")
        }
        LearningTopic::Decision if hint.name.as_deref() == Some("escolha") => {
            text("Troque o valor de `escolha` ou de um caso e observe qual bloco executa.")
        }
        LearningTopic::Decision => match (&hint.expression, &hint.experiment_expression) {
            (Some(current), Some(changed)) => format!(
                "{} “{current}” {} “{changed}” {}",
                text("Troque a condição"),
                text("por"),
                text("e execute de novo. O resultado muda?")
            ),
            _ => text("Mude a condição e execute novamente. O resultado muda?"),
        },
        LearningTopic::Loop
            if hint
                .name
                .as_deref()
                .and_then(|name| name.split_whitespace().next())
                == Some("repita") =>
        {
            text("Altere a condição e confirme que o bloco executa antes da verificação.")
        }
        LearningTopic::Loop
            if hint
                .name
                .as_deref()
                .and_then(|name| name.split_whitespace().next())
                == Some("para") =>
        {
            text("Altere um limite do `para` e observe quantas vezes o bloco se repete.")
        }
        LearningTopic::Loop => text("Altere a condição e observe quantas vezes o bloco se repete."),
        LearningTopic::Input => {
            text("Digite outro valor e observe como o restante do programa reage.")
        }
        LearningTopic::Output => match output_line_break(hint) {
            Some(true) => text(
                "Execute duas instruções de saída seguidas e compare em quais linhas aparecem.",
            ),
            Some(false) => {
                text("Execute duas instruções de saída seguidas e observe se ficam na mesma linha.")
            }
            None => text("Altere o texto e execute novamente para observar a saída."),
        },
        LearningTopic::Variable if hint.variables.iter().any(|variable| variable.is_constant) => {
            String::new()
        }
        LearningTopic::Variable => {
            text("Altere o valor inicial ou um elemento do vetor e observe o efeito no resultado.")
        }
        LearningTopic::Assignment => hint.name.as_deref().map_or_else(
            || text("Altere o valor à direita do operador e observe a variável."),
            |target| {
                format!(
                    "{} `{target}`.",
                    text("Altere o valor à direita do operador e observe o valor de")
                )
            },
        ),
        LearningTopic::Call | LearningTopic::Import | LearningTopic::LibraryCall => String::new(),
        LearningTopic::Return => {
            text("Altere o valor devolvido e veja como a chamada da função reage.")
        }
        LearningTopic::Program => {
            text("Selecione uma linha com entrada, saída, condição ou laço para experimentar.")
        }
        LearningTopic::Unrecognized => String::new(),
    };
    (explanation, concept, experiment)
}

pub(super) fn format_learning_symbol(
    symbol: &LearningSymbol,
    dialect: Language,
    ui_language: UiLanguage,
    parameter: bool,
) -> String {
    let text = |portuguese| tr(ui_language, portuguese).to_owned();
    let dimensions = match symbol.dimensions.len() {
        0 => String::new(),
        1 => {
            let dimension = symbol.dimensions[0];
            match (dialect, dimension.lower_bound, dimension.upper_bound) {
                (Language::Visualg, Some(lower), Some(upper)) => format!(
                    "; {} {lower} {} {upper}",
                    text("vetor com índices de"),
                    text("até")
                ),
                (Language::Portugol | Language::GPortugol, Some(lower), Some(upper)) => format!(
                    "; {} {} {} {}",
                    text("vetor com"),
                    upper.saturating_sub(lower).saturating_add(1),
                    text("posições, índices de"),
                    format!("{lower} {} {upper}", text("até"))
                ),
                _ => format!("; {}", text("vetor")),
            }
        }
        dimensions => format!(
            "; {} {dimensions} {}",
            text("matriz com"),
            text("dimensões")
        ),
    };
    let type_explanation = match symbol.type_name.as_str() {
        "cadeia" => format!(" ({})", text("tipo usado para guardar texto")),
        "literal" => format!(" ({})", text("tipo usado para guardar texto")),
        "inteiro" => format!(" ({})", text("número sem parte decimal")),
        "real" => format!(" ({})", text("número que pode ter parte decimal")),
        "logico" | "lógico" => format!(" ({})", text("valor verdadeiro ou falso")),
        "caracter" | "caractere" => format!(" ({})", text("um caractere")),
        _ => String::new(),
    };
    let role = if parameter {
        String::new()
    } else if symbol.is_constant {
        text("é uma constante do tipo")
    } else {
        text("é uma variável do tipo")
    };
    let reference = if symbol.by_reference {
        format!(
            "; {}",
            text("passado por referência, pode alterar o valor original")
        )
    } else {
        String::new()
    };
    let initial = symbol
        .initializer
        .as_ref()
        .map_or_else(String::new, |value| {
            format!("; {} `{value}`", text("valor inicial"))
        });
    if parameter {
        format!(
            "`{}`: `{}`{}{}{}{}",
            symbol.name, symbol.type_name, type_explanation, dimensions, reference, initial
        )
    } else {
        format!(
            "`{}` {} `{}`{}{}{}{}.",
            symbol.name,
            role,
            symbol.type_name,
            type_explanation,
            if symbol.is_constant {
                format!("; {}", text("seu valor não muda durante a execução"))
            } else {
                format!("; {}", text("seu valor pode mudar durante a execução"))
            },
            dimensions,
            initial
        )
    }
}

pub(super) fn update_cursor_position(
    buffer: &gtk::TextBuffer,
    label: &gtk::Label,
    language: UiLanguage,
) {
    let cursor = buffer.iter_at_offset(buffer.cursor_position());
    label.set_text(&format!(
        "{} {} · {} {}",
        tr(language, "Linha"),
        cursor.line() + 1,
        tr(language, "Coluna"),
        cursor.line_offset() + 1
    ));
}
