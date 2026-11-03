use crate::components::{
    content_panel::{self, Language, ProjectTemplate, SourceDocument, SyntaxTags},
    header, icon, sidebar,
    sidebar::ProjectFile,
    start_page,
};
use crate::execution::{self, ControlAction, DebuggerSnapshot, ExecutionEvent, InputPrompt};
use crate::learning::{
    LearningHint, LearningSymbol, LearningTopic, learning_hint_with_runtime_cancellable,
};
use gtk::prelude::*;
use relm4::prelude::*;
use std::io::{self, Write};
use std::{
    cell::{Cell, RefCell},
    fs,
    fs::OpenOptions,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::Sender,
    },
    time::Duration,
};

const MAX_EXECUTION_EVENTS_PER_TICK: usize = 128;
static NEXT_ATOMIC_SAVE_ID: AtomicU64 = AtomicU64::new(1);

fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "destination has no file name")
    })?;
    let mut created = None;
    for _ in 0..16 {
        let id = NEXT_ATOMIC_SAVE_ID.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".{}.{}.{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            id
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => {
                created = Some((temporary, file));
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    let (temporary, mut file) = created.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate temporary save file",
        )
    })?;
    let result = (|| {
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UiLanguage {
    Portuguese,
    English,
    Spanish,
}

impl UiLanguage {
    fn key(self) -> &'static str {
        match self {
            Self::Portuguese => "pt-BR",
            Self::English => "en",
            Self::Spanish => "es",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        match key {
            "pt" | "pt-BR" => Some(Self::Portuguese),
            "en" => Some(Self::English),
            "es" => Some(Self::Spanish),
            _ => None,
        }
    }

    fn from_locale_name(locale: &str) -> Option<Self> {
        let language = locale
            .split(['_', '-', '.', '@'])
            .next()?
            .to_ascii_lowercase();
        match language.as_str() {
            "pt" => Some(Self::Portuguese),
            "en" => Some(Self::English),
            "es" => Some(Self::Spanish),
            _ => None,
        }
    }

    fn from_language_names<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        names
            .into_iter()
            .find_map(|name| Self::from_locale_name(name.as_ref()))
            .unwrap_or(Self::English)
    }

    fn from_preference<I, S>(saved: Option<&str>, system_languages: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        saved
            .and_then(Self::from_key)
            .unwrap_or_else(|| Self::from_language_names(system_languages))
    }

    fn from_system() -> Self {
        Self::from_language_names(gtk::glib::language_names())
    }

    fn selected_index(self) -> u32 {
        match self {
            Self::Portuguese => 0,
            Self::English => 1,
            Self::Spanish => 2,
        }
    }

    fn from_selected_index(index: u32) -> Self {
        match index {
            1 => Self::English,
            2 => Self::Spanish,
            _ => Self::Portuguese,
        }
    }
}

pub(crate) fn tr(language: UiLanguage, portuguese: &'static str) -> &'static str {
    let translations = match language {
        UiLanguage::Portuguese => return portuguese,
        UiLanguage::English => match portuguese {
            "Meu primeiro programa" => "My first program",
            "Início" => "Home",
            "Vamos programar?" => "Ready to code?",
            "Continue de onde parou ou comece a transformar suas ideias em algoritmos." => {
                "Pick up where you left off or start turning your ideas into algorithms."
            }
            "Programas recentes" => "Recent programs",
            "Ver todos os programas recentes" => "View all recent programs",
            "Remover dos recentes" => "Remove from recent list",
            "Comece um novo programa" => "Start a new program",
            "Retome um trabalho existente" => "Continue existing work",
            "Projeto Portugol" => "Portugol project",
            "Projeto VisuAlg" => "VisuAlg project",
            "Projeto Potigol" => "Potigol project",
            "Projeto G-Portugol" => "G-Portugol project",
            "Comece com um arquivo .por" => "Start with a .por file",
            "Comece com um arquivo .alg" => "Start with a .alg file",
            "Dialetos do Portugol: G-Portugol, Portugol 2.0 e VisuAlg" => {
                "Portugol dialects: G-Portugol, Portugol 2.0, and VisuAlg"
            }
            "3 variantes" => "3 variants",
            "2 variantes" => "2 variants",
            "Criar programa inicial (.poti)" => "Create a starter program (.poti)",
            "Selecionar uma pasta" => "Choose a folder",
            "Abra um projeto que não aparece na lista." => "Open a project that is not listed.",
            "Abra um arquivo .por, .alg, .poti ou .gpt diretamente no editor." => {
                "Open a .por, .alg, .poti, or .gpt file directly in the editor."
            }
            "Projeto" => "Project",
            "Abrir um programa recente" => "Open a recent program",
            "O arquivo ou a pasta recente não está mais disponível." => {
                "This recent file or folder is no longer available."
            }
            "Abrir pasta" => "Open folder",
            "Abrir pasta de projeto" => "Open project folder",
            "Abrir projeto" => "Open project",
            "Abrir arquivo" => "Open file",
            "Selecionar arquivo" => "Select file",
            "Todos os arquivos" => "All files",
            "Abra um projeto antes de selecionar um arquivo." => {
                "Open a project before selecting a file."
            }
            "Não foi possível selecionar o arquivo" => "Could not select the file",
            "Abrir projeto ou programa" => "Open project or program",
            "Nenhum projeto aberto" => "No project open",
            "Abra uma pasta ou um programa para começar." => {
                "Open a folder or program to get started."
            }
            "Salvar arquivo" => "Save file",
            "Salvar" => "Save",
            "Configurações" => "Settings",
            "Fechar" => "Close",
            "Sobre" => "About",
            "Sobre o Potoo IDE" => "About Potoo IDE",
            "Código do projeto" => "Project source code",
            "Versão e links do projeto" => "Project version and links",
            "← Configurações" => "← Settings",
            "Criado por Diego Pereira do Nascimento" => "Created by Diego Pereira do Nascimento",
            "Versão" => "Version",
            "Relatar um problema" => "Report an issue",
            "Repositório do projeto" => "Project repository",
            "Site do projeto" => "Project website",
            "Ajuda" => "Help",
            "Doar" => "Donate",
            "Contato" => "Contact",
            "Site do criador do projeto" => "Project creator's website",
            "Contato do criador" => "Contact the creator",
            "Executar" => "Run",
            "Minimizar" => "Minimize",
            "Maximizar" => "Maximize",
            "Restaurar" => "Restore",
            "Arquivos" => "Files",
            "Criar arquivo" => "Create file",
            "Criar item no projeto" => "Create project item",
            "Novo projeto" => "New project",
            "Criar projeto" => "Create project",
            "Criar um projeto Portugol, VisuAlg, Potigol ou G-Portugol" => {
                "Create a Portugol, VisuAlg, Potigol, or G-Portugol project"
            }
            "Selecione a linguagem/pseudolinguagem do projeto" => {
                "Select the project's language/pseudolanguage"
            }
            "Linguagem/pseudolinguagem" => "Language/pseudolanguage",
            "Depuração encerrada com erro" => "Debugging ended with an error",
            "Variante do Portugol" => "Portugol variant",
            "Variante do Potigol" => "Potigol variant",
            "Esta análise do Potigol é heurística e pode não reconhecer todos os casos da linguagem." => {
                "Potigol analysis here is heuristic and may not recognize every language construct."
            }
            "Uma rotina executa ações sem devolver um resultado; cada linguagem define como seus parâmetros são passados." => {
                "A routine performs actions without returning a result; each language defines how its parameters are passed."
            }
            "Portugol 2.0" => "Portugol 2.0",
            "G-Portugol" => "G-Portugol",
            "Potigol Clássico" => "Classic Potigol",
            "Potigol Jerimum (Jogo 2D)" => "Potigol Jerimum (2D Game)",
            "Arquivo inicial .por" => "Initial .por file",
            "Arquivo inicial .alg" => "Initial .alg file",
            "Arquivo inicial .poti" => "Initial .poti file",
            "Arquivo inicial .gpt" => "Initial .gpt file",
            "Crie uma pasta para reunir seus programas e exercícios." => {
                "Create a folder to keep your programs and exercises together."
            }
            "Nome do projeto" => "Project name",
            "Por exemplo: Meu primeiro projeto" => "For example: My first project",
            "Meu projeto" => "My project",
            "Local do projeto" => "Project location",
            "Pasta" => "Folder",
            "Criar dentro de" => "Create inside",
            "Pasta do projeto" => "Project folder",
            "A pasta selecionada não está disponível." => "The selected folder is unavailable.",
            "Erros de sintaxe e execução aparecerão aqui." => {
                "Syntax and runtime errors will appear here."
            }
            "Escolher pasta" => "Choose folder",
            "Escolha a pasta de destino" => "Choose a destination folder",
            "Selecione uma pasta local para criar o projeto." => {
                "Choose a local folder to create the project."
            }
            "Não foi possível escolher a pasta do projeto" => {
                "Could not choose the project folder"
            }
            "Digite um nome válido para o projeto." => "Enter a valid project name.",
            "Já existe um projeto com esse nome." => "A project with that name already exists.",
            "Não foi possível criar o projeto" => "Could not create the project",
            "Projeto criado" => "Project created",
            "Novo arquivo" => "New file",
            "Criar um arquivo Portugol, VisuAlg, Potigol ou G-Portugol" => {
                "Create a Portugol, VisuAlg, Potigol, or G-Portugol file"
            }
            "Nova pasta" => "New folder",
            "Criar uma pasta no projeto" => "Create a folder in the project",
            "Criar pasta" => "Create folder",
            "Organize seus programas em uma pasta do projeto." => {
                "Organize your programs in a project folder."
            }
            "Nome da pasta" => "Folder name",
            "Nome da pasta, por exemplo: exercicios" => "Folder name, for example: exercises",
            "Pasta criada" => "Folder created",
            "Já existe uma pasta com esse nome." => "A folder with that name already exists.",
            "Digite um nome válido para a pasta." => "Enter a valid folder name.",
            "Não foi possível criar a pasta" => "Could not create folder",
            "Abra uma pasta de projeto antes de criar itens." => {
                "Open a project folder before creating items."
            }
            "Aprenda enquanto programa" => "Learn as you code",
            "Abrir Aprenda enquanto programa" => "Open Learn as you code",
            "Recolher Aprenda enquanto programa" => "Collapse Learn as you code",
            "Abrir depurador" => "Open debugger",
            "Recolher depurador" => "Collapse debugger",
            "Abrir prévia do jogo" => "Open game preview",
            "Recolher prévia do jogo" => "Hide game preview",
            "Prévia do jogo" => "Game preview",
            "Execute um programa Potigol Jerimum para ver o jogo aqui." => {
                "Run a Potigol Jerimum program to see the game here."
            }
            "Analisando a linha selecionada…" => "Analyzing selected line…",
            "Linha 1 · programa" => "Line 1 · program",
            "Selecione uma linha para ver uma explicação contextual do programa." => {
                "Select a line to see a contextual explanation of the program."
            }
            "Mova o cursor para uma condição, um laço, uma entrada ou uma saída." => {
                "Move the cursor to a condition, loop, input, or output statement."
            }
            "Conceito: programa" => "Concept: program",
            "O painel acompanha a linha selecionada e o conteúdo atual do editor." => {
                "This panel follows the selected line and the editor's current contents."
            }
            "decisão" => "decision",
            "laço" => "loop",
            "entrada" => "input",
            "saída" => "output",
            "variável" => "variable",
            "função" => "function",
            "procedimento" => "procedure",
            "atribuição" => "assignment",
            "retorno" => "return",
            "programa" => "program",
            "Problemas nesta linha" => "Problems on this line",
            "Nenhum problema conhecido nesta linha." => "No known problem on this line.",
            "Erro de sintaxe" => "Syntax error",
            "Erro de tipo" => "Type error",
            "Erro durante a execução" => "Runtime error",
            "Possível problema lógico" => "Possible logic issue",
            "Índice fora dos limites" => "Index out of bounds",
            "A condição é sempre verdadeira; o ramo alternativo pode nunca ser executado." => {
                "This condition is always true; the alternative branch may never run."
            }
            "A condição é sempre falsa; o bloco pode nunca ser executado." => {
                "This condition is always false; the block may never run."
            }
            "Confira se a condição deveria comparar uma variável." => {
                "Check whether the condition should compare a variable."
            }
            "O índice {value} está fora dos limites de `{name}` ({lower} a {upper})." => {
                "Index {value} is outside the bounds of `{name}` ({lower} to {upper})."
            }
            "Use um índice dentro da faixa declarada para esse vetor ou matriz." => {
                "Use an index within the declared range for this array or matrix."
            }
            "Há um problema de sintaxe nesta linha." => "There is a syntax problem on this line.",
            "O interpretador encontrou uma incompatibilidade de tipos nesta linha." => {
                "The interpreter found a type mismatch on this line."
            }
            "A execução encontrou um problema nesta linha." => {
                "Execution encountered a problem on this line."
            }
            "Detalhe original do interpretador (português): {message}" => {
                "Original interpreter detail (Portuguese): {message}"
            }
            "Conceito:" => "Concept:",
            "Uma condição escolhe qual bloco executar com base em verdadeiro ou falso." => {
                "A condition selects which block to run based on a true or false result."
            }
            "Um laço repete instruções de acordo com uma condição ou faixa de valores." => {
                "A loop repeats instructions based on a condition or range of values."
            }
            "leia solicita um valor para uma variável antes de continuar a execução." => {
                "leia requests a value for a variable before execution continues."
            }
            "escreva e escreval mostram valores na saída do programa." => {
                "escreva and escreval display values in program output."
            }
            "Uma variável guarda um valor que o programa pode consultar ou alterar." => {
                "A variable stores a value that the program can read or change."
            }
            "A linha selecionada participa do fluxo do algoritmo. Selecione outra linha para ver seu papel." => {
                "The selected line is part of the algorithm's flow. Select another line to see its role."
            }
            "Troque a condição" => "Change the condition",
            "por" => "to",
            "e execute de novo. O resultado muda?" => "and run it again. Does the result change?",
            "Mude a condição e execute novamente. O resultado muda?" => {
                "Change the condition and run it again. Does the result change?"
            }
            "Mude a condição ou os limites do laço e observe quantas vezes ele repete." => {
                "Change the condition or loop bounds and see how many times it repeats."
            }
            "Digite outro valor e observe como o restante do programa reage." => {
                "Enter a different value and see how the rest of the program responds."
            }
            "Altere o texto ou o valor exibido e execute novamente." => {
                "Change the displayed text or value and run it again."
            }
            "Altere o valor armazenado e observe o efeito no resultado." => {
                "Change the stored value and observe its effect on the result."
            }
            "Selecione uma linha com entrada, saída, condição ou laço para experimentar." => {
                "Select an input, output, condition, or loop line to experiment."
            }
            "A condição" => "The condition",
            "é avaliada conforme os valores atuais." => "is evaluated using the current values.",
            "Cada repetição executa novamente as instruções do bloco." => {
                "Each repetition runs the block's instructions again."
            }
            "A entrada é convertida para o tipo da variável solicitada." => {
                "Input is converted to the requested variable's type."
            }
            "A saída permite observar valores calculados pelo programa." => {
                "Output lets you observe values calculated by the program."
            }
            "O nome identifica onde um valor fica guardado." => {
                "The name identifies where a value is stored."
            }
            "Uma instrução é executada na ordem definida pelo algoritmo." => {
                "An instruction runs in the order defined by the algorithm."
            }
            "A condição é avaliada como verdadeira ou falsa." => {
                "The condition evaluates to true or false."
            }
            "A função" => "The function",
            "Esta declaração parece ser de uma função; complete a assinatura para identificar nome, parâmetros e retorno." => {
                "This appears to be a function declaration; complete its signature to identify its name, parameters, and return type."
            }
            "Esta declaração parece ser de um procedimento; complete a assinatura para identificar nome e parâmetros." => {
                "This appears to be a procedure declaration; complete its signature to identify its name and parameters."
            }
            "O procedimento" => "The procedure",
            "recebe" => "receives",
            "nenhum parâmetro" => "no parameters",
            "O tipo `vazio` indica que a função não devolve um valor." => {
                "The `vazio` type means the function does not return a value."
            }
            "Esta função não declara um tipo de retorno." => {
                "This function does not declare a return type."
            }
            "Ela devolve um valor do tipo" => "It returns a value of type",
            "Esse tipo representa texto." => "This type represents text.",
            "Esse tipo representa texto no G-Portugol." => {
                "This type represents text in G-Portugol."
            }
            "A função produz texto, representado pelo tipo `literal` no G-Portugol." => {
                "The function produces text, represented by the `literal` type in G-Portugol."
            }
            "Os parâmetros recebem os dados usados pela função; a ausência de tipo indica que ela não devolve resultado." => {
                "Parameters receive the data used by the function; no declared type means it does not return a result."
            }
            "O retorno não foi identificado." => "The return type could not be identified.",
            "Ele executa ações e não devolve um valor como resultado." => {
                "It performs actions and does not return a value."
            }
            "A declaração define o nome e o tipo de um dado usado pelo algoritmo." => {
                "A declaration defines the name and type of data used by the algorithm."
            }
            "determina qual bloco será executado conforme resulte em verdadeiro ou falso." => {
                "determines which block runs based on whether it is true or false."
            }
            "leia solicita um valor e o armazena na variável informada antes de o programa continuar." => {
                "leia requests a value and stores it in the given variable before the program continues."
            }
            "escreva e escreval mostram na saída os valores e textos informados." => {
                "escreva and escreval display the given values and text in the output."
            }
            "imprima mostra na saída os valores e textos informados." => {
                "imprima displays the given values and text in the output."
            }
            "A atribuição calcula o lado direito e atualiza o valor da variável à esquerda." => {
                "An assignment evaluates the right side and updates the variable on the left."
            }
            "Esta função não devolve valor; retorne encerra sua execução." => {
                "This function returns no value; retorne ends its execution."
            }
            "retorne encerra a rotina e pode entregar um resultado a quem a chamou." => {
                "retorne ends the routine and can provide a result to its caller."
            }
            "retorne encerra a função e entrega um valor do tipo" => {
                "retorne ends the function and gives a value of type"
            }
            "a quem a chamou" => "to its caller",
            "Parâmetros são os dados recebidos pela função. `vazio` informa que nenhum resultado é devolvido." => {
                "Parameters are the data received by a function. `vazio` means no result is returned."
            }
            "A função produz um valor de texto, representado pelo tipo `cadeia` em Portugol 2.0." => {
                "The function produces text, represented by the `cadeia` type in Portugol 2.0."
            }
            "A função devolve um resultado do tipo `{return_type}`. Seus parâmetros recebem os dados usados no cálculo." => {
                "The function returns a `{return_type}` value. Its parameters receive the data used in the calculation."
            }
            "A função devolve um resultado do tipo" => "The function returns a value of type",
            "Seus parâmetros recebem os dados usados no cálculo." => {
                "Its parameters receive the data used in the calculation."
            }
            "Parâmetros são os dados recebidos pela função; o tipo de retorno descreve o resultado produzido." => {
                "Parameters are the data received by a function; its return type describes the result it produces."
            }
            "No VisuAlg, procedimento é uma rotina de ações. Parâmetros por referência podem alterar a variável enviada por quem chamou." => {
                "In VisuAlg, a procedure is a routine of actions. Reference parameters can change the variable passed by its caller."
            }
            "O tipo define quais valores podem ser guardados; o formato indica se há um valor, um vetor ou uma matriz." => {
                "The type defines which values can be stored; the shape indicates a scalar, vector, or matrix."
            }
            "Atribuir associa um novo valor a uma variável; o operador determina como o valor é calculado." => {
                "Assignment gives a variable a new value; the operator determines how that value is calculated."
            }
            "O retorno leva o resultado da função de volta ao ponto que a chamou." => {
                "The return carries the function result back to the point that called it."
            }
            "Troque o valor atribuído e acompanhe a próxima leitura da variável." => {
                "Change the assigned value and follow the next read of the variable."
            }
            "Altere o valor devolvido e veja como a chamada da função reage." => {
                "Change the returned value and see how the function call responds."
            }
            "Altere o valor inicial ou um elemento do vetor e observe o efeito no resultado." => {
                "Change the initial value or a vector element and observe the effect on the result."
            }
            "Mude um argumento na chamada de" => "Change an argument in the call to",
            "e compare o resultado devolvido" => "and compare the returned result",
            "Altere um parâmetro usado ao chamar" => "Change a parameter passed when calling",
            "e observe quais valores mudam" => "and observe which values change",
            "vetor com índices de" => "vector with indices from",
            "vetor com" => "vector with",
            "posições, índices de" => "positions, with indices from",
            "matriz com" => "matrix with",
            "dimensões" => "dimensions",
            "vetor" => "vector",
            "tipo usado para guardar texto" => "type used to store text",
            "número sem parte decimal" => "number without a fractional part",
            "número que pode ter parte decimal" => "number that can have a fractional part",
            "valor verdadeiro ou falso" => "true or false value",
            "um caractere" => "one character",
            "é uma constante do tipo" => "is a constant of type",
            "é uma variável do tipo" => "is a variable of type",
            "passado por referência, pode alterar o valor original" => {
                "passed by reference, so it can change the original value"
            }
            "valor inicial" => "initial value",
            "seu valor não muda durante a execução" => {
                "its value does not change during execution"
            }
            "seu valor pode mudar durante a execução" => "its value can change during execution",
            "Experimente" => "Try it",
            "Saída" => "Output",
            "Problemas" => "Problems",
            "Depurador" => "Debugger",
            "Interface" => "Interface",
            "Espaços" => "Spaces",
            "Linha" => "Line",
            "Coluna" => "Column",
            "A saída aparecerá aqui após a execução." => {
                "Program output will appear here after you run it."
            }
            "Pronto para executar" => "Ready to run",
            "Executar o programa atual" => "Run the current program",
            "Executando o programa…" => "Running program…",
            "Interrompendo execução…" => "Stopping execution…",
            "Execução concluída." => "Execution finished.",
            "O programa terminou com um erro." => "The program ended with an error.",
            "Abra um programa antes de executar." => "Open a program before running it.",
            "Interrompa a execução atual antes de iniciar a depuração." => {
                "Stop the current execution before starting the debugger."
            }
            "Depuração iniciada…" => "Debugging started…",
            "Execução interrompida." => "Execution stopped.",
            "Entrada solicitada:" => "Input requested:",
            "Digite a entrada do programa" => "Enter program input",
            "Pressione Esc para cancelar a execução." => "Press Esc to cancel execution.",
            "Iniciar depuração" => "Start debugging",
            "Continuar" => "Continue",
            "Avançar um passo" => "Step",
            "Parar" => "Stop",
            "Parar depuração" => "Stop debugging",
            "Aguardando depuração" => "Waiting for debugging",
            "Analisando linha…" => "Analyzing line…",
            "Aguardando a próxima linha…" => "Waiting for the next line…",
            "Os valores das variáveis aparecerão durante a execução passo a passo." => {
                "Variable values will appear while stepping through the program."
            }
            "Nenhuma variável visível." => "No visible variables.",
            "Pausado na linha" => "Paused at line",
            "passo a passo" => "step",
            "pausa" => "pause",
            "ponto de depuração" => "debug point",
            "depuração" => "debugging",
            "Digite um valor e pressione Enter" => "Enter a value and press Enter",
            "O interpretador foi encerrado antes de concluir o programa." => {
                "The interpreter exited before the program finished."
            }
            "Não foi possível concluir a execução." => "Could not finish execution.",
            "Não foi possível enviar a entrada ao programa." => {
                "Could not send input to the program."
            }
            "Alterações da sessão anterior recuperadas." => {
                "Changes from the previous session were recovered."
            }
            "Personalize a aparência e o editor." => "Customize the appearance and editor.",
            "Aparência" => "Appearance",
            "Sistema" => "System",
            "Seguir o sistema" => "Follow system",
            "Claro" => "Light",
            "Escuro" => "Dark",
            "Tema" => "Theme",
            "Escolha como o Potoo aparece nesta tela." => "Choose how Potoo looks on this screen.",
            "Editor" => "Editor",
            "Tamanho da fonte" => "Font size",
            "Ajuste a leitura do código." => "Adjust code readability.",
            "Numeração de linhas" => "Line numbers",
            "Mostra o número de cada linha do código." => "Shows the number of each code line.",
            "Quebra automática" => "Word wrap",
            "Ajusta linhas longas à largura do editor." => "Wrap long lines to fit the editor.",
            "Idioma da interface" => "Interface language",
            "A alteração de idioma será aplicada após salvar e reiniciar o aplicativo." => {
                "The language change will be applied after saving and restarting the app."
            }
            "Salve este arquivo antes de alterar o idioma." => {
                "Save this file before changing the language."
            }
            "A alteração de idioma foi cancelada." => "The language change was canceled.",
            "Não foi possível reiniciar o aplicativo" => "Could not restart the app",
            "Não foi possível restaurar as configurações" => "Could not restore the settings",
            "Recuo" => "Indentation",
            "Espaços por nível de recuo." => "Spaces inserted for each indentation level.",
            "2 espaços" => "2 spaces",
            "4 espaços" => "4 spaces",
            "Esta pasta não contém arquivos visíveis." => {
                "This folder contains no visible files."
            }
            "Nenhum programa .por, .alg, .poti ou .gpt nesta pasta. Os demais arquivos aparecem na lista." => {
                "No .por, .alg, .poti, or .gpt programs in this folder. Other files appear in the list."
            }
            "Fechar arquivo" => "Close file",
            "Escolha um nome e a linguagem do novo arquivo." => {
                "Choose a name and language for the new file."
            }
            "Crie um arquivo dentro do projeto atual." => "Create a file in the current project.",
            "Nome do arquivo" => "File name",
            "Nome do arquivo, por exemplo: exercicio-1" => "File name, for example: exercise-1",
            "Linguagem" => "Language",
            "Cancelar" => "Cancel",
            "Alterações não salvas" => "Unsaved changes",
            "Descartar e abrir" => "Discard and open",
            "Este arquivo tem alterações não salvas" => "This file has unsaved changes",
            "O arquivo aberto tem alterações que ainda não foram salvas. O que você gostaria de fazer?" => {
                "This file has unsaved changes. What would you like to do?"
            }
            "Salve o arquivo antes de fechar ou descarte as alterações para continuar." => {
                "Save this file before closing it, or discard the changes to continue."
            }
            "Continuar editando" => "Keep editing",
            "Descartar alterações" => "Discard changes",
            "Salvar e fechar" => "Save and close",
            "Há alterações não salvas" => "There are unsaved changes",
            "Descartar e sair" => "Discard and quit",
            "Salve os arquivos abertos antes de trocar de pasta ou descarte as alterações para continuar." => {
                "Save open files before switching folders, or discard the changes to continue."
            }
            "Salve os arquivos antes de sair ou descarte as alterações para fechar o Potoo IDE." => {
                "Save your files before quitting, or discard the changes to close Potoo IDE."
            }
            "Nenhum problema detectado" => "No problems detected",
            "Abrindo arquivo…" => "Opening file…",
            "Abrindo pasta…" => "Opening folder…",
            "Não foi possível atualizar os arquivos do projeto" => {
                "Could not refresh the project files"
            }
            "Os erros de sintaxe e execução aparecerão aqui." => {
                "Syntax and runtime errors will appear here."
            }
            "Arquivo aberto" => "File opened",
            "Arquivo salvo" => "File saved",
            "Pasta aberta. Crie um arquivo .por, .alg, .poti ou .gpt para começar." => {
                "Folder opened. Create a .por, .alg, .poti, or .gpt file to get started."
            }
            "Novo arquivo criado" => "New file created",
            "Já existe um arquivo com esse nome." => "A file with that name already exists.",
            "Já existe um item com esse nome." => "An item with that name already exists.",
            "Renomear" => "Rename",
            "Excluir" => "Delete",
            "Renomear arquivo" => "Rename file",
            "Renomear pasta" => "Rename folder",
            "Escolha um novo nome para este item do projeto." => {
                "Choose a new name for this project item."
            }
            "Novo nome" => "New name",
            "O item selecionado não está mais disponível." => {
                "The selected item is no longer available."
            }
            "Item renomeado." => "Item renamed.",
            "Item excluído." => "Item deleted.",
            "Não foi possível renomear o item" => "Could not rename the item",
            "Não foi possível excluir o item" => "Could not delete the item",
            "Excluir item do projeto" => "Delete project item",
            "Deseja excluir este item do projeto?" => "Do you want to delete this project item?",
            "Há alterações não salvas em arquivos deste item; elas serão perdidas." => {
                "Unsaved changes in files in this item will be lost."
            }
            "O item e todo o seu conteúdo serão excluídos permanentemente." => {
                "This item and all of its contents will be permanently deleted."
            }
            "Não foi possível abrir o arquivo" => "Could not open file",
            "Não foi possível abrir a pasta" => "Could not open folder",
            "Não foi possível salvar o arquivo" => "Could not save file",
            "Não foi possível criar o arquivo" => "Could not create file",
            "O editor abre apenas arquivos .por, .alg, .poti e .gpt." => {
                "The editor opens only .por, .alg, .poti, and .gpt files."
            }
            "Abra uma pasta de projeto antes de criar arquivos." => {
                "Open a project folder before creating files."
            }
            "Crie ou abra um arquivo antes de salvar." => "Create or open a file before saving.",
            "Não foi possível salvar as configurações" => "Could not save settings",
            "Não foi possível salvar a sessão" => "Could not save session",
            "Use um nome simples, sem pastas ou caracteres especiais." => {
                "Use a simple name without folders or special characters."
            }
            "Use a extensão .por, .alg, .poti ou .gpt." => {
                "Use the .por, .alg, .poti, or .gpt extension."
            }
            "Salve o código com a extensão .por, .alg, .poti ou .gpt." => {
                "Save the code with the .por, .alg, .poti, or .gpt extension."
            }
            "Selecione uma pasta local para abrir." => "Choose a local folder to open.",
            "Selecione um arquivo local." => "Choose a local file.",
            "Escolha um caminho local para salvar o arquivo." => {
                "Choose a local path to save the file."
            }
            "Não foi possível abrir a janela de salvamento" => "Could not open the save dialog",
            "Abrir arquivo de código" => "Open source file",
            "Portugol, VisuAlg, Potigol e G-Portugol (*.por, *.alg, *.poti, *.gpt)" => {
                "Portugol, VisuAlg, Potigol, and G-Portugol (*.por, *.alg, *.poti, *.gpt)"
            }
            "Não foi possível abrir o arquivo de código" => "Could not open the source file",
            "novo-arquivo" => "new-file",
            "caminho de configuração inválido" => "invalid settings path",
            "Não foi possível ler a sessão anterior. Os arquivos originais continuam intactos." => {
                "The previous session could not be read. Your original files are unchanged."
            }
            "Salvar arquivo (⌘S)" => "Save file (⌘S)",
            "Salvar arquivo (Ctrl+S)" => "Save file (Ctrl+S)",
            _ => portuguese,
        },
        UiLanguage::Spanish => match portuguese {
            "Meu primeiro programa" => "Mi primer programa",
            "Início" => "Inicio",
            "Vamos programar?" => "¿Vamos a programar?",
            "Continue de onde parou ou comece a transformar suas ideias em algoritmos." => {
                "Continúa donde lo dejaste o empieza a convertir tus ideas en algoritmos."
            }
            "Programas recentes" => "Programas recientes",
            "Ver todos os programas recentes" => "Ver todos los programas recientes",
            "Remover dos recentes" => "Quitar de recientes",
            "Comece um novo programa" => "Empieza un programa nuevo",
            "Retome um trabalho existente" => "Retoma un trabajo existente",
            "Projeto Portugol" => "Proyecto Portugol",
            "Projeto VisuAlg" => "Proyecto VisuAlg",
            "Projeto Potigol" => "Proyecto Potigol",
            "Projeto G-Portugol" => "Proyecto G-Portugol",
            "Comece com um arquivo .por" => "Empieza con un archivo .por",
            "Comece com um arquivo .alg" => "Empieza con un archivo .alg",
            "Dialetos do Portugol: G-Portugol, Portugol 2.0 e VisuAlg" => {
                "Dialectos de Portugol: G-Portugol, Portugol 2.0 y VisuAlg"
            }
            "3 variantes" => "3 variantes",
            "2 variantes" => "2 variantes",
            "Criar programa inicial (.poti)" => "Crear un programa inicial (.poti)",
            "Selecionar uma pasta" => "Seleccionar una carpeta",
            "Abra um projeto que não aparece na lista." => {
                "Abre un proyecto que no aparece en la lista."
            }
            "Abra um arquivo .por, .alg, .poti ou .gpt diretamente no editor." => {
                "Abre un archivo .por, .alg, .poti o .gpt directamente en el editor."
            }
            "Projeto" => "Proyecto",
            "Abrir um programa recente" => "Abrir un programa reciente",
            "O arquivo ou a pasta recente não está mais disponível." => {
                "Este archivo o carpeta reciente ya no está disponible."
            }
            "Abrir pasta" => "Abrir carpeta",
            "Abrir pasta de projeto" => "Abrir carpeta del proyecto",
            "Abrir projeto" => "Abrir proyecto",
            "Abrir arquivo" => "Abrir archivo",
            "Selecionar arquivo" => "Seleccionar archivo",
            "Todos os arquivos" => "Todos los archivos",
            "Abra um projeto antes de selecionar um arquivo." => {
                "Abre un proyecto antes de seleccionar un archivo."
            }
            "Não foi possível selecionar o arquivo" => "No se pudo seleccionar el archivo",
            "Abrir projeto ou programa" => "Abrir proyecto o programa",
            "Nenhum projeto aberto" => "No hay ningún proyecto abierto",
            "Abra uma pasta ou um programa para começar." => {
                "Abre una carpeta o un programa para empezar."
            }
            "Salvar arquivo" => "Guardar archivo",
            "Salvar" => "Guardar",
            "Configurações" => "Configuración",
            "Fechar" => "Cerrar",
            "Sobre" => "Acerca de",
            "Sobre o Potoo IDE" => "Acerca de Potoo IDE",
            "Código do projeto" => "Código del proyecto",
            "Versão e links do projeto" => "Versión y enlaces del proyecto",
            "← Configurações" => "← Configuración",
            "Criado por Diego Pereira do Nascimento" => "Creado por Diego Pereira do Nascimento",
            "Versão" => "Versión",
            "Relatar um problema" => "Informar de un problema",
            "Repositório do projeto" => "Repositorio del proyecto",
            "Site do projeto" => "Sitio del proyecto",
            "Ajuda" => "Ayuda",
            "Doar" => "Donar",
            "Contato" => "Contacto",
            "Site do criador do projeto" => "Sitio del creador del proyecto",
            "Contato do criador" => "Contactar al creador",
            "Executar" => "Ejecutar",
            "Minimizar" => "Minimizar",
            "Maximizar" => "Maximizar",
            "Restaurar" => "Restaurar",
            "Arquivos" => "Archivos",
            "Criar arquivo" => "Crear archivo",
            "Criar item no projeto" => "Crear elemento del proyecto",
            "Novo projeto" => "Nuevo proyecto",
            "Criar projeto" => "Crear proyecto",
            "Criar um projeto Portugol, VisuAlg, Potigol ou G-Portugol" => {
                "Crear un proyecto Portugol, VisuAlg, Potigol o G-Portugol"
            }
            "Selecione a linguagem/pseudolinguagem do projeto" => {
                "Selecciona el lenguaje/pseudolenguaje del proyecto"
            }
            "Linguagem/pseudolinguagem" => "Lenguaje/pseudolenguaje",
            "Depuração encerrada com erro" => "La depuración terminó con un error",
            "Variante do Portugol" => "Variante de Portugol",
            "Variante do Potigol" => "Variante de Potigol",
            "Esta análise do Potigol é heurística e pode não reconhecer todos os casos da linguagem." => {
                "El análisis de Potigol es heurístico y puede no reconocer todos los casos del lenguaje."
            }
            "Uma rotina executa ações sem devolver um resultado; cada linguagem define como seus parâmetros são passados." => {
                "Una rutina realiza acciones sin devolver un resultado; cada lenguaje define cómo se pasan sus parámetros."
            }
            "Portugol 2.0" => "Portugol 2.0",
            "G-Portugol" => "G-Portugol",
            "Potigol Clássico" => "Potigol clásico",
            "Potigol Jerimum (Jogo 2D)" => "Potigol Jerimum (juego 2D)",
            "Arquivo inicial .por" => "Archivo inicial .por",
            "Arquivo inicial .alg" => "Archivo inicial .alg",
            "Arquivo inicial .poti" => "Archivo inicial .poti",
            "Arquivo inicial .gpt" => "Archivo inicial .gpt",
            "Crie uma pasta para reunir seus programas e exercícios." => {
                "Crea una carpeta para reunir tus programas y ejercicios."
            }
            "Nome do projeto" => "Nombre del proyecto",
            "Por exemplo: Meu primeiro projeto" => "Por ejemplo: Mi primer proyecto",
            "Meu projeto" => "Mi proyecto",
            "Local do projeto" => "Ubicación del proyecto",
            "Pasta" => "Carpeta",
            "Criar dentro de" => "Crear dentro de",
            "Pasta do projeto" => "Carpeta del proyecto",
            "A pasta selecionada não está disponível." => {
                "La carpeta seleccionada no está disponible."
            }
            "Erros de sintaxe e execução aparecerão aqui." => {
                "Los errores de sintaxis y ejecución aparecerán aquí."
            }
            "Escolher pasta" => "Elegir carpeta",
            "Escolha a pasta de destino" => "Elige una carpeta de destino",
            "Selecione uma pasta local para criar o projeto." => {
                "Elige una carpeta local para crear el proyecto."
            }
            "Não foi possível escolher a pasta do projeto" => {
                "No se pudo elegir la carpeta del proyecto"
            }
            "Digite um nome válido para o projeto." => {
                "Escribe un nombre válido para el proyecto."
            }
            "Já existe um projeto com esse nome." => "Ya existe un proyecto con ese nombre.",
            "Não foi possível criar o projeto" => "No se pudo crear el proyecto",
            "Projeto criado" => "Proyecto creado",
            "Novo arquivo" => "Nuevo archivo",
            "Criar um arquivo Portugol, VisuAlg, Potigol ou G-Portugol" => {
                "Crear un archivo Portugol, VisuAlg, Potigol o G-Portugol"
            }
            "Nova pasta" => "Nueva carpeta",
            "Criar uma pasta no projeto" => "Crear una carpeta en el proyecto",
            "Criar pasta" => "Crear carpeta",
            "Organize seus programas em uma pasta do projeto." => {
                "Organiza tus programas en una carpeta del proyecto."
            }
            "Nome da pasta" => "Nombre de la carpeta",
            "Nome da pasta, por exemplo: exercicios" => {
                "Nombre de la carpeta, por ejemplo: ejercicios"
            }
            "Pasta criada" => "Carpeta creada",
            "Já existe uma pasta com esse nome." => "Ya existe una carpeta con ese nombre.",
            "Digite um nome válido para a pasta." => "Escribe un nombre válido para la carpeta.",
            "Não foi possível criar a pasta" => "No se pudo crear la carpeta",
            "Abra uma pasta de projeto antes de criar itens." => {
                "Abre una carpeta de proyecto antes de crear elementos."
            }
            "Aprenda enquanto programa" => "Aprende mientras programas",
            "Abrir Aprenda enquanto programa" => "Abrir Aprende mientras programas",
            "Recolher Aprenda enquanto programa" => "Contraer Aprende mientras programas",
            "Abrir depurador" => "Abrir depurador",
            "Recolher depurador" => "Contraer depurador",
            "Abrir prévia do jogo" => "Abrir vista previa del juego",
            "Recolher prévia do jogo" => "Ocultar vista previa del juego",
            "Prévia do jogo" => "Vista previa del juego",
            "Execute um programa Potigol Jerimum para ver o jogo aqui." => {
                "Ejecuta un programa de Potigol Jerimum para ver el juego aquí."
            }
            "Analisando a linha selecionada…" => "Analizando la línea seleccionada…",
            "Linha 1 · programa" => "Línea 1 · programa",
            "Selecione uma linha para ver uma explicação contextual do programa." => {
                "Selecciona una línea para ver una explicación contextual del programa."
            }
            "Mova o cursor para uma condição, um laço, uma entrada ou uma saída." => {
                "Mueve el cursor a una condición, un ciclo, una entrada o una salida."
            }
            "Conceito: programa" => "Concepto: programa",
            "O painel acompanha a linha selecionada e o conteúdo atual do editor." => {
                "Este panel sigue la línea seleccionada y el contenido actual del editor."
            }
            "decisão" => "decisión",
            "laço" => "ciclo",
            "entrada" => "entrada",
            "saída" => "salida",
            "variável" => "variable",
            "função" => "función",
            "procedimento" => "procedimiento",
            "atribuição" => "asignación",
            "retorno" => "retorno",
            "programa" => "programa",
            "Problemas nesta linha" => "Problemas en esta línea",
            "Nenhum problema conhecido nesta linha." => "No se conocen problemas en esta línea.",
            "Erro de sintaxe" => "Error de sintaxis",
            "Erro de tipo" => "Error de tipo",
            "Erro durante a execução" => "Error durante la ejecución",
            "Possível problema lógico" => "Posible problema lógico",
            "Índice fora dos limites" => "Índice fuera de los límites",
            "A condição é sempre verdadeira; o ramo alternativo pode nunca ser executado." => {
                "Esta condición siempre es verdadera; es posible que la alternativa nunca se ejecute."
            }
            "A condição é sempre falsa; o bloco pode nunca ser executado." => {
                "Esta condición siempre es falsa; es posible que el bloque nunca se ejecute."
            }
            "Confira se a condição deveria comparar uma variável." => {
                "Comprueba si la condición debería comparar una variable."
            }
            "O índice {value} está fora dos limites de `{name}` ({lower} a {upper})." => {
                "El índice {value} está fuera de los límites de `{name}` ({lower} a {upper})."
            }
            "Use um índice dentro da faixa declarada para esse vetor ou matriz." => {
                "Usa un índice dentro del rango declarado para este vector o matriz."
            }
            "Há um problema de sintaxe nesta linha." => {
                "Hay un problema de sintaxis en esta línea."
            }
            "O interpretador encontrou uma incompatibilidade de tipos nesta linha." => {
                "El intérprete encontró una incompatibilidad de tipos en esta línea."
            }
            "A execução encontrou um problema nesta linha." => {
                "La ejecución encontró un problema en esta línea."
            }
            "Detalhe original do interpretador (português): {message}" => {
                "Detalle original del intérprete (portugués): {message}"
            }
            "Conceito:" => "Concepto:",
            "Uma condição escolhe qual bloco executar com base em verdadeiro ou falso." => {
                "Una condición elige qué bloque ejecutar según un resultado verdadero o falso."
            }
            "Um laço repete instruções de acordo com uma condição ou faixa de valores." => {
                "Un ciclo repite instrucciones según una condición o un rango de valores."
            }
            "leia solicita um valor para uma variável antes de continuar a execução." => {
                "leia solicita un valor para una variable antes de continuar la ejecución."
            }
            "escreva e escreval mostram valores na saída do programa." => {
                "escreva y escreval muestran valores en la salida del programa."
            }
            "Uma variável guarda um valor que o programa pode consultar ou alterar." => {
                "Una variable guarda un valor que el programa puede consultar o cambiar."
            }
            "A linha selecionada participa do fluxo do algoritmo. Selecione outra linha para ver seu papel." => {
                "La línea seleccionada forma parte del flujo del algoritmo. Selecciona otra para ver su función."
            }
            "Troque a condição" => "Cambia la condición",
            "por" => "por",
            "e execute de novo. O resultado muda?" => "y vuelve a ejecutar. ¿Cambia el resultado?",
            "Mude a condição e execute novamente. O resultado muda?" => {
                "Cambia la condición y vuelve a ejecutar. ¿Cambia el resultado?"
            }
            "Mude a condição ou os limites do laço e observe quantas vezes ele repete." => {
                "Cambia la condición o los límites del ciclo y observa cuántas veces se repite."
            }
            "Digite outro valor e observe como o restante do programa reage." => {
                "Escribe otro valor y observa cómo responde el resto del programa."
            }
            "Altere o texto ou o valor exibido e execute novamente." => {
                "Cambia el texto o el valor mostrado y vuelve a ejecutar."
            }
            "Altere o valor armazenado e observe o efeito no resultado." => {
                "Cambia el valor almacenado y observa el efecto en el resultado."
            }
            "Selecione uma linha com entrada, saída, condição ou laço para experimentar." => {
                "Selecciona una línea de entrada, salida, condición o ciclo para probar."
            }
            "A condição" => "La condición",
            "é avaliada conforme os valores atuais." => "se evalúa según los valores actuales.",
            "Cada repetição executa novamente as instruções do bloco." => {
                "Cada repetición vuelve a ejecutar las instrucciones del bloque."
            }
            "A entrada é convertida para o tipo da variável solicitada." => {
                "La entrada se convierte al tipo de la variable solicitada."
            }
            "A saída permite observar valores calculados pelo programa." => {
                "La salida permite observar los valores calculados por el programa."
            }
            "O nome identifica onde um valor fica guardado." => {
                "El nombre identifica dónde se guarda un valor."
            }
            "Uma instrução é executada na ordem definida pelo algoritmo." => {
                "Una instrucción se ejecuta en el orden definido por el algoritmo."
            }
            "A condição é avaliada como verdadeira ou falsa." => {
                "La condición se evalúa como verdadera o falsa."
            }
            "A função" => "La función",
            "Esta declaração parece ser de uma função; complete a assinatura para identificar nome, parâmetros e retorno." => {
                "Esta parece ser una declaración de función; completa la firma para identificar el nombre, los parámetros y el retorno."
            }
            "Esta declaração parece ser de um procedimento; complete a assinatura para identificar nome e parâmetros." => {
                "Esta parece ser una declaración de procedimiento; completa la firma para identificar el nombre y los parámetros."
            }
            "O procedimento" => "El procedimiento",
            "recebe" => "recibe",
            "nenhum parâmetro" => "ningún parámetro",
            "O tipo `vazio` indica que a função não devolve um valor." => {
                "El tipo `vazio` indica que la función no devuelve ningún valor."
            }
            "Esta função não declara um tipo de retorno." => {
                "Esta función no declara un tipo de retorno."
            }
            "Ela devolve um valor do tipo" => "Devuelve un valor del tipo",
            "Esse tipo representa texto." => "Este tipo representa texto.",
            "Esse tipo representa texto no G-Portugol." => {
                "Este tipo representa texto en G-Portugol."
            }
            "A função produz texto, representado pelo tipo `literal` no G-Portugol." => {
                "La función produce texto, representado por el tipo `literal` en G-Portugol."
            }
            "Os parâmetros recebem os dados usados pela função; a ausência de tipo indica que ela não devolve resultado." => {
                "Los parámetros reciben los datos usados por la función; no declarar un tipo indica que no devuelve un resultado."
            }
            "O retorno não foi identificado." => "No se pudo identificar el tipo de retorno.",
            "Ele executa ações e não devolve um valor como resultado." => {
                "Ejecuta acciones y no devuelve un valor como resultado."
            }
            "A declaração define o nome e o tipo de um dado usado pelo algoritmo." => {
                "La declaración define el nombre y el tipo de un dato usado por el algoritmo."
            }
            "determina qual bloco será executado conforme resulte em verdadeiro ou falso." => {
                "determina qué bloque se ejecuta según sea verdadera o falsa."
            }
            "leia solicita um valor e o armazena na variável informada antes de o programa continuar." => {
                "leia solicita un valor y lo guarda en la variable indicada antes de continuar."
            }
            "escreva e escreval mostram na saída os valores e textos informados." => {
                "escreva y escreval muestran los valores y textos indicados en la salida."
            }
            "imprima mostra na saída os valores e textos informados." => {
                "imprima muestra los valores y textos indicados en la salida."
            }
            "A atribuição calcula o lado direito e atualiza o valor da variável à esquerda." => {
                "La asignación evalúa el lado derecho y actualiza la variable de la izquierda."
            }
            "Esta função não devolve valor; retorne encerra sua execução." => {
                "Esta función no devuelve un valor; retorne termina su ejecución."
            }
            "retorne encerra a rotina e pode entregar um resultado a quem a chamou." => {
                "retorne termina la rutina y puede entregar un resultado a quien la llamó."
            }
            "retorne encerra a função e entrega um valor do tipo" => {
                "retorne termina la función y entrega un valor del tipo"
            }
            "a quem a chamou" => "a quien la llamó",
            "Parâmetros são os dados recebidos pela função. `vazio` informa que nenhum resultado é devolvido." => {
                "Los parámetros son los datos que recibe la función. `vazio` indica que no devuelve un resultado."
            }
            "A função produz um valor de texto, representado pelo tipo `cadeia` em Portugol 2.0." => {
                "La función produce texto, representado por el tipo `cadeia` en Portugol 2.0."
            }
            "A função devolve um resultado do tipo `{return_type}`. Seus parâmetros recebem os dados usados no cálculo." => {
                "La función devuelve un valor `{return_type}`. Sus parámetros reciben los datos usados en el cálculo."
            }
            "A função devolve um resultado do tipo" => "La función devuelve un valor del tipo",
            "Seus parâmetros recebem os dados usados no cálculo." => {
                "Sus parámetros reciben los datos usados en el cálculo."
            }
            "Parâmetros são os dados recebidos pela função; o tipo de retorno descreve o resultado produzido." => {
                "Los parámetros son los datos que recibe la función; el tipo de retorno describe el resultado producido."
            }
            "No VisuAlg, procedimento é uma rotina de ações. Parâmetros por referência podem alterar a variável enviada por quem chamou." => {
                "En VisuAlg, un procedimiento es una rutina de acciones. Los parámetros por referencia pueden cambiar la variable enviada por quien lo llamó."
            }
            "O tipo define quais valores podem ser guardados; o formato indica se há um valor, um vetor ou uma matriz." => {
                "El tipo define qué valores se pueden guardar; la forma indica si es un escalar, un vector o una matriz."
            }
            "Atribuir associa um novo valor a uma variável; o operador determina como o valor é calculado." => {
                "Asignar da un nuevo valor a una variable; el operador determina cómo se calcula."
            }
            "O retorno leva o resultado da função de volta ao ponto que a chamou." => {
                "El retorno lleva el resultado de la función al punto que la llamó."
            }
            "Troque o valor atribuído e acompanhe a próxima leitura da variável." => {
                "Cambia el valor asignado y sigue la próxima lectura de la variable."
            }
            "Altere o valor devolvido e veja como a chamada da função reage." => {
                "Cambia el valor devuelto y observa cómo responde la llamada de la función."
            }
            "Altere o valor inicial ou um elemento do vetor e observe o efeito no resultado." => {
                "Cambia el valor inicial o un elemento del vector y observa el efecto en el resultado."
            }
            "Mude um argumento na chamada de" => "Cambia un argumento en la llamada a",
            "e compare o resultado devolvido" => "y compara el resultado devuelto",
            "Altere um parâmetro usado ao chamar" => "Cambia un parámetro al llamar a",
            "e observe quais valores mudam" => "y observa qué valores cambian",
            "vetor com índices de" => "vector con índices de",
            "vetor com" => "vector con",
            "posições, índices de" => "posiciones, con índices de",
            "matriz com" => "matriz con",
            "dimensões" => "dimensiones",
            "vetor" => "vector",
            "tipo usado para guardar texto" => "tipo usado para guardar texto",
            "número sem parte decimal" => "número sin parte decimal",
            "número que pode ter parte decimal" => "número que puede tener parte decimal",
            "valor verdadeiro ou falso" => "valor verdadero o falso",
            "um caractere" => "un carácter",
            "é uma constante do tipo" => "es una constante del tipo",
            "é uma variável do tipo" => "es una variable del tipo",
            "passado por referência, pode alterar o valor original" => {
                "pasado por referencia, puede cambiar el valor original"
            }
            "valor inicial" => "valor inicial",
            "seu valor não muda durante a execução" => "su valor no cambia durante la ejecución",
            "seu valor pode mudar durante a execução" => {
                "su valor puede cambiar durante la ejecución"
            }
            "Experimente" => "Pruébalo",
            "Saída" => "Salida",
            "Problemas" => "Problemas",
            "Depurador" => "Depurador",
            "Interface" => "Interfaz",
            "Espaços" => "Espacios",
            "Linha" => "Línea",
            "Coluna" => "Columna",
            "A saída aparecerá aqui após a execução." => {
                "La salida aparecerá aquí después de ejecutar el programa."
            }
            "Pronto para executar" => "Listo para ejecutar",
            "Executar o programa atual" => "Ejecutar el programa actual",
            "Executando o programa…" => "Ejecutando el programa…",
            "Interrompendo execução…" => "Deteniendo la ejecución…",
            "Execução concluída." => "Ejecución finalizada.",
            "O programa terminou com um erro." => "El programa terminó con un error.",
            "Abra um programa antes de executar." => "Abre un programa antes de ejecutarlo.",
            "Interrompa a execução atual antes de iniciar a depuração." => {
                "Detén la ejecución actual antes de iniciar el depurador."
            }
            "Depuração iniciada…" => "Depuración iniciada…",
            "Execução interrompida." => "Ejecución detenida.",
            "Entrada solicitada:" => "Entrada solicitada:",
            "Digite a entrada do programa" => "Escribe la entrada del programa",
            "Pressione Esc para cancelar a execução." => "Pulsa Esc para cancelar la ejecución.",
            "Iniciar depuração" => "Iniciar depuración",
            "Continuar" => "Continuar",
            "Avançar um passo" => "Avanzar un paso",
            "Parar" => "Detener",
            "Parar depuração" => "Detener depuración",
            "Aguardando depuração" => "Esperando depuración",
            "Analisando linha…" => "Analizando línea…",
            "Aguardando a próxima linha…" => "Esperando la siguiente línea…",
            "Os valores das variáveis aparecerão durante a execução passo a passo." => {
                "Los valores de las variables aparecerán al recorrer el programa paso a paso."
            }
            "Nenhuma variável visível." => "No hay variables visibles.",
            "Pausado na linha" => "Pausado en la línea",
            "passo a passo" => "paso a paso",
            "pausa" => "pausa",
            "ponto de depuração" => "punto de depuración",
            "depuração" => "depuración",
            "Digite um valor e pressione Enter" => "Escribe un valor y pulsa Enter",
            "O interpretador foi encerrado antes de concluir o programa." => {
                "El intérprete se cerró antes de terminar el programa."
            }
            "Não foi possível concluir a execução." => "No se pudo terminar la ejecución.",
            "Não foi possível enviar a entrada ao programa." => {
                "No se pudo enviar la entrada al programa."
            }
            "Alterações da sessão anterior recuperadas." => {
                "Se recuperaron los cambios de la sesión anterior."
            }
            "Personalize a aparência e o editor." => "Personaliza la apariencia y el editor.",
            "Aparência" => "Apariencia",
            "Sistema" => "Sistema",
            "Seguir o sistema" => "Seguir el sistema",
            "Claro" => "Claro",
            "Escuro" => "Oscuro",
            "Tema" => "Tema",
            "Escolha como o Potoo aparece nesta tela." => {
                "Elige cómo se ve Potoo en esta pantalla."
            }
            "Editor" => "Editor",
            "Tamanho da fonte" => "Tamaño de fuente",
            "Ajuste a leitura do código." => "Ajusta la lectura del código.",
            "Numeração de linhas" => "Números de línea",
            "Mostra o número de cada linha do código." => {
                "Muestra el número de cada línea de código."
            }
            "Quebra automática" => "Ajuste de línea",
            "Ajusta linhas longas à largura do editor." => {
                "Ajusta las líneas largas al ancho del editor."
            }
            "Idioma da interface" => "Idioma de la interfaz",
            "A alteração de idioma será aplicada após salvar e reiniciar o aplicativo." => {
                "El cambio de idioma se aplicará después de guardar y reiniciar la aplicación."
            }
            "Salve este arquivo antes de alterar o idioma." => {
                "Guarda este archivo antes de cambiar el idioma."
            }
            "A alteração de idioma foi cancelada." => "Se canceló el cambio de idioma.",
            "Não foi possível reiniciar o aplicativo" => "No se pudo reiniciar la aplicación",
            "Não foi possível restaurar as configurações" => {
                "No se pudo restaurar la configuración"
            }
            "Recuo" => "Sangría",
            "Espaços por nível de recuo." => "Espacios insertados por cada nivel de sangría.",
            "2 espaços" => "2 espacios",
            "4 espaços" => "4 espacios",
            "Esta pasta não contém arquivos visíveis." => {
                "Esta carpeta no tiene archivos visibles."
            }
            "Nenhum programa .por, .alg, .poti ou .gpt nesta pasta. Os demais arquivos aparecem na lista." => {
                "No hay programas .por, .alg, .poti o .gpt en esta carpeta. Los demás archivos aparecen en la lista."
            }
            "Fechar arquivo" => "Cerrar archivo",
            "Escolha um nome e a linguagem do novo arquivo." => {
                "Elige un nombre y el lenguaje del archivo nuevo."
            }
            "Crie um arquivo dentro do projeto atual." => "Crea un archivo en el proyecto actual.",
            "Nome do arquivo" => "Nombre del archivo",
            "Nome do arquivo, por exemplo: exercicio-1" => {
                "Nombre del archivo, por ejemplo: ejercicio-1"
            }
            "Linguagem" => "Lenguaje",
            "Cancelar" => "Cancelar",
            "Alterações não salvas" => "Cambios sin guardar",
            "Descartar e abrir" => "Descartar y abrir",
            "Este arquivo tem alterações não salvas" => "Este archivo tiene cambios sin guardar",
            "O arquivo aberto tem alterações que ainda não foram salvas. O que você gostaria de fazer?" => {
                "Este archivo tiene cambios sin guardar. ¿Qué quieres hacer?"
            }
            "Salve o arquivo antes de fechar ou descarte as alterações para continuar." => {
                "Guarda este archivo antes de cerrarlo o descarta los cambios para continuar."
            }
            "Continuar editando" => "Seguir editando",
            "Descartar alterações" => "Descartar cambios",
            "Salvar e fechar" => "Guardar y cerrar",
            "Há alterações não salvas" => "Hay cambios sin guardar",
            "Descartar e sair" => "Descartar y salir",
            "Salve os arquivos abertos antes de trocar de pasta ou descarte as alterações para continuar." => {
                "Guarda los archivos abiertos antes de cambiar de carpeta o descarta los cambios para continuar."
            }
            "Salve os arquivos antes de sair ou descarte as alterações para fechar o Potoo IDE." => {
                "Guarda tus archivos antes de salir o descarta los cambios para cerrar Potoo IDE."
            }
            "Nenhum problema detectado" => "No se detectaron problemas",
            "Abrindo arquivo…" => "Abriendo archivo…",
            "Abrindo pasta…" => "Abriendo carpeta…",
            "Não foi possível atualizar os arquivos do projeto" => {
                "No se pudieron actualizar los archivos del proyecto"
            }
            "Os erros de sintaxe e execução aparecerão aqui." => {
                "Los errores de sintaxis y ejecución aparecerán aquí."
            }
            "Arquivo aberto" => "Archivo abierto",
            "Arquivo salvo" => "Archivo guardado",
            "Pasta aberta. Crie um arquivo .por, .alg, .poti ou .gpt para começar." => {
                "Carpeta abierta. Crea un archivo .por, .alg, .poti o .gpt para empezar."
            }
            "Novo arquivo criado" => "Archivo nuevo creado",
            "Já existe um arquivo com esse nome." => "Ya existe un archivo con ese nombre.",
            "Já existe um item com esse nome." => "Ya existe un elemento con ese nombre.",
            "Renomear" => "Renombrar",
            "Excluir" => "Eliminar",
            "Renomear arquivo" => "Renombrar archivo",
            "Renomear pasta" => "Renombrar carpeta",
            "Escolha um novo nome para este item do projeto." => {
                "Elige un nombre nuevo para este elemento del proyecto."
            }
            "Novo nome" => "Nombre nuevo",
            "O item selecionado não está mais disponível." => {
                "El elemento seleccionado ya no está disponible."
            }
            "Item renomeado." => "Elemento renombrado.",
            "Item excluído." => "Elemento eliminado.",
            "Não foi possível renomear o item" => "No se pudo renombrar el elemento",
            "Não foi possível excluir o item" => "No se pudo eliminar el elemento",
            "Excluir item do projeto" => "Eliminar elemento del proyecto",
            "Deseja excluir este item do projeto?" => {
                "¿Quieres eliminar este elemento del proyecto?"
            }
            "Há alterações não salvas em arquivos deste item; elas serão perdidas." => {
                "Se perderán los cambios sin guardar en los archivos de este elemento."
            }
            "O item e todo o seu conteúdo serão excluídos permanentemente." => {
                "Este elemento y todo su contenido se eliminarán de forma permanente."
            }
            "Não foi possível abrir o arquivo" => "No se pudo abrir el archivo",
            "Não foi possível abrir a pasta" => "No se pudo abrir la carpeta",
            "Não foi possível salvar o arquivo" => "No se pudo guardar el archivo",
            "Não foi possível criar o arquivo" => "No se pudo crear el archivo",
            "O editor abre apenas arquivos .por, .alg, .poti e .gpt." => {
                "El editor solo abre archivos .por, .alg, .poti y .gpt."
            }
            "Abra uma pasta de projeto antes de criar arquivos." => {
                "Abre una carpeta de proyecto antes de crear archivos."
            }
            "Crie ou abra um arquivo antes de salvar." => {
                "Crea o abre un archivo antes de guardar."
            }
            "Não foi possível salvar as configurações" => "No se pudo guardar la configuración",
            "Não foi possível salvar a sessão" => "No se pudo guardar la sesión",
            "Use um nome simples, sem pastas ou caracteres especiais." => {
                "Usa un nombre sencillo, sin carpetas ni caracteres especiales."
            }
            "Use a extensão .por, .alg, .poti ou .gpt." => {
                "Usa la extensión .por, .alg, .poti o .gpt."
            }
            "Salve o código com a extensão .por, .alg, .poti ou .gpt." => {
                "Guarda el código con la extensión .por, .alg, .poti o .gpt."
            }
            "Selecione uma pasta local para abrir." => "Selecciona una carpeta local para abrir.",
            "Selecione um arquivo local." => "Selecciona un archivo local.",
            "Escolha um caminho local para salvar o arquivo." => {
                "Elige una ruta local para guardar el archivo."
            }
            "Não foi possível abrir a janela de salvamento" => {
                "No se pudo abrir la ventana para guardar"
            }
            "Abrir arquivo de código" => "Abrir archivo de código",
            "Portugol, VisuAlg, Potigol e G-Portugol (*.por, *.alg, *.poti, *.gpt)" => {
                "Portugol, VisuAlg, Potigol y G-Portugol (*.por, *.alg, *.poti, *.gpt)"
            }
            "Não foi possível abrir o arquivo de código" => {
                "No se pudo abrir el archivo de código"
            }
            "novo-arquivo" => "nuevo-archivo",
            "caminho de configuração inválido" => "ruta de configuración no válida",
            "Não foi possível ler a sessão anterior. Os arquivos originais continuam intactos." => {
                "No se pudo leer la sesión anterior. Tus archivos originales siguen intactos."
            }
            "Salvar arquivo (⌘S)" => "Guardar archivo (⌘S)",
            "Salvar arquivo (Ctrl+S)" => "Guardar archivo (Ctrl+S)",
            _ => portuguese,
        },
    };
    translations
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ThemePreference {
    System,
    Light,
    Dark,
}

impl ThemePreference {
    fn selected_index(self) -> u32 {
        match self {
            Self::System => 0,
            Self::Light => 1,
            Self::Dark => 2,
        }
    }

    fn from_selected_index(index: u32) -> Self {
        match index {
            1 => Self::Light,
            2 => Self::Dark,
            _ => Self::System,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn from_key(key: &str) -> Self {
        match key {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }
}

#[cfg(test)]
const UI_TRANSLATION_KEYS: &[&str] = &[
    "Meu primeiro programa",
    "Início",
    "Vamos programar?",
    "Continue de onde parou ou comece a transformar suas ideias em algoritmos.",
    "Programas recentes",
    "Ver todos os programas recentes",
    "Remover dos recentes",
    "Comece um novo programa",
    "Retome um trabalho existente",
    "Projeto Portugol",
    "Projeto VisuAlg",
    "Projeto Potigol",
    "Projeto G-Portugol",
    "Comece com um arquivo .por",
    "Comece com um arquivo .alg",
    "Dialetos do Portugol: G-Portugol, Portugol 2.0 e VisuAlg",
    "3 variantes",
    "2 variantes",
    "Criar programa inicial (.poti)",
    "Pressione Esc para cancelar a execução.",
    "Analisando linha…",
    "Aguardando a próxima linha…",
    "Selecionar uma pasta",
    "Abra um projeto que não aparece na lista.",
    "Abra um arquivo .por, .alg, .poti ou .gpt diretamente no editor.",
    "Projeto",
    "Abrir um programa recente",
    "O arquivo ou a pasta recente não está mais disponível.",
    "Abrir pasta",
    "Abrir pasta de projeto",
    "Abrir projeto",
    "Abrir arquivo",
    "Abrir projeto ou programa",
    "Nenhum projeto aberto",
    "Abra uma pasta ou um programa para começar.",
    "Salvar arquivo",
    "Salvar",
    "Configurações",
    "Fechar",
    "Sobre",
    "Sobre o Potoo IDE",
    "Código do projeto",
    "Versão e links do projeto",
    "← Configurações",
    "Criado por Diego Pereira do Nascimento",
    "Versão",
    "Relatar um problema",
    "Repositório do projeto",
    "Site do projeto",
    "Ajuda",
    "Doar",
    "Contato",
    "Site do criador do projeto",
    "Contato do criador",
    "Executar",
    "Minimizar",
    "Maximizar",
    "Restaurar",
    "Arquivos",
    "Criar arquivo",
    "Criar item no projeto",
    "Novo projeto",
    "Criar projeto",
    "Criar um projeto Portugol, VisuAlg, Potigol ou G-Portugol",
    "Selecione a linguagem/pseudolinguagem do projeto",
    "Linguagem/pseudolinguagem",
    "Depuração encerrada com erro",
    "Variante do Portugol",
    "Variante do Potigol",
    "Portugol 2.0",
    "G-Portugol",
    "Potigol Clássico",
    "Potigol Jerimum (Jogo 2D)",
    "Arquivo inicial .por",
    "Arquivo inicial .alg",
    "Arquivo inicial .poti",
    "Arquivo inicial .gpt",
    "Crie uma pasta para reunir seus programas e exercícios.",
    "Nome do projeto",
    "Por exemplo: Meu primeiro projeto",
    "Meu projeto",
    "Local do projeto",
    "Pasta",
    "Criar dentro de",
    "Pasta do projeto",
    "A pasta selecionada não está disponível.",
    "Erros de sintaxe e execução aparecerão aqui.",
    "Escolher pasta",
    "Escolha a pasta de destino",
    "Selecione uma pasta local para criar o projeto.",
    "Não foi possível escolher a pasta do projeto",
    "Digite um nome válido para o projeto.",
    "Já existe um projeto com esse nome.",
    "Não foi possível criar o projeto",
    "Projeto criado",
    "Novo arquivo",
    "Criar um arquivo Portugol, VisuAlg, Potigol ou G-Portugol",
    "Nova pasta",
    "Criar uma pasta no projeto",
    "Criar pasta",
    "Organize seus programas em uma pasta do projeto.",
    "Nome da pasta",
    "Nome da pasta, por exemplo: exercicios",
    "Pasta criada",
    "Já existe uma pasta com esse nome.",
    "Digite um nome válido para a pasta.",
    "Não foi possível criar a pasta",
    "Abra uma pasta de projeto antes de criar itens.",
    "Renomear",
    "Excluir",
    "Renomear arquivo",
    "Renomear pasta",
    "Escolha um novo nome para este item do projeto.",
    "Novo nome",
    "O item selecionado não está mais disponível.",
    "Já existe um item com esse nome.",
    "Item renomeado.",
    "Item excluído.",
    "Não foi possível renomear o item",
    "Não foi possível excluir o item",
    "Excluir item do projeto",
    "Deseja excluir este item do projeto?",
    "Há alterações não salvas em arquivos deste item; elas serão perdidas.",
    "O item e todo o seu conteúdo serão excluídos permanentemente.",
    "Aprenda enquanto programa",
    "Abrir Aprenda enquanto programa",
    "Recolher Aprenda enquanto programa",
    "Abrir depurador",
    "Recolher depurador",
    "Abrir prévia do jogo",
    "Recolher prévia do jogo",
    "Prévia do jogo",
    "Execute um programa Potigol Jerimum para ver o jogo aqui.",
    "Analisando a linha selecionada…",
    "função",
    "procedimento",
    "atribuição",
    "retorno",
    "Problemas nesta linha",
    "Nenhum problema conhecido nesta linha.",
    "Erro de sintaxe",
    "Erro de tipo",
    "Erro durante a execução",
    "Possível problema lógico",
    "Índice fora dos limites",
    "A condição é sempre verdadeira; o ramo alternativo pode nunca ser executado.",
    "A condição é sempre falsa; o bloco pode nunca ser executado.",
    "Confira se a condição deveria comparar uma variável.",
    "O índice {value} está fora dos limites de `{name}` ({lower} a {upper}).",
    "Use um índice dentro da faixa declarada para esse vetor ou matriz.",
    "Há um problema de sintaxe nesta linha.",
    "O interpretador encontrou uma incompatibilidade de tipos nesta linha.",
    "A execução encontrou um problema nesta linha.",
    "Detalhe original do interpretador (português): {message}",
    "A função",
    "Esta declaração parece ser de uma função; complete a assinatura para identificar nome, parâmetros e retorno.",
    "Esta declaração parece ser de um procedimento; complete a assinatura para identificar nome e parâmetros.",
    "O procedimento",
    "recebe",
    "nenhum parâmetro",
    "O tipo `vazio` indica que a função não devolve um valor.",
    "Esta função não declara um tipo de retorno.",
    "Ela devolve um valor do tipo",
    "Esse tipo representa texto.",
    "Esse tipo representa texto no G-Portugol.",
    "A função produz texto, representado pelo tipo `literal` no G-Portugol.",
    "Os parâmetros recebem os dados usados pela função; a ausência de tipo indica que ela não devolve resultado.",
    "O retorno não foi identificado.",
    "Ele executa ações e não devolve um valor como resultado.",
    "A declaração define o nome e o tipo de um dado usado pelo algoritmo.",
    "determina qual bloco será executado conforme resulte em verdadeiro ou falso.",
    "leia solicita um valor e o armazena na variável informada antes de o programa continuar.",
    "escreva e escreval mostram na saída os valores e textos informados.",
    "imprima mostra na saída os valores e textos informados.",
    "A atribuição calcula o lado direito e atualiza o valor da variável à esquerda.",
    "Esta função não devolve valor; retorne encerra sua execução.",
    "retorne encerra a rotina e pode entregar um resultado a quem a chamou.",
    "retorne encerra a função e entrega um valor do tipo",
    "a quem a chamou",
    "Parâmetros são os dados recebidos pela função. `vazio` informa que nenhum resultado é devolvido.",
    "A função produz um valor de texto, representado pelo tipo `cadeia` em Portugol 2.0.",
    "A função devolve um resultado do tipo `{return_type}`. Seus parâmetros recebem os dados usados no cálculo.",
    "A função devolve um resultado do tipo",
    "Seus parâmetros recebem os dados usados no cálculo.",
    "Parâmetros são os dados recebidos pela função; o tipo de retorno descreve o resultado produzido.",
    "Uma rotina executa ações sem devolver um resultado; cada linguagem define como seus parâmetros são passados.",
    "Esta análise do Potigol é heurística e pode não reconhecer todos os casos da linguagem.",
    "O tipo define quais valores podem ser guardados; o formato indica se há um valor, um vetor ou uma matriz.",
    "Atribuir associa um novo valor a uma variável; o operador determina como o valor é calculado.",
    "O retorno leva o resultado da função de volta ao ponto que a chamou.",
    "Troque o valor atribuído e acompanhe a próxima leitura da variável.",
    "Altere o valor devolvido e veja como a chamada da função reage.",
    "Altere o valor inicial ou um elemento do vetor e observe o efeito no resultado.",
    "Mude um argumento na chamada de",
    "e compare o resultado devolvido",
    "Altere um parâmetro usado ao chamar",
    "e observe quais valores mudam",
    "vetor com índices de",
    "vetor com",
    "posições, índices de",
    "matriz com",
    "dimensões",
    "vetor",
    "tipo usado para guardar texto",
    "número sem parte decimal",
    "número que pode ter parte decimal",
    "valor verdadeiro ou falso",
    "um caractere",
    "é uma constante do tipo",
    "é uma variável do tipo",
    "passado por referência, pode alterar o valor original",
    "valor inicial",
    "seu valor não muda durante a execução",
    "seu valor pode mudar durante a execução",
    "Experimente",
    "Saída",
    "Problemas",
    "Depurador",
    "Interface",
    "Espaços",
    "Linha",
    "Coluna",
    "A saída aparecerá aqui após a execução.",
    "Pronto para executar",
    "Executar o programa atual",
    "Executando o programa…",
    "Interrompa a execução atual antes de iniciar a depuração.",
    "Execução concluída.",
    "Parar depuração",
    "O programa terminou com um erro.",
    "Digite um valor e pressione Enter",
    "O interpretador foi encerrado antes de concluir o programa.",
    "Não foi possível concluir a execução.",
    "Não foi possível enviar a entrada ao programa.",
    "Alterações da sessão anterior recuperadas.",
    "Personalize a aparência e o editor.",
    "Aparência",
    "Sistema",
    "Seguir o sistema",
    "Claro",
    "Escuro",
    "Tema",
    "Escolha como o Potoo aparece nesta tela.",
    "Editor",
    "Tamanho da fonte",
    "Ajuste a leitura do código.",
    "Numeração de linhas",
    "Mostra o número de cada linha do código.",
    "Quebra automática",
    "Ajusta linhas longas à largura do editor.",
    "Idioma da interface",
    "A alteração de idioma será aplicada após salvar e reiniciar o aplicativo.",
    "Salve este arquivo antes de alterar o idioma.",
    "A alteração de idioma foi cancelada.",
    "Não foi possível reiniciar o aplicativo",
    "Não foi possível restaurar as configurações",
    "Recuo",
    "Espaços por nível de recuo.",
    "2 espaços",
    "4 espaços",
    "Esta pasta não contém arquivos visíveis.",
    "Nenhum programa .por, .alg, .poti ou .gpt nesta pasta. Os demais arquivos aparecem na lista.",
    "Fechar arquivo",
    "Escolha um nome e a linguagem do novo arquivo.",
    "Crie um arquivo dentro do projeto atual.",
    "Nome do arquivo",
    "Nome do arquivo, por exemplo: exercicio-1",
    "Linguagem",
    "Cancelar",
    "Alterações não salvas",
    "Descartar e abrir",
    "Este arquivo tem alterações não salvas",
    "O arquivo aberto tem alterações que ainda não foram salvas. O que você gostaria de fazer?",
    "Salve o arquivo antes de fechar ou descarte as alterações para continuar.",
    "Continuar editando",
    "Descartar alterações",
    "Salvar e fechar",
    "Há alterações não salvas",
    "Descartar e sair",
    "Salve os arquivos abertos antes de trocar de pasta ou descarte as alterações para continuar.",
    "Salve os arquivos antes de sair ou descarte as alterações para fechar o Potoo IDE.",
    "Nenhum problema detectado",
    "Abrindo arquivo…",
    "Abrindo pasta…",
    "Não foi possível atualizar os arquivos do projeto",
    "Arquivo aberto",
    "Arquivo salvo",
    "Pasta aberta. Crie um arquivo .por, .alg, .poti ou .gpt para começar.",
    "Novo arquivo criado",
    "Já existe um arquivo com esse nome.",
    "Não foi possível abrir o arquivo",
    "Não foi possível abrir a pasta",
    "Não foi possível salvar o arquivo",
    "Não foi possível criar o arquivo",
    "O editor abre apenas arquivos .por, .alg, .poti e .gpt.",
    "Abra uma pasta de projeto antes de criar arquivos.",
    "Crie ou abra um arquivo antes de salvar.",
    "Não foi possível salvar as configurações",
    "Não foi possível salvar a sessão",
    "Use um nome simples, sem pastas ou caracteres especiais.",
    "Use a extensão .por, .alg, .poti ou .gpt.",
    "Salve o código com a extensão .por, .alg, .poti ou .gpt.",
    "Selecione uma pasta local para abrir.",
    "Selecione um arquivo local.",
    "Escolha um caminho local para salvar o arquivo.",
    "Não foi possível abrir a janela de salvamento",
    "Abrir arquivo de código",
    "Portugol, VisuAlg, Potigol e G-Portugol (*.por, *.alg, *.poti, *.gpt)",
    "Não foi possível abrir o arquivo de código",
    "novo-arquivo",
    "caminho de configuração inválido",
    "Não foi possível ler a sessão anterior. Os arquivos originais continuam intactos.",
    "Salvar arquivo (⌘S)",
    "Salvar arquivo (Ctrl+S)",
    "light",
    "dark",
];

#[cfg(test)]
fn translate_known_text(from: UiLanguage, to: UiLanguage, text: &str) -> Option<String> {
    UI_TRANSLATION_KEYS
        .iter()
        .find_map(|key| (tr(from, key) == text).then(|| tr(to, key).to_string()))
}

fn system_theme_is_dark() -> bool {
    gtk::Settings::default().is_some_and(|settings| {
        settings.property::<gtk::InterfaceColorScheme>("gtk-interface-color-scheme")
            == gtk::InterfaceColorScheme::Dark
    })
}

fn theme_is_dark(theme: ThemePreference) -> bool {
    match theme {
        ThemePreference::System => system_theme_is_dark(),
        ThemePreference::Light => false,
        ThemePreference::Dark => true,
    }
}

#[derive(Clone, Copy)]
struct Preferences {
    theme: ThemePreference,
    font_size: u32,
    line_numbers_visible: bool,
    word_wrap: bool,
    indent_width: u32,
    default_language: Language,
    ui_language: UiLanguage,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            font_size: 14,
            line_numbers_visible: true,
            word_wrap: false,
            indent_width: 4,
            default_language: Language::Portugol,
            ui_language: UiLanguage::from_system(),
        }
    }
}

#[derive(Clone)]
struct SessionDocument {
    path: Option<PathBuf>,
    name: String,
    language: Language,
    contents: String,
    saved_contents: String,
}

struct SessionSnapshot {
    clean_exit: bool,
    project_root: Option<PathBuf>,
    active_index: usize,
    documents: Vec<SessionDocument>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RightPanelSection {
    Learning,
    Debugger,
    GamePreview,
}

#[derive(Debug)]
pub(crate) enum MainWindowMsg {
    InterfaceLanguageSelected(UiLanguage),
    ProjectSelected(PathBuf),
    RecentSelected(PathBuf),
    RecentRemoved(PathBuf),
    ProjectSwitchDecision(bool),
    ProjectScanCompleted {
        project_generation: u64,
        scan_generation: u64,
        root: PathBuf,
        load_project: bool,
        result: Result<Vec<ProjectFile>, String>,
    },
    DocumentReadCompleted {
        project_generation: u64,
        path: PathBuf,
        standalone: bool,
        result: Result<String, String>,
    },
    CreateProject(PathBuf, String, ProjectTemplate),
    CreateFile(String, ProjectTemplate, PathBuf),
    CreateFolder(String, PathBuf),
    RenameProjectItem(PathBuf),
    RenameProjectItemConfirmed(PathBuf, String),
    DeleteProjectItem(PathBuf),
    DeleteProjectItemConfirmed(PathBuf),
    OpenFile(PathBuf),
    ActivateDocument(u64),
    CloseDocument(u64),
    CloseDocumentDecision(u64, bool),
    SaveAndCloseDocument(u64),
    SaveCurrent,
    SaveAsSelected(u64, PathBuf),
    SaveAsCanceled(u64),
    FileOperationFailed(String),
    RequestClose,
    CloseAppDecision(bool),
    ThemeSelected(ThemePreference),
    SystemThemeChanged(bool),
    FontSizeSelected(u32),
    LineNumbersChanged(bool),
    WordWrapChanged(bool),
    IndentWidthSelected(u32),
    OpenSettings,
    CloseSettings,
    Run,
    StartDebug,
    StepDebug,
    StopDebug,
    ExecutionEvent(u64, ExecutionEvent),
    ExecutionLost(u64),
    FileSelectionResolved(u64, Result<String, String>),
    SubmitProgramInput(String),
    SourceChanged(String),
    CursorMoved(usize),
    SelectRightPanel(RightPanelSection),
    LearningDebounceElapsed(u64),
    LearningAnalysisReady {
        generation: u64,
        document_id: u64,
        revision: u64,
        line: usize,
        language: Language,
        hint: LearningHint,
    },
    PersistRecovery(u64),
}

struct LearningAnalysisRequest {
    generation: u64,
    document_id: u64,
    revision: u64,
    line: usize,
    contents: String,
    language: Language,
    runtime_diagnostics: Vec<execution::ExecutionDiagnostic>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LearningAnalysisIdentity {
    generation: u64,
    document_id: u64,
    revision: u64,
    line: usize,
    language: Language,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CurrentLearningAnalysis {
    generation: u64,
    active_document_id: Option<u64>,
    learning_document_id: Option<u64>,
    revision: u64,
    line: usize,
    language: Option<Language>,
}

impl CurrentLearningAnalysis {
    fn accepts(self, result: LearningAnalysisIdentity) -> bool {
        self.generation == result.generation
            && self.active_document_id == Some(result.document_id)
            && self.learning_document_id == Some(result.document_id)
            && self.revision == result.revision
            && self.line == result.line
            && self.language == Some(result.language)
    }
}

fn invalidate_learning_hint(generation: &mut u64, hint: &mut Option<LearningHint>) -> u64 {
    *generation = generation.wrapping_add(1).max(1);
    *hint = None;
    *generation
}

fn project_scan_is_current(
    current_project_generation: u64,
    current_scan_generation: u64,
    current_root: Option<&Path>,
    project_generation: u64,
    scan_generation: u64,
    root: &Path,
    loading_project: bool,
) -> bool {
    current_project_generation == project_generation
        && current_scan_generation == scan_generation
        && (loading_project || current_root == Some(root))
}

#[derive(Default)]
struct LearningAnalysisWorkerState {
    latest_request: Option<LearningAnalysisRequest>,
    shutdown: bool,
}

struct LearningAnalysisWorker {
    state: Arc<(Mutex<LearningAnalysisWorkerState>, Condvar)>,
    generation: Arc<AtomicU64>,
}

impl LearningAnalysisWorker {
    fn new(sender: ComponentSender<MainWindow>) -> Self {
        let state = Arc::new((
            Mutex::new(LearningAnalysisWorkerState::default()),
            Condvar::new(),
        ));
        let generation = Arc::new(AtomicU64::new(0));
        let state_for_worker = Arc::clone(&state);
        let generation_for_worker = Arc::clone(&generation);
        std::thread::spawn(move || {
            loop {
                let request = {
                    let (state_lock, wake) = &*state_for_worker;
                    let mut state = state_lock
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    while state.latest_request.is_none() && !state.shutdown {
                        state = wake
                            .wait(state)
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                    }
                    if state.shutdown {
                        return;
                    }
                    state.latest_request.take().expect("request was checked")
                };
                if generation_for_worker.load(Ordering::Acquire) != request.generation {
                    continue;
                }

                let request_generation = request.generation;
                let hint = learning_hint_with_runtime_cancellable(
                    &request.contents,
                    request.line,
                    request.language,
                    &request.runtime_diagnostics,
                    || generation_for_worker.load(Ordering::Acquire) != request_generation,
                );
                if let Some(hint) = hint
                    && generation_for_worker.load(Ordering::Acquire) == request_generation
                {
                    sender.input(MainWindowMsg::LearningAnalysisReady {
                        generation: request.generation,
                        document_id: request.document_id,
                        revision: request.revision,
                        line: request.line,
                        language: request.language,
                        hint,
                    });
                }
            }
        });
        Self { state, generation }
    }

    fn invalidate(&self, generation: u64) {
        self.generation.store(generation, Ordering::Release);
        let (state_lock, _) = &*self.state;
        let mut state = state_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.latest_request = None;
    }

    fn submit(&self, request: LearningAnalysisRequest) {
        if self.generation.load(Ordering::Acquire) != request.generation {
            return;
        }
        let (state_lock, wake) = &*self.state;
        let mut state = state_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.shutdown || self.generation.load(Ordering::Acquire) != request.generation {
            return;
        }
        state.latest_request = Some(request);
        wake.notify_one();
    }
}

impl Drop for LearningAnalysisWorker {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        let (state_lock, wake) = &*self.state;
        let mut state = state_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.shutdown = true;
        state.latest_request = None;
        wake.notify_one();
    }
}

pub(crate) struct MainWindow {
    window: gtk::ApplicationWindow,
    audio_host: crate::audio::AudioHost,
    graphics_host: crate::graphics::GraphicsHost,
    new_file_language: Language,
    documents: Vec<SourceDocument>,
    active_document_id: Option<u64>,
    next_document_id: u64,
    project_root: Option<PathBuf>,
    project_name: String,
    project_files: Vec<ProjectFile>,
    project_generation: u64,
    project_scan_generation: u64,
    pending_project: Option<PathBuf>,
    pending_project_scan: Option<(PathBuf, Vec<ProjectFile>)>,
    approved_project_switch: Option<PathBuf>,
    project_load_pending: bool,
    project_refresh_needed: bool,
    pending_close_document_id: Option<u64>,
    pending_close_after_save_id: Option<u64>,
    pending_save_as_id: Option<u64>,
    pending_ui_language: Option<UiLanguage>,
    pending_language_save_as_id: Option<u64>,
    reopen_settings_after_language_restart: bool,
    pending_close_app: bool,
    close_app_ready: bool,
    ui_language: UiLanguage,
    preferred_ui_language: UiLanguage,
    theme: ThemePreference,
    dark: bool,
    font_size: u32,
    line_numbers_visible: bool,
    word_wrap: bool,
    indent_width: u32,
    recovery_generation: u64,
    settings_visible: bool,
    output_text: String,
    output_status: String,
    problem_text: String,
    debugger_error: bool,
    game_preview_active: bool,
    next_execution_id: u64,
    active_execution_id: Option<u64>,
    debugging_active: bool,
    execution_document_id: Option<u64>,
    execution_line: Option<usize>,
    execution_project_root: Option<PathBuf>,
    debugger_document_id: Option<u64>,
    execution_input: Option<Sender<String>>,
    execution_control: Option<Sender<ControlAction>>,
    execution_cancellable: Option<gtk::gio::Cancellable>,
    file_selection_cancellable: Option<(u64, gtk::gio::Cancellable)>,
    input_requested: bool,
    input_prompt: Option<InputPrompt>,
    debugger_snapshot: Option<DebuggerSnapshot>,
    debugger_learning_hint: Option<LearningHint>,
    execution_diagnostics: Vec<execution::ExecutionDiagnostic>,
    execution_diagnostics_document_id: Option<u64>,
    execution_source_fingerprint: Option<u64>,
    learning_generation: u64,
    learning_revision: u64,
    learning_cursor_line: usize,
    learning_document_id: Option<u64>,
    learning_language: Option<Language>,
    learning_hint: Option<LearningHint>,
    learning_loading: bool,
    right_panel_section: RightPanelSection,
    right_panel_collapsed: bool,
    learning_worker: LearningAnalysisWorker,
    recent_paths: Vec<PathBuf>,
    start_page_visible: bool,
}

fn execution_reply_is_current(reply_execution_id: u64, active_execution_id: Option<u64>) -> bool {
    active_execution_id == Some(reply_execution_id)
}

impl MainWindow {
    fn text(&self, portuguese: &'static str) -> &'static str {
        tr(self.ui_language, portuguese)
    }

    fn preferences(&self, ui_language: UiLanguage) -> Preferences {
        Preferences {
            theme: self.theme,
            font_size: self.font_size,
            line_numbers_visible: self.line_numbers_visible,
            word_wrap: self.word_wrap,
            indent_width: self.indent_width,
            default_language: self.new_file_language,
            ui_language,
        }
    }

    fn save_preferences(&mut self) {
        let preferences = self.preferences(self.preferred_ui_language);
        if let Err(error) = persist_preferences(preferences) {
            self.output_status = format!(
                "{}: {error}",
                self.text("Não foi possível salvar as configurações")
            );
        }
    }

    fn persist_session(&mut self, clean_exit: bool, discard_unsaved: bool) {
        if let Err(error) = persist_session(self, clean_exit, discard_unsaved) {
            self.output_status =
                format!("{}: {error}", self.text("Não foi possível salvar a sessão"));
        }
    }

    fn remember_recent(&mut self, path: PathBuf) {
        let Some(path) = normalize_recent_path(path) else {
            return;
        };
        self.recent_paths.retain(|recent| recent != &path);
        self.recent_paths.insert(0, path);
        self.recent_paths.truncate(MAX_RECENT_ITEMS);
        if let Err(error) = persist_recent_paths(&self.recent_paths) {
            self.output_status = format!(
                "{}: {error}",
                self.text("Não foi possível atualizar programas recentes")
            );
        }
    }

    fn forget_recent(&mut self, path: &Path) -> Result<(), String> {
        self.recent_paths.retain(|recent| recent != path);
        persist_recent_paths(&self.recent_paths)
    }

    fn finish_language_change(&mut self, sender: ComponentSender<Self>) {
        let Some(language) = self.pending_ui_language else {
            return;
        };

        if let Some(document_id) = self
            .documents
            .iter()
            .find(|document| document.is_dirty() && document.path.is_none())
            .map(|document| document.id)
        {
            self.pending_language_save_as_id = Some(document_id);
            self.pending_save_as_id = Some(document_id);
            self.output_status = self
                .text("Salve este arquivo antes de alterar o idioma.")
                .to_string();
            return;
        }

        let dirty_files = self
            .documents
            .iter()
            .filter(|document| document.is_dirty())
            .filter_map(|document| document.path.clone().map(|path| (document.id, path)))
            .collect::<Vec<_>>();
        for (id, path) in dirty_files {
            if !self.save_document(id, path, sender.clone()) {
                self.abort_language_change(false);
                return;
            }
        }

        if let Err(error) = persist_session(self, false, false) {
            self.output_status =
                format!("{}: {error}", self.text("Não foi possível salvar a sessão"));
            self.abort_language_change(false);
            return;
        }
        if let Err(error) = persist_preferences(self.preferences(language)) {
            self.output_status = format!(
                "{}: {error}",
                self.text("Não foi possível salvar as configurações")
            );
            self.abort_language_change(false);
            return;
        }

        self.preferred_ui_language = language;
        match crate::restart_application() {
            Ok(()) => {
                #[cfg(windows)]
                {
                    self.pending_ui_language = None;
                    self.reopen_settings_after_language_restart = false;
                    self.close_app_ready = true;
                }
            }
            Err(error) => {
                self.preferred_ui_language = self.ui_language;
                if let Err(rollback_error) = persist_preferences(self.preferences(self.ui_language))
                {
                    self.output_status = format!(
                        "{}: {error}; {}: {rollback_error}",
                        self.text("Não foi possível reiniciar o aplicativo"),
                        self.text("Não foi possível restaurar as configurações")
                    );
                } else {
                    self.output_status = format!(
                        "{}: {error}",
                        self.text("Não foi possível reiniciar o aplicativo")
                    );
                }
                self.abort_language_change(false);
            }
        }
    }

    fn abort_language_change(&mut self, canceled: bool) {
        self.pending_ui_language = None;
        self.pending_language_save_as_id = None;
        self.pending_save_as_id = None;
        self.preferred_ui_language = self.ui_language;
        if canceled {
            self.output_status = self
                .text("A alteração de idioma foi cancelada.")
                .to_string();
        }
        self.settings_visible = std::mem::take(&mut self.reopen_settings_after_language_restart);
    }
}

fn preference_file() -> PathBuf {
    gtk::glib::user_config_dir()
        .join("potoo-ide")
        .join("settings.ini")
}

fn session_file() -> PathBuf {
    gtk::glib::user_data_dir()
        .join("potoo-ide")
        .join("session.ini")
}

fn recent_file() -> PathBuf {
    gtk::glib::user_data_dir()
        .join("potoo-ide")
        .join("recent.ini")
}

const MAX_RECENT_ITEMS: usize = 5;

fn normalize_recent_path(path: PathBuf) -> Option<PathBuf> {
    let path = path.canonicalize().unwrap_or(path);
    if path.is_dir() || (path.is_file() && Language::from_path(&path).is_some()) {
        Some(path)
    } else {
        None
    }
}

fn load_recent_paths() -> Vec<PathBuf> {
    let Some(key_file) = load_key_file(&recent_file()) else {
        return Vec::new();
    };
    let count = key_file
        .integer("recent", "count")
        .unwrap_or(0)
        .clamp(0, MAX_RECENT_ITEMS as i32) as usize;
    let mut paths = Vec::with_capacity(count);
    for index in 0..count {
        let key = format!("path-{index}");
        let Ok(path) = key_file.string("recent", &key) else {
            continue;
        };
        let Some(path) = normalize_recent_path(PathBuf::from(path.as_str())) else {
            continue;
        };
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    paths
}

fn persist_recent_paths(paths: &[PathBuf]) -> Result<(), String> {
    let key_file = gtk::glib::KeyFile::new();
    let paths = paths.iter().take(MAX_RECENT_ITEMS).collect::<Vec<_>>();
    key_file.set_integer("recent", "count", paths.len() as i32);
    for (index, path) in paths.into_iter().enumerate() {
        key_file.set_string("recent", &format!("path-{index}"), &path.to_string_lossy());
    }
    save_key_file(&key_file, &recent_file())
}

fn load_key_file(path: &Path) -> Option<gtk::glib::KeyFile> {
    let key_file = gtk::glib::KeyFile::new();
    if key_file
        .load_from_file(path, gtk::glib::KeyFileFlags::NONE)
        .is_ok()
    {
        return Some(key_file);
    }

    let temporary_path = path.with_extension("ini.tmp");
    let fallback = gtk::glib::KeyFile::new();
    fallback
        .load_from_file(&temporary_path, gtk::glib::KeyFileFlags::NONE)
        .ok()
        .map(|()| fallback)
}

fn save_key_file(key_file: &gtk::glib::KeyFile, path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "caminho de configuração inválido".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary_path = path.with_extension("ini.tmp");
    key_file
        .save_to_file(&temporary_path)
        .map_err(|error| error.to_string())?;

    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }

    fs::rename(&temporary_path, path).map_err(|error| error.to_string())
}

fn load_preferences() -> Preferences {
    let Some(key_file) = load_key_file(&preference_file()) else {
        return Preferences::default();
    };
    let defaults = Preferences::default();
    let settings_version = key_file.integer("editor", "settings-version").unwrap_or(1);
    Preferences {
        theme: key_file
            .string("editor", "theme")
            .map(|value| ThemePreference::from_key(&value))
            .unwrap_or(defaults.theme),
        font_size: key_file
            .integer("editor", "font-size")
            .ok()
            .map(|size| {
                if settings_version < 2 {
                    match size {
                        13 => 14,
                        15 => 16,
                        17 => 18,
                        14 | 16 | 18 => size as u32,
                        _ => defaults.font_size,
                    }
                } else {
                    match size {
                        14 | 15 | 16 | 18 => size as u32,
                        _ => defaults.font_size,
                    }
                }
            })
            .unwrap_or(defaults.font_size),
        line_numbers_visible: key_file
            .boolean("editor", "line-numbers")
            .unwrap_or(defaults.line_numbers_visible),
        word_wrap: key_file
            .boolean("editor", "word-wrap")
            .unwrap_or(defaults.word_wrap),
        indent_width: key_file
            .integer("editor", "indent-width")
            .ok()
            .filter(|width| matches!(width, 2 | 4))
            .map(|width| width as u32)
            .unwrap_or(defaults.indent_width),
        default_language: key_file
            .string("editor", "default-language")
            .map(|value| Language::from_key(&value))
            .unwrap_or(defaults.default_language),
        ui_language: UiLanguage::from_preference(
            key_file.string("interface", "language").ok().as_deref(),
            gtk::glib::language_names(),
        ),
    }
}

fn persist_preferences(preferences: Preferences) -> Result<(), String> {
    let key_file = gtk::glib::KeyFile::new();
    key_file.set_string("editor", "theme", preferences.theme.key());
    key_file.set_integer("editor", "settings-version", 2);
    key_file.set_integer("editor", "font-size", preferences.font_size as i32);
    key_file.set_boolean("editor", "line-numbers", preferences.line_numbers_visible);
    key_file.set_boolean("editor", "word-wrap", preferences.word_wrap);
    key_file.set_integer("editor", "indent-width", preferences.indent_width as i32);
    key_file.set_string(
        "editor",
        "default-language",
        preferences.default_language.key(),
    );
    key_file.set_string("interface", "language", preferences.ui_language.key());
    save_key_file(&key_file, &preference_file())
}

fn load_session() -> Option<SessionSnapshot> {
    let key_file = load_key_file(&session_file())?;
    let document_count = key_file
        .integer("session", "document-count")
        .ok()?
        .clamp(0, 500) as usize;
    let mut documents = Vec::with_capacity(document_count);
    for index in 0..document_count {
        let group = format!("document-{index}");
        let path = key_file
            .string(&group, "path")
            .ok()
            .and_then(|path| (!path.is_empty()).then(|| PathBuf::from(path.as_str())));
        let contents = key_file
            .string(&group, "contents")
            .map(|value| value.to_string())
            .ok()?;
        let saved_contents = key_file
            .string(&group, "saved-contents")
            .map(|value| value.to_string())
            .ok()?;
        let name = key_file
            .string(&group, "name")
            .map(|value| value.to_string())
            .unwrap_or_else(|_| "principal.por".to_string());
        let language = key_file
            .string(&group, "language")
            .map(|value| Language::from_key(&value))
            .unwrap_or_else(|_| {
                Language::from_path(path.as_deref().unwrap_or(Path::new(".por")))
                    .unwrap_or(Language::Portugol)
            });
        documents.push(SessionDocument {
            path,
            name,
            language,
            contents,
            saved_contents,
        });
    }

    let project_root = key_file
        .string("session", "project-root")
        .ok()
        .and_then(|path| (!path.is_empty()).then(|| PathBuf::from(path.as_str())));
    Some(SessionSnapshot {
        clean_exit: key_file.boolean("session", "clean-exit").unwrap_or(true),
        project_root,
        active_index: key_file
            .integer("session", "active-index")
            .unwrap_or(0)
            .max(0) as usize,
        documents,
    })
}

fn persist_session(
    model: &MainWindow,
    clean_exit: bool,
    discard_unsaved: bool,
) -> Result<(), String> {
    let key_file = gtk::glib::KeyFile::new();
    key_file.set_boolean("session", "clean-exit", clean_exit);
    key_file.set_integer("session", "document-count", model.documents.len() as i32);
    key_file.set_integer(
        "session",
        "active-index",
        model
            .documents
            .iter()
            .position(|document| Some(document.id) == model.active_document_id)
            .unwrap_or(0) as i32,
    );
    let project_root = model
        .project_root
        .as_deref()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    key_file.set_string("session", "project-root", &project_root);

    for (index, document) in model.documents.iter().enumerate() {
        let group = format!("document-{index}");
        let path = document
            .path
            .as_deref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default();
        key_file.set_string(&group, "path", &path);
        key_file.set_string(&group, "name", &document.name);
        key_file.set_string(&group, "language", document.language.key());
        let contents = if discard_unsaved {
            &document.saved_contents
        } else {
            &document.contents
        };
        key_file.set_string(&group, "contents", contents);
        key_file.set_string(&group, "saved-contents", &document.saved_contents);
    }
    save_key_file(&key_file, &session_file())
}

fn restore_session(snapshot: SessionSnapshot) -> (Vec<SourceDocument>, Option<u64>, bool) {
    let mut documents = Vec::new();
    let mut active_document_id = None;
    let mut recovered_changes = false;
    for (index, saved) in snapshot.documents.into_iter().enumerate() {
        let (contents, saved_contents) = if snapshot.clean_exit {
            match saved.path.as_deref() {
                Some(path) => match fs::read_to_string(path) {
                    Ok(contents) => (contents.clone(), contents),
                    Err(_) => continue,
                },
                None => (saved.saved_contents.clone(), saved.saved_contents),
            }
        } else {
            (saved.contents, saved.saved_contents)
        };
        let id = documents.len() as u64 + 1;
        recovered_changes |= contents != saved_contents;
        if index == snapshot.active_index {
            active_document_id = Some(id);
        }
        documents.push(SourceDocument {
            id,
            path: saved.path,
            name: saved.name,
            language: saved.language,
            contents,
            saved_contents,
        });
    }
    if active_document_id.is_none() {
        active_document_id = documents.first().map(|document| document.id);
    }
    (
        documents,
        active_document_id,
        !snapshot.clean_exit && recovered_changes,
    )
}

impl MainWindow {
    fn active_document(&self) -> Option<&SourceDocument> {
        self.documents
            .iter()
            .find(|document| Some(document.id) == self.active_document_id)
    }

    fn source_fingerprint(source: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        hasher.finish()
    }

    fn schedule_learning_refresh(&mut self, sender: ComponentSender<Self>, delay: Duration) {
        let generation =
            invalidate_learning_hint(&mut self.learning_generation, &mut self.learning_hint);
        self.learning_worker.invalidate(generation);
        self.learning_document_id = self.active_document_id;
        self.learning_language = self.active_document().map(|document| document.language);
        self.learning_loading = self.learning_document_id.is_some();
        if self.learning_document_id.is_some() {
            gtk::glib::timeout_add_local_once(delay, move || {
                sender.input(MainWindowMsg::LearningDebounceElapsed(generation));
            });
        }
    }

    fn cancel_file_selection(&mut self, execution_id: u64) {
        if self
            .file_selection_cancellable
            .as_ref()
            .is_some_and(|(picker_execution_id, _)| *picker_execution_id == execution_id)
            && let Some((_, cancellable)) = self.file_selection_cancellable.take()
        {
            cancellable.cancel();
        }
    }

    fn start_execution(&mut self, sender: ComponentSender<Self>, debugging: bool) {
        if self.active_execution_id.is_some() {
            return;
        }
        let Some(document) = self.active_document().cloned() else {
            self.output_status = self.text("Abra um programa antes de executar.").to_string();
            return;
        };

        self.output_text.clear();
        self.problem_text.clear();
        self.input_requested = false;
        self.input_prompt = None;
        self.debugger_snapshot = None;
        self.debugger_error = false;
        self.debugger_learning_hint = None;
        self.debugging_active = debugging;
        self.execution_line = None;
        self.execution_diagnostics.clear();
        self.execution_diagnostics_document_id = None;
        self.schedule_learning_refresh(sender.clone(), Duration::from_millis(20));
        self.execution_source_fingerprint = Some(Self::source_fingerprint(&document.contents));
        self.output_status = self
            .text(if debugging {
                "Depuração iniciada…"
            } else {
                "Executando o programa…"
            })
            .to_string();
        let execution_id = self.next_execution_id;
        self.next_execution_id = self.next_execution_id.wrapping_add(1).max(1);
        self.active_execution_id = Some(execution_id);
        self.execution_document_id = Some(document.id);
        self.debugger_document_id = debugging.then_some(document.id);
        let execution_root = self.project_root.clone().or_else(|| {
            document
                .path
                .as_deref()
                .and_then(Path::parent)
                .map(Path::to_path_buf)
        });
        self.execution_project_root = execution_root.clone();

        let running = execution::start(
            document.contents,
            document.language,
            document.path.clone(),
            execution_root,
            debugging,
        );
        self.execution_input = Some(running.input);
        self.execution_control = Some(running.control);
        self.execution_cancellable = Some(running.cancellable);
        let events = running.events;
        gtk::glib::timeout_add_local(Duration::from_millis(16), move || {
            for _ in 0..MAX_EXECUTION_EVENTS_PER_TICK {
                match events.try_recv() {
                    Ok(event) => {
                        let finished = matches!(&event, ExecutionEvent::Finished { .. });
                        sender.input(MainWindowMsg::ExecutionEvent(execution_id, event));
                        if finished {
                            return gtk::glib::ControlFlow::Break;
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        return gtk::glib::ControlFlow::Continue;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        sender.input(MainWindowMsg::ExecutionLost(execution_id));
                        return gtk::glib::ControlFlow::Break;
                    }
                }
            }
            gtk::glib::ControlFlow::Continue
        });
    }

    fn send_execution_control(&self, action: ControlAction) {
        if action == ControlAction::Stop
            && let Some((execution_id, cancellable)) = &self.file_selection_cancellable
            && self.active_execution_id == Some(*execution_id)
        {
            cancellable.cancel();
        }
        if action == ControlAction::Stop
            && let Some(cancellable) = &self.execution_cancellable
        {
            cancellable.cancel();
        }
        if let Some(control) = &self.execution_control {
            let _ = control.send(action);
        }
    }

    fn create_document(
        &mut self,
        path: Option<PathBuf>,
        name: String,
        language: Language,
        contents: String,
        saved_contents: String,
    ) -> u64 {
        let id = self.next_document_id;
        self.next_document_id += 1;
        self.documents.push(SourceDocument {
            id,
            path,
            name,
            language,
            contents,
            saved_contents,
        });
        self.active_document_id = Some(id);
        id
    }

    fn open_document(&mut self, path: PathBuf, sender: ComponentSender<Self>) {
        let standalone = self
            .project_root
            .as_ref()
            .map_or(true, |root| !path.starts_with(root));
        if let Some(document) = self
            .documents
            .iter()
            .find(|doc| doc.path.as_ref() == Some(&path))
        {
            self.active_document_id = Some(document.id);
            self.start_page_visible = false;
            return;
        }
        if Language::from_path(&path).is_none() {
            self.output_status = self
                .text("O editor abre apenas arquivos .por, .alg, .poti e .gpt.")
                .to_string();
            return;
        }
        let project_generation = self.project_generation;
        self.output_status = self.text("Abrindo arquivo…").to_string();
        std::thread::spawn(move || {
            let result = fs::read_to_string(&path).map_err(|error| error.to_string());
            sender.input(MainWindowMsg::DocumentReadCompleted {
                project_generation,
                path,
                standalone,
                result,
            });
        });
    }

    fn schedule_project_scan(
        &mut self,
        root: PathBuf,
        load_project: bool,
        sender: ComponentSender<Self>,
    ) {
        if !load_project && self.project_load_pending {
            return;
        }
        self.project_scan_generation = self.project_scan_generation.wrapping_add(1).max(1);
        let scan_generation = self.project_scan_generation;
        let project_generation = self.project_generation;
        if load_project {
            self.project_load_pending = true;
        }
        std::thread::spawn(move || {
            let result = scan_project_files(&root).map_err(|error| error.to_string());
            sender.input(MainWindowMsg::ProjectScanCompleted {
                project_generation,
                scan_generation,
                root,
                load_project,
                result,
            });
        });
    }

    fn refresh_project_scan(&mut self, root: PathBuf, sender: ComponentSender<Self>) {
        if self.project_load_pending {
            self.project_refresh_needed = true;
            return;
        }
        self.schedule_project_scan(root, false, sender);
    }

    fn load_project(
        &mut self,
        root: PathBuf,
        discard_changes_confirmed: bool,
        sender: ComponentSender<Self>,
    ) {
        self.project_generation = self.project_generation.wrapping_add(1).max(1);
        self.pending_project_scan = None;
        self.approved_project_switch = discard_changes_confirmed.then(|| root.clone());
        self.project_refresh_needed = false;
        self.output_status = self.text("Abrindo pasta…").to_string();
        self.schedule_project_scan(root, true, sender);
    }

    fn finish_document_read(
        &mut self,
        project_generation: u64,
        path: PathBuf,
        standalone: bool,
        result: Result<String, String>,
        sender: ComponentSender<Self>,
    ) {
        if project_generation != self.project_generation {
            return;
        }
        if let Some(document) = self
            .documents
            .iter()
            .find(|document| document.path.as_ref() == Some(&path))
        {
            self.active_document_id = Some(document.id);
            self.start_page_visible = false;
            return;
        }
        let contents = match result {
            Ok(contents) => contents,
            Err(error) => {
                self.output_status =
                    format!("{}: {error}", self.text("Não foi possível abrir o arquivo"));
                return;
            }
        };
        if self.project_root.is_none()
            && let Some(root) = path.parent().map(Path::to_path_buf)
        {
            self.project_name = root
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| root.display().to_string());
            self.project_root = Some(root.clone());
            self.schedule_project_scan(root, false, sender);
        }
        let Some(language) = Language::from_path(&path) else {
            return;
        };
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        self.create_document(
            Some(path.clone()),
            name,
            language,
            contents.clone(),
            contents,
        );
        self.start_page_visible = false;
        if standalone {
            self.remember_recent(path);
        }
        self.output_status = self.text("Arquivo aberto").to_string();
    }

    fn finish_project_scan(
        &mut self,
        project_generation: u64,
        scan_generation: u64,
        root: PathBuf,
        load_project: bool,
        result: Result<Vec<ProjectFile>, String>,
        sender: ComponentSender<Self>,
    ) {
        if !project_scan_is_current(
            self.project_generation,
            self.project_scan_generation,
            self.project_root.as_deref(),
            project_generation,
            scan_generation,
            &root,
            load_project,
        ) {
            return;
        }
        let refresh_after_load = if load_project {
            self.project_load_pending = false;
            std::mem::take(&mut self.project_refresh_needed)
        } else {
            false
        };
        let files = match result {
            Ok(files) => files,
            Err(error) => {
                self.pending_project_scan = None;
                if load_project {
                    self.approved_project_switch = None;
                }
                let message = if load_project {
                    "Não foi possível abrir a pasta"
                } else {
                    "Não foi possível atualizar os arquivos do projeto"
                };
                self.output_status = format!("{}: {error}", self.text(message));
                if refresh_after_load && let Some(current_root) = self.project_root.clone() {
                    self.refresh_project_scan(current_root, sender);
                }
                return;
            }
        };
        if !load_project {
            self.project_files = files;
            return;
        }
        if self.documents.iter().any(SourceDocument::is_dirty)
            && self.approved_project_switch.as_deref() != Some(root.as_path())
        {
            self.pending_project = Some(root.clone());
            self.pending_project_scan = Some((root, files));
            self.project_refresh_needed = refresh_after_load;
            return;
        }
        self.approved_project_switch = None;
        self.apply_loaded_project(root.clone(), files, sender.clone());
        if refresh_after_load {
            self.refresh_project_scan(root, sender);
        }
    }

    fn apply_loaded_project(
        &mut self,
        root: PathBuf,
        files: Vec<ProjectFile>,
        sender: ComponentSender<Self>,
    ) {
        self.project_name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string());
        self.remember_recent(root.clone());
        self.project_root = Some(root);
        self.start_page_visible = false;
        self.project_files = files;
        self.documents.clear();
        self.active_document_id = None;
        let root_file = self.project_root.as_ref().and_then(|root| {
            self.project_files
                .iter()
                .find(|file| {
                    !file.is_directory
                        && file.path.parent() == Some(root.as_path())
                        && Language::from_path(&file.path).is_some()
                })
                .map(|file| file.path.clone())
        });
        let first_source_file = root_file.or_else(|| {
            self.project_files
                .iter()
                .find(|file| !file.is_directory && Language::from_path(&file.path).is_some())
                .map(|file| file.path.clone())
        });
        if let Some(path) = first_source_file {
            self.open_document(path, sender);
        } else {
            self.output_status = self
                .text("Pasta aberta. Crie um arquivo .por, .alg, .poti ou .gpt para começar.")
                .to_string();
        }
    }

    fn save_document(&mut self, id: u64, path: PathBuf, sender: ComponentSender<Self>) -> bool {
        let Some(index) = self.documents.iter().position(|doc| doc.id == id) else {
            return false;
        };
        let (path, language) =
            match supported_save_path(path, self.documents[index].language, self.ui_language) {
                Ok(path) => path,
                Err(error) => {
                    self.output_status = error;
                    return false;
                }
            };
        let standalone = self
            .project_root
            .as_ref()
            .map_or(true, |root| !path.starts_with(root));
        match atomic_write(&path, self.documents[index].contents.as_bytes()) {
            Ok(()) => {
                let document = &mut self.documents[index];
                document.path = Some(path.clone());
                document.language = language;
                document.name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                document.saved_contents.clone_from(&document.contents);
                if self.project_root.is_none() {
                    self.project_root = path.parent().map(Path::to_path_buf);
                    if let Some(root) = &self.project_root {
                        self.project_name = root
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| root.display().to_string());
                    }
                }
                if let Some(root) = self.project_root.clone() {
                    self.refresh_project_scan(root, sender);
                }
                if standalone {
                    self.remember_recent(path.clone());
                }
                self.output_status = self.text("Arquivo salvo").to_string();
                true
            }
            Err(error) => {
                self.output_status = format!(
                    "{}: {error}",
                    self.text("Não foi possível salvar o arquivo")
                );
                false
            }
        }
    }

    fn create_project_file(
        &mut self,
        name: String,
        template: ProjectTemplate,
        relative_folder: PathBuf,
        sender: ComponentSender<Self>,
    ) {
        let Some(root) = self.project_root.clone() else {
            self.output_status = self
                .text("Abra uma pasta de projeto antes de criar itens.")
                .to_string();
            return;
        };
        let folder = match resolve_project_directory(&root, &relative_folder, self.ui_language) {
            Ok(folder) => folder,
            Err(error) => {
                self.output_status = error;
                return;
            }
        };
        let (path, language) =
            match new_source_path(&folder, &name, template.language(), self.ui_language) {
                Ok(source) => source,
                Err(error) => {
                    self.output_status = error;
                    return;
                }
            };
        let contents = if language == Language::Potigol {
            template.new_file_sample()
        } else {
            ""
        };

        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(error) = file.write_all(contents.as_bytes()) {
                    let _ = fs::remove_file(&path);
                    self.output_status =
                        format!("{}: {error}", self.text("Não foi possível criar o arquivo"));
                    return;
                }
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                self.create_document(
                    Some(path.clone()),
                    name,
                    language,
                    contents.to_owned(),
                    contents.to_owned(),
                );
                self.new_file_language = language;
                self.refresh_project_scan(root, sender);
                self.output_status = self.text("Novo arquivo criado").to_string();
                self.save_preferences();
            }
            Err(error) => {
                self.output_status = if error.kind() == std::io::ErrorKind::AlreadyExists {
                    self.text("Já existe um arquivo com esse nome.").to_string()
                } else {
                    format!("{}: {error}", self.text("Não foi possível criar o arquivo"))
                };
            }
        }
    }

    fn create_project_folder(
        &mut self,
        name: String,
        relative_folder: PathBuf,
        sender: ComponentSender<Self>,
    ) {
        let Some(root) = self.project_root.clone() else {
            self.output_status = self
                .text("Abra uma pasta de projeto antes de criar itens.")
                .to_string();
            return;
        };
        let name = name.trim();
        if !is_valid_directory_name(name) {
            self.output_status = self.text("Digite um nome válido para a pasta.").to_string();
            return;
        }

        let folder = match resolve_project_directory(&root, &relative_folder, self.ui_language) {
            Ok(folder) => folder,
            Err(error) => {
                self.output_status = error;
                return;
            }
        };

        match fs::create_dir(folder.join(name)) {
            Ok(()) => {
                self.refresh_project_scan(root, sender);
                self.output_status = self.text("Pasta criada").to_string();
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                self.output_status = self.text("Já existe uma pasta com esse nome.").to_string();
            }
            Err(error) => {
                self.output_status =
                    format!("{}: {error}", self.text("Não foi possível criar a pasta"));
            }
        }
    }

    fn validate_project_item(&self, path: &Path) -> Result<bool, String> {
        let unavailable = || {
            self.text("O item selecionado não está mais disponível.")
                .to_string()
        };
        let Some(root) = self.project_root.as_deref() else {
            return Err(unavailable());
        };
        let Some(item) = self.project_files.iter().find(|item| item.path == path) else {
            return Err(unavailable());
        };
        let root = fs::canonicalize(root).map_err(|_| unavailable())?;
        let item_path = fs::canonicalize(path).map_err(|_| unavailable())?;
        if item_path == root
            || !item_path.starts_with(&root)
            || item_path.is_dir() != item.is_directory
        {
            return Err(unavailable());
        }
        Ok(item.is_directory)
    }

    fn rename_project_item(&mut self, path: PathBuf, name: String, sender: ComponentSender<Self>) {
        let is_directory = match self.validate_project_item(&path) {
            Ok(is_directory) => is_directory,
            Err(error) => {
                self.output_status = error;
                return;
            }
        };
        let name = match normalized_project_item_name(&path, &name, is_directory, self.ui_language)
        {
            Ok(name) => name,
            Err(error) => {
                self.output_status = error;
                return;
            }
        };
        let Some(parent) = path.parent() else {
            self.output_status = self
                .text("O item selecionado não está mais disponível.")
                .to_string();
            return;
        };
        let new_path = parent.join(name);
        if new_path == path {
            return;
        }
        if new_path.exists() && fs::canonicalize(&new_path).ok() != fs::canonicalize(&path).ok() {
            self.output_status = self.text("Já existe um item com esse nome.").to_string();
            return;
        }

        if let Err(error) = fs::rename(&path, &new_path) {
            self.output_status =
                format!("{}: {error}", self.text("Não foi possível renomear o item"));
            return;
        }

        for document in &mut self.documents {
            let Some(document_path) = document.path.clone() else {
                continue;
            };
            let Ok(relative_path) = document_path.strip_prefix(&path) else {
                continue;
            };
            if document_path == path {
                document.name = new_path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| new_path.display().to_string());
                if let Some(language) = Language::from_path(&new_path) {
                    document.language = language;
                }
            }
            document.path = Some(new_path.join(relative_path));
        }

        if let Some(root) = self.project_root.clone() {
            self.refresh_project_scan(root, sender);
        }
        self.output_status = self.text("Item renomeado.").to_string();
    }

    fn delete_project_item(&mut self, path: PathBuf, sender: ComponentSender<Self>) {
        let is_directory = match self.validate_project_item(&path) {
            Ok(is_directory) => is_directory,
            Err(error) => {
                self.output_status = error;
                return;
            }
        };
        let result = if is_directory {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        if let Err(error) = result {
            self.output_status =
                format!("{}: {error}", self.text("Não foi possível excluir o item"));
            return;
        }

        let removed_documents = self
            .documents
            .iter()
            .filter(|document| {
                document.path.as_deref().is_some_and(|document_path| {
                    document_path == path || (is_directory && document_path.starts_with(&path))
                })
            })
            .map(|document| document.id)
            .collect::<Vec<_>>();
        for id in removed_documents {
            self.close_document(id);
        }

        self.project_files
            .retain(|item| item.path != path && !item.path.starts_with(&path));
        if let Some(root) = self.project_root.clone() {
            self.refresh_project_scan(root, sender);
        }
        self.output_status = self.text("Item excluído.").to_string();
    }

    fn create_project(
        &mut self,
        parent: PathBuf,
        name: String,
        template: ProjectTemplate,
        sender: ComponentSender<Self>,
    ) {
        let name = name.trim();
        if !is_valid_directory_name(name) {
            self.output_status = self
                .text("Digite um nome válido para o projeto.")
                .to_string();
            return;
        }

        let root = parent.join(name);
        if let Err(error) = fs::create_dir(&root) {
            self.output_status = if error.kind() == std::io::ErrorKind::AlreadyExists {
                self.text("Já existe um projeto com esse nome.").to_string()
            } else {
                format!("{}: {error}", self.text("Não foi possível criar o projeto"))
            };
            return;
        }

        let source_path = root.join(template.file_name());
        if let Err(error) = fs::write(&source_path, template.sample()) {
            let _ = fs::remove_file(&source_path);
            let cleanup_error = fs::remove_dir(&root).err();
            self.output_status = match cleanup_error {
                Some(cleanup_error) => format!(
                    "{}: {error} ({}: {cleanup_error})",
                    self.text("Não foi possível criar o projeto"),
                    self.text("Não foi possível remover a pasta incompleta")
                ),
                None => format!("{}: {error}", self.text("Não foi possível criar o projeto")),
            };
            return;
        }

        self.output_status = self.text("Projeto criado").to_string();
        sender.input(MainWindowMsg::ProjectSelected(root));
    }

    fn close_document(&mut self, id: u64) {
        let Some(index) = self.documents.iter().position(|document| document.id == id) else {
            return;
        };
        self.documents.remove(index);
        if self.active_document_id == Some(id) {
            self.active_document_id = self
                .documents
                .get(index)
                .or_else(|| {
                    index
                        .checked_sub(1)
                        .and_then(|previous| self.documents.get(previous))
                })
                .map(|document| document.id);
        }
    }

    fn request_save(&mut self, sender: ComponentSender<Self>) {
        match self
            .active_document()
            .map(|document| (document.id, document.path.clone()))
        {
            Some((id, Some(path))) => {
                self.save_document(id, path, sender);
            }
            Some((id, _)) => self.pending_save_as_id = Some(id),
            None => {
                self.output_status = self
                    .text("Crie ou abra um arquivo antes de salvar.")
                    .to_string();
            }
        }
    }
}

struct SettingsUi {
    window: gtk::Window,
    theme_buttons: [gtk::Button; 3],
    font_size_picker: gtk::DropDown,
    line_numbers_switch: gtk::Switch,
    word_wrap_switch: gtk::Switch,
    indent_width_picker: gtk::DropDown,
    ui_language_picker: gtk::DropDown,
}

pub struct MainUi {
    window: gtk::ApplicationWindow,
    content: gtk::Box,
    main_stack: gtk::Stack,
    dark_theme: Rc<Cell<bool>>,
    default_new_language: Rc<Cell<Language>>,
    project_root_for_new_items: Rc<RefCell<Option<PathBuf>>>,
    project_files_for_new_items: Rc<RefCell<Vec<ProjectFile>>>,
    settings: SettingsUi,
    project_title: gtk::Label,
    run_button: gtk::Button,
    run_icon: gtk::Image,
    run_label: gtk::Label,
    start_page: start_page::StartPageUi,
    project_row: gtk::Box,
    project_label: gtk::Label,
    file_list: gtk::Box,
    new_file_button: gtk::MenuButton,
    empty_state: gtk::Widget,
    expanded_folders: sidebar::ExpandedFolders,
    editor_tabs: gtk::Box,
    status_language: gtk::Label,
    status_details: gtk::Label,
    status_position: gtk::Label,
    indent_width: Rc<Cell<u32>>,
    code_buffer: gtk::TextBuffer,
    code_view: gtk::TextView,
    syntax_tags: SyntaxTags,
    line_numbers: gtk::Label,
    line_numbers_separator: gtk::Separator,
    output_buffer: gtk::TextBuffer,
    output_view: gtk::TextView,
    problems_buffer: gtk::TextBuffer,
    problems_view: gtk::TextView,
    problems_tab: gtk::Button,
    input_entry: gtk::Entry,
    output_status: gtk::Label,
    debugger_start: gtk::Button,
    debugger_controls: gtk::Stack,
    debugger_step: gtk::Button,
    debugger_stop: gtk::Button,
    debugger_position: gtk::Label,
    debugger_variables: gtk::TextBuffer,
    debugger_learning: content_panel::DebuggerLearningUi,
    right_panel_revealer: gtk::Revealer,
    right_panel_stack: gtk::Stack,
    learning_tool_button: gtk::Button,
    debugger_tool_button: gtk::Button,
    game_preview_button: gtk::Button,
    game_preview_stack: gtk::Stack,
    game_preview_area: gtk::DrawingArea,
    learning_spinner: gtk::Spinner,
    learning_analysis_stack: gtk::Stack,
    learning_line_title: gtk::Label,
    learning_explanation: gtk::Label,
    learning_experiment: gtk::Label,
    learning_concept_title: gtk::Label,
    learning_concept_text: gtk::Label,
    learning_problem_list: gtk::Box,
    learning_problem_empty: gtk::Label,
    displayed_language: Option<Language>,
    displayed_active_document_id: Option<u64>,
    displayed_project_name: String,
    displayed_project_files: Vec<(PathBuf, bool)>,
    displayed_project_open: bool,
    displayed_recent_paths: Vec<PathBuf>,
    displayed_start_page: bool,
    displayed_run_active: bool,
    displayed_run_language: UiLanguage,
    displayed_tabs: Vec<(u64, String, bool, bool)>,
    displayed_pending_project: Option<PathBuf>,
    displayed_pending_close_document_id: Option<u64>,
    displayed_pending_close_app: bool,
    displayed_pending_save_as_id: Option<u64>,
    displayed_close_app_ready: bool,
    close_authorized: Rc<Cell<bool>>,
    displayed_theme: ThemePreference,
    displayed_dark: bool,
    displayed_font_size: u32,
    displayed_line_numbers_visible: bool,
    displayed_word_wrap: bool,
    displayed_indent_width: u32,
    displayed_preferred_ui_language: UiLanguage,
    displayed_settings_visible: bool,
    displayed_output_text: String,
    displayed_problem_text: String,
    displayed_input_requested: bool,
    displayed_output_status: String,
    displayed_debugger_snapshot: Option<DebuggerSnapshot>,
    displayed_debugger_error: bool,
    displayed_execution_line: Option<usize>,
    displayed_debugger_learning_hint: Option<LearningHint>,
    displayed_debugger_analysis_state: &'static str,
    displayed_learning_hint: Option<LearningHint>,
    displayed_learning_loading: bool,
    displayed_right_panel_section: Option<RightPanelSection>,
    displayed_right_panel_collapsed: bool,
    displayed_game_preview_active: bool,
}

impl MainWindow {
    fn handle_execution_event(
        &mut self,
        execution_id: u64,
        event: ExecutionEvent,
        sender: ComponentSender<Self>,
    ) {
        if execution_reply_is_current(execution_id, self.active_execution_id) {
            match event {
                ExecutionEvent::Output(text) => self.output_text.push_str(&text),
                ExecutionEvent::ClearOutput => self.output_text.clear(),
                ExecutionEvent::InputRequested(prompt) => {
                    if let (Some(name), Some(data_type)) =
                        (&prompt.variable_name, &prompt.data_type)
                    {
                        self.output_status =
                            format!("{} {name} ({data_type})", self.text("Entrada solicitada:"));
                    }
                    self.input_prompt = Some(prompt);
                    self.input_requested = true;
                }
                ExecutionEvent::FileSelectionRequested {
                    extensions,
                    allow_all_files,
                } => {
                    self.input_prompt = None;
                    self.input_requested = true;
                    if self.execution_input.is_none() {
                        self.input_requested = false;
                        self.output_status = self
                            .text("Não foi possível enviar a entrada ao programa.")
                            .to_string();
                        return;
                    }
                    let cancellable = gtk::gio::Cancellable::new();
                    self.file_selection_cancellable = Some((execution_id, cancellable.clone()));
                    show_portugol_file_selection_dialog(
                        &self.window,
                        self.ui_language,
                        execution_id,
                        self.execution_project_root.clone(),
                        extensions,
                        allow_all_files,
                        cancellable,
                        sender.clone(),
                    );
                }
                ExecutionEvent::HostRequest { request, response } => {
                    let result = match request {
                        execution::HostRequest::GraphicsWindowIsOpen => {
                            Ok(Some(potoo_portugol_interpreter::HostValue::Logical(
                                self.graphics_host.is_open(),
                            )))
                        }
                        execution::HostRequest::JerimumPreviewStart => {
                            self.graphics_host.start_jerimum_preview().map(|value| {
                                self.right_panel_section = RightPanelSection::GamePreview;
                                self.right_panel_collapsed = false;
                                self.game_preview_active = true;
                                Some(value)
                            })
                        }
                        execution::HostRequest::Audio {
                            function,
                            arguments,
                        } => {
                            let root = self.execution_project_root.clone();
                            self.audio_host
                                .handle(root.as_deref(), &function, &arguments)
                                .map(Some)
                        }
                        execution::HostRequest::Graphics {
                            function,
                            arguments,
                        } => {
                            let root = self.execution_project_root.clone();
                            self.graphics_host
                                .graphics(root.as_deref(), &function, &arguments)
                                .map(Some)
                        }
                        request => self
                            .graphics_host
                            .handle_input_request(request, response.clone()),
                    };
                    match result {
                        Ok(Some(value)) => {
                            let _ = response.send(Ok(value));
                        }
                        Ok(None) => {}
                        Err(error) => {
                            let _ = response.send(Err(error));
                        }
                    }
                }
                ExecutionEvent::LineChanged(line) => self.execution_line = Some(line),
                ExecutionEvent::Debugger(snapshot) => {
                    self.debugging_active = true;
                    self.debugger_document_id = self.execution_document_id;
                    self.right_panel_section = RightPanelSection::Debugger;
                    self.right_panel_collapsed = false;
                    self.output_status =
                        format!("{} {}", self.text("Pausado na linha"), snapshot.line);
                    if self.execution_document_id == self.active_document_id {
                        self.debugger_learning_hint = self
                            .learning_hint
                            .as_ref()
                            .filter(|hint| hint.line == snapshot.line)
                            .cloned();
                        if self.learning_cursor_line != snapshot.line
                            || (self.debugger_learning_hint.is_none() && !self.learning_loading)
                        {
                            self.learning_cursor_line = snapshot.line;
                            self.schedule_learning_refresh(
                                sender.clone(),
                                Duration::from_millis(40),
                            );
                        }
                    } else {
                        self.debugger_learning_hint = None;
                    }
                    self.debugger_snapshot = Some(snapshot);
                }
                ExecutionEvent::Finished { result, stopped } => {
                    self.cancel_file_selection(execution_id);
                    self.audio_host.stop_all();
                    self.graphics_host.finish();
                    self.game_preview_active = false;
                    let finished_document_id = self.execution_document_id;
                    let debugger_run = self.debugger_document_id == finished_document_id;
                    let source_matches_execution = finished_document_id
                        .and_then(|id| self.documents.iter().find(|document| document.id == id))
                        .is_some_and(|document| {
                            self.execution_source_fingerprint
                                == Some(Self::source_fingerprint(&document.contents))
                        });
                    self.active_execution_id = None;
                    self.debugging_active = false;
                    self.execution_document_id = None;
                    self.execution_line = None;
                    self.execution_project_root = None;
                    self.debugger_document_id = None;
                    self.execution_source_fingerprint = None;
                    self.execution_input = None;
                    self.execution_control = None;
                    self.execution_cancellable = None;
                    self.input_requested = false;
                    self.input_prompt = None;
                    match result {
                        Ok(()) => {
                            self.execution_diagnostics.clear();
                            self.execution_diagnostics_document_id = None;
                            self.output_status = if stopped {
                                self.text("Execução interrompida.").to_string()
                            } else {
                                self.text("Execução concluída.").to_string()
                            };
                        }
                        Err(diagnostics) => {
                            let diagnostic_text = format_diagnostics(&diagnostics);
                            self.problem_text.clone_from(&diagnostic_text);
                            self.debugger_error = debugger_run;
                            self.output_status =
                                self.text("O programa terminou com um erro.").to_string();
                            if source_matches_execution {
                                self.execution_diagnostics = diagnostics;
                                self.execution_diagnostics_document_id = finished_document_id;
                                self.schedule_learning_refresh(
                                    sender.clone(),
                                    Duration::from_millis(20),
                                );
                            } else {
                                self.execution_diagnostics.clear();
                                self.execution_diagnostics_document_id = None;
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_execution_lost(&mut self, execution_id: u64) {
        if execution_reply_is_current(execution_id, self.active_execution_id) {
            self.cancel_file_selection(execution_id);
            self.audio_host.stop_all();
            self.graphics_host.finish();
            self.game_preview_active = false;
            self.active_execution_id = None;
            self.debugging_active = false;
            self.execution_document_id = None;
            self.execution_line = None;
            self.execution_project_root = None;
            self.debugger_document_id = None;
            self.execution_source_fingerprint = None;
            self.execution_input = None;
            self.execution_control = None;
            self.execution_cancellable = None;
            self.input_requested = false;
            self.input_prompt = None;
            self.debugger_snapshot = None;
            self.debugger_learning_hint = None;
            self.problem_text = self
                .text("O interpretador foi encerrado antes de concluir o programa.")
                .to_string();
            self.output_status = self
                .text("Não foi possível concluir a execução.")
                .to_string();
        }
    }
}

impl SimpleComponent for MainWindow {
    type Init = ();
    type Input = MainWindowMsg;
    type Output = ();
    type Root = gtk::ApplicationWindow;
    type Widgets = MainUi;

    fn init_root() -> Self::Root {
        gtk::ApplicationWindow::default()
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        root.set_title(Some("Potoo IDE"));
        root.set_default_size(980, 640);
        root.add_css_class("potoo-window");
        let preferences = load_preferences();
        let saved_session = load_session();
        let unreadable_session = session_file().exists() && saved_session.is_none();
        let recovering_session = saved_session
            .as_ref()
            .is_some_and(|session| !session.clean_exit);
        let restored_root = saved_session
            .as_ref()
            .filter(|_| recovering_session)
            .and_then(|session| {
                session
                    .project_root
                    .as_ref()
                    .filter(|path| path.is_dir())
                    .cloned()
                    .or_else(|| {
                        let document_parent = |document: &SessionDocument| {
                            document
                                .path
                                .as_deref()
                                .and_then(Path::parent)
                                .filter(|path| path.is_dir())
                                .map(Path::to_path_buf)
                        };
                        session
                            .documents
                            .get(session.active_index)
                            .and_then(document_parent)
                            .or_else(|| session.documents.iter().find_map(document_parent))
                    })
            });
        let mut recent_paths = load_recent_paths();
        if recent_paths.is_empty() {
            if let Some(session) = &saved_session {
                let migration_paths = session.project_root.iter().chain(
                    session
                        .documents
                        .iter()
                        .filter_map(|document| document.path.as_ref()),
                );
                for path in migration_paths {
                    if let Some(path) = normalize_recent_path(path.clone()) {
                        if !recent_paths.contains(&path) {
                            recent_paths.push(path);
                        }
                    }
                }
                recent_paths.truncate(MAX_RECENT_ITEMS);
                if !recent_paths.is_empty() {
                    let _ = persist_recent_paths(&recent_paths);
                }
            }
        }
        let (documents, active_document_id, recovered_changes) = if recovering_session {
            saved_session
                .map(restore_session)
                .unwrap_or_else(|| (Vec::new(), None, false))
        } else {
            (Vec::new(), None, false)
        };
        let start_page_visible =
            !recovering_session || (restored_root.is_none() && documents.is_empty());
        if !start_page_visible {
            root.maximize();
        }
        let project_files = Vec::new();
        let project_name = restored_root
            .as_deref()
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| tr(preferences.ui_language, "Meu primeiro programa").to_string());
        let initial_dark = theme_is_dark(preferences.theme);
        if let Some(settings) = gtk::Settings::default() {
            let sender_for_system_theme = sender.clone();
            settings.connect_notify_local(
                Some("gtk-interface-color-scheme"),
                move |settings, _| {
                    let dark = settings
                        .property::<gtk::InterfaceColorScheme>("gtk-interface-color-scheme")
                        == gtk::InterfaceColorScheme::Dark;
                    sender_for_system_theme.input(MainWindowMsg::SystemThemeChanged(dark));
                },
            );
        }
        let learning_worker = LearningAnalysisWorker::new(sender.clone());

        let mut model = Self {
            window: root.clone(),
            audio_host: crate::audio::AudioHost::default(),
            graphics_host: {
                let sender = sender.clone();
                crate::graphics::GraphicsHost::new(root.clone(), move || {
                    sender.input(MainWindowMsg::StopDebug);
                })
            },
            new_file_language: preferences.default_language,
            next_document_id: documents.len() as u64 + 1,
            documents,
            active_document_id,
            project_root: restored_root,
            project_name,
            project_files,
            project_generation: 0,
            project_scan_generation: 0,
            pending_project: None,
            pending_project_scan: None,
            approved_project_switch: None,
            project_load_pending: false,
            project_refresh_needed: false,
            pending_close_document_id: None,
            pending_close_after_save_id: None,
            pending_save_as_id: None,
            pending_ui_language: None,
            pending_language_save_as_id: None,
            reopen_settings_after_language_restart: false,
            pending_close_app: false,
            close_app_ready: false,
            ui_language: preferences.ui_language,
            preferred_ui_language: preferences.ui_language,
            theme: preferences.theme,
            dark: initial_dark,
            font_size: preferences.font_size,
            line_numbers_visible: preferences.line_numbers_visible,
            word_wrap: preferences.word_wrap,
            indent_width: preferences.indent_width,
            recovery_generation: 0,
            settings_visible: false,
            output_text: tr(
                preferences.ui_language,
                "A saída aparecerá aqui após a execução.",
            )
            .to_string(),
            output_status: if recovered_changes {
                tr(
                    preferences.ui_language,
                    "Alterações da sessão anterior recuperadas.",
                )
                .to_string()
            } else if unreadable_session {
                tr(
                    preferences.ui_language,
                    "Não foi possível ler a sessão anterior. Os arquivos originais continuam intactos.",
                )
                .to_string()
            } else {
                tr(preferences.ui_language, "Pronto para executar").to_string()
            },
            problem_text: tr(preferences.ui_language, "Nenhum problema detectado").to_string(),
            game_preview_active: false,
            next_execution_id: 1,
            active_execution_id: None,
            debugging_active: false,
            execution_document_id: None,
            execution_line: None,
            execution_project_root: None,
            debugger_document_id: None,
            execution_input: None,
            execution_control: None,
            execution_cancellable: None,
            file_selection_cancellable: None,
            input_requested: false,
            input_prompt: None,
            debugger_snapshot: None,
            debugger_error: false,
            debugger_learning_hint: None,
            execution_diagnostics: Vec::new(),
            execution_diagnostics_document_id: None,
            execution_source_fingerprint: None,
            learning_generation: 0,
            learning_revision: 0,
            learning_cursor_line: 1,
            learning_document_id: None,
            learning_language: None,
            learning_hint: None,
            learning_loading: false,
            right_panel_section: RightPanelSection::Learning,
            right_panel_collapsed: false,
            learning_worker,
            recent_paths,
            start_page_visible,
        };
        let project_files_for_new_items = Rc::new(RefCell::new(model.project_files.clone()));
        let mut widgets = build_window_content(
            root.clone(),
            model.project_root.clone(),
            model.start_page_visible,
            &model.recent_paths,
            project_files_for_new_items,
            sender.clone(),
            preferences,
            initial_dark,
        );
        model
            .graphics_host
            .set_jerimum_preview_area(widgets.game_preview_area.clone());
        root.set_child(Some(&widgets.content));
        if let Some(project_root) = model.project_root.clone() {
            model.schedule_project_scan(project_root, false, sender.clone());
        }
        content_panel::rebuild_editor_tabs(
            &widgets.editor_tabs,
            &model.documents,
            model.active_document_id,
            model.ui_language,
            sender.clone(),
        );
        sender.input(MainWindowMsg::CursorMoved(1));
        let recent_preview = &model.recent_paths[..model.recent_paths.len().min(2)];
        widgets
            .start_page
            .recent_section
            .set_visible(!model.recent_paths.is_empty());
        start_page::rebuild_recent_items(
            &widgets.start_page.recent_list,
            recent_preview,
            model.ui_language,
            sender.clone(),
        );
        start_page::rebuild_recent_items(
            &widgets.start_page.all_recent_list,
            &model.recent_paths,
            model.ui_language,
            sender,
        );
        widgets
            .start_page
            .recent_more_button
            .set_visible(model.recent_paths.len() > 2);
        widgets.displayed_tabs = model
            .documents
            .iter()
            .map(|document| {
                (
                    document.id,
                    document.name.clone(),
                    document.is_dirty(),
                    Some(document.id) == model.active_document_id,
                )
            })
            .collect();

        if !unreadable_session {
            if let Err(error) = persist_session(&model, false, false) {
                model.output_status = format!(
                    "{}: {error}",
                    tr(preferences.ui_language, "Não foi possível salvar a sessão")
                );
            }
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        let persist_after = matches!(
            &message,
            MainWindowMsg::ProjectSelected(_)
                | MainWindowMsg::RecentSelected(_)
                | MainWindowMsg::ProjectSwitchDecision(_)
                | MainWindowMsg::ProjectScanCompleted { .. }
                | MainWindowMsg::DocumentReadCompleted { .. }
                | MainWindowMsg::CreateProject(_, _, _)
                | MainWindowMsg::CreateFile(_, _, _)
                | MainWindowMsg::CreateFolder(_, _)
                | MainWindowMsg::RenameProjectItemConfirmed(_, _)
                | MainWindowMsg::DeleteProjectItemConfirmed(_)
                | MainWindowMsg::OpenFile(_)
                | MainWindowMsg::ActivateDocument(_)
                | MainWindowMsg::CloseDocument(_)
                | MainWindowMsg::CloseDocumentDecision(_, _)
                | MainWindowMsg::SaveAndCloseDocument(_)
                | MainWindowMsg::SaveCurrent
                | MainWindowMsg::SaveAsSelected(_, _)
        );
        match message {
            MainWindowMsg::InterfaceLanguageSelected(language) => {
                if self.ui_language != language
                    && self.preferred_ui_language != language
                    && self.pending_ui_language.is_none()
                    && self.pending_save_as_id.is_none()
                    && self.pending_close_after_save_id.is_none()
                {
                    self.pending_ui_language = Some(language);
                    self.reopen_settings_after_language_restart = self.settings_visible;
                    self.settings_visible = false;
                    self.finish_language_change(sender.clone());
                }
            }
            MainWindowMsg::ProjectSelected(path) => {
                if self.documents.iter().any(SourceDocument::is_dirty) {
                    self.pending_project = Some(path);
                    self.pending_project_scan = None;
                    self.approved_project_switch = None;
                } else {
                    self.load_project(path, false, sender.clone());
                }
            }
            MainWindowMsg::RecentSelected(path) => {
                if let Some(path) = normalize_recent_path(path.clone()) {
                    if path.is_dir() {
                        if self.documents.iter().any(SourceDocument::is_dirty) {
                            self.pending_project = Some(path);
                            self.pending_project_scan = None;
                            self.approved_project_switch = None;
                        } else {
                            self.load_project(path, false, sender.clone());
                        }
                    } else {
                        self.open_document(path, sender.clone());
                    }
                } else {
                    self.output_status = match self.forget_recent(&path) {
                        Ok(()) => self
                            .text("O arquivo ou a pasta recente não está mais disponível.")
                            .to_string(),
                        Err(error) => format!(
                            "{}: {error}",
                            self.text("Não foi possível atualizar programas recentes")
                        ),
                    };
                }
            }
            MainWindowMsg::RecentRemoved(path) => {
                if let Err(error) = self.forget_recent(&path) {
                    self.output_status = format!(
                        "{}: {error}",
                        self.text("Não foi possível atualizar programas recentes")
                    );
                }
            }
            MainWindowMsg::ProjectSwitchDecision(discard_changes) => {
                if discard_changes {
                    if let Some(path) = self.pending_project.take() {
                        if let Some((scanned_path, files)) = self.pending_project_scan.take()
                            && scanned_path == path
                        {
                            let refresh_after = std::mem::take(&mut self.project_refresh_needed);
                            self.approved_project_switch = None;
                            self.apply_loaded_project(path.clone(), files, sender.clone());
                            if refresh_after {
                                self.refresh_project_scan(path, sender.clone());
                            }
                        } else {
                            self.load_project(path, true, sender.clone());
                        }
                    }
                } else {
                    self.pending_project = None;
                    self.pending_project_scan = None;
                    self.approved_project_switch = None;
                    self.project_refresh_needed = false;
                }
            }
            MainWindowMsg::ProjectScanCompleted {
                project_generation,
                scan_generation,
                root,
                load_project,
                result,
            } => self.finish_project_scan(
                project_generation,
                scan_generation,
                root,
                load_project,
                result,
                sender.clone(),
            ),
            MainWindowMsg::DocumentReadCompleted {
                project_generation,
                path,
                standalone,
                result,
            } => self.finish_document_read(
                project_generation,
                path,
                standalone,
                result,
                sender.clone(),
            ),
            MainWindowMsg::CreateProject(parent, name, template) => {
                self.create_project(parent, name, template, sender.clone());
            }
            MainWindowMsg::CreateFile(name, language, relative_folder) => {
                self.create_project_file(name, language, relative_folder, sender.clone())
            }
            MainWindowMsg::CreateFolder(name, relative_folder) => {
                self.create_project_folder(name, relative_folder, sender.clone())
            }
            MainWindowMsg::RenameProjectItem(path) => match self.validate_project_item(&path) {
                Ok(is_directory) => show_rename_project_item_dialog(
                    &self.window,
                    path,
                    is_directory,
                    self.ui_language,
                    self.dark,
                    sender.clone(),
                ),
                Err(error) => self.output_status = error,
            },
            MainWindowMsg::RenameProjectItemConfirmed(path, name) => {
                self.rename_project_item(path, name, sender.clone());
            }
            MainWindowMsg::DeleteProjectItem(path) => match self.validate_project_item(&path) {
                Ok(is_directory) => {
                    let has_unsaved_changes = self.documents.iter().any(|document| {
                        document.is_dirty()
                            && document.path.as_deref().is_some_and(|document_path| {
                                document_path == path
                                    || (is_directory && document_path.starts_with(&path))
                            })
                    });
                    let sender_for_delete = sender.clone();
                    let path_for_delete = path.clone();
                    show_potoo_confirmation(
                        &self.window,
                        self.ui_language,
                        self.dark,
                        "Excluir item do projeto",
                        "Deseja excluir este item do projeto?",
                        if has_unsaved_changes {
                            "Há alterações não salvas em arquivos deste item; elas serão perdidas."
                        } else {
                            "O item e todo o seu conteúdo serão excluídos permanentemente."
                        },
                        vec!["Cancelar", "Excluir"],
                        Some(1),
                        Some(1),
                        move |response| {
                            if response == 1 {
                                sender_for_delete.input(MainWindowMsg::DeleteProjectItemConfirmed(
                                    path_for_delete.clone(),
                                ));
                            }
                        },
                    );
                }
                Err(error) => self.output_status = error,
            },
            MainWindowMsg::DeleteProjectItemConfirmed(path) => {
                self.delete_project_item(path, sender.clone());
            }
            MainWindowMsg::OpenFile(path) => self.open_document(path, sender.clone()),
            MainWindowMsg::ActivateDocument(id) => {
                if self.documents.iter().any(|doc| doc.id == id) {
                    self.active_document_id = Some(id);
                    if self.debugger_document_id == Some(id)
                        && let Some(snapshot) = &self.debugger_snapshot
                    {
                        self.learning_cursor_line = snapshot.line;
                    }
                }
            }
            MainWindowMsg::CloseDocument(id) => {
                if self
                    .documents
                    .iter()
                    .find(|document| document.id == id)
                    .is_some_and(SourceDocument::is_dirty)
                {
                    self.pending_close_document_id = Some(id);
                } else {
                    self.close_document(id);
                }
            }
            MainWindowMsg::CloseDocumentDecision(id, discard_changes) => {
                if self.pending_close_document_id == Some(id) {
                    self.pending_close_document_id = None;
                    if discard_changes {
                        self.close_document(id);
                    }
                }
            }
            MainWindowMsg::SaveAndCloseDocument(id) => {
                if self.pending_close_document_id == Some(id) {
                    self.pending_close_document_id = None;
                    if let Some(path) = self
                        .documents
                        .iter()
                        .find(|document| document.id == id)
                        .and_then(|document| document.path.clone())
                    {
                        self.save_document(id, path, sender.clone());
                        if self
                            .documents
                            .iter()
                            .find(|document| document.id == id)
                            .is_some_and(|document| !document.is_dirty())
                        {
                            self.close_document(id);
                        }
                    } else {
                        self.pending_close_after_save_id = Some(id);
                        self.pending_save_as_id = Some(id);
                    }
                }
            }
            MainWindowMsg::SaveCurrent => self.request_save(sender.clone()),
            MainWindowMsg::SaveAsSelected(id, path) => {
                let language_save = self.pending_language_save_as_id == Some(id);
                self.pending_save_as_id = None;
                let saved = self.save_document(id, path, sender.clone());
                if language_save {
                    self.pending_language_save_as_id = None;
                    if saved {
                        self.finish_language_change(sender.clone());
                    } else {
                        self.abort_language_change(false);
                    }
                }
                if self.pending_close_after_save_id == Some(id) {
                    self.pending_close_after_save_id = None;
                    if self
                        .documents
                        .iter()
                        .find(|document| document.id == id)
                        .is_some_and(|document| !document.is_dirty())
                    {
                        self.close_document(id);
                    }
                }
            }
            MainWindowMsg::SaveAsCanceled(id) => {
                let language_save = self.pending_language_save_as_id == Some(id);
                if self.pending_save_as_id == Some(id) {
                    self.pending_save_as_id = None;
                }
                if language_save {
                    self.abort_language_change(true);
                }
                if self.pending_close_after_save_id == Some(id) {
                    self.pending_close_after_save_id = None;
                }
            }
            MainWindowMsg::FileOperationFailed(message) => self.output_status = message,
            MainWindowMsg::RequestClose => {
                if self.documents.iter().any(SourceDocument::is_dirty) {
                    self.pending_close_app = true;
                } else {
                    self.close_app_ready = true;
                    self.persist_session(true, false);
                }
            }
            MainWindowMsg::CloseAppDecision(discard_changes) => {
                self.pending_close_app = false;
                self.close_app_ready = discard_changes;
                if discard_changes {
                    self.persist_session(true, true);
                }
            }
            MainWindowMsg::ThemeSelected(theme) => {
                self.theme = theme;
                self.dark = theme_is_dark(theme);
                self.save_preferences();
            }
            MainWindowMsg::SystemThemeChanged(dark) => {
                if self.theme == ThemePreference::System {
                    self.dark = dark;
                }
            }
            MainWindowMsg::FontSizeSelected(font_size) => {
                self.font_size = font_size;
                self.save_preferences();
            }
            MainWindowMsg::LineNumbersChanged(visible) => {
                self.line_numbers_visible = visible;
                self.save_preferences();
            }
            MainWindowMsg::WordWrapChanged(enabled) => {
                self.word_wrap = enabled;
                self.save_preferences();
            }
            MainWindowMsg::IndentWidthSelected(width) => {
                self.indent_width = if width == 2 { 2 } else { 4 };
                self.save_preferences();
            }
            MainWindowMsg::OpenSettings => self.settings_visible = true,
            MainWindowMsg::CloseSettings => self.settings_visible = false,
            MainWindowMsg::Run => {
                if self.active_execution_id.is_some() {
                    self.send_execution_control(ControlAction::Stop);
                    self.output_status = self.text("Interrompendo execução…").to_string();
                } else {
                    self.start_execution(sender.clone(), false);
                }
            }
            MainWindowMsg::StartDebug => {
                if self.active_execution_id.is_some() {
                    self.output_status = self
                        .text("Interrompa a execução atual antes de iniciar a depuração.")
                        .to_string();
                } else {
                    self.start_execution(sender.clone(), true);
                }
            }
            MainWindowMsg::StepDebug => {
                self.send_execution_control(ControlAction::Step);
                self.debugger_snapshot = None;
            }
            MainWindowMsg::StopDebug => {
                self.send_execution_control(ControlAction::Stop);
                self.debugger_snapshot = None;
            }
            MainWindowMsg::ExecutionEvent(execution_id, event) => {
                self.handle_execution_event(execution_id, event, sender.clone());
            }
            MainWindowMsg::ExecutionLost(execution_id) => {
                self.handle_execution_lost(execution_id);
            }
            MainWindowMsg::FileSelectionResolved(execution_id, selection) => {
                if execution_reply_is_current(execution_id, self.active_execution_id) {
                    if self.file_selection_cancellable.as_ref().is_some_and(
                        |(picker_execution_id, _)| *picker_execution_id == execution_id,
                    ) {
                        self.file_selection_cancellable = None;
                    }
                    if self.input_requested {
                        let input = selection.unwrap_or_else(|error| {
                            self.output_status = error;
                            String::new()
                        });
                        if self
                            .execution_input
                            .as_ref()
                            .is_some_and(|input_sender| input_sender.send(input).is_ok())
                        {
                            self.input_requested = false;
                            self.input_prompt = None;
                        } else {
                            self.output_status = self
                                .text("Não foi possível enviar a entrada ao programa.")
                                .to_string();
                        }
                    }
                }
            }
            MainWindowMsg::SubmitProgramInput(input) => {
                if self.input_requested {
                    if self
                        .execution_input
                        .as_ref()
                        .is_some_and(|sender| sender.send(input).is_ok())
                    {
                        self.input_requested = false;
                        self.input_prompt = None;
                    } else {
                        self.output_status = self
                            .text("Não foi possível enviar a entrada ao programa.")
                            .to_string();
                    }
                }
            }
            MainWindowMsg::SourceChanged(contents) => {
                if let Some(id) = self.active_document_id {
                    if let Some(document) = self.documents.iter_mut().find(|doc| doc.id == id) {
                        document.contents = contents;
                    }
                }
                self.learning_revision = self.learning_revision.wrapping_add(1);
                self.execution_diagnostics.clear();
                self.execution_diagnostics_document_id = None;
                self.schedule_learning_refresh(sender.clone(), Duration::from_millis(280));
                self.recovery_generation += 1;
                let generation = self.recovery_generation;
                let sender_for_recovery = sender.clone();
                gtk::glib::timeout_add_local_once(Duration::from_millis(800), move || {
                    sender_for_recovery.input(MainWindowMsg::PersistRecovery(generation));
                });
            }
            MainWindowMsg::CursorMoved(line) => {
                let line = line.max(1);
                if self.learning_cursor_line != line {
                    self.learning_cursor_line = line;
                    self.schedule_learning_refresh(sender.clone(), Duration::from_millis(100));
                }
            }
            MainWindowMsg::SelectRightPanel(section) => {
                if self.right_panel_collapsed {
                    self.right_panel_section = section;
                    self.right_panel_collapsed = false;
                } else if self.right_panel_section == section {
                    self.right_panel_collapsed = true;
                } else {
                    self.right_panel_section = section;
                }
            }
            MainWindowMsg::LearningDebounceElapsed(generation) => {
                if generation == self.learning_generation {
                    if let Some(document) = self.active_document().cloned() {
                        let line = self.learning_cursor_line;
                        let revision = self.learning_revision;
                        let runtime_diagnostics =
                            if self.execution_diagnostics_document_id == Some(document.id) {
                                self.execution_diagnostics.clone()
                            } else {
                                Vec::new()
                            };
                        self.learning_worker.submit(LearningAnalysisRequest {
                            generation,
                            document_id: document.id,
                            revision,
                            line,
                            contents: document.contents,
                            language: document.language,
                            runtime_diagnostics,
                        });
                    } else {
                        self.learning_loading = false;
                    }
                }
            }
            MainWindowMsg::LearningAnalysisReady {
                generation,
                document_id,
                revision,
                line,
                language,
                hint,
            } => {
                let current = CurrentLearningAnalysis {
                    generation: self.learning_generation,
                    active_document_id: self.active_document_id,
                    learning_document_id: self.learning_document_id,
                    revision: self.learning_revision,
                    line: self.learning_cursor_line,
                    language: self.learning_language,
                };
                let result = LearningAnalysisIdentity {
                    generation,
                    document_id,
                    revision,
                    line,
                    language,
                };
                if current.accepts(result) {
                    if self.debugger_document_id == Some(document_id)
                        && self.active_document_id == Some(document_id)
                        && self
                            .debugger_snapshot
                            .as_ref()
                            .is_some_and(|snapshot| snapshot.line == line)
                    {
                        self.debugger_learning_hint = Some(hint.clone());
                    }
                    self.learning_hint = Some(hint);
                    self.learning_loading = false;
                }
            }
            MainWindowMsg::PersistRecovery(generation) => {
                if generation == self.recovery_generation {
                    self.persist_session(false, false);
                }
            }
        }

        let active_language = self.active_document().map(|document| document.language);
        if self.learning_document_id != self.active_document_id
            || self.learning_language != active_language
        {
            self.learning_revision = self.learning_revision.wrapping_add(1);
            self.schedule_learning_refresh(sender.clone(), Duration::from_millis(80));
        }

        if persist_after {
            self.persist_session(false, false);
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        let ui = widgets;
        ui.project_root_for_new_items
            .replace(self.project_root.clone());
        ui.project_files_for_new_items
            .replace(self.project_files.clone());

        let active_document = self.active_document();
        let active_language = active_document.map(|document| document.language);
        let game_preview_available = active_language == Some(Language::Potigol);
        ui.game_preview_button.set_visible(game_preview_available);
        let right_panel_section = if !game_preview_available
            && self.right_panel_section == RightPanelSection::GamePreview
        {
            RightPanelSection::Learning
        } else {
            self.right_panel_section
        };

        if ui.displayed_right_panel_section != Some(right_panel_section)
            || ui.displayed_right_panel_collapsed != self.right_panel_collapsed
        {
            let collapsed = self.right_panel_collapsed;
            let section = right_panel_section;
            ui.right_panel_stack.set_visible_child_name(match section {
                RightPanelSection::Learning => "learning",
                RightPanelSection::Debugger => "debugger",
                RightPanelSection::GamePreview => "game-preview",
            });
            ui.right_panel_revealer.set_reveal_child(!collapsed);

            let learning_selected = section == RightPanelSection::Learning;
            let debugger_selected = section == RightPanelSection::Debugger;
            let game_preview_selected = section == RightPanelSection::GamePreview;
            ui.learning_tool_button
                .set_css_classes(if learning_selected {
                    &["potoo-tool-rail-button", "potoo-tool-rail-button-selected"]
                } else {
                    &["potoo-tool-rail-button"]
                });
            ui.debugger_tool_button
                .set_css_classes(if debugger_selected {
                    &["potoo-tool-rail-button", "potoo-tool-rail-button-selected"]
                } else {
                    &["potoo-tool-rail-button"]
                });
            ui.game_preview_button
                .set_css_classes(if game_preview_selected {
                    &["potoo-tool-rail-button", "potoo-tool-rail-button-selected"]
                } else {
                    &["potoo-tool-rail-button"]
                });
            ui.learning_tool_button.set_tooltip_text(Some(self.text(
                if learning_selected && !collapsed {
                    "Recolher Aprenda enquanto programa"
                } else {
                    "Abrir Aprenda enquanto programa"
                },
            )));
            ui.debugger_tool_button.set_tooltip_text(Some(self.text(
                if section == RightPanelSection::Debugger && !collapsed {
                    "Recolher depurador"
                } else {
                    "Abrir depurador"
                },
            )));
            ui.game_preview_button.set_tooltip_text(Some(self.text(
                if game_preview_selected && !collapsed {
                    "Recolher prévia do jogo"
                } else {
                    "Abrir prévia do jogo"
                },
            )));
            if game_preview_selected && !collapsed {
                ui.game_preview_area.grab_focus();
            }

            ui.displayed_right_panel_section = Some(section);
            ui.displayed_right_panel_collapsed = collapsed;
        }

        if ui.displayed_game_preview_active != self.game_preview_active {
            ui.game_preview_stack
                .set_visible_child_name(if self.game_preview_active {
                    "canvas"
                } else {
                    "idle"
                });
            ui.displayed_game_preview_active = self.game_preview_active;
        }

        let active_document_id = active_document.map(|document| document.id);
        let current_learning_hint = if self.learning_loading {
            ui.displayed_learning_hint.clone()
        } else {
            self.learning_hint.clone()
        };
        if ui.displayed_learning_hint != current_learning_hint {
            if let Some(hint) = &current_learning_hint {
                let topic_name = learning_topic_label(hint.topic, self.ui_language);
                ui.learning_line_title.set_text(&format!(
                    "{} {} · {topic_name}",
                    self.text("Linha"),
                    hint.line
                ));
                let (explanation, concept, experiment) = learning_copy(hint, self.ui_language);
                ui.learning_explanation.set_text(&explanation);
                ui.learning_experiment.set_text(&experiment);
                ui.learning_concept_title
                    .set_text(&format!("{} {topic_name}", self.text("Conceito:")));
                ui.learning_concept_text.set_text(&concept);
                content_panel::show_learning_problems(
                    &ui.learning_problem_list,
                    &ui.learning_problem_empty,
                    &hint.problems,
                    self.ui_language,
                    &ui.code_buffer,
                    &ui.code_view,
                );
            } else {
                ui.learning_line_title
                    .set_text(self.text("Linha 1 · programa"));
                ui.learning_explanation.set_text(
                    self.text(
                        "Selecione uma linha para ver uma explicação contextual do programa.",
                    ),
                );
                ui.learning_experiment.set_text(
                    self.text(
                        "Mova o cursor para uma condição, um laço, uma entrada ou uma saída.",
                    ),
                );
                ui.learning_concept_title
                    .set_text(self.text("Conceito: programa"));
                ui.learning_concept_text.set_text(
                    self.text(
                        "O painel acompanha a linha selecionada e o conteúdo atual do editor.",
                    ),
                );
                content_panel::show_learning_problems(
                    &ui.learning_problem_list,
                    &ui.learning_problem_empty,
                    &[],
                    self.ui_language,
                    &ui.code_buffer,
                    &ui.code_view,
                );
            }
            ui.displayed_learning_hint
                .clone_from(&current_learning_hint);
        }
        if ui.displayed_learning_loading != self.learning_loading {
            ui.learning_spinner.set_spinning(self.learning_loading);
            ui.learning_spinner.set_visible(self.learning_loading);
            ui.learning_analysis_stack
                .set_visible_child_name(if self.learning_loading {
                    "loading"
                } else {
                    "content"
                });
            ui.displayed_learning_loading = self.learning_loading;
        }
        let active_document_changed = ui.displayed_active_document_id != active_document_id;
        if active_document_changed {
            ui.code_buffer.set_text(
                active_document
                    .map(|document| document.contents.as_str())
                    .unwrap_or_default(),
            );
            ui.displayed_active_document_id = active_document_id;
        }
        if active_document_changed || ui.displayed_execution_line != self.execution_line {
            if self.execution_document_id == self.active_document_id
                && let Some(line) = self.execution_line
                && let Some(iter) = ui.code_buffer.iter_at_line(line.saturating_sub(1) as i32)
            {
                ui.code_buffer.place_cursor(&iter);
            }
            ui.displayed_execution_line = self.execution_line;
        }
        if ui.displayed_language != active_language {
            ui.status_language
                .set_text(active_language.map(Language::label).unwrap_or(""));
            ui.status_language.set_visible(active_language.is_some());
            ui.displayed_language = active_language;
        }
        ui.default_new_language.set(self.new_file_language);

        if ui.displayed_indent_width != self.indent_width {
            ui.indent_width.set(self.indent_width);
            ui.settings
                .indent_width_picker
                .set_selected(if self.indent_width == 2 { 0 } else { 1 });
            ui.status_details.set_text(&format!(
                "UTF-8  ·  {}: {}",
                tr(self.ui_language, "Espaços"),
                self.indent_width
            ));
            ui.displayed_indent_width = self.indent_width;
        }

        let displayed_preferred_ui_language = self
            .pending_ui_language
            .unwrap_or(self.preferred_ui_language);
        if ui.displayed_preferred_ui_language != displayed_preferred_ui_language {
            ui.settings
                .ui_language_picker
                .set_selected(displayed_preferred_ui_language.selected_index());
            ui.displayed_preferred_ui_language = displayed_preferred_ui_language;
        }

        if ui.displayed_dark != self.dark {
            ui.dark_theme.set(self.dark);
            if self.dark {
                ui.window.add_css_class("potoo-dark");
                ui.settings.window.add_css_class("potoo-dark");
            } else {
                ui.window.remove_css_class("potoo-dark");
                ui.settings.window.remove_css_class("potoo-dark");
            }
            content_panel::set_syntax_palette(&ui.syntax_tags, self.dark);
            ui.displayed_dark = self.dark;
        }

        if ui.displayed_theme != self.theme {
            select_theme_button(&ui.settings.theme_buttons, self.theme.selected_index());
            ui.displayed_theme = self.theme;
        }
        if ui.displayed_font_size != self.font_size {
            set_font_size(&ui.code_view, ui.displayed_font_size, self.font_size);
            set_font_size(&ui.line_numbers, ui.displayed_font_size, self.font_size);
            ui.settings
                .font_size_picker
                .set_selected(font_size_index(self.font_size));
            ui.displayed_font_size = self.font_size;
        }
        if ui.displayed_line_numbers_visible != self.line_numbers_visible {
            ui.line_numbers.set_visible(self.line_numbers_visible);
            ui.line_numbers_separator
                .set_visible(self.line_numbers_visible);
            ui.settings
                .line_numbers_switch
                .set_active(self.line_numbers_visible);
            ui.displayed_line_numbers_visible = self.line_numbers_visible;
        }
        if ui.displayed_word_wrap != self.word_wrap {
            ui.code_view.set_wrap_mode(if self.word_wrap {
                gtk::WrapMode::WordChar
            } else {
                gtk::WrapMode::None
            });
            ui.settings.word_wrap_switch.set_active(self.word_wrap);
            ui.displayed_word_wrap = self.word_wrap;
        }
        if ui.displayed_settings_visible != self.settings_visible {
            if self.settings_visible {
                ui.settings.window.present();
            } else {
                ui.settings.window.set_visible(false);
            }
            ui.displayed_settings_visible = self.settings_visible;
        }

        if ui.displayed_start_page != self.start_page_visible {
            if ui.displayed_start_page && !self.start_page_visible {
                ui.window.maximize();
            }
            ui.main_stack
                .set_visible_child_name(if self.start_page_visible {
                    "start"
                } else {
                    "workspace"
                });
            ui.run_button.set_visible(!self.start_page_visible);
            ui.project_title.set_text(if self.start_page_visible {
                tr(self.ui_language, "Início")
            } else {
                &self.project_name
            });
            ui.displayed_start_page = self.start_page_visible;
        }

        if ui.displayed_project_name != self.project_name {
            if !self.start_page_visible {
                ui.project_title.set_text(&self.project_name);
            }
            ui.project_label.set_text(&self.project_name);
            ui.displayed_project_name.clone_from(&self.project_name);
        }

        if ui.displayed_recent_paths != self.recent_paths {
            let recent_preview = &self.recent_paths[..self.recent_paths.len().min(2)];
            ui.start_page
                .recent_section
                .set_visible(!self.recent_paths.is_empty());
            start_page::rebuild_recent_items(
                &ui.start_page.recent_list,
                recent_preview,
                self.ui_language,
                sender.clone(),
            );
            start_page::rebuild_recent_items(
                &ui.start_page.all_recent_list,
                &self.recent_paths,
                self.ui_language,
                sender.clone(),
            );
            ui.start_page
                .recent_more_button
                .set_visible(self.recent_paths.len() > 2);
            ui.displayed_recent_paths.clone_from(&self.recent_paths);
        }

        let active_path = active_document.and_then(|document| document.path.as_ref());
        let project_file_signature = self
            .project_files
            .iter()
            .map(|file| (file.path.clone(), active_path == Some(&file.path)))
            .collect::<Vec<_>>();
        let project_open = self.project_root.is_some();
        if ui.displayed_project_files != project_file_signature
            || ui.displayed_project_open != project_open
        {
            ui.expanded_folders.borrow_mut().retain(|expanded| {
                self.project_files
                    .iter()
                    .any(|item| item.is_directory && item.path == *expanded)
            });
            sidebar::rebuild_file_list(
                &ui.file_list,
                &self.project_files,
                active_path,
                project_open,
                self.project_root.as_deref(),
                &ui.empty_state,
                self.ui_language,
                &ui.expanded_folders,
                sender.clone(),
            );
            ui.displayed_project_files = project_file_signature;
            ui.displayed_project_open = project_open;
            ui.project_row.set_visible(project_open);
            ui.new_file_button.set_sensitive(project_open);
        }

        let tab_signature = self
            .documents
            .iter()
            .map(|document| {
                (
                    document.id,
                    document.name.clone(),
                    document.is_dirty(),
                    Some(document.id) == self.active_document_id,
                )
            })
            .collect::<Vec<_>>();
        if ui.displayed_tabs != tab_signature {
            content_panel::rebuild_editor_tabs(
                &ui.editor_tabs,
                &self.documents,
                self.active_document_id,
                self.ui_language,
                sender.clone(),
            );
            ui.displayed_tabs = tab_signature;
        }

        if ui.displayed_pending_project != self.pending_project {
            ui.displayed_pending_project
                .clone_from(&self.pending_project);
            if self.pending_project.is_some() {
                show_project_switch_confirmation(
                    &ui.window,
                    self.ui_language,
                    ui.dark_theme.get(),
                    sender.clone(),
                );
            }
        }

        if ui.displayed_pending_close_document_id != self.pending_close_document_id {
            ui.displayed_pending_close_document_id = self.pending_close_document_id;
            if let Some(id) = self.pending_close_document_id {
                show_close_document_confirmation(
                    &ui.window,
                    id,
                    self.ui_language,
                    ui.dark_theme.get(),
                    sender.clone(),
                );
            }
        }

        if ui.displayed_pending_close_app != self.pending_close_app {
            ui.displayed_pending_close_app = self.pending_close_app;
            if self.pending_close_app {
                show_close_app_confirmation(
                    &ui.window,
                    self.ui_language,
                    ui.dark_theme.get(),
                    sender.clone(),
                );
            }
        }

        if ui.displayed_pending_save_as_id != self.pending_save_as_id {
            ui.displayed_pending_save_as_id = self.pending_save_as_id;
            if let Some(id) = self.pending_save_as_id {
                if let Some(document) = self.documents.iter().find(|doc| doc.id == id) {
                    show_save_dialog(
                        &ui.window,
                        document,
                        self.project_root.as_deref(),
                        self.ui_language,
                        sender.clone(),
                    );
                }
            }
        }

        if self.close_app_ready && !ui.displayed_close_app_ready {
            ui.displayed_close_app_ready = true;
            ui.close_authorized.set(true);
            ui.window.close();
        }

        if ui.displayed_output_text != self.output_text {
            ui.output_buffer.set_text(&self.output_text);
            if self.output_text == self.text("A saída aparecerá aqui após a execução.") {
                ui.output_view
                    .add_css_class("potoo-output-placeholder-text");
            } else {
                ui.output_view
                    .remove_css_class("potoo-output-placeholder-text");
            }
            ui.displayed_output_text.clone_from(&self.output_text);
        }
        if ui.displayed_output_status != self.output_status {
            ui.output_status.set_text(&self.output_status);
            ui.displayed_output_status.clone_from(&self.output_status);
        }
        if ui.displayed_problem_text != self.problem_text {
            ui.problems_buffer.set_text(&self.problem_text);
            let is_placeholder = self.problem_text == self.text("Nenhum problema detectado");
            if is_placeholder || self.problem_text.is_empty() {
                ui.problems_view
                    .add_css_class("potoo-output-placeholder-text");
            } else {
                ui.problems_view
                    .remove_css_class("potoo-output-placeholder-text");
            }
            if !self.problem_text.is_empty() && !is_placeholder {
                ui.problems_tab.emit_clicked();
            }
            ui.displayed_problem_text.clone_from(&self.problem_text);
        }
        if ui.displayed_input_requested != self.input_requested {
            ui.input_entry.set_visible(self.input_requested);
            if self.input_requested {
                if let Some(prompt) = &self.input_prompt {
                    let placeholder = match (&prompt.variable_name, &prompt.data_type) {
                        (Some(name), Some(data_type)) => format!("{} ({data_type})", name),
                        _ => self.text("Digite a entrada do programa").to_owned(),
                    };
                    ui.input_entry.set_placeholder_text(Some(&placeholder));
                }
                ui.input_entry.grab_focus();
            }
            ui.displayed_input_requested = self.input_requested;
        }
        ui.debugger_controls
            .set_visible_child_name(if self.debugging_active {
                "controls"
            } else {
                "start"
            });
        ui.debugger_start.set_tooltip_text(Some(self.text(
            if self.active_execution_id.is_some() {
                "Interrompa a execução atual antes de iniciar a depuração."
            } else {
                "Iniciar depuração"
            },
        )));
        ui.debugger_start
            .set_sensitive(self.active_execution_id.is_none());
        ui.debugger_step.set_sensitive(
            self.debugging_active
                && self.active_execution_id.is_some()
                && self.debugger_snapshot.is_some(),
        );
        ui.debugger_stop.set_visible(self.debugging_active);
        ui.debugger_stop
            .set_sensitive(self.debugging_active && self.active_execution_id.is_some());
        let debugger_error = self.debugger_error
            && self.active_document_id.is_some()
            && self.execution_diagnostics_document_id == self.active_document_id;
        if ui.displayed_debugger_snapshot != self.debugger_snapshot
            || ui.displayed_debugger_error != debugger_error
        {
            if let Some(snapshot) = &self.debugger_snapshot {
                let reason = match snapshot.pause_reason {
                    Some(execution::DebugPauseReason::Step) => self.text("passo a passo"),
                    Some(execution::DebugPauseReason::Pause) => self.text("pausa"),
                    Some(execution::DebugPauseReason::Debug) => self.text("ponto de depuração"),
                    None => self.text("depuração"),
                };
                ui.debugger_position.set_text(&format!(
                    "{} {} · {reason}",
                    self.text("Linha"),
                    snapshot.line
                ));
                let variables = if snapshot.variables.is_empty() {
                    self.text("Nenhuma variável visível.").to_owned()
                } else {
                    snapshot
                        .variables
                        .iter()
                        .map(|(name, value)| format!("{name} = {value}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                ui.debugger_variables.set_text(&variables);
                if self.execution_document_id == self.active_document_id
                    && let Some(iter) = ui
                        .code_buffer
                        .iter_at_line(snapshot.line.saturating_sub(1) as i32)
                {
                    ui.code_buffer.place_cursor(&iter);
                }
            } else if debugger_error {
                ui.debugger_position
                    .set_text(self.text("Depuração encerrada com erro"));
                ui.debugger_variables.set_text(&self.problem_text);
            } else {
                ui.debugger_position
                    .set_text(self.text("Aguardando depuração"));
                ui.debugger_variables.set_text(
                    self.text(
                        "Os valores das variáveis aparecerão durante a execução passo a passo.",
                    ),
                );
            }
            ui.displayed_debugger_snapshot
                .clone_from(&self.debugger_snapshot);
            ui.displayed_debugger_error = debugger_error;
        }
        let current_debugger_hint = self
            .debugger_learning_hint
            .as_ref()
            .filter(|hint| {
                self.debugger_document_id == self.active_document_id
                    && self
                        .debugger_snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.line == hint.line)
            })
            .cloned();
        if ui.displayed_debugger_learning_hint != current_debugger_hint {
            ui.debugger_learning
                .content_scroll
                .vadjustment()
                .set_value(0.0);
            if let Some(hint) = &current_debugger_hint {
                let topic_name = learning_topic_label(hint.topic, self.ui_language);
                ui.debugger_learning.line_title.set_text(&format!(
                    "{} {} · {topic_name}",
                    self.text("Linha"),
                    hint.line
                ));
                let (explanation, concept, _) = learning_copy(hint, self.ui_language);
                ui.debugger_learning.explanation.set_text(&explanation);
                ui.debugger_learning
                    .concept_title
                    .set_text(&format!("{} {topic_name}", self.text("Conceito:")));
                ui.debugger_learning.concept_text.set_text(&concept);
                content_panel::show_learning_problems(
                    &ui.debugger_learning.problem_list,
                    &ui.debugger_learning.problem_empty,
                    &hint.problems,
                    self.ui_language,
                    &ui.code_buffer,
                    &ui.code_view,
                );
            } else {
                ui.debugger_learning
                    .line_title
                    .set_text(self.text("Linha 1 · programa"));
                ui.debugger_learning.explanation.set_text("");
                ui.debugger_learning
                    .concept_title
                    .set_text(self.text("Conceito: programa"));
                ui.debugger_learning.concept_text.set_text("");
                content_panel::show_learning_problems(
                    &ui.debugger_learning.problem_list,
                    &ui.debugger_learning.problem_empty,
                    &[],
                    self.ui_language,
                    &ui.code_buffer,
                    &ui.code_view,
                );
            }
            ui.displayed_debugger_learning_hint
                .clone_from(&current_debugger_hint);
        }
        let debugger_analysis_state = if !self.debugging_active {
            "idle"
        } else if self.debugger_snapshot.is_none()
            || self.debugger_document_id != self.active_document_id
        {
            "waiting"
        } else if current_debugger_hint.is_some() {
            "content"
        } else {
            "loading"
        };
        if ui.displayed_debugger_analysis_state != debugger_analysis_state {
            let loading = matches!(debugger_analysis_state, "waiting" | "loading");
            ui.debugger_learning.spinner.set_spinning(loading);
            ui.debugger_learning.spinner.set_visible(loading);
            if loading {
                ui.debugger_learning.loading_label.set_text(self.text(
                    if debugger_analysis_state == "waiting" {
                        "Executando o programa…"
                    } else {
                        "Analisando linha…"
                    },
                ));
            }
            let idle = debugger_analysis_state == "idle";
            ui.debugger_learning.analysis_stack.set_visible(!idle);
            if !idle {
                ui.debugger_learning.analysis_stack.set_visible_child_name(
                    if debugger_analysis_state == "content" {
                        "content"
                    } else {
                        "loading"
                    },
                );
            }
            ui.displayed_debugger_analysis_state = debugger_analysis_state;
        }
        let execution_active = self.active_execution_id.is_some();
        if ui.displayed_run_active != execution_active
            || ui.displayed_run_language != self.ui_language
        {
            let (icon_name, label) = if execution_active && !self.debugging_active {
                ("potoo-stop-symbolic", "Parar")
            } else {
                ("potoo-play-symbolic", "Executar")
            };
            if execution_active && !self.debugging_active {
                ui.run_button.add_css_class("potoo-stop-button");
            } else {
                ui.run_button.remove_css_class("potoo-stop-button");
            }
            ui.run_icon.set_icon_name(Some(icon_name));
            ui.run_label.set_text(self.text(label));
            ui.run_button.set_tooltip_text(Some(self.text(
                if execution_active && !self.debugging_active {
                    "Parar"
                } else {
                    "Executar o programa atual"
                },
            )));
            ui.displayed_run_active = execution_active;
            ui.displayed_run_language = self.ui_language;
        }
        ui.run_button.set_sensitive(!self.debugging_active);

        content_panel::update_line_numbers(&ui.code_buffer, &ui.line_numbers);
        content_panel::highlight_buffer(
            &ui.code_buffer,
            active_language.unwrap_or(self.new_file_language),
            &ui.syntax_tags,
        );
        update_cursor_position(&ui.code_buffer, &ui.status_position, self.ui_language);
        ui.status_position.set_visible(active_document.is_some());
    }
}

fn format_diagnostics(diagnostics: &[execution::ExecutionDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let category = match diagnostic.kind {
                execution::DiagnosticKind::Lexical => "Erro léxico",
                execution::DiagnosticKind::Syntax => "Erro de sintaxe",
                execution::DiagnosticKind::Type => "Erro de tipo",
                execution::DiagnosticKind::Runtime => "Erro de execução",
            };
            format!(
                "{category} · Linha {}, coluna {}: {}",
                diagnostic.line, diagnostic.column, diagnostic.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn learning_topic_label(topic: LearningTopic, language: UiLanguage) -> &'static str {
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
            LearningTopic::Return => "retorno",
            LearningTopic::Program => "programa",
        },
    )
}

fn learning_copy(hint: &LearningHint, language: UiLanguage) -> (String, String, String) {
    let text = |portuguese| tr(language, portuguese).to_owned();
    let has_syntax_error = hint
        .problems
        .iter()
        .any(|problem| problem.kind == crate::learning::LearningProblemKind::Syntax);
    let parameters = hint
        .parameters
        .iter()
        .map(|symbol| format_learning_symbol(symbol, hint.language, language, true))
        .collect::<Vec<_>>();
    let name = hint.name.as_deref().unwrap_or("?");
    let mut explanation = match hint.topic {
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
        LearningTopic::Decision => match hint.expression.as_deref() {
            Some(expression) => format!(
                "{} “{expression}” {}",
                text("A condição"),
                text(
                    "determina qual bloco será executado conforme resulte em verdadeiro ou falso."
                )
            ),
            None => {
                text("Uma condição escolhe qual bloco executar com base em verdadeiro ou falso.")
            }
        },
        LearningTopic::Loop => {
            text("Um laço repete instruções de acordo com uma condição ou faixa de valores.")
        }
        LearningTopic::Input => text(
            "leia solicita um valor e o armazena na variável informada antes de o programa continuar.",
        ),
        LearningTopic::Output => {
            if hint.language == Language::GPortugol {
                text("imprima mostra na saída os valores e textos informados.")
            } else {
                text("escreva e escreval mostram na saída os valores e textos informados.")
            }
        }
        LearningTopic::Assignment => {
            text("A atribuição calcula o lado direito e atualiza o valor da variável à esquerda.")
        }
        LearningTopic::Return => match hint.return_type.as_deref() {
            Some("vazio") => text("Esta função não devolve valor; retorne encerra sua execução."),
            Some(data_type) => format!(
                "{} `{data_type}` {}.",
                text("retorne encerra a função e entrega um valor do tipo"),
                text("a quem a chamou")
            ),
            None => text("retorne encerra a rotina e pode entregar um resultado a quem a chamou."),
        },
        LearningTopic::Program => text(
            "A linha selecionada participa do fluxo do algoritmo. Selecione outra linha para ver seu papel.",
        ),
    };

    let concept = match hint.topic {
        LearningTopic::Function => match hint.return_type.as_deref() {
            Some("vazio") => text(
                "Parâmetros são os dados recebidos pela função. `vazio` informa que nenhum resultado é devolvido.",
            ),
            Some("cadeia") => text(
                "A função produz um valor de texto, representado pelo tipo `cadeia` em Portugol 2.0.",
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
        LearningTopic::Assignment => text(
            "Atribuir associa um novo valor a uma variável; o operador determina como o valor é calculado.",
        ),
        LearningTopic::Return => {
            text("O retorno leva o resultado da função de volta ao ponto que a chamou.")
        }
        LearningTopic::Decision => text("A condição é avaliada como verdadeira ou falsa."),
        LearningTopic::Loop => text("Cada repetição executa novamente as instruções do bloco."),
        LearningTopic::Input => text("A entrada é convertida para o tipo da variável solicitada."),
        LearningTopic::Output => text("A saída permite observar valores calculados pelo programa."),
        LearningTopic::Program => {
            text("Uma instrução é executada na ordem definida pelo algoritmo.")
        }
    };

    if hint.language == Language::Potigol {
        explanation.push(' ');
        explanation.push_str(&text(
            "Esta análise do Potigol é heurística e pode não reconhecer todos os casos da linguagem.",
        ));
    }

    let experiment = match hint.topic {
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
        LearningTopic::Decision => match (&hint.expression, &hint.experiment_expression) {
            (Some(current), Some(changed)) => format!(
                "{} “{current}” {} “{changed}” {}",
                text("Troque a condição"),
                text("por"),
                text("e execute de novo. O resultado muda?")
            ),
            _ => text("Mude a condição e execute novamente. O resultado muda?"),
        },
        LearningTopic::Loop => {
            text("Mude a condição ou os limites do laço e observe quantas vezes ele repete.")
        }
        LearningTopic::Input => {
            text("Digite outro valor e observe como o restante do programa reage.")
        }
        LearningTopic::Output => text("Altere o texto ou o valor exibido e execute novamente."),
        LearningTopic::Variable => {
            text("Altere o valor inicial ou um elemento do vetor e observe o efeito no resultado.")
        }
        LearningTopic::Assignment => {
            text("Troque o valor atribuído e acompanhe a próxima leitura da variável.")
        }
        LearningTopic::Return => {
            text("Altere o valor devolvido e veja como a chamada da função reage.")
        }
        LearningTopic::Program => {
            text("Selecione uma linha com entrada, saída, condição ou laço para experimentar.")
        }
    };
    (explanation, concept, experiment)
}

fn format_learning_symbol(
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

fn update_cursor_position(buffer: &gtk::TextBuffer, label: &gtk::Label, language: UiLanguage) {
    let cursor = buffer.iter_at_offset(buffer.cursor_position());
    label.set_text(&format!(
        "{} {} · {} {}",
        tr(language, "Linha"),
        cursor.line() + 1,
        tr(language, "Coluna"),
        cursor.line_offset() + 1
    ));
}

fn input_cancel_controller(
    entry: &gtk::Entry,
    on_cancel: impl Fn() + 'static,
) -> gtk::EventControllerKey {
    let controller = gtk::EventControllerKey::new();
    let entry = entry.clone();
    controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            entry.set_text("");
            on_cancel();
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    controller
}

fn build_window_content(
    window: gtk::ApplicationWindow,
    initial_project_root: Option<PathBuf>,
    initial_start_page: bool,
    initial_recent_paths: &[PathBuf],
    project_files_for_new_items: Rc<RefCell<Vec<ProjectFile>>>,
    sender: ComponentSender<MainWindow>,
    preferences: Preferences,
    initial_dark: bool,
) -> MainUi {
    let ui_language = preferences.ui_language;
    let ui_language_state = Rc::new(Cell::new(ui_language));
    let close_authorized = Rc::new(Cell::new(false));
    let close_authorized_for_window = close_authorized.clone();
    let sender_for_close = sender.clone();
    window.connect_close_request(move |_| {
        if close_authorized_for_window.replace(false) {
            gtk::glib::Propagation::Proceed
        } else {
            sender_for_close.input(MainWindowMsg::RequestClose);
            gtk::glib::Propagation::Stop
        }
    });

    let header = header::build_header(&window, ui_language_state.clone());
    let project_title = header.project_title.clone();
    let run_button = header.run_button.clone();
    let run_icon = header.run_icon.clone();
    let run_label = header.run_label.clone();
    project_title.set_text(if initial_start_page {
        tr(ui_language, "Início")
    } else {
        tr(ui_language, "Meu primeiro programa")
    });
    run_button.set_visible(!initial_start_page);

    let start_page = start_page::build_start_page(ui_language);

    let dark_theme = Rc::new(Cell::new(initial_dark));
    let settings = build_settings_window(
        &window,
        sender.clone(),
        preferences,
        initial_dark,
        ui_language_state.clone(),
    );

    let file_sidebar = sidebar::build_file_sidebar(ui_language);
    let expanded_folders = file_sidebar.expanded_folders.clone();
    let sidebar_widget = file_sidebar.widget;
    let project_row = file_sidebar.project_row;
    let project_label = file_sidebar.project_label;
    let new_project_button = file_sidebar.new_project_button;
    let file_list = file_sidebar.file_list;
    let new_file_button = file_sidebar.new_file_button;
    let empty_state = file_sidebar.empty_state;
    let empty_open_project_button = file_sidebar.empty_open_project_button;
    let empty_open_file_button = file_sidebar.empty_open_file_button;
    let new_program_button = file_sidebar.new_program_button;
    let open_file_menu_button = file_sidebar.open_file_menu_button;
    let new_folder_button = file_sidebar.new_folder_button;
    let new_item_popover = file_sidebar.new_item_popover;
    let settings_button = file_sidebar.settings_button;
    let default_new_language = Rc::new(Cell::new(preferences.default_language));
    let project_root_for_new_items = Rc::new(RefCell::new(initial_project_root));

    let code_buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
    code_buffer.set_text(Language::Portugol.sample());
    let syntax_tags = content_panel::make_syntax_tags(&code_buffer);
    content_panel::set_syntax_palette(&syntax_tags, initial_dark);
    content_panel::highlight_buffer(&code_buffer, Language::Portugol, &syntax_tags);

    let line_numbers = gtk::Label::new(None);
    line_numbers.set_xalign(1.0);
    line_numbers.set_yalign(0.0);
    line_numbers.add_css_class("dim-label");
    line_numbers.add_css_class("monospace");
    line_numbers.add_css_class("potoo-line-numbers");
    line_numbers.add_css_class(&format!("potoo-font-{}", preferences.font_size));
    line_numbers.set_visible(preferences.line_numbers_visible);
    line_numbers.set_margin_top(14);
    line_numbers.set_margin_bottom(14);
    line_numbers.set_margin_start(12);
    line_numbers.set_margin_end(12);
    content_panel::update_line_numbers(&code_buffer, &line_numbers);

    let line_number_gutter = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    line_number_gutter.add_css_class("potoo-line-number-gutter");
    line_number_gutter.append(&line_numbers);
    let line_numbers_separator = gtk::Separator::new(gtk::Orientation::Vertical);
    line_numbers_separator.add_css_class("potoo-gutter-separator");
    line_numbers_separator.set_visible(preferences.line_numbers_visible);
    line_number_gutter.append(&line_numbers_separator);

    let code_view = gtk::TextView::builder()
        .buffer(&code_buffer)
        .monospace(true)
        .wrap_mode(if preferences.word_wrap {
            gtk::WrapMode::WordChar
        } else {
            gtk::WrapMode::None
        })
        .left_margin(12)
        .right_margin(16)
        .top_margin(14)
        .bottom_margin(14)
        .build();
    code_view.set_gutter(gtk::TextWindowType::Left, Some(&line_number_gutter));
    code_view.add_css_class("potoo-code-editor");
    code_view.add_css_class(&format!("potoo-font-{}", preferences.font_size));

    let code_scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&code_view)
        .build();

    let editor_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    editor_row.add_css_class("potoo-editor-row");
    editor_row.append(&code_scroll);

    let editor_tabs = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    editor_tabs.set_hexpand(true);
    let save_tooltip = tr(
        ui_language,
        if cfg!(target_os = "macos") {
            "Salvar arquivo (⌘S)"
        } else {
            "Salvar arquivo (Ctrl+S)"
        },
    );
    let save_button = gtk::Button::new();
    let save_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    save_content.append(&icon("save"));
    save_content.append(&gtk::Label::new(Some(tr(ui_language, "Salvar"))));
    save_button.set_child(Some(&save_content));
    save_button.set_tooltip_text(Some(save_tooltip));
    save_button.add_css_class("potoo-save-button");

    let editor_actions = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    editor_actions.add_css_class("potoo-editor-tab-actions");
    editor_actions.append(&save_button);
    let tab_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    tab_bar.add_css_class("potoo-editor-tabbar");
    tab_bar.append(&editor_tabs);
    tab_bar.append(&editor_actions);

    let editor = gtk::Box::new(gtk::Orientation::Vertical, 0);
    editor.add_css_class("potoo-editor");
    editor.append(&tab_bar);
    editor.append(&editor_row);

    let output_ui: content_panel::OutputPanelUi =
        content_panel::build_output_panel(ui_language, sender.clone());
    let output_panel = output_ui.panel;
    let debugger_page = output_ui.debugger_page;
    let output_buffer = output_ui.output_buffer;
    let output_tab_output = output_ui.output_tab;
    let output_view = output_ui.output_view;
    let problems_buffer = output_ui.problems_buffer;
    let problems_view = output_ui.problems_view;
    let problems_tab = output_ui.problems_tab;
    let debugger_controls = output_ui.debugger_controls;
    let debugger_start = output_ui.debugger_start;
    let debugger_step = output_ui.debugger_step;
    let debugger_stop = output_ui.debugger_stop;
    let debugger_position = output_ui.debugger_position;
    let debugger_variables = output_ui.debugger_variables;
    let debugger_learning = output_ui.debugger_learning;
    let input_entry = gtk::Entry::new();
    input_entry.add_css_class("potoo-program-input");
    input_entry.set_placeholder_text(Some(tr(ui_language, "Digite um valor e pressione Enter")));
    input_entry.set_tooltip_text(Some(tr(
        ui_language,
        "Pressione Esc para cancelar a execução.",
    )));
    input_entry.set_margin_start(12);
    input_entry.set_margin_end(12);
    input_entry.set_margin_top(6);
    input_entry.set_margin_bottom(6);
    input_entry.set_visible(false);
    let output_status = gtk::Label::new(Some(tr(ui_language, "Pronto para executar")));
    output_status.set_xalign(0.0);
    output_status.add_css_class("potoo-muted");
    let output_status_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    output_status_row.add_css_class("potoo-output-status-row");
    let output_ready_icon = icon("ready");
    output_ready_icon.set_margin_start(12);
    output_status.set_margin_end(12);
    output_status_row.append(&output_ready_icon);
    output_status_row.append(&output_status);

    let output_area = gtk::Box::new(gtk::Orientation::Vertical, 0);
    output_area.add_css_class("potoo-output-area");
    output_area.append(&output_panel);
    output_area.append(&input_entry);
    output_area.append(&output_status_row);

    let editor_output = gtk::Paned::new(gtk::Orientation::Vertical);
    editor_output.set_wide_handle(false);
    editor_output.set_start_child(Some(&editor));
    editor_output.set_end_child(Some(&output_area));
    editor_output.set_resize_start_child(true);
    editor_output.set_resize_end_child(false);
    editor_output.set_shrink_start_child(true);
    editor_output.set_shrink_end_child(false);
    editor_output.set_position(570);
    let window_for_output = window.clone();
    editor_output.connect_map(move |paned| {
        let paned = paned.clone();
        let window = window_for_output.clone();
        gtk::glib::timeout_add_local(Duration::from_millis(32), move || {
            let height = paned.height();
            let window_height = window.height();
            if height <= 0 || window_height <= 0 || height < window_height.saturating_sub(150) {
                return gtk::glib::ControlFlow::Continue;
            }
            paned.set_position(height.saturating_sub(218).max(0));
            gtk::glib::ControlFlow::Break
        });
    });

    let learning_ui = content_panel::build_learning_panel(ui_language);
    let tools_ui = content_panel::build_tool_panel(
        learning_ui.panel.clone(),
        debugger_page,
        ui_language,
        sender.clone(),
    );
    let tools_panel = tools_ui.panel.clone();
    let editor_and_tools = gtk::Paned::new(gtk::Orientation::Horizontal);
    editor_and_tools.set_wide_handle(false);
    editor_and_tools.set_start_child(Some(&editor_output));
    editor_and_tools.set_end_child(Some(&tools_panel));
    editor_and_tools.set_resize_start_child(true);
    editor_and_tools.set_resize_end_child(false);
    editor_and_tools.set_shrink_start_child(false);
    editor_and_tools.set_shrink_end_child(false);
    editor_and_tools.set_position(820);
    editor_and_tools.connect_map(|paned| {
        let paned = paned.clone();
        gtk::glib::idle_add_local(move || {
            let width = paned.width();
            if width <= 0 {
                return gtk::glib::ControlFlow::Continue;
            }
            paned.set_position(width.saturating_sub(348).max(440));
            gtk::glib::ControlFlow::Break
        });
    });
    let paned_for_tools = editor_and_tools.clone();
    tools_ui
        .revealer
        .connect_child_revealed_notify(move |revealer| {
            let width = paned_for_tools.width();
            if width > 0 {
                let panel_width = if revealer.is_child_revealed() {
                    348
                } else {
                    42
                };
                paned_for_tools.set_position(width.saturating_sub(panel_width).max(440));
            }
        });

    let workspace = gtk::Paned::new(gtk::Orientation::Horizontal);
    workspace.set_wide_handle(false);
    workspace.set_start_child(Some(&sidebar_widget));
    workspace.set_end_child(Some(&editor_and_tools));
    workspace.set_resize_start_child(false);
    workspace.set_resize_end_child(true);
    workspace.set_shrink_start_child(false);
    workspace.set_shrink_end_child(false);
    workspace.set_position(242);
    workspace.set_vexpand(true);

    let status_language = gtk::Label::new(Some("Portugol"));
    let status_details = gtk::Label::new(Some(&format!(
        "UTF-8  ·  {}: {}",
        tr(ui_language, "Espaços"),
        preferences.indent_width
    )));
    status_language.add_css_class("potoo-status-primary");
    status_details.add_css_class("potoo-muted");
    let status_position = gtk::Label::new(Some("Linha 1 · Coluna 1"));
    status_position.add_css_class("potoo-muted");
    let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    status_bar.add_css_class("potoo-statusbar");
    status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    status_bar.append(&status_language);
    status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    status_bar.append(&status_details);
    let status_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    status_spacer.set_hexpand(true);
    status_bar.append(&status_spacer);
    status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    status_bar.append(&status_position);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&workspace);
    content.append(&status_bar);

    let content_root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content_root.add_css_class("potoo-root");
    if let Some(header_widget) = &header.widget {
        content_root.append(header_widget);
    }
    let main_stack = gtk::Stack::new();
    main_stack.add_css_class("potoo-main-stack");
    main_stack.set_hexpand(true);
    main_stack.set_vexpand(true);
    main_stack.add_named(&content, Some("workspace"));
    main_stack.add_named(&start_page.widget, Some("start"));
    main_stack.set_visible_child_name(if initial_start_page {
        "start"
    } else {
        "workspace"
    });
    content_root.append(&main_stack);
    if initial_dark {
        window.add_css_class("potoo-dark");
    }

    let sender_for_empty_open_project = sender.clone();
    let window_for_empty_open_project = window.clone();
    let language_for_empty_open_project = ui_language_state.clone();
    empty_open_project_button.connect_clicked(move |_| {
        show_open_project_dialog(
            &window_for_empty_open_project,
            language_for_empty_open_project.get(),
            sender_for_empty_open_project.clone(),
            None,
        );
    });

    let sender_for_empty_open_file = sender.clone();
    let window_for_empty_open_file = window.clone();
    let language_for_empty_open_file = ui_language_state.clone();
    empty_open_file_button.connect_clicked(move |_| {
        show_open_file_dialog(
            &window_for_empty_open_file,
            language_for_empty_open_file.get(),
            sender_for_empty_open_file.clone(),
        );
    });

    let sender_for_new_project = sender.clone();
    let window_for_new_project = window.clone();
    let default_new_language_for_project = default_new_language.clone();
    let dark_theme_for_project = dark_theme.clone();
    let language_for_new_project = ui_language_state.clone();
    new_project_button.connect_clicked(move |_| {
        show_new_project_dialog(
            &window_for_new_project,
            language_for_new_project.get(),
            ProjectTemplate::default_for_language(default_new_language_for_project.get()),
            true,
            dark_theme_for_project.get(),
            sender_for_new_project.clone(),
        );
    });

    let sender_for_start_portugol = sender.clone();
    let window_for_start_portugol = window.clone();
    let dark_for_start_portugol = dark_theme.clone();
    let language_for_start_portugol = ui_language_state.clone();
    start_page.portugol_button.connect_clicked(move |_| {
        show_new_project_dialog(
            &window_for_start_portugol,
            language_for_start_portugol.get(),
            ProjectTemplate::Portugol20,
            false,
            dark_for_start_portugol.get(),
            sender_for_start_portugol.clone(),
        );
    });

    let sender_for_start_potigol = sender.clone();
    let window_for_start_potigol = window.clone();
    let dark_for_start_potigol = dark_theme.clone();
    let language_for_start_potigol = ui_language_state.clone();
    start_page.potigol_button.connect_clicked(move |_| {
        show_new_project_dialog(
            &window_for_start_potigol,
            language_for_start_potigol.get(),
            ProjectTemplate::PotigolClassic,
            false,
            dark_for_start_potigol.get(),
            sender_for_start_potigol.clone(),
        );
    });

    let sender_for_start_open = sender.clone();
    let window_for_start_open = window.clone();
    let language_for_start_open = ui_language_state.clone();
    start_page.open_folder_button.connect_clicked(move |_| {
        show_open_project_dialog(
            &window_for_start_open,
            language_for_start_open.get(),
            sender_for_start_open.clone(),
            None,
        );
    });

    let sender_for_start_open_file = sender.clone();
    let window_for_start_open_file = window.clone();
    let language_for_start_open_file = ui_language_state.clone();
    start_page.open_file_button.connect_clicked(move |_| {
        show_open_file_dialog(
            &window_for_start_open_file,
            language_for_start_open_file.get(),
            sender_for_start_open_file.clone(),
        );
    });

    let sender_for_save = sender.clone();
    save_button.connect_clicked(move |_| {
        sender_for_save.input(MainWindowMsg::SaveCurrent);
    });

    let sender_for_new_file = sender.clone();
    let window_for_new_file = window.clone();
    let default_new_language_for_new_file = default_new_language.clone();
    let project_root_for_new_file = project_root_for_new_items.clone();
    let project_files_for_new_file = project_files_for_new_items.clone();
    let dark_theme_for_new_file = dark_theme.clone();
    let language_for_new_file = ui_language_state.clone();
    let popover_for_new_file = new_item_popover.clone();
    new_program_button.connect_clicked(move |_| {
        popover_for_new_file.popdown();
        let Some(project_root) = project_root_for_new_file.borrow().clone() else {
            return;
        };
        show_new_file_dialog(
            &window_for_new_file,
            default_new_language_for_new_file.get(),
            language_for_new_file.get(),
            project_directory_options(
                &project_root,
                language_for_new_file.get(),
                &project_files_for_new_file.borrow(),
            ),
            dark_theme_for_new_file.get(),
            sender_for_new_file.clone(),
        );
    });

    let sender_for_open_file_menu = sender.clone();
    let window_for_open_file_menu = window.clone();
    let language_for_open_file_menu = ui_language_state.clone();
    let popover_for_open_file_menu = new_item_popover.clone();
    open_file_menu_button.connect_clicked(move |_| {
        popover_for_open_file_menu.popdown();
        show_open_file_dialog(
            &window_for_open_file_menu,
            language_for_open_file_menu.get(),
            sender_for_open_file_menu.clone(),
        );
    });

    let sender_for_new_folder = sender.clone();
    let window_for_new_folder = window.clone();
    let project_root_for_new_folder = project_root_for_new_items.clone();
    let project_files_for_new_folder = project_files_for_new_items.clone();
    let dark_theme_for_new_folder = dark_theme.clone();
    let language_for_new_folder = ui_language_state.clone();
    let popover_for_new_folder = new_item_popover.clone();
    new_folder_button.connect_clicked(move |_| {
        popover_for_new_folder.popdown();
        let Some(project_root) = project_root_for_new_folder.borrow().clone() else {
            return;
        };
        show_new_folder_dialog(
            &window_for_new_folder,
            language_for_new_folder.get(),
            project_directory_options(
                &project_root,
                language_for_new_folder.get(),
                &project_files_for_new_folder.borrow(),
            ),
            dark_theme_for_new_folder.get(),
            sender_for_new_folder.clone(),
        );
    });

    let sender_for_settings = sender.clone();
    settings_button.connect_clicked(move |_| {
        sender_for_settings.input(MainWindowMsg::OpenSettings);
    });

    let sender_for_run = sender.clone();
    let output_tab_for_run = output_tab_output.clone();
    run_button.connect_clicked(move |_| {
        output_tab_for_run.emit_clicked();
        sender_for_run.input(MainWindowMsg::Run);
    });

    let sender_for_program_input = sender.clone();
    input_entry.connect_activate(move |entry| {
        sender_for_program_input.input(MainWindowMsg::SubmitProgramInput(entry.text().to_string()));
        entry.set_text("");
    });

    let sender_for_input_cancel = sender.clone();
    input_entry.add_controller(input_cancel_controller(&input_entry, move || {
        sender_for_input_cancel.input(MainWindowMsg::StopDebug);
    }));

    let sender_for_source = sender.clone();
    code_buffer.connect_changed(move |buffer| {
        let contents = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
        sender_for_source.input(MainWindowMsg::SourceChanged(contents.to_string()));
    });

    let sender_for_cursor = sender.clone();
    code_buffer.connect_mark_set(move |buffer, _, _| {
        let line = (buffer.iter_at_offset(buffer.cursor_position()).line() + 1) as usize;
        sender_for_cursor.input(MainWindowMsg::CursorMoved(line));
    });

    let key_controller = gtk::EventControllerKey::new();
    let sender_for_shortcuts = sender;
    let window_for_shortcuts = window.clone();
    let language_for_shortcuts = ui_language_state.clone();
    key_controller.connect_key_pressed(move |_, key, _, modifiers| {
        let primary_modifier = modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK)
            || (cfg!(target_os = "macos") && modifiers.contains(gtk::gdk::ModifierType::META_MASK));
        if key == gtk::gdk::Key::s
            && primary_modifier
            && !modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK)
        {
            sender_for_shortcuts.input(MainWindowMsg::SaveCurrent);
            gtk::glib::Propagation::Stop
        } else if key == gtk::gdk::Key::o && primary_modifier {
            show_open_file_dialog(
                &window_for_shortcuts,
                language_for_shortcuts.get(),
                sender_for_shortcuts.clone(),
            );
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    window.add_controller(key_controller);

    let indent_width = Rc::new(Cell::new(preferences.indent_width));
    let indent_controller = gtk::EventControllerKey::new();
    let buffer_for_indent = code_buffer.clone();
    let indent_width_for_key = indent_width.clone();
    indent_controller.connect_key_pressed(move |_, key, _, modifiers| {
        if key == gtk::gdk::Key::Tab
            && !modifiers.intersects(
                gtk::gdk::ModifierType::CONTROL_MASK
                    | gtk::gdk::ModifierType::ALT_MASK
                    | gtk::gdk::ModifierType::META_MASK,
            )
        {
            buffer_for_indent.insert_at_cursor(&" ".repeat(indent_width_for_key.get() as usize));
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    code_view.add_controller(indent_controller);

    MainUi {
        window,
        content: content_root,
        main_stack,
        dark_theme,
        default_new_language,
        project_root_for_new_items,
        project_files_for_new_items,
        settings,
        project_title,
        run_button,
        run_icon,
        run_label,
        start_page,
        project_row,
        project_label,
        file_list,
        new_file_button,
        empty_state,
        expanded_folders,
        editor_tabs,
        status_language,
        status_details,
        status_position,
        indent_width,
        code_buffer,
        code_view,
        syntax_tags,
        line_numbers,
        line_numbers_separator,
        output_buffer,
        output_view,
        problems_buffer,
        problems_view,
        problems_tab,
        input_entry,
        output_status,
        debugger_start,
        debugger_controls,
        debugger_step,
        debugger_stop,
        debugger_position,
        debugger_variables,
        debugger_learning,
        right_panel_revealer: tools_ui.revealer,
        right_panel_stack: tools_ui.stack,
        learning_tool_button: tools_ui.learning_button,
        debugger_tool_button: tools_ui.debugger_button,
        game_preview_button: tools_ui.game_preview_button,
        game_preview_stack: tools_ui.game_preview_stack,
        game_preview_area: tools_ui.game_preview_area,
        learning_spinner: learning_ui.spinner,
        learning_analysis_stack: learning_ui.analysis_stack,
        learning_line_title: learning_ui.line_title,
        learning_explanation: learning_ui.explanation,
        learning_experiment: learning_ui.experiment,
        learning_concept_title: learning_ui.concept_title,
        learning_concept_text: learning_ui.concept_text,
        learning_problem_list: learning_ui.problem_list,
        learning_problem_empty: learning_ui.problem_empty,
        displayed_language: Some(Language::Portugol),
        displayed_active_document_id: Some(0),
        displayed_project_name: String::new(),
        displayed_project_files: Vec::new(),
        displayed_project_open: false,
        displayed_recent_paths: initial_recent_paths.to_vec(),
        displayed_start_page: initial_start_page,
        displayed_run_active: false,
        displayed_run_language: ui_language,
        displayed_tabs: Vec::new(),
        displayed_pending_project: None,
        displayed_pending_close_document_id: None,
        displayed_pending_close_app: false,
        displayed_pending_save_as_id: None,
        displayed_close_app_ready: false,
        close_authorized,
        displayed_theme: preferences.theme,
        displayed_dark: initial_dark,
        displayed_font_size: preferences.font_size,
        displayed_line_numbers_visible: preferences.line_numbers_visible,
        displayed_word_wrap: preferences.word_wrap,
        displayed_indent_width: preferences.indent_width,
        displayed_preferred_ui_language: preferences.ui_language,
        displayed_settings_visible: false,
        displayed_output_text: tr(ui_language, "A saída aparecerá aqui após a execução.")
            .to_string(),
        displayed_problem_text: tr(ui_language, "Nenhum problema detectado").to_string(),
        displayed_input_requested: false,
        displayed_output_status: tr(ui_language, "Pronto para executar").to_string(),
        displayed_debugger_snapshot: None,
        displayed_debugger_error: false,
        displayed_execution_line: None,
        displayed_debugger_learning_hint: None,
        displayed_debugger_analysis_state: "idle",
        displayed_learning_hint: None,
        displayed_learning_loading: false,
        displayed_right_panel_section: None,
        displayed_right_panel_collapsed: true,
        displayed_game_preview_active: false,
    }
}

fn build_settings_window(
    parent: &gtk::ApplicationWindow,
    sender: ComponentSender<MainWindow>,
    preferences: Preferences,
    initial_dark: bool,
    ui_language_state: Rc<Cell<UiLanguage>>,
) -> SettingsUi {
    let language = preferences.ui_language;
    let window = gtk::Window::new();
    window.set_title(Some(tr(language, "Configurações")));
    window.set_default_size(740, 685);
    window.set_resizable(true);
    window.set_modal(true);
    close_window_on_escape(&window);
    window.set_transient_for(Some(parent));
    window.add_css_class("potoo-window");
    window.add_css_class("potoo-settings-window");
    if initial_dark {
        window.add_css_class("potoo-dark");
    }

    let titlebar = gtk::HeaderBar::new();
    titlebar.add_css_class("potoo-settings-titlebar");
    titlebar.set_show_title_buttons(false);
    let title_label = gtk::Label::new(Some(tr(language, "Configurações")));
    title_label.add_css_class("potoo-settings-window-title");
    titlebar.set_title_widget(Some(&title_label));
    let back_button = gtk::Button::new();
    back_button.add_css_class("potoo-settings-back");
    let back_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    back_content.append(&icon("chevron-left"));
    back_content.append(&gtk::Label::new(Some(tr(language, "Configurações"))));
    back_button.set_child(Some(&back_content));
    back_button.set_visible(false);
    titlebar.pack_start(&back_button);
    let close_button = gtk::Button::new();
    close_button.add_css_class("potoo-dialog-close");
    close_button.set_child(Some(&icon("close")));
    close_button.set_tooltip_text(Some(tr(language, "Fechar")));
    titlebar.pack_end(&close_button);
    window.set_titlebar(Some(&titlebar));
    let window_for_titlebar_close = window.clone();
    close_button.connect_clicked(move |_| window_for_titlebar_close.close());

    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page.add_css_class("potoo-settings-page");
    page.set_margin_start(22);
    page.set_margin_end(22);
    page.set_margin_top(18);
    page.set_margin_bottom(20);

    let description = gtk::Label::new(Some(tr(language, "Personalize a aparência e o editor.")));
    description.set_xalign(0.0);
    description.add_css_class("potoo-settings-description");
    description.set_margin_bottom(14);
    page.append(&description);

    let appearance = settings_section(tr(language, "Aparência"));
    let theme_buttons = [
        gtk::Button::with_label(tr(language, "Sistema")),
        gtk::Button::with_label(tr(language, "Claro")),
        gtk::Button::with_label(tr(language, "Escuro")),
    ];
    for button in &theme_buttons {
        button.add_css_class("potoo-theme-choice");
    }
    select_theme_button(&theme_buttons, preferences.theme.selected_index());
    let theme_options = gtk::Box::new(gtk::Orientation::Horizontal, 3);
    theme_options.add_css_class("potoo-theme-options");
    theme_options.set_homogeneous(true);
    for button in &theme_buttons {
        theme_options.append(button);
    }
    appearance.append(&settings_row(
        tr(language, "Tema"),
        tr(language, "Escolha como o Potoo aparece nesta tela."),
        &theme_options,
    ));
    page.append(&appearance);

    let editor = settings_section(tr(language, "Editor"));
    editor.add_css_class("potoo-settings-section-separated");
    let font_size_picker = gtk::DropDown::from_strings(&["14 px", "15 px", "16 px", "18 px"]);
    font_size_picker.add_css_class("potoo-settings-picker");
    font_size_picker.set_selected(font_size_index(preferences.font_size));
    editor.append(&settings_row(
        tr(language, "Tamanho da fonte"),
        tr(language, "Ajuste a leitura do código."),
        &font_size_picker,
    ));

    let line_numbers_switch = gtk::Switch::new();
    line_numbers_switch.add_css_class("potoo-settings-switch");
    line_numbers_switch.set_active(preferences.line_numbers_visible);
    editor.append(&settings_row(
        tr(language, "Numeração de linhas"),
        tr(language, "Mostra o número de cada linha do código."),
        &line_numbers_switch,
    ));

    let word_wrap_switch = gtk::Switch::new();
    word_wrap_switch.add_css_class("potoo-settings-switch");
    word_wrap_switch.set_active(preferences.word_wrap);
    editor.append(&settings_row(
        tr(language, "Quebra automática"),
        tr(language, "Ajusta linhas longas à largura do editor."),
        &word_wrap_switch,
    ));

    let indent_width_picker =
        gtk::DropDown::from_strings(&[tr(language, "2 espaços"), tr(language, "4 espaços")]);
    indent_width_picker.add_css_class("potoo-settings-picker");
    indent_width_picker.set_selected(if preferences.indent_width == 2 { 0 } else { 1 });
    editor.append(&settings_row(
        tr(language, "Recuo"),
        tr(language, "Espaços por nível de recuo."),
        &indent_width_picker,
    ));

    page.append(&editor);

    let interface = settings_section(tr(language, "Interface"));
    interface.add_css_class("potoo-settings-section-separated");
    let ui_language_picker = gtk::DropDown::from_strings(&["Português", "English", "Español"]);
    ui_language_picker.add_css_class("potoo-settings-picker");
    ui_language_picker.set_selected(preferences.ui_language.selected_index());
    interface.append(&settings_row(
        tr(language, "Idioma da interface"),
        tr(
            language,
            "A alteração de idioma será aplicada após salvar e reiniciar o aplicativo.",
        ),
        &ui_language_picker,
    ));
    page.append(&interface);

    let about_section = settings_section(tr(language, "Sobre"));
    about_section.add_css_class("potoo-settings-section-separated");
    let about_button = gtk::Button::new();
    about_button.add_css_class("potoo-settings-about-entry");
    about_button.set_hexpand(true);
    let about_button_content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    about_button_content.set_hexpand(true);
    let about_button_copy = gtk::Box::new(gtk::Orientation::Vertical, 4);
    about_button_copy.set_hexpand(true);
    about_button_copy.set_valign(gtk::Align::Center);
    let about_button_title = gtk::Label::new(Some(tr(language, "Sobre")));
    about_button_title.set_xalign(0.0);
    about_button_title.add_css_class("potoo-settings-row-title");
    let about_button_description = gtk::Label::new(Some(tr(language, "Versão e links do projeto")));
    about_button_description.set_xalign(0.0);
    about_button_description.add_css_class("potoo-settings-row-description");
    about_button_copy.append(&about_button_title);
    about_button_copy.append(&about_button_description);
    about_button_content.append(&about_button_copy);
    let about_arrow = icon("chevron-right");
    about_arrow.add_css_class("potoo-settings-about-arrow");
    about_button_content.append(&about_arrow);
    about_button.set_child(Some(&about_button_content));
    about_section.append(&about_button);
    page.append(&about_section);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&page)
        .build();

    let about_page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    about_page.add_css_class("potoo-about-page");
    about_page.set_hexpand(true);
    about_page.set_vexpand(true);

    let about_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    about_content.add_css_class("potoo-about-content");
    about_content.set_halign(gtk::Align::Center);
    about_content.set_valign(gtk::Align::Start);
    about_content.set_size_request(500, -1);

    let identity = gtk::Box::new(gtk::Orientation::Vertical, 6);
    identity.add_css_class("potoo-about-identity");
    identity.set_halign(gtk::Align::Center);
    let logo = header::app_icon();
    logo.set_pixel_size(66);
    logo.add_css_class("potoo-about-logo");
    let app_name = gtk::Label::new(Some("Potoo IDE"));
    app_name.add_css_class("potoo-about-title");
    let creator = gtk::Label::new(Some(tr(language, "Criado por Diego Pereira do Nascimento")));
    creator.add_css_class("potoo-about-creator");
    creator.set_wrap(true);
    creator.set_justify(gtk::Justification::Center);
    let version = gtk::Label::new(Some(&format!(
        "{} {}",
        tr(language, "Versão"),
        env!("CARGO_PKG_VERSION")
    )));
    version.add_css_class("potoo-about-version");
    identity.append(&logo);
    identity.append(&app_name);
    identity.append(&creator);
    identity.append(&version);
    about_content.append(&identity);

    let about_links = gtk::Box::new(gtk::Orientation::Vertical, 0);
    about_links.add_css_class("potoo-about-links");
    let link_items = [
        (
            tr(language, "Relatar um problema"),
            "https://github.com/diegopn/potoo-ide/issues",
        ),
        (
            tr(language, "Código do projeto"),
            "https://github.com/diegopn/potoo-ide",
        ),
        (
            tr(language, "Site do projeto"),
            "https://diegopn.github.io/potoo-site/",
        ),
        (
            tr(language, "Ajuda"),
            "https://diegopn.github.io/potoo-site/manual/",
        ),
        (tr(language, "Doar"), "https://github.com/sponsors/diegopn"),
        (
            tr(language, "Contato"),
            "https://diegopn.github.io/#contato",
        ),
    ];
    for (index, (label, uri)) in link_items.iter().enumerate() {
        let link = gtk::LinkButton::with_label(uri, label);
        link.add_css_class("potoo-about-link");
        link.set_hexpand(true);
        link.set_halign(gtk::Align::Fill);
        link.set_tooltip_text(Some(uri));
        let link_content = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        link_content.set_hexpand(true);
        let link_label = gtk::Label::new(Some(label));
        link_label.set_xalign(0.0);
        link_label.set_hexpand(true);
        link_label.add_css_class("potoo-about-link-label");
        link_content.append(&link_label);
        let external_icon = icon("open-external");
        external_icon.set_pixel_size(14);
        link_content.append(&external_icon);
        link.set_child(Some(&link_content));
        about_links.append(&link);
        if index < link_items.len() - 1 {
            let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
            separator.add_css_class("potoo-about-link-separator");
            about_links.append(&separator);
        }
    }
    about_content.append(&about_links);
    about_page.append(&about_content);

    let about_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&about_page)
        .build();

    let stack = gtk::Stack::new();
    stack.add_css_class("potoo-settings-stack");
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_vhomogeneous(false);
    stack.add_named(&scroll, Some("preferences"));
    stack.add_named(&about_scroll, Some("about"));
    stack.set_visible_child_name("preferences");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.add_css_class("potoo-settings-root");
    content.append(&stack);
    window.set_child(Some(&content));

    let stack_for_about = stack.clone();
    let window_for_about = window.clone();
    let title_for_about = title_label.clone();
    let back_for_about = back_button.clone();
    let language_for_about = ui_language_state.clone();
    about_button.connect_clicked(move |_| {
        stack_for_about.set_visible_child_name("about");
        window_for_about.set_default_size(740, 560);
        back_for_about.set_visible(true);
        title_for_about.set_text(tr(language_for_about.get(), "Sobre o Potoo IDE"));
        window_for_about.set_title(Some(tr(language_for_about.get(), "Sobre o Potoo IDE")));
    });

    let stack_for_back = stack.clone();
    let window_for_back = window.clone();
    let title_for_back = title_label.clone();
    let back_for_back = back_button.clone();
    let language_for_back = ui_language_state.clone();
    back_button.connect_clicked(move |_| {
        stack_for_back.set_visible_child_name("preferences");
        window_for_back.set_default_size(740, 685);
        back_for_back.set_visible(false);
        title_for_back.set_text(tr(language_for_back.get(), "Configurações"));
        window_for_back.set_title(Some(tr(language_for_back.get(), "Configurações")));
    });

    let sender_for_theme = sender.clone();
    for (index, button) in theme_buttons.iter().enumerate() {
        let theme_buttons_for_click = theme_buttons.clone();
        let sender_for_click = sender_for_theme.clone();
        button.connect_clicked(move |_| {
            let index = index as u32;
            select_theme_button(&theme_buttons_for_click, index);
            sender_for_click.input(MainWindowMsg::ThemeSelected(
                ThemePreference::from_selected_index(index),
            ));
        });
    }

    let sender_for_font_size = sender.clone();
    font_size_picker.connect_selected_notify(move |picker| {
        let font_size = match picker.selected() {
            1 => 15,
            2 => 16,
            3 => 18,
            _ => 14,
        };
        sender_for_font_size.input(MainWindowMsg::FontSizeSelected(font_size));
    });

    let sender_for_line_numbers = sender.clone();
    line_numbers_switch.connect_active_notify(move |switch| {
        sender_for_line_numbers.input(MainWindowMsg::LineNumbersChanged(switch.is_active()));
    });

    let sender_for_word_wrap = sender.clone();
    word_wrap_switch.connect_active_notify(move |switch| {
        sender_for_word_wrap.input(MainWindowMsg::WordWrapChanged(switch.is_active()));
    });

    let sender_for_indent = sender.clone();
    indent_width_picker.connect_selected_notify(move |picker| {
        sender_for_indent.input(MainWindowMsg::IndentWidthSelected(
            if picker.selected() == 0 { 2 } else { 4 },
        ));
    });

    let sender_for_ui_language = sender.clone();
    ui_language_picker.connect_selected_notify(move |picker| {
        sender_for_ui_language.input(MainWindowMsg::InterfaceLanguageSelected(
            UiLanguage::from_selected_index(picker.selected()),
        ));
    });

    let stack_for_close = stack.clone();
    let title_for_close = title_label.clone();
    let back_for_close = back_button.clone();
    let window_for_close = window.clone();
    let language_for_close = ui_language_state.clone();
    window.connect_close_request(move |_| {
        stack_for_close.set_visible_child_name("preferences");
        window_for_close.set_default_size(740, 685);
        back_for_close.set_visible(false);
        title_for_close.set_text(tr(language_for_close.get(), "Configurações"));
        window_for_close.set_title(Some(tr(language_for_close.get(), "Configurações")));
        sender.input(MainWindowMsg::CloseSettings);
        gtk::glib::Propagation::Stop
    });

    SettingsUi {
        window,
        theme_buttons,
        font_size_picker,
        line_numbers_switch,
        word_wrap_switch,
        indent_width_picker,
        ui_language_picker,
    }
}

fn settings_section(title: &str) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 0);
    section.add_css_class("potoo-settings-section");
    let heading = gtk::Label::new(Some(title));
    heading.set_xalign(0.0);
    heading.add_css_class("potoo-settings-section-title");
    section.append(&heading);
    section
}

fn select_theme_button(buttons: &[gtk::Button; 3], selected_index: u32) {
    for (index, button) in buttons.iter().enumerate() {
        if index as u32 == selected_index {
            button.add_css_class("potoo-theme-choice-selected");
        } else {
            button.remove_css_class("potoo-theme-choice-selected");
        }
    }
}

fn settings_row<W: IsA<gtk::Widget>>(title: &str, description: &str, control: &W) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 18);
    row.add_css_class("potoo-settings-row");
    control.set_valign(gtk::Align::Center);

    let labels = gtk::Box::new(gtk::Orientation::Vertical, 2);
    labels.set_hexpand(true);
    let title_label = gtk::Label::new(Some(title));
    title_label.set_xalign(0.0);
    title_label.add_css_class("potoo-settings-row-title");
    let description_label = gtk::Label::new(Some(description));
    description_label.set_xalign(0.0);
    description_label.set_wrap(true);
    description_label.add_css_class("potoo-settings-row-description");
    labels.append(&title_label);
    labels.append(&description_label);
    row.append(&labels);
    row.append(control);
    row
}

fn font_size_index(font_size: u32) -> u32 {
    match font_size {
        15 => 1,
        16 => 2,
        18 => 3,
        _ => 0,
    }
}

fn set_font_size(widget: &impl IsA<gtk::Widget>, old_size: u32, new_size: u32) {
    widget.remove_css_class(&format!("potoo-font-{old_size}"));
    widget.add_css_class(&format!("potoo-font-{new_size}"));
}

fn show_open_project_dialog(
    parent: &impl IsA<gtk::Window>,
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
    close_parent_on_success: Option<gtk::Window>,
) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title(tr(language, "Abrir pasta de projeto"));
    dialog.set_modal(true);
    dialog.select_folder(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |result| match result {
            Ok(folder) => match folder.path() {
                Some(path) => {
                    if let Some(window) = close_parent_on_success {
                        window.close();
                    }
                    sender.input(MainWindowMsg::ProjectSelected(path));
                }
                None => sender.input(MainWindowMsg::FileOperationFailed(
                    tr(language, "Selecione uma pasta local para abrir.").to_string(),
                )),
            },
            Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => {}
            Err(error) => sender.input(MainWindowMsg::FileOperationFailed(format!(
                "{}: {error}",
                tr(language, "Não foi possível abrir a pasta")
            ))),
        },
    );
}

fn show_open_file_dialog(
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title(tr(language, "Abrir arquivo de código"));
    let filter = gtk::FileFilter::new();
    filter.set_name(Some(tr(
        language,
        "Portugol, VisuAlg, Potigol e G-Portugol (*.por, *.alg, *.poti, *.gpt)",
    )));
    filter.add_suffix("por");
    filter.add_suffix("alg");
    filter.add_suffix("poti");
    filter.add_suffix("gpt");
    let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    dialog.set_filters(Some(&filters));
    dialog.open(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |result| match result {
            Ok(file) => match file.path() {
                Some(path) => sender.input(MainWindowMsg::OpenFile(path)),
                None => sender.input(MainWindowMsg::FileOperationFailed(
                    tr(language, "Selecione um arquivo local.").to_string(),
                )),
            },
            Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => {}
            Err(error) => sender.input(MainWindowMsg::FileOperationFailed(format!(
                "{}: {error}",
                tr(language, "Não foi possível abrir o arquivo de código")
            ))),
        },
    );
}

fn show_portugol_file_selection_dialog(
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    execution_id: u64,
    project_root: Option<PathBuf>,
    formats: Vec<String>,
    allow_all_files: bool,
    cancellable: gtk::gio::Cancellable,
    sender: ComponentSender<MainWindow>,
) {
    let Some(project_root) = project_root else {
        sender.input(MainWindowMsg::FileSelectionResolved(
            execution_id,
            Err(tr(language, "Abra um projeto antes de selecionar um arquivo.").to_owned()),
        ));
        return;
    };

    let dialog = gtk::FileDialog::new();
    dialog.set_title(tr(language, "Selecionar arquivo"));
    let root_file = gtk::gio::File::for_path(&project_root);
    dialog.set_initial_folder(Some(&root_file));
    let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
    for format in formats {
        let (label, suffixes) = format
            .split_once('|')
            .map_or((format.as_str(), format.as_str()), |(label, suffixes)| {
                (label, suffixes)
            });
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(label));
        if let Some(normalized_suffixes) = execution::normalize_file_selection_suffixes(suffixes) {
            for normalized_suffix in normalized_suffixes {
                filter.add_suffix(&normalized_suffix);
            }
        } else {
            filter.add_pattern("*");
        }
        filters.append(&filter);
    }
    if allow_all_files || filters.n_items() == 0 {
        let all_files = gtk::FileFilter::new();
        all_files.set_name(Some(tr(language, "Todos os arquivos")));
        all_files.add_pattern("*");
        filters.append(&all_files);
    }
    dialog.set_filters(Some(&filters));
    dialog.open(Some(parent), Some(&cancellable), move |result| {
        let selection = match result {
            Ok(file) => file
                .path()
                .map(|path| {
                    let canonical_root = fs::canonicalize(&project_root).unwrap_or(project_root);
                    let canonical_path = fs::canonicalize(&path).unwrap_or(path.clone());
                    canonical_path
                        .strip_prefix(&canonical_root)
                        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_else(|_| canonical_path.to_string_lossy().into_owned())
                })
                .ok_or_else(|| tr(language, "Selecione um arquivo local.").to_owned()),
            Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => Ok(String::new()),
            Err(error) => Err(format!(
                "{}: {error}",
                tr(language, "Não foi possível selecionar o arquivo")
            )),
        };
        sender.input(MainWindowMsg::FileSelectionResolved(
            execution_id,
            selection,
        ));
    });
}

fn update_project_template_selector(
    template: ProjectTemplate,
    ui_language: UiLanguage,
    field: &gtk::Box,
    label: &gtk::Label,
    portugol_options: &gtk::Box,
    portugol_buttons: [&gtk::Button; 3],
    potigol_options: &gtk::Box,
    potigol_buttons: [&gtk::Button; 2],
) {
    match template {
        ProjectTemplate::GPortugol | ProjectTemplate::Portugol20 | ProjectTemplate::Visualg => {
            label.set_text(tr(ui_language, "Variante do Portugol"));
            portugol_options.set_visible(true);
            potigol_options.set_visible(false);
            let selected = match template {
                ProjectTemplate::GPortugol => 0,
                ProjectTemplate::Portugol20 => 1,
                ProjectTemplate::Visualg => 2,
                _ => unreachable!(),
            };
            set_selected_variant_button(&portugol_buttons, selected);
            for button in potigol_buttons {
                button.remove_css_class("selected");
            }
        }
        ProjectTemplate::PotigolClassic | ProjectTemplate::PotigolJerimum => {
            label.set_text(tr(ui_language, "Variante do Potigol"));
            portugol_options.set_visible(false);
            potigol_options.set_visible(true);
            let selected = if template == ProjectTemplate::PotigolJerimum {
                1
            } else {
                0
            };
            set_selected_variant_button(&potigol_buttons, selected);
            for button in portugol_buttons {
                button.remove_css_class("selected");
            }
        }
    }
    field.set_visible(true);
}

fn project_template_variant_index(template: ProjectTemplate) -> u32 {
    match template {
        ProjectTemplate::GPortugol | ProjectTemplate::PotigolClassic => 0,
        ProjectTemplate::Portugol20 | ProjectTemplate::PotigolJerimum => 1,
        ProjectTemplate::Visualg => 2,
    }
}

fn project_template_for_variant(language: Language, selected: u32) -> Option<ProjectTemplate> {
    match (language, selected) {
        (Language::Potigol, 0) => Some(ProjectTemplate::PotigolClassic),
        (Language::Potigol, 1) => Some(ProjectTemplate::PotigolJerimum),
        (Language::Potigol, _) => None,
        (_, 0) => Some(ProjectTemplate::GPortugol),
        (_, 1) => Some(ProjectTemplate::Portugol20),
        (_, 2) => Some(ProjectTemplate::Visualg),
        (_, _) => None,
    }
}

fn set_selected_variant_button(buttons: &[&gtk::Button], selected: usize) {
    for (index, button) in buttons.iter().enumerate() {
        if index == selected {
            button.add_css_class("selected");
        } else {
            button.remove_css_class("selected");
        }
    }
}

fn connect_project_template_choices(
    buttons: &[gtk::Button],
    templates: &[ProjectTemplate],
    selected_template: Rc<Cell<ProjectTemplate>>,
) {
    for (button, template) in buttons.iter().zip(templates) {
        let selected_button = (*button).clone();
        let button_for_connection = selected_button.clone();
        let buttons = buttons.to_vec();
        let selected_template = selected_template.clone();
        let template = *template;
        button_for_connection.connect_clicked(move |_| {
            selected_template.set(template);
            for choice in &buttons {
                choice.remove_css_class("selected");
            }
            selected_button.add_css_class("selected");
        });
    }
}

fn show_new_project_dialog(
    parent: &gtk::ApplicationWindow,
    ui_language: UiLanguage,
    default_template: ProjectTemplate,
    show_language_selector: bool,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(ui_language, "Criar projeto")));
    let default_language = default_template.language();
    dialog.set_default_size(680, if show_language_selector { 520 } else { 430 });
    dialog.set_resizable(false);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_css_class("potoo-window");
    dialog.add_css_class("potoo-dialog-window");
    if initial_dark {
        dialog.add_css_class("potoo-dark");
    }
    set_dialog_titlebar(&dialog, ui_language, "Criar projeto");

    let dialog_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    dialog_content.add_css_class("potoo-dialog-surface");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.add_css_class("potoo-dialog-body");
    content.set_vexpand(true);
    content.set_margin_start(22);
    content.set_margin_end(22);
    content.set_margin_top(20);
    content.set_margin_bottom(20);

    let description = gtk::Label::new(Some(tr(
        ui_language,
        "Crie uma pasta para reunir seus programas e exercícios.",
    )));
    description.set_xalign(0.0);
    description.set_wrap(true);
    description.add_css_class("potoo-dialog-intro");
    description.set_margin_bottom(18);
    content.append(&description);

    let name_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    name_field.add_css_class("potoo-dialog-field");
    name_field.set_margin_bottom(16);
    let name_label = gtk::Label::new(Some(tr(ui_language, "Nome do projeto")));
    name_label.set_xalign(0.0);
    name_label.add_css_class("potoo-project-field-label");
    let name = gtk::Entry::new();
    name.set_placeholder_text(Some(tr(ui_language, "Por exemplo: Meu primeiro projeto")));
    name.set_text(tr(ui_language, "Meu projeto"));
    name.add_css_class("potoo-project-name-entry");
    name_field.append(&name_label);
    name_field.append(&name);
    content.append(&name_field);

    let language_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    language_field.add_css_class("potoo-dialog-field");
    language_field.set_margin_bottom(16);
    language_field.set_visible(show_language_selector);
    let language_label = gtk::Label::new(Some(tr(
        ui_language,
        "Selecione a linguagem/pseudolinguagem do projeto",
    )));
    language_label.set_xalign(0.0);
    language_label.add_css_class("potoo-project-field-label");
    let language_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    language_row.set_margin_top(1);
    let portugol_button = language_option_button("Portugol", tr(ui_language, "3 variantes"));
    let potigol_button = language_option_button("Potigol", tr(ui_language, "2 variantes"));
    for button in [&portugol_button, &potigol_button] {
        button.add_css_class("potoo-project-language-choice");
        button.set_hexpand(true);
    }
    match default_language {
        Language::Portugol | Language::GPortugol | Language::Visualg => {
            portugol_button.add_css_class("selected")
        }
        Language::Potigol => potigol_button.add_css_class("selected"),
    }
    language_row.append(&portugol_button);
    language_row.append(&potigol_button);
    language_field.append(&language_label);
    language_field.append(&language_row);
    content.append(&language_field);

    let selected_template = Rc::new(Cell::new(default_template));
    let portugol_options = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    portugol_options.set_homogeneous(true);
    portugol_options.set_hexpand(true);
    let portugol_variant_buttons = [
        language_option_button("G-Portugol", ".gpt"),
        language_option_button("Portugol 2.0", ".por"),
        language_option_button("VisuAlg", ".alg"),
    ];
    let potigol_options = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    potigol_options.set_homogeneous(true);
    potigol_options.set_hexpand(true);
    let potigol_variant_buttons = [
        language_option_button("Potigol Clássico", ".poti"),
        language_option_button("Potigol Jerimum (Jogo 2D)", ".poti"),
    ];
    for button in portugol_variant_buttons
        .iter()
        .chain(potigol_variant_buttons.iter())
    {
        button.add_css_class("potoo-project-language-choice");
        button.set_hexpand(true);
    }
    for button in &portugol_variant_buttons {
        portugol_options.append(button);
    }
    for button in &potigol_variant_buttons {
        potigol_options.append(button);
    }
    let variant_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    variant_field.add_css_class("potoo-dialog-field");
    variant_field.set_margin_bottom(16);
    let variant_label = gtk::Label::new(None);
    variant_label.set_xalign(0.0);
    variant_label.add_css_class("potoo-project-field-label");
    let portugol_variant_picker =
        gtk::DropDown::from_strings(&["G-Portugol", "Portugol 2.0", "VisuAlg"]);
    portugol_variant_picker.add_css_class("potoo-settings-picker");
    portugol_variant_picker.set_hexpand(true);
    portugol_variant_picker.set_selected(project_template_variant_index(default_template));
    let potigol_variant_picker =
        gtk::DropDown::from_strings(&["Potigol Clássico", "Potigol Jerimum (Jogo 2D)"]);
    potigol_variant_picker.add_css_class("potoo-settings-picker");
    potigol_variant_picker.set_hexpand(true);
    potigol_variant_picker.set_selected(if default_template == ProjectTemplate::PotigolJerimum {
        1
    } else {
        0
    });
    variant_field.append(&variant_label);
    variant_field.append(&portugol_options);
    variant_field.append(&potigol_options);
    variant_field.append(&portugol_variant_picker);
    variant_field.append(&potigol_variant_picker);
    content.append(&variant_field);
    update_project_template_selector(
        default_template,
        ui_language,
        &variant_field,
        &variant_label,
        &portugol_options,
        [
            &portugol_variant_buttons[0],
            &portugol_variant_buttons[1],
            &portugol_variant_buttons[2],
        ],
        &potigol_options,
        [&potigol_variant_buttons[0], &potigol_variant_buttons[1]],
    );
    portugol_variant_picker.set_visible(false);
    potigol_variant_picker.set_visible(false);
    if show_language_selector {
        portugol_options.set_visible(false);
        potigol_options.set_visible(false);
        match default_language {
            Language::Portugol | Language::GPortugol | Language::Visualg => {
                portugol_variant_picker.set_visible(true)
            }
            Language::Potigol => potigol_variant_picker.set_visible(true),
        }
    }
    connect_project_template_choices(
        &portugol_variant_buttons,
        &[
            ProjectTemplate::GPortugol,
            ProjectTemplate::Portugol20,
            ProjectTemplate::Visualg,
        ],
        selected_template.clone(),
    );
    connect_project_template_choices(
        &potigol_variant_buttons,
        &[
            ProjectTemplate::PotigolClassic,
            ProjectTemplate::PotigolJerimum,
        ],
        selected_template.clone(),
    );
    if show_language_selector {
        let selected_template_for_portugol = selected_template.clone();
        portugol_variant_picker.connect_selected_notify(move |picker| {
            if let Some(template) =
                project_template_for_variant(Language::Portugol, picker.selected())
            {
                selected_template_for_portugol.set(template);
            }
        });
        let selected_template_for_potigol = selected_template.clone();
        potigol_variant_picker.connect_selected_notify(move |picker| {
            if let Some(template) =
                project_template_for_variant(Language::Potigol, picker.selected())
            {
                selected_template_for_potigol.set(template);
            }
        });
    }

    let potigol_button_for_portugol = potigol_button.clone();
    let selected_template_for_portugol = selected_template.clone();
    let variant_field_for_portugol = variant_field.clone();
    let variant_label_for_portugol = variant_label.clone();
    let portugol_options_for_portugol = portugol_options.clone();
    let portugol_buttons_for_portugol = portugol_variant_buttons.clone();
    let potigol_options_for_portugol = potigol_options.clone();
    let potigol_buttons_for_portugol = potigol_variant_buttons.clone();
    let show_variant_picker_for_portugol = show_language_selector;
    let portugol_picker_for_portugol = portugol_variant_picker.clone();
    let potigol_picker_for_portugol = potigol_variant_picker.clone();
    portugol_button.connect_clicked(move |button| {
        button.add_css_class("selected");
        potigol_button_for_portugol.remove_css_class("selected");
        let template = ProjectTemplate::Portugol20;
        selected_template_for_portugol.set(template);
        update_project_template_selector(
            template,
            ui_language,
            &variant_field_for_portugol,
            &variant_label_for_portugol,
            &portugol_options_for_portugol,
            [
                &portugol_buttons_for_portugol[0],
                &portugol_buttons_for_portugol[1],
                &portugol_buttons_for_portugol[2],
            ],
            &potigol_options_for_portugol,
            [
                &potigol_buttons_for_portugol[0],
                &potigol_buttons_for_portugol[1],
            ],
        );
        if show_variant_picker_for_portugol {
            portugol_options_for_portugol.set_visible(false);
            potigol_options_for_portugol.set_visible(false);
            portugol_picker_for_portugol.set_visible(true);
            potigol_picker_for_portugol.set_visible(false);
            portugol_picker_for_portugol.set_selected(1);
        }
    });
    let portugol_button_for_potigol = portugol_button.clone();
    let selected_template_for_potigol = selected_template.clone();
    let variant_field_for_potigol = variant_field.clone();
    let variant_label_for_potigol = variant_label.clone();
    let portugol_options_for_potigol = portugol_options.clone();
    let portugol_buttons_for_potigol = portugol_variant_buttons.clone();
    let potigol_options_for_potigol = potigol_options.clone();
    let potigol_buttons_for_potigol = potigol_variant_buttons.clone();
    let show_variant_picker_for_potigol = show_language_selector;
    let portugol_picker_for_potigol = portugol_variant_picker.clone();
    let potigol_picker_for_potigol = potigol_variant_picker.clone();
    potigol_button.connect_clicked(move |button| {
        button.add_css_class("selected");
        portugol_button_for_potigol.remove_css_class("selected");
        let template = ProjectTemplate::PotigolClassic;
        selected_template_for_potigol.set(template);
        update_project_template_selector(
            template,
            ui_language,
            &variant_field_for_potigol,
            &variant_label_for_potigol,
            &portugol_options_for_potigol,
            [
                &portugol_buttons_for_potigol[0],
                &portugol_buttons_for_potigol[1],
                &portugol_buttons_for_potigol[2],
            ],
            &potigol_options_for_potigol,
            [
                &potigol_buttons_for_potigol[0],
                &potigol_buttons_for_potigol[1],
            ],
        );
        if show_variant_picker_for_potigol {
            portugol_options_for_potigol.set_visible(false);
            potigol_options_for_potigol.set_visible(false);
            portugol_picker_for_potigol.set_visible(false);
            potigol_picker_for_potigol.set_visible(true);
            potigol_picker_for_potigol.set_selected(0);
        }
    });

    let location_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    location_field.add_css_class("potoo-dialog-field");
    let location_label = gtk::Label::new(Some(tr(ui_language, "Local do projeto")));
    location_label.set_xalign(0.0);
    location_label.add_css_class("potoo-project-field-label");
    let initial_folder = project_default_directory();
    let selected_folder = Rc::new(RefCell::new(initial_folder.clone()));
    let location_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    location_row.set_margin_top(1);
    let location = gtk::Label::new(Some(&initial_folder.display().to_string()));
    location.set_xalign(0.0);
    location.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    location.set_hexpand(true);
    location.add_css_class("potoo-project-location");
    location.set_tooltip_text(Some(&initial_folder.display().to_string()));
    let choose_folder = gtk::Button::with_label(tr(ui_language, "Escolher pasta"));
    choose_folder.add_css_class("potoo-project-location-button");
    location_row.append(&location);
    location_row.append(&choose_folder);
    location_field.append(&location_label);
    location_field.append(&location_row);
    content.append(&location_field);

    let error_label = gtk::Label::new(Some(tr(
        ui_language,
        "Digite um nome válido para o projeto.",
    )));
    error_label.set_xalign(0.0);
    error_label.add_css_class("potoo-project-error");
    error_label.set_visible(false);
    content.append(&error_label);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.add_css_class("potoo-project-actions");
    actions.add_css_class("potoo-dialog-actions");
    actions.set_hexpand(true);
    let open_project_button = gtk::Button::new();
    open_project_button.add_css_class("potoo-project-open-button");
    open_project_button.set_visible(show_language_selector);
    open_project_button.set_tooltip_text(Some(tr(ui_language, "Abrir projeto")));
    let open_project_content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    open_project_content.append(&icon("folder-open"));
    let open_project_label = gtk::Label::new(Some(tr(ui_language, "Abrir projeto")));
    open_project_content.append(&open_project_label);
    open_project_button.set_child(Some(&open_project_content));
    let actions_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    actions_spacer.set_hexpand(true);
    let cancel_button = gtk::Button::with_label(tr(ui_language, "Cancelar"));
    cancel_button.add_css_class("potoo-project-cancel-button");
    let create_button = gtk::Button::with_label(tr(ui_language, "Criar projeto"));
    create_button.add_css_class("potoo-run-button");
    actions.append(&open_project_button);
    actions.append(&actions_spacer);
    actions.append(&cancel_button);
    actions.append(&create_button);
    dialog_content.append(&content);
    dialog_content.append(&actions);
    dialog.set_child(Some(&dialog_content));

    let dialog_for_cancel = dialog.clone();
    cancel_button.connect_clicked(move |_| dialog_for_cancel.close());

    let dialog_for_open_project = dialog.clone();
    let dialog_to_close_after_open = dialog.clone();
    let sender_for_open_project = sender.clone();
    open_project_button.connect_clicked(move |_| {
        show_open_project_dialog(
            &dialog_for_open_project,
            ui_language,
            sender_for_open_project.clone(),
            Some(dialog_to_close_after_open.clone()),
        );
    });

    let dialog_for_picker = dialog.clone();
    let selected_folder_for_picker = selected_folder.clone();
    let location_for_picker = location.clone();
    let sender_for_picker = sender.clone();
    choose_folder.connect_clicked(move |_| {
        let picker = gtk::FileDialog::new();
        picker.set_title(tr(ui_language, "Escolha a pasta de destino"));
        picker.set_modal(true);
        let current_folder = selected_folder_for_picker.borrow().clone();
        picker.set_initial_folder(Some(&gtk::gio::File::for_path(current_folder)));
        let selected_folder_for_result = selected_folder_for_picker.clone();
        let location_for_result = location_for_picker.clone();
        let sender_for_result = sender_for_picker.clone();
        picker.select_folder(
            Some(&dialog_for_picker),
            None::<&gtk::gio::Cancellable>,
            move |result| match result {
                Ok(folder) => match folder.path() {
                    Some(path) => {
                        selected_folder_for_result.replace(path.clone());
                        let path_text = path.display().to_string();
                        location_for_result.set_text(&path_text);
                        location_for_result.set_tooltip_text(Some(&path_text));
                    }
                    None => sender_for_result.input(MainWindowMsg::FileOperationFailed(
                        tr(
                            ui_language,
                            "Selecione uma pasta local para criar o projeto.",
                        )
                        .to_string(),
                    )),
                },
                Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => {}
                Err(error) => sender_for_result.input(MainWindowMsg::FileOperationFailed(format!(
                    "{}: {error}",
                    tr(ui_language, "Não foi possível escolher a pasta do projeto")
                ))),
            },
        );
    });

    let dialog_for_create = dialog.clone();
    let name_for_create = name.clone();
    let error_for_create = error_label.clone();
    let selected_template_for_create = selected_template.clone();
    let selected_folder_for_create = selected_folder.clone();
    create_button.connect_clicked(move |_| {
        let project_name = name_for_create.text().to_string();
        if !is_valid_directory_name(project_name.trim()) {
            error_for_create.set_visible(true);
            return;
        }
        sender.input(MainWindowMsg::CreateProject(
            selected_folder_for_create.borrow().clone(),
            project_name,
            selected_template_for_create.get(),
        ));
        dialog_for_create.close();
    });
    let create_button_for_entry = create_button.clone();
    let error_for_name = error_label.clone();
    name.connect_changed(move |entry| {
        let name_is_valid = !entry.text().trim().is_empty();
        create_button_for_entry.set_sensitive(name_is_valid);
        if name_is_valid {
            error_for_name.set_visible(false);
        }
    });
    let create_button_for_entry = create_button.clone();
    name.connect_activate(move |_| create_button_for_entry.emit_clicked());
    dialog.present();
}

fn project_default_directory() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn project_directory_options(
    root: &Path,
    language: UiLanguage,
    files: &[ProjectFile],
) -> Vec<(String, PathBuf)> {
    let root_label = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| tr(language, "Pasta do projeto").to_string());
    let mut folders = vec![(root_label, PathBuf::new())];
    folders.extend(files.iter().filter(|file| file.is_directory).map(|file| {
        (
            file.relative_path.clone(),
            PathBuf::from(&file.relative_path),
        )
    }));
    folders
}

fn set_dialog_titlebar(window: &gtk::Window, language: UiLanguage, title: &'static str) {
    close_window_on_escape(window);

    let titlebar = gtk::HeaderBar::new();
    titlebar.add_css_class("potoo-dialog-titlebar");
    titlebar.set_show_title_buttons(false);
    let title_label = gtk::Label::new(Some(tr(language, title)));
    title_label.add_css_class("potoo-settings-window-title");
    titlebar.set_title_widget(Some(&title_label));

    let close_button = gtk::Button::new();
    close_button.add_css_class("potoo-dialog-close");
    close_button.set_child(Some(&icon("close")));
    close_button.set_tooltip_text(Some(tr(language, "Fechar")));
    titlebar.pack_end(&close_button);
    window.set_titlebar(Some(&titlebar));

    let window_for_close = window.clone();
    close_button.connect_clicked(move |_| window_for_close.close());
}

fn close_window_on_escape(window: &gtk::Window) {
    let key_controller = gtk::EventControllerKey::new();
    let weak_window = window.downgrade();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            if let Some(window) = weak_window.upgrade() {
                window.close();
            }
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    window.add_controller(key_controller);
}

fn language_option_button(name: &str, detail: &str) -> gtk::Button {
    let button = gtk::Button::new();
    let choice = gtk::Box::new(gtk::Orientation::Vertical, 3);
    choice.set_halign(gtk::Align::Start);
    let name_label = gtk::Label::new(Some(name));
    name_label.set_xalign(0.0);
    name_label.add_css_class("potoo-project-language-name");
    let detail_label = gtk::Label::new(Some(detail));
    detail_label.set_xalign(0.0);
    detail_label.add_css_class("potoo-project-language-detail");
    choice.append(&name_label);
    choice.append(&detail_label);
    button.set_child(Some(&choice));
    button.set_tooltip_text(Some(detail));
    button
}

fn show_new_file_dialog(
    parent: &gtk::ApplicationWindow,
    default_language: Language,
    ui_language: UiLanguage,
    folder_options: Vec<(String, PathBuf)>,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(ui_language, "Novo arquivo")));
    dialog.set_default_size(680, 510);
    dialog.set_resizable(false);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_css_class("potoo-window");
    dialog.add_css_class("potoo-dialog-window");
    if initial_dark {
        dialog.add_css_class("potoo-dark");
    }
    set_dialog_titlebar(&dialog, ui_language, "Novo arquivo");

    let dialog_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    dialog_content.add_css_class("potoo-dialog-surface");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.add_css_class("potoo-dialog-body");
    content.set_vexpand(true);
    content.set_margin_start(22);
    content.set_margin_end(22);
    content.set_margin_top(20);
    content.set_margin_bottom(20);
    let description = gtk::Label::new(Some(tr(
        ui_language,
        "Crie um arquivo dentro do projeto atual.",
    )));
    description.set_xalign(0.0);
    description.add_css_class("potoo-dialog-intro");
    description.set_margin_bottom(18);
    content.append(&description);

    let name_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    name_field.add_css_class("potoo-dialog-field");
    name_field.set_margin_bottom(16);
    let name_label = gtk::Label::new(Some(tr(ui_language, "Nome do arquivo")));
    name_label.set_xalign(0.0);
    name_label.add_css_class("potoo-project-field-label");
    let name = gtk::Entry::new();
    name.set_placeholder_text(Some(tr(
        ui_language,
        "Nome do arquivo, por exemplo: exercicio-1",
    )));
    name.set_text(tr(ui_language, "novo-arquivo"));
    name.add_css_class("potoo-project-name-entry");
    name_field.append(&name_label);
    name_field.append(&name);
    content.append(&name_field);

    let language_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    language_field.add_css_class("potoo-dialog-field");
    language_field.set_margin_bottom(16);
    let language_label = gtk::Label::new(Some(tr(ui_language, "Linguagem/pseudolinguagem")));
    language_label.set_xalign(0.0);
    language_label.add_css_class("potoo-project-field-label");
    let language_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    language_row.set_margin_top(1);
    let portugol_button = language_option_button("Portugol", tr(ui_language, "3 variantes"));
    let potigol_button = language_option_button("Potigol", tr(ui_language, "2 variantes"));
    for button in [&portugol_button, &potigol_button] {
        button.add_css_class("potoo-project-language-choice");
        button.set_hexpand(true);
    }
    let default_template = ProjectTemplate::default_for_language(default_language);
    match default_template.language() {
        Language::Portugol | Language::Visualg | Language::GPortugol => {
            portugol_button.add_css_class("selected")
        }
        Language::Potigol => potigol_button.add_css_class("selected"),
    }
    language_row.append(&portugol_button);
    language_row.append(&potigol_button);
    language_field.append(&language_label);
    language_field.append(&language_row);
    content.append(&language_field);

    let selected_template = Rc::new(Cell::new(default_template));
    let variant_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    variant_field.add_css_class("potoo-dialog-field");
    variant_field.set_margin_bottom(16);
    let variant_label = gtk::Label::new(None);
    variant_label.set_xalign(0.0);
    variant_label.add_css_class("potoo-project-field-label");
    let portugol_variant_picker =
        gtk::DropDown::from_strings(&["G-Portugol", "Portugol 2.0", "VisuAlg"]);
    portugol_variant_picker.add_css_class("potoo-settings-picker");
    portugol_variant_picker.set_hexpand(true);
    portugol_variant_picker.set_selected(match default_template {
        ProjectTemplate::GPortugol => 0,
        ProjectTemplate::Visualg => 2,
        _ => 1,
    });
    let potigol_variant_picker =
        gtk::DropDown::from_strings(&["Potigol Clássico", "Potigol Jerimum (Jogo 2D)"]);
    potigol_variant_picker.add_css_class("potoo-settings-picker");
    potigol_variant_picker.set_hexpand(true);
    potigol_variant_picker.set_selected(if default_template == ProjectTemplate::PotigolJerimum {
        1
    } else {
        0
    });
    variant_field.append(&variant_label);
    variant_field.append(&portugol_variant_picker);
    variant_field.append(&potigol_variant_picker);
    content.append(&variant_field);
    match default_template.language() {
        Language::Portugol | Language::Visualg | Language::GPortugol => {
            variant_label.set_text(tr(ui_language, "Variante do Portugol"));
            potigol_variant_picker.set_visible(false);
        }
        Language::Potigol => {
            variant_label.set_text(tr(ui_language, "Variante do Potigol"));
            portugol_variant_picker.set_visible(false);
        }
    }
    portugol_variant_picker.connect_selected_notify({
        let selected_template = selected_template.clone();
        move |picker| {
            if let Some(template) =
                project_template_for_variant(Language::Portugol, picker.selected())
            {
                selected_template.set(template);
            }
        }
    });
    potigol_variant_picker.connect_selected_notify({
        let selected_template = selected_template.clone();
        move |picker| {
            if let Some(template) =
                project_template_for_variant(Language::Potigol, picker.selected())
            {
                selected_template.set(template);
            }
        }
    });

    let potigol_for_portugol = potigol_button.clone();
    let selected_for_portugol = selected_template.clone();
    let variant_label_for_portugol = variant_label.clone();
    let portugol_picker_for_portugol = portugol_variant_picker.clone();
    let potigol_picker_for_portugol = potigol_variant_picker.clone();
    portugol_button.connect_clicked(move |button| {
        button.add_css_class("selected");
        potigol_for_portugol.remove_css_class("selected");
        let template = ProjectTemplate::Portugol20;
        selected_for_portugol.set(template);
        variant_label_for_portugol.set_text(tr(ui_language, "Variante do Portugol"));
        portugol_picker_for_portugol.set_visible(true);
        potigol_picker_for_portugol.set_visible(false);
        portugol_picker_for_portugol.set_selected(1);
    });
    let portugol_for_potigol = portugol_button.clone();
    let selected_for_potigol = selected_template.clone();
    let variant_label_for_potigol = variant_label.clone();
    let portugol_picker_for_potigol = portugol_variant_picker.clone();
    let potigol_picker_for_potigol = potigol_variant_picker.clone();
    potigol_button.connect_clicked(move |button| {
        button.add_css_class("selected");
        portugol_for_potigol.remove_css_class("selected");
        let template = ProjectTemplate::PotigolClassic;
        selected_for_potigol.set(template);
        variant_label_for_potigol.set_text(tr(ui_language, "Variante do Potigol"));
        portugol_picker_for_potigol.set_visible(false);
        potigol_picker_for_potigol.set_visible(true);
        potigol_picker_for_potigol.set_selected(0);
    });

    let folder_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    folder_field.add_css_class("potoo-dialog-field");
    let folder_label = gtk::Label::new(Some(tr(ui_language, "Pasta")));
    folder_label.set_xalign(0.0);
    folder_label.add_css_class("potoo-project-field-label");
    let folder_names = folder_options
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    let folder_picker = gtk::DropDown::from_strings(&folder_names);
    folder_picker.add_css_class("potoo-settings-picker");
    folder_picker.set_hexpand(true);
    folder_picker.set_margin_top(1);
    folder_field.append(&folder_label);
    folder_field.append(&folder_picker);
    content.append(&folder_field);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.add_css_class("potoo-dialog-actions");
    actions.set_hexpand(true);
    let actions_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    actions_spacer.set_hexpand(true);
    let cancel_button = gtk::Button::with_label(tr(ui_language, "Cancelar"));
    cancel_button.add_css_class("potoo-project-cancel-button");
    let create_button = gtk::Button::with_label(tr(ui_language, "Criar arquivo"));
    create_button.add_css_class("potoo-run-button");
    actions.append(&actions_spacer);
    actions.append(&cancel_button);
    actions.append(&create_button);
    dialog_content.append(&content);
    dialog_content.append(&actions);
    dialog.set_child(Some(&dialog_content));

    let dialog_for_cancel = dialog.clone();
    cancel_button.connect_clicked(move |_| dialog_for_cancel.close());
    let dialog_for_create = dialog.clone();
    let name_for_create = name.clone();
    let selected_template_for_create = selected_template.clone();
    let folder_picker_for_create = folder_picker.clone();
    create_button.connect_clicked(move |_| {
        let relative_folder = folder_options
            .get(folder_picker_for_create.selected() as usize)
            .map(|(_, path)| path.clone())
            .unwrap_or_default();
        sender.input(MainWindowMsg::CreateFile(
            name_for_create.text().to_string(),
            selected_template_for_create.get(),
            relative_folder,
        ));
        dialog_for_create.close();
    });
    let create_button_for_entry = create_button.clone();
    name.connect_activate(move |_| create_button_for_entry.emit_clicked());
    dialog.present();
}

fn show_rename_project_item_dialog(
    parent: &gtk::ApplicationWindow,
    path: PathBuf,
    is_directory: bool,
    ui_language: UiLanguage,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    let title = if is_directory {
        "Renomear pasta"
    } else {
        "Renomear arquivo"
    };
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(ui_language, title)));
    dialog.set_default_size(520, 260);
    dialog.set_resizable(false);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_css_class("potoo-window");
    dialog.add_css_class("potoo-dialog-window");
    if initial_dark {
        dialog.add_css_class("potoo-dark");
    }
    set_dialog_titlebar(&dialog, ui_language, title);

    let dialog_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    dialog_content.add_css_class("potoo-dialog-surface");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.add_css_class("potoo-dialog-body");
    content.set_vexpand(true);
    content.set_margin_start(22);
    content.set_margin_end(22);
    content.set_margin_top(20);
    content.set_margin_bottom(20);

    let description = gtk::Label::new(Some(tr(
        ui_language,
        "Escolha um novo nome para este item do projeto.",
    )));
    description.set_xalign(0.0);
    description.add_css_class("potoo-dialog-intro");
    description.set_margin_bottom(18);
    content.append(&description);

    let name_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    name_field.add_css_class("potoo-dialog-field");
    let name_label = gtk::Label::new(Some(tr(ui_language, "Novo nome")));
    name_label.set_xalign(0.0);
    name_label.add_css_class("potoo-project-field-label");
    let name = gtk::Entry::new();
    if let Some(file_name) = path.file_name() {
        name.set_text(&file_name.to_string_lossy());
    }
    name.add_css_class("potoo-project-name-entry");
    name.select_region(0, -1);
    let error = gtk::Label::new(None);
    error.set_xalign(0.0);
    error.set_wrap(true);
    error.add_css_class("potoo-project-error");
    error.set_visible(false);
    name_field.append(&name_label);
    name_field.append(&name);
    name_field.append(&error);
    content.append(&name_field);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.add_css_class("potoo-dialog-actions");
    actions.set_hexpand(true);
    let actions_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    actions_spacer.set_hexpand(true);
    let cancel_button = gtk::Button::with_label(tr(ui_language, "Cancelar"));
    cancel_button.add_css_class("potoo-project-cancel-button");
    let rename_button = gtk::Button::with_label(tr(ui_language, "Renomear"));
    rename_button.add_css_class("potoo-run-button");
    actions.append(&actions_spacer);
    actions.append(&cancel_button);
    actions.append(&rename_button);
    dialog_content.append(&content);
    dialog_content.append(&actions);
    dialog.set_child(Some(&dialog_content));

    let dialog_for_cancel = dialog.clone();
    cancel_button.connect_clicked(move |_| dialog_for_cancel.close());

    let dialog_for_rename = dialog.clone();
    let name_for_rename = name.clone();
    let error_for_rename = error.clone();
    let path_for_rename = path.clone();
    rename_button.connect_clicked(move |_| {
        let new_name = match normalized_project_item_name(
            &path_for_rename,
            &name_for_rename.text(),
            is_directory,
            ui_language,
        ) {
            Ok(name) => name,
            Err(message) => {
                error_for_rename.set_text(&message);
                error_for_rename.set_visible(true);
                return;
            }
        };
        let Some(parent) = path_for_rename.parent() else {
            error_for_rename.set_text(tr(
                ui_language,
                "O item selecionado não está mais disponível.",
            ));
            error_for_rename.set_visible(true);
            return;
        };
        let new_path = parent.join(&new_name);
        if new_path.exists()
            && fs::canonicalize(&new_path).ok() != fs::canonicalize(&path_for_rename).ok()
        {
            error_for_rename.set_text(tr(ui_language, "Já existe um item com esse nome."));
            error_for_rename.set_visible(true);
            return;
        }
        sender.input(MainWindowMsg::RenameProjectItemConfirmed(
            path_for_rename.clone(),
            new_name,
        ));
        dialog_for_rename.close();
    });
    let rename_button_for_entry = rename_button.clone();
    name.connect_activate(move |_| rename_button_for_entry.emit_clicked());
    dialog.present();
}

fn show_new_folder_dialog(
    parent: &gtk::ApplicationWindow,
    ui_language: UiLanguage,
    folder_options: Vec<(String, PathBuf)>,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(ui_language, "Nova pasta")));
    dialog.set_default_size(520, 340);
    dialog.set_resizable(false);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_css_class("potoo-window");
    dialog.add_css_class("potoo-dialog-window");
    if initial_dark {
        dialog.add_css_class("potoo-dark");
    }
    set_dialog_titlebar(&dialog, ui_language, "Nova pasta");

    let dialog_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    dialog_content.add_css_class("potoo-dialog-surface");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.add_css_class("potoo-dialog-body");
    content.set_vexpand(true);
    content.set_margin_start(22);
    content.set_margin_end(22);
    content.set_margin_top(20);
    content.set_margin_bottom(20);
    let description = gtk::Label::new(Some(tr(
        ui_language,
        "Organize seus programas em uma pasta do projeto.",
    )));
    description.set_xalign(0.0);
    description.add_css_class("potoo-dialog-intro");
    description.set_margin_bottom(18);
    content.append(&description);

    let name_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    name_field.add_css_class("potoo-dialog-field");
    name_field.set_margin_bottom(16);
    let name_label = gtk::Label::new(Some(tr(ui_language, "Nome da pasta")));
    name_label.set_xalign(0.0);
    name_label.add_css_class("potoo-project-field-label");
    let name = gtk::Entry::new();
    name.set_placeholder_text(Some(tr(
        ui_language,
        "Nome da pasta, por exemplo: exercicios",
    )));
    name.add_css_class("potoo-project-name-entry");
    name_field.append(&name_label);
    name_field.append(&name);
    content.append(&name_field);

    let parent_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    parent_field.add_css_class("potoo-dialog-field");
    let parent_label = gtk::Label::new(Some(tr(ui_language, "Criar dentro de")));
    parent_label.set_xalign(0.0);
    parent_label.add_css_class("potoo-project-field-label");
    let folder_names = folder_options
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    let folder_picker = gtk::DropDown::from_strings(&folder_names);
    folder_picker.add_css_class("potoo-settings-picker");
    folder_picker.set_hexpand(true);
    folder_picker.set_margin_top(1);
    parent_field.append(&parent_label);
    parent_field.append(&folder_picker);
    content.append(&parent_field);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.add_css_class("potoo-dialog-actions");
    actions.set_hexpand(true);
    let actions_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    actions_spacer.set_hexpand(true);
    let cancel_button = gtk::Button::with_label(tr(ui_language, "Cancelar"));
    cancel_button.add_css_class("potoo-project-cancel-button");
    let create_button = gtk::Button::with_label(tr(ui_language, "Criar pasta"));
    create_button.add_css_class("potoo-run-button");
    actions.append(&actions_spacer);
    actions.append(&cancel_button);
    actions.append(&create_button);
    dialog_content.append(&content);
    dialog_content.append(&actions);
    dialog.set_child(Some(&dialog_content));

    let dialog_for_cancel = dialog.clone();
    cancel_button.connect_clicked(move |_| dialog_for_cancel.close());
    let dialog_for_create = dialog.clone();
    let name_for_create = name.clone();
    let folder_picker_for_create = folder_picker.clone();
    create_button.connect_clicked(move |_| {
        let relative_folder = folder_options
            .get(folder_picker_for_create.selected() as usize)
            .map(|(_, path)| path.clone())
            .unwrap_or_default();
        sender.input(MainWindowMsg::CreateFolder(
            name_for_create.text().to_string(),
            relative_folder,
        ));
        dialog_for_create.close();
    });
    let create_button_for_entry = create_button.clone();
    name.connect_activate(move |_| create_button_for_entry.emit_clicked());
    dialog.present();
}

fn show_save_dialog(
    parent: &gtk::ApplicationWindow,
    document: &SourceDocument,
    project_root: Option<&Path>,
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title(tr(language, "Salvar arquivo"));
    dialog.set_modal(true);
    dialog.set_initial_name(Some(&document.name));
    if let Some(folder) = document
        .path
        .as_deref()
        .and_then(Path::parent)
        .or(project_root)
    {
        dialog.set_initial_folder(Some(&gtk::gio::File::for_path(folder)));
    }
    let id = document.id;
    dialog.save(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |result| match result {
            Ok(file) => match file.path() {
                Some(path) => sender.input(MainWindowMsg::SaveAsSelected(id, path)),
                None => {
                    sender.input(MainWindowMsg::SaveAsCanceled(id));
                    sender.input(MainWindowMsg::FileOperationFailed(
                        tr(language, "Escolha um caminho local para salvar o arquivo.").to_string(),
                    ));
                }
            },
            Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => {
                sender.input(MainWindowMsg::SaveAsCanceled(id));
            }
            Err(error) => {
                sender.input(MainWindowMsg::SaveAsCanceled(id));
                sender.input(MainWindowMsg::FileOperationFailed(format!(
                    "{}: {error}",
                    tr(language, "Não foi possível abrir a janela de salvamento")
                )));
            }
        },
    );
}

fn show_project_switch_confirmation(
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    show_potoo_confirmation(
        parent,
        language,
        initial_dark,
        "Alterações não salvas",
        "Alterações não salvas",
        "Salve os arquivos abertos antes de trocar de pasta ou descarte as alterações para continuar.",
        vec!["Cancelar", "Descartar e abrir"],
        Some(1),
        None,
        move |response| sender.input(MainWindowMsg::ProjectSwitchDecision(response == 1)),
    );
}

fn show_close_document_confirmation(
    parent: &gtk::ApplicationWindow,
    id: u64,
    language: UiLanguage,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    show_potoo_confirmation(
        parent,
        language,
        initial_dark,
        "Alterações não salvas",
        "O arquivo aberto tem alterações que ainda não foram salvas. O que você gostaria de fazer?",
        "Salve o arquivo antes de fechar ou descarte as alterações para continuar.",
        vec![
            "Continuar editando",
            "Descartar alterações",
            "Salvar e fechar",
        ],
        Some(2),
        None,
        move |response| match response {
            1 => sender.input(MainWindowMsg::CloseDocumentDecision(id, true)),
            2 => sender.input(MainWindowMsg::SaveAndCloseDocument(id)),
            _ => sender.input(MainWindowMsg::CloseDocumentDecision(id, false)),
        },
    );
}

fn show_close_app_confirmation(
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    show_potoo_confirmation(
        parent,
        language,
        initial_dark,
        "Alterações não salvas",
        "Alterações não salvas",
        "Salve os arquivos antes de sair ou descarte as alterações para fechar o Potoo IDE.",
        vec!["Continuar editando", "Descartar e sair"],
        Some(1),
        None,
        move |response| sender.input(MainWindowMsg::CloseAppDecision(response == 1)),
    );
}

fn show_potoo_confirmation(
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    initial_dark: bool,
    title: &'static str,
    message: &'static str,
    detail: &'static str,
    actions: Vec<&'static str>,
    primary_action: Option<usize>,
    destructive_action: Option<usize>,
    on_response: impl Fn(usize) + 'static,
) {
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(language, title)));
    dialog.set_default_size(520, -1);
    dialog.set_resizable(false);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_css_class("potoo-window");
    dialog.add_css_class("potoo-dialog-window");
    if initial_dark {
        dialog.add_css_class("potoo-dark");
    }
    set_dialog_titlebar(&dialog, language, title);

    let surface = gtk::Box::new(gtk::Orientation::Vertical, 0);
    surface.add_css_class("potoo-dialog-surface");

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    content.add_css_class("potoo-confirm-content");
    content.set_margin_start(22);
    content.set_margin_end(22);
    content.set_margin_top(22);
    content.set_margin_bottom(22);

    let mark = gtk::Label::new(Some("!"));
    mark.add_css_class("potoo-confirm-mark");
    mark.set_valign(gtk::Align::Start);
    content.append(&mark);

    let copy = gtk::Box::new(gtk::Orientation::Vertical, 6);
    copy.set_valign(gtk::Align::Center);
    let message_label = gtk::Label::new(Some(tr(language, message)));
    message_label.set_xalign(0.0);
    message_label.set_wrap(true);
    message_label.add_css_class("potoo-confirm-message");
    let detail_label = gtk::Label::new(Some(tr(language, detail)));
    detail_label.set_xalign(0.0);
    detail_label.set_wrap(true);
    detail_label.add_css_class("potoo-confirm-detail");
    copy.append(&message_label);
    copy.append(&detail_label);
    content.append(&copy);
    surface.append(&content);

    let actions_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions_row.add_css_class("potoo-dialog-actions");
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    actions_row.append(&spacer);

    let response_sent = Rc::new(Cell::new(false));
    let on_response: Rc<dyn Fn(usize)> = Rc::new(on_response);
    for (index, label) in actions.into_iter().enumerate() {
        let button = gtk::Button::with_label(tr(language, label));
        button.add_css_class(if Some(index) == destructive_action {
            "potoo-destructive-button"
        } else if Some(index) == primary_action {
            "potoo-run-button"
        } else {
            "potoo-project-cancel-button"
        });
        let dialog_for_button = dialog.clone();
        let response_sent_for_button = response_sent.clone();
        let on_response_for_button = on_response.clone();
        button.connect_clicked(move |_| {
            if !response_sent_for_button.replace(true) {
                on_response_for_button(index);
            }
            dialog_for_button.close();
        });
        actions_row.append(&button);
    }
    surface.append(&actions_row);
    dialog.set_child(Some(&surface));

    let response_sent_for_close = response_sent;
    dialog.connect_close_request(move |_| {
        if !response_sent_for_close.replace(true) {
            on_response(0);
        }
        gtk::glib::Propagation::Proceed
    });
    dialog.present();
}

fn new_source_path(
    root: &Path,
    name: &str,
    default_language: Language,
    ui_language: UiLanguage,
) -> Result<(PathBuf, Language), String> {
    let name = name.trim();
    if name.is_empty()
        || name.starts_with('.')
        || name.ends_with('.')
        || name.contains('/')
        || name.contains('\\')
        || is_reserved_windows_file_name(name)
        || name
            .chars()
            .any(|character| character.is_control() || "<>:\"|?*".contains(character))
    {
        return Err(tr(
            ui_language,
            "Use um nome simples, sem pastas ou caracteres especiais.",
        )
        .to_string());
    }

    let requested_path = Path::new(name);
    let language = match requested_path.extension() {
        Some(_) => Language::from_path(requested_path).ok_or_else(|| {
            tr(ui_language, "Use a extensão .por, .alg, .poti ou .gpt.").to_string()
        })?,
        None => default_language,
    };
    let file_name = if requested_path.extension().is_some() {
        name.to_string()
    } else {
        format!("{name}.{}", language.extension())
    };
    Ok((root.join(file_name), language))
}

fn resolve_project_directory(
    root: &Path,
    relative_directory: &Path,
    ui_language: UiLanguage,
) -> Result<PathBuf, String> {
    if relative_directory
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(tr(ui_language, "A pasta selecionada não está disponível.").to_string());
    }

    let project_root = fs::canonicalize(root)
        .map_err(|_| tr(ui_language, "A pasta selecionada não está disponível.").to_string())?;
    let directory = fs::canonicalize(project_root.join(relative_directory))
        .map_err(|_| tr(ui_language, "A pasta selecionada não está disponível.").to_string())?;
    if directory.is_dir() && directory.starts_with(&project_root) {
        Ok(directory)
    } else {
        Err(tr(ui_language, "A pasta selecionada não está disponível.").to_string())
    }
}

fn is_reserved_windows_file_name(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or(name)
        .trim_end_matches(' ')
        .to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix).is_some_and(|number| {
            matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
    })
}

fn is_valid_directory_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.ends_with('.')
        && !name.contains('/')
        && !name.contains('\\')
        && !is_reserved_windows_file_name(name)
        && !name
            .chars()
            .any(|character| character.is_control() || "<>:\"|?*".contains(character))
}

fn normalized_project_item_name(
    path: &Path,
    name: &str,
    is_directory: bool,
    ui_language: UiLanguage,
) -> Result<String, String> {
    let name = name.trim();
    if !is_valid_directory_name(name) {
        return Err(tr(
            ui_language,
            "Use um nome simples, sem pastas ou caracteres especiais.",
        )
        .to_string());
    }

    if !is_directory && let Some(language) = Language::from_path(path) {
        let requested_path = Path::new(name);
        return match requested_path.extension() {
            None => Ok(format!("{name}.{}", language.extension())),
            Some(_) if Language::from_path(requested_path).is_some() => Ok(name.to_string()),
            Some(_) => {
                Err(tr(ui_language, "Use a extensão .por, .alg, .poti ou .gpt.").to_string())
            }
        };
    }

    Ok(name.to_string())
}

fn supported_save_path(
    path: PathBuf,
    default_language: Language,
    ui_language: UiLanguage,
) -> Result<(PathBuf, Language), String> {
    match path.extension() {
        Some(_) => Language::from_path(&path)
            .map(|language| (path, language))
            .ok_or_else(|| {
                tr(
                    ui_language,
                    "Salve o código com a extensão .por, .alg, .poti ou .gpt.",
                )
                .to_string()
            }),
        None => Ok((
            path.with_extension(default_language.extension()),
            default_language,
        )),
    }
}

fn scan_project_files(root: &Path) -> std::io::Result<Vec<ProjectFile>> {
    let mut entries = Vec::new();
    scan_source_directory(root, root, &mut entries)?;
    Ok(entries)
}

fn scan_source_directory(
    root: &Path,
    directory: &Path,
    project_entries: &mut Vec<ProjectFile>,
) -> std::io::Result<()> {
    let mut children = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    children.sort_by(|left, right| {
        let left_is_directory = left.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        let right_is_directory = right.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        right_is_directory.cmp(&left_is_directory).then_with(|| {
            left.file_name()
                .to_string_lossy()
                .to_lowercase()
                .cmp(&right.file_name().to_string_lossy().to_lowercase())
        })
    });
    for entry in children {
        let name = entry.file_name();
        let visible_name = name.to_string_lossy();
        if visible_name.starts_with('.')
            || matches!(visible_name.as_ref(), "target" | "node_modules")
        {
            continue;
        }
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            project_entries.push(ProjectFile {
                relative_path: path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned(),
                path: path.clone(),
                is_directory: true,
            });
            scan_source_directory(root, &path, project_entries)?;
        } else if file_type.is_file() {
            project_entries.push(ProjectFile {
                relative_path: path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned(),
                path,
                is_directory: false,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod ui_language_tests {
    use super::{
        UI_TRANSLATION_KEYS, UiLanguage, learning_copy, new_source_path,
        normalized_project_item_name, project_scan_is_current, supported_save_path, tr,
        translate_known_text,
    };
    use crate::components::content_panel::Language;
    use crate::learning::{LearningTopic, learning_hint};
    use std::path::{Path, PathBuf};

    #[test]
    fn detects_supported_system_locales() {
        assert_eq!(
            UiLanguage::from_locale_name("pt_BR.UTF-8"),
            Some(UiLanguage::Portuguese)
        );
        assert_eq!(
            UiLanguage::from_locale_name("es-MX"),
            Some(UiLanguage::Spanish)
        );
        assert_eq!(
            UiLanguage::from_locale_name("en_GB"),
            Some(UiLanguage::English)
        );
        assert_eq!(UiLanguage::from_locale_name("fr_FR"), None);
    }

    #[test]
    fn uses_first_supported_system_language_or_english() {
        assert_eq!(
            UiLanguage::from_language_names(["fr_FR", "es_MX", "pt_BR"]),
            UiLanguage::Spanish
        );
        assert_eq!(
            UiLanguage::from_language_names(["fr_FR", "C"]),
            UiLanguage::English
        );
    }

    #[test]
    fn saved_language_preference_takes_precedence_over_system_language() {
        assert_eq!(
            UiLanguage::from_preference(Some("en"), ["pt_BR"]),
            UiLanguage::English
        );
        assert_eq!(
            UiLanguage::from_preference(Some("pt-BR"), ["en_US"]),
            UiLanguage::Portuguese
        );
        assert_eq!(
            UiLanguage::from_preference(None, ["es_ES"]),
            UiLanguage::Spanish
        );
        assert_eq!(
            UiLanguage::from_preference(Some("invalid"), ["fr_FR"]),
            UiLanguage::English
        );
    }

    #[test]
    fn translates_existing_interface_copy_between_supported_languages() {
        for key in UI_TRANSLATION_KEYS {
            for from in [
                UiLanguage::Portuguese,
                UiLanguage::English,
                UiLanguage::Spanish,
            ] {
                for to in [
                    UiLanguage::Portuguese,
                    UiLanguage::English,
                    UiLanguage::Spanish,
                ] {
                    assert_eq!(
                        translate_known_text(from, to, tr(from, key)),
                        Some(tr(to, key).to_string()),
                        "failed translating {key:?} from {from:?} to {to:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn gportugol_creation_save_and_rename_preserve_dialect_extensions() {
        let root = Path::new("/tmp/potoo-project");
        let (created, language) = new_source_path(
            root,
            "principal",
            Language::GPortugol,
            UiLanguage::Portuguese,
        )
        .expect("a G-Portugol starter file should be created with its own extension");
        assert_eq!(created, root.join("principal.gpt"));
        assert_eq!(language, Language::GPortugol);

        let (saved, language) = supported_save_path(
            PathBuf::from("/tmp/edited.gpt"),
            Language::Portugol,
            UiLanguage::Portuguese,
        )
        .expect("G-Portugol files should be saveable");
        assert_eq!(saved, PathBuf::from("/tmp/edited.gpt"));
        assert_eq!(language, Language::GPortugol);
        assert_eq!(
            normalized_project_item_name(&created, "renamed", false, UiLanguage::Portuguese),
            Ok("renamed.gpt".to_string())
        );
        assert_eq!(
            new_source_path(
                root,
                "main.por",
                Language::GPortugol,
                UiLanguage::Portuguese
            )
            .expect("the .por extension must remain associated with Portugol 2.0")
            .1,
            Language::Portugol
        );
    }

    #[test]
    fn learning_copy_teaches_dialect_types_and_array_shapes() {
        let portugol = "programa {\n funcao vazio registrar(inteiro valor) {}\n funcao cadeia nome(inteiro valor) { retorne \"Ana\" }\n funcao vazio inicio() {\n  inteiro valores[3]\n  inteiro matriz[2][3]\n }\n}";
        let void_function = learning_hint(portugol, 2, Language::Portugol);
        let (void_text, _, _) = learning_copy(&void_function, UiLanguage::Portuguese);
        assert!(void_text.contains("vazio"));
        assert!(void_text.contains("não devolve um valor"));

        let text_function = learning_hint(portugol, 3, Language::Portugol);
        let (text_explanation, text_concept, _) =
            learning_copy(&text_function, UiLanguage::English);
        assert!(text_explanation.contains("cadeia"));
        assert!(text_concept.contains("text"));

        let vector = learning_hint(portugol, 5, Language::Portugol);
        let (vector_text, _, _) = learning_copy(&vector, UiLanguage::Portuguese);
        assert!(vector_text.contains("vetor com 3 posições"));

        let matrix = learning_hint(portugol, 6, Language::Portugol);
        let (matrix_text, _, _) = learning_copy(&matrix, UiLanguage::Portuguese);
        assert!(matrix_text.contains("matriz com 2 dimensões"));
        assert_eq!(matrix.topic, LearningTopic::Variable);

        let visualg =
            "algoritmo \"aula\"\nvar\ndados: vetor[2..5] de inteiro\ninicio\nfimalgoritmo\n";
        let vector = learning_hint(visualg, 3, Language::Visualg);
        let (visualg_text, _, _) = learning_copy(&vector, UiLanguage::Portuguese);
        assert!(visualg_text.contains("índices de 2 até 5"));
    }

    #[test]
    fn learning_copy_uses_gportugol_terms_and_types() {
        let source = "algoritmo aula;\n\
variáveis\n\
  texto: literal;\n\
  valores: matriz[4] de caracteres;\n\
fim-variáveis\n\
início\n\
  imprima(\"Olá\");\n\
fim\n\
função nome(): literal\n\
início\n\
  retorne \"Ana\";\n\
fim";

        let (text_variable, _, _) = learning_copy(
            &learning_hint(source, 3, Language::GPortugol),
            UiLanguage::English,
        );
        assert!(text_variable.contains("literal"));
        assert!(text_variable.contains("type used to store text"));

        let (vector, _, _) = learning_copy(
            &learning_hint(source, 4, Language::GPortugol),
            UiLanguage::Portuguese,
        );
        assert!(vector.contains("índices de 0 até 3"));

        let (output, _, _) = learning_copy(
            &learning_hint(source, 7, Language::GPortugol),
            UiLanguage::English,
        );
        assert!(output.contains("imprima"));
        assert!(!output.contains("escreva"));

        let (function, concept, _) = learning_copy(
            &learning_hint(source, 9, Language::GPortugol),
            UiLanguage::English,
        );
        assert!(function.contains("literal"));
        assert!(concept.contains("literal"));
        assert!(concept.contains("G-Portugol"));
    }

    #[test]
    fn learning_copy_discloses_potigol_heuristics_and_uses_neutral_procedure_concept() {
        let potigol = learning_copy(
            &learning_hint("soma = 1 + 2", 1, Language::Potigol),
            UiLanguage::Portuguese,
        );
        assert!(potigol.0.contains("heurística"));

        let procedure_hint = super::LearningHint {
            line: 1,
            column: 1,
            language: Language::Portugol,
            topic: LearningTopic::Procedure,
            name: Some("registrar".to_owned()),
            return_type: None,
            parameters: Vec::new(),
            variables: Vec::new(),
            expression: None,
            experiment_expression: None,
            problems: Vec::new(),
        };
        let procedure = learning_copy(&procedure_hint, UiLanguage::Portuguese);
        assert!(procedure.1.contains("rotina"));
        assert!(!procedure.1.contains("VisuAlg"));
    }

    #[test]
    fn atomic_save_replaces_file_contents_without_leaving_temporary_files() {
        let directory = std::env::temp_dir().join(format!(
            "potoo-atomic-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("programa.por");
        std::fs::write(&path, "versão antiga").unwrap();

        super::atomic_write(&path, "versão nova".as_bytes()).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "versão nova");
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn project_scan_results_are_rejected_when_the_workspace_generation_is_stale() {
        let old_root = Path::new("/tmp/old-project");
        let new_root = Path::new("/tmp/new-project");
        assert!(project_scan_is_current(
            4,
            9,
            Some(old_root),
            4,
            9,
            new_root,
            true
        ));
        assert!(!project_scan_is_current(
            5,
            9,
            Some(old_root),
            4,
            9,
            new_root,
            true
        ));
        assert!(!project_scan_is_current(
            4,
            10,
            Some(old_root),
            4,
            9,
            old_root,
            false
        ));
        assert!(!project_scan_is_current(
            4,
            9,
            Some(old_root),
            4,
            9,
            new_root,
            false
        ));
    }
}

#[cfg(test)]
mod input_cancel_tests {
    use super::input_cancel_controller;
    use gtk::prelude::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    #[ignore = "requires a GTK display; run with xvfb-run"]
    fn escape_clears_pending_input_and_requests_cancellation() {
        gtk::init().expect("GTK should initialize under Xvfb");
        let entry = gtk::Entry::new();
        entry.set_text("entrada não enviada");
        let cancel_count = Rc::new(Cell::new(0));
        let cancel_count_for_callback = cancel_count.clone();
        let controller = input_cancel_controller(&entry, move || {
            cancel_count_for_callback.set(cancel_count_for_callback.get() + 1);
        });
        entry.add_controller(controller.clone());

        let normal_key = controller.emit_by_name::<bool>(
            "key-pressed",
            &[&gtk::gdk::Key::a, &0_u32, &gtk::gdk::ModifierType::empty()],
        );
        assert!(!normal_key);
        assert_eq!(entry.text(), "entrada não enviada");
        assert_eq!(cancel_count.get(), 0);

        let escape_key = controller.emit_by_name::<bool>(
            "key-pressed",
            &[
                &gtk::gdk::Key::Escape,
                &0_u32,
                &gtk::gdk::ModifierType::empty(),
            ],
        );
        assert!(escape_key);
        assert_eq!(entry.text(), "");
        assert_eq!(cancel_count.get(), 1);
    }
}

#[cfg(test)]
mod file_picker_execution_tests {
    use super::execution_reply_is_current;

    #[test]
    fn old_file_picker_reply_is_rejected_after_a_new_execution_starts() {
        assert!(!execution_reply_is_current(41, Some(42)));
        assert!(execution_reply_is_current(42, Some(42)));
    }
}

#[cfg(test)]
#[path = "../tests/learning/app_state_tests.rs"]
mod learning_state_tests;
