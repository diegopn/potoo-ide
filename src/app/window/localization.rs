use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UiLanguage {
    Portuguese,
    English,
    Spanish,
}

impl UiLanguage {
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Portuguese => "pt-BR",
            Self::English => "en",
            Self::Spanish => "es",
        }
    }

    pub(super) fn from_key(key: &str) -> Option<Self> {
        match key {
            "pt" | "pt-BR" => Some(Self::Portuguese),
            "en" => Some(Self::English),
            "es" => Some(Self::Spanish),
            _ => None,
        }
    }

    pub(super) fn from_locale_name(locale: &str) -> Option<Self> {
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

    pub(super) fn from_language_names<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        names
            .into_iter()
            .find_map(|name| Self::from_locale_name(name.as_ref()))
            .unwrap_or(Self::English)
    }

    pub(super) fn from_preference<I, S>(saved: Option<&str>, system_languages: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        saved
            .and_then(Self::from_key)
            .unwrap_or_else(|| Self::from_language_names(system_languages))
    }

    pub(super) fn from_system() -> Self {
        Self::from_language_names(gtk::glib::language_names())
    }

    pub(super) fn selected_index(self) -> u32 {
        match self {
            Self::Portuguese => 0,
            Self::English => 1,
            Self::Spanish => 2,
        }
    }

    pub(super) fn from_selected_index(index: u32) -> Self {
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
            "Dialetos do Portugol: G-Portugol, VisuAlg, Portugol Studio 2.0 e Mapler" => {
                "Portugol dialects: G-Portugol, VisuAlg, Portugol Studio 2.0, and Mapler"
            }
            "4 variantes" => "4 variants",
            "2 variantes" => "2 variants",
            "Criar programa inicial (.clg)" => "Create a starter program (.clg)",
            "Criar programa inicial (.poti)" => "Create a starter program (.poti)",
            "Selecionar uma pasta" => "Choose a folder",
            "Abra um projeto que não aparece na lista." => "Open a project that is not listed.",
            "Abra um arquivo .por, .alg, .poti, .gpt, .clg ou .mapler." => {
                "Open a .por, .alg, .poti, .gpt, .clg, or .mapler file."
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
            "Criar um projeto Calango, G-Portugol, VisuAlg, Portugol Studio 2.0, Mapler ou Potigol" => {
                "Create a Calango, G-Portugol, VisuAlg, Portugol Studio 2.0, Mapler, or Potigol project"
            }
            "Selecione a linguagem/pseudolinguagem do projeto" => {
                "Select the project's language/pseudolanguage"
            }
            "Linguagem/pseudolinguagem" => "Language/pseudolanguage",
            "Depuração encerrada com erro" => "Debugging ended with an error",
            "Variante do Portugol" => "Portugol variant",
            "Mapler" => "Mapler",
            "Variante do Potigol" => "Potigol variant",
            "Uma rotina executa ações sem devolver um resultado; cada linguagem define como seus parâmetros são passados." => {
                "A routine performs actions without returning a result; each language defines how its parameters are passed."
            }
            "Portugol Studio 2.0" => "Portugol Studio 2.0",
            "G-Portugol" => "G-Portugol",
            "Potigol Clássico" => "Classic Potigol",
            "Potigol Jerimum (Jogo 2D)" => "Potigol Jerimum (2D Game)",
            "Arquivo inicial .por" => "Initial .por file",
            "Arquivo inicial .alg" => "Initial .alg file",
            "Arquivo inicial .poti" => "Initial .poti file",
            "Arquivo inicial .gpt" => "Initial .gpt file",
            "Arquivo inicial .mapler" => "Initial .mapler file",
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
            "Criar um arquivo Calango, G-Portugol, VisuAlg, Portugol Studio 2.0, Mapler ou Potigol" => {
                "Create a Calango, G-Portugol, VisuAlg, Portugol Studio 2.0, Mapler, or Potigol file"
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
            "Abrir em janela separada" => "Open in separate window",
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
            "palavra-chave" => "keyword",
            "biblioteca" => "library",
            "chamada de biblioteca" => "library call",
            "membro" => "member",
            "snippet" => "snippet",
            "símbolo" => "symbol",
            "procedimento" => "procedure",
            "atribuição" => "assignment",
            "retorno" => "return",
            "programa" => "program",
            "Problemas nesta linha" => "Problems on this line",
            "Nenhum problema conhecido nesta linha." => "No known problem on this line.",
            "Erro" => "Error",
            "Aviso" => "Warning",
            "Informação" => "Information",
            "Dica" => "Hint",
            "Erro léxico" => "Lexical error",
            "Erro de sintaxe" => "Syntax error",
            "Erro de tipo" => "Type error",
            "Erro de execução" => "Runtime error",
            "Análise estática" => "Static analysis",
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
            "O comando" => "The command",
            "O laço" => "The loop",
            "exibe os valores e encerra a linha" => "displays the values and ends the line",
            "exibe os valores sem encerrar a linha" => {
                "displays the values without ending the line"
            }
            "A quebra de linha determina onde a próxima saída começa." => {
                "A line break determines where the next output begins."
            }
            "Execute duas instruções de saída seguidas e compare em quais linhas aparecem." => {
                "Run two output statements in a row and compare which lines they appear on."
            }
            "Execute duas instruções de saída seguidas e observe se ficam na mesma linha." => {
                "Run two output statements in a row and see whether they stay on the same line."
            }
            "Altere o texto e execute novamente para observar a saída." => {
                "Change the text and run the program again to observe the output."
            }
            "lê um inteiro" => "reads an integer",
            "lê um número real" => "reads a real number",
            "lê um texto" => "reads text",
            "lê um caractere" => "reads a character",
            "lê um valor lógico" => "reads a logical value",
            "lê uma lista de inteiros" => "reads a list of integers",
            "lê uma lista de números reais" => "reads a list of real numbers",
            "lê uma lista de textos" => "reads a list of text values",
            "A instrução" => "The instruction",
            "armazena o valor informado na variável de destino" => {
                "stores the entered value in the target variable"
            }
            "fornece um valor para a expressão que a utiliza" => {
                "provides a value to the expression that uses it"
            }
            "O tipo solicitado define como o valor de entrada é interpretado." => {
                "The requested type determines how the input value is interpreted."
            }
            "A seleção compara um valor com alternativas explícitas." => {
                "Selection compares a value with explicit alternatives."
            }
            "O bloco é executado antes da verificação da condição." => {
                "The block runs before the condition is checked."
            }
            "A atribuição calcula o lado direito e atualiza o valor da variável à esquerda." => {
                "An assignment evaluates the right side and updates the variable on the left."
            }
            "O operador" => "The operator",
            "atribui o valor à variável" => "assigns the value to variable",
            "A expressão à direita do operador produz o valor armazenado." => {
                "The expression to the right of the operator produces the stored value."
            }
            "Altere o valor à direita do operador e observe a variável." => {
                "Change the value to the right of the operator and observe the variable."
            }
            "Altere o valor à direita do operador e observe o valor de" => {
                "Change the value to the right of the operator and observe"
            }
            "é testada por" => "is tested by",
            "e determina qual bloco será executado." => "and determines which block runs.",
            "compara o valor selecionado com cada caso e executa o bloco correspondente." => {
                "compares the selected value with each case and runs the matching block."
            }
            "Troque o valor de `escolha` ou de um caso e observe qual bloco executa." => {
                "Change the value in `escolha` or a case and observe which block runs."
            }
            "testa a condição antes de cada repetição" => {
                "tests the condition before each iteration"
            }
            "verifica a condição depois de executar o bloco" => {
                "checks the condition after running the block"
            }
            "percorre os valores definidos para a variável de controle" => {
                "iterates over the values defined for the control variable"
            }
            "Altere a condição e observe quantas vezes o bloco se repete." => {
                "Change the condition and observe how many times the block repeats."
            }
            "Altere a condição e confirme que o bloco executa antes da verificação." => {
                "Change the condition and confirm the block runs before it is checked."
            }
            "Altere um limite do `para` e observe quantas vezes o bloco se repete." => {
                "Change a `para` limit and observe how many times the block repeats."
            }
            "chamada" => "call",
            "importação" => "import",
            "não identificada" => "unrecognized",
            "A chamada" => "The call",
            "obtém dados de entrada para o programa" => "reads input for the program",
            "A instrução de entrada fornece valores para o programa usar." => {
                "An input instruction provides values for the program to use."
            }
            "Esta instrução apresenta os dados informados na saída do programa." => {
                "This instruction presents the provided data in the program output."
            }
            "executa a função ou operação identificada nesta linha" => {
                "executes the function or operation identified on this line"
            }
            "executa uma operação disponível na biblioteca importada" => {
                "executes an operation provided by the imported library"
            }
            "A importação" => "The import",
            "disponibiliza os elementos dessa biblioteca ao programa" => {
                "makes this library's members available to the program"
            }
            "Esta linha importa elementos para uso no programa." => {
                "This line imports members for use in the program."
            }
            "A análise não reconheceu esta linha; consulte a mensagem em Problemas nesta linha." => {
                "The analysis did not recognize this line; see the message under Problems on this line."
            }
            "Esta linha não contém uma instrução reconhecida; comentários e linhas vazias não executam código." => {
                "This line has no recognized executable instruction; comments and blank lines do not run code."
            }
            "A análise não reconheceu esta construção; não foi possível verificar problemas." => {
                "The analysis did not recognize this construct, so it could not check this line for problems."
            }
            "Uma chamada transfere a execução para uma função ou operação e depois continua no ponto seguinte." => {
                "A call transfers execution to a function or operation, then continues at the next point."
            }
            "Uma biblioteca fornece operações que o programa pode reutilizar." => {
                "A library provides operations the program can reuse."
            }
            "A importação torna disponíveis nomes e operações definidos em outra biblioteca." => {
                "An import makes names and operations from another library available."
            }
            "iniciar configura e abre a janela do jogo com título, dimensões, funções de atualização e desenho, e taxa de quadros." => {
                "iniciar configures and opens the game window with a title, dimensions, update and draw functions, and frame rate."
            }
            "O laço do jogo chama a atualização e o desenho repetidamente enquanto a janela estiver aberta." => {
                "The game loop repeatedly calls the update and draw functions while the window is open."
            }
            "Altere a largura e a altura da chamada e observe como o espaço do jogo muda." => {
                "Change the call's width and height and see how the game area changes."
            }
            "incluir registra uma função de desenho para compor a cena do jogo." => {
                "incluir registers a draw function that contributes to the game scene."
            }
            "Separar cada elemento em uma função de desenho ajuda a organizar a cena." => {
                "Giving each element its own draw function helps organize the scene."
            }
            "Comente temporariamente esta inclusão e observe qual elemento deixa de aparecer." => {
                "Temporarily comment out this inclusion and see which element disappears."
            }
            "setColor define a cor usada pelas próximas operações gráficas deste contexto." => {
                "setColor sets the color used by subsequent graphics operations in this context."
            }
            "A cor é um estado do contexto gráfico e permanece ativa até outra chamada de setColor." => {
                "Color is state on the graphics context and remains active until another setColor call."
            }
            "Troque o argumento de cor por outra constante Cor e observe os próximos desenhos." => {
                "Change the color argument to another Cor constant and observe the next drawings."
            }
            "fillRect desenha um retângulo preenchido usando posição x/y e largura/altura." => {
                "fillRect draws a filled rectangle using x/y position and width/height."
            }
            "As coordenadas indicam a posição do retângulo; largura e altura definem seu tamanho." => {
                "Coordinates set the rectangle's position; width and height set its size."
            }
            "Aumente os dois últimos argumentos e observe o tamanho do retângulo." => {
                "Increase the last two arguments and observe the rectangle's size."
            }
            "drawString desenha o texto informado nas coordenadas x/y da área gráfica." => {
                "drawString draws the given text at the x/y coordinates in the graphics area."
            }
            "A posição do texto é definida por coordenadas dentro da tela do jogo." => {
                "The text position is set by coordinates within the game screen."
            }
            "Mude o texto ou suas coordenadas e observe o resultado na tela." => {
                "Change the text or its coordinates and observe the result on screen."
            }
            "Esta função termina sem devolver um valor." => {
                "This function ends without returning a value."
            }
            "O retorno entrega um valor do tipo" => "The return provides a value of type",
            "ao ponto que chamou a função" => "to the point that called the function",
            "O retorno devolve o controle ao ponto que chamou a rotina." => {
                "The return passes control back to the point that called the routine."
            }
            "Parâmetros são os dados recebidos pela função. `vazio` informa que nenhum resultado é devolvido." => {
                "Parameters are the data received by a function. `vazio` means no result is returned."
            }
            "A função produz um valor de texto, representado pelo tipo `cadeia` em Portugol Studio 2.0." => {
                "The function produces text, represented by the `cadeia` type in Portugol Studio 2.0."
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
            "Chame a função e observe as ações que ela executa." => {
                "Call the function and observe the actions it performs."
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
            "Nenhum programa .por, .alg, .poti, .gpt, .clg ou .mapler nesta pasta. Os demais arquivos aparecem na lista." => {
                "No .por, .alg, .poti, .gpt, .clg, or .mapler programs in this folder. Other files appear in the list."
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
            "Pasta aberta. Crie um arquivo .por, .alg, .poti, .gpt, .clg ou .mapler para começar." => {
                "Folder opened. Create a .por, .alg, .poti, .gpt, .clg, or .mapler file to get started."
            }
            "Novo arquivo criado" => "New file created",
            "Já existe um arquivo com esse nome." => "A file with that name already exists.",
            "Já existe um arquivo neste caminho; escolha outro nome em Salvar como." => {
                "A file already exists at this path; choose another name in Save As."
            }
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
            "O editor abre arquivos .por, .alg, .poti, .gpt, .clg e .mapler; arquivos .txt exigem confirmação como Mapler." => {
                "The editor opens .por, .alg, .poti, .gpt, .clg, and .mapler files; .txt files require confirmation as Mapler."
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
            "Use a extensão .por, .alg, .poti, .gpt, .clg ou .mapler." => {
                "Use the .por, .alg, .poti, .gpt, .clg, or .mapler extension."
            }
            "Salve o código com a extensão .por, .alg, .poti, .gpt, .clg ou .mapler." => {
                "Save the code with the .por, .alg, .poti, .gpt, .clg, or .mapler extension."
            }
            "Abrir arquivo Mapler" => "Open Mapler file",
            "Deseja abrir este arquivo .txt como Mapler?" => "Open this .txt file as Mapler?",
            "A IDE original do Mapler usa .txt. O Potoo usa .mapler; arquivos .txt só são abertos como Mapler após esta confirmação." => {
                "The original Mapler IDE uses .txt. Potoo uses .mapler; .txt files are opened as Mapler only after this confirmation."
            }
            "Abrir como Mapler" => "Open as Mapler",
            "Selecione uma pasta local para abrir." => "Choose a local folder to open.",
            "Selecione um arquivo local." => "Choose a local file.",
            "Escolha um caminho local para salvar o arquivo." => {
                "Choose a local path to save the file."
            }
            "Não foi possível abrir a janela de salvamento" => "Could not open the save dialog",
            "Abrir arquivo de código" => "Open source file",
            "Arquivos Mapler e outros dialetos (*.mapler, *.clg, *.gpt, *.alg, *.por, *.poti, *.txt para Mapler)" => {
                "Mapler and other dialect files (*.mapler, *.clg, *.gpt, *.alg, *.por, *.poti, *.txt for Mapler)"
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
            "Dialetos do Portugol: G-Portugol, VisuAlg, Portugol Studio 2.0 e Mapler" => {
                "Dialectos de Portugol: G-Portugol, VisuAlg, Portugol Studio 2.0 y Mapler"
            }
            "4 variantes" => "4 variantes",
            "2 variantes" => "2 variantes",
            "Criar programa inicial (.clg)" => "Crear un programa inicial (.clg)",
            "Criar programa inicial (.poti)" => "Crear un programa inicial (.poti)",
            "Selecionar uma pasta" => "Seleccionar una carpeta",
            "Abra um projeto que não aparece na lista." => {
                "Abre un proyecto que no aparece en la lista."
            }
            "Abra um arquivo .por, .alg, .poti, .gpt, .clg ou .mapler." => {
                "Abre un archivo .por, .alg, .poti, .gpt, .clg o .mapler."
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
            "Criar um projeto Calango, G-Portugol, VisuAlg, Portugol Studio 2.0, Mapler ou Potigol" => {
                "Crear un proyecto de Calango, G-Portugol, VisuAlg, Portugol Studio 2.0, Mapler o Potigol"
            }
            "Selecione a linguagem/pseudolinguagem do projeto" => {
                "Selecciona el lenguaje/pseudolenguaje del proyecto"
            }
            "Linguagem/pseudolinguagem" => "Lenguaje/pseudolenguaje",
            "Depuração encerrada com erro" => "La depuración terminó con un error",
            "Variante do Portugol" => "Variante de Portugol",
            "Mapler" => "Mapler",
            "Variante do Potigol" => "Variante de Potigol",
            "Uma rotina executa ações sem devolver um resultado; cada linguagem define como seus parâmetros são passados." => {
                "Una rutina realiza acciones sin devolver un resultado; cada lenguaje define cómo se pasan sus parámetros."
            }
            "Portugol Studio 2.0" => "Portugol Studio 2.0",
            "G-Portugol" => "G-Portugol",
            "Potigol Clássico" => "Potigol clásico",
            "Potigol Jerimum (Jogo 2D)" => "Potigol Jerimum (juego 2D)",
            "Arquivo inicial .por" => "Archivo inicial .por",
            "Arquivo inicial .alg" => "Archivo inicial .alg",
            "Arquivo inicial .poti" => "Archivo inicial .poti",
            "Arquivo inicial .gpt" => "Archivo inicial .gpt",
            "Arquivo inicial .mapler" => "Archivo inicial .mapler",
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
            "Criar um arquivo Calango, G-Portugol, VisuAlg, Portugol Studio 2.0, Mapler ou Potigol" => {
                "Crear un archivo de Calango, G-Portugol, VisuAlg, Portugol Studio 2.0, Mapler o Potigol"
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
            "Abrir em janela separada" => "Abrir en ventana separada",
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
            "palavra-chave" => "palabra clave",
            "biblioteca" => "biblioteca",
            "chamada de biblioteca" => "llamada de biblioteca",
            "membro" => "miembro",
            "snippet" => "fragmento",
            "símbolo" => "símbolo",
            "procedimento" => "procedimiento",
            "atribuição" => "asignación",
            "retorno" => "retorno",
            "programa" => "programa",
            "Problemas nesta linha" => "Problemas en esta línea",
            "Nenhum problema conhecido nesta linha." => "No se conocen problemas en esta línea.",
            "Erro" => "Error",
            "Aviso" => "Aviso",
            "Informação" => "Información",
            "Dica" => "Sugerencia",
            "Erro léxico" => "Error léxico",
            "Erro de sintaxe" => "Error de sintaxis",
            "Erro de tipo" => "Error de tipo",
            "Erro de execução" => "Error de ejecución",
            "Análise estática" => "Análisis estático",
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
            "O comando" => "El comando",
            "O laço" => "El ciclo",
            "exibe os valores e encerra a linha" => "muestra los valores y termina la línea",
            "exibe os valores sem encerrar a linha" => "muestra los valores sin terminar la línea",
            "A quebra de linha determina onde a próxima saída começa." => {
                "El salto de línea determina dónde comienza la siguiente salida."
            }
            "Execute duas instruções de saída seguidas e compare em quais linhas aparecem." => {
                "Ejecuta dos instrucciones de salida seguidas y compara en qué líneas aparecen."
            }
            "Execute duas instruções de saída seguidas e observe se ficam na mesma linha." => {
                "Ejecuta dos instrucciones de salida seguidas y observa si quedan en la misma línea."
            }
            "Altere o texto e execute novamente para observar a saída." => {
                "Cambia el texto y vuelve a ejecutar para observar la salida."
            }
            "lê um inteiro" => "lee un entero",
            "lê um número real" => "lee un número real",
            "lê um texto" => "lee texto",
            "lê um caractere" => "lee un carácter",
            "lê um valor lógico" => "lee un valor lógico",
            "lê uma lista de inteiros" => "lee una lista de enteros",
            "lê uma lista de números reais" => "lee una lista de números reales",
            "lê uma lista de textos" => "lee una lista de textos",
            "A instrução" => "La instrucción",
            "armazena o valor informado na variável de destino" => {
                "guarda el valor indicado en la variable de destino"
            }
            "fornece um valor para a expressão que a utiliza" => {
                "proporciona un valor a la expresión que la utiliza"
            }
            "O tipo solicitado define como o valor de entrada é interpretado." => {
                "El tipo solicitado determina cómo se interpreta el valor de entrada."
            }
            "A seleção compara um valor com alternativas explícitas." => {
                "La selección compara un valor con alternativas explícitas."
            }
            "O bloco é executado antes da verificação da condição." => {
                "El bloque se ejecuta antes de comprobar la condición."
            }
            "A atribuição calcula o lado direito e atualiza o valor da variável à esquerda." => {
                "La asignación evalúa el lado derecho y actualiza la variable de la izquierda."
            }
            "O operador" => "El operador",
            "atribui o valor à variável" => "asigna el valor a la variable",
            "A expressão à direita do operador produz o valor armazenado." => {
                "La expresión a la derecha del operador produce el valor almacenado."
            }
            "Altere o valor à direita do operador e observe a variável." => {
                "Cambia el valor a la derecha del operador y observa la variable."
            }
            "Altere o valor à direita do operador e observe o valor de" => {
                "Cambia el valor a la derecha del operador y observa"
            }
            "é testada por" => "es probada por",
            "e determina qual bloco será executado." => "y determina qué bloque se ejecuta.",
            "compara o valor selecionado com cada caso e executa o bloco correspondente." => {
                "compara el valor seleccionado con cada caso y ejecuta el bloque correspondiente."
            }
            "Troque o valor de `escolha` ou de um caso e observe qual bloco executa." => {
                "Cambia el valor de `escolha` o de un caso y observa qué bloque se ejecuta."
            }
            "testa a condição antes de cada repetição" => {
                "prueba la condición antes de cada repetición"
            }
            "verifica a condição depois de executar o bloco" => {
                "comprueba la condición después de ejecutar el bloque"
            }
            "percorre os valores definidos para a variável de controle" => {
                "recorre los valores definidos para la variable de control"
            }
            "Altere a condição e observe quantas vezes o bloco se repete." => {
                "Cambia la condición y observa cuántas veces se repite el bloque."
            }
            "Altere a condição e confirme que o bloco executa antes da verificação." => {
                "Cambia la condición y confirma que el bloque se ejecuta antes de comprobarla."
            }
            "Altere um limite do `para` e observe quantas vezes o bloco se repete." => {
                "Cambia un límite de `para` y observa cuántas veces se repite el bloque."
            }
            "chamada" => "llamada",
            "importação" => "importación",
            "não identificada" => "no identificada",
            "A chamada" => "La llamada",
            "obtém dados de entrada para o programa" => {
                "obtiene datos de entrada para el programa"
            }
            "A instrução de entrada fornece valores para o programa usar." => {
                "Una instrucción de entrada proporciona valores para que el programa los use."
            }
            "Esta instrução apresenta os dados informados na saída do programa." => {
                "Esta instrucción presenta los datos indicados en la salida del programa."
            }
            "executa a função ou operação identificada nesta linha" => {
                "ejecuta la función u operación identificada en esta línea"
            }
            "executa uma operação disponível na biblioteca importada" => {
                "ejecuta una operación disponible en la biblioteca importada"
            }
            "A importação" => "La importación",
            "disponibiliza os elementos dessa biblioteca ao programa" => {
                "pone los elementos de esta biblioteca a disposición del programa"
            }
            "Esta linha importa elementos para uso no programa." => {
                "Esta línea importa elementos para usarlos en el programa."
            }
            "A análise não reconheceu esta linha; consulte a mensagem em Problemas nesta linha." => {
                "El análisis no reconoció esta línea; consulta el mensaje en Problemas en esta línea."
            }
            "Esta linha não contém uma instrução reconhecida; comentários e linhas vazias não executam código." => {
                "Esta línea no tiene una instrucción ejecutable reconocida; los comentarios y las líneas vacías no ejecutan código."
            }
            "A análise não reconheceu esta construção; não foi possível verificar problemas." => {
                "El análisis no reconoció esta construcción, así que no pudo comprobar si hay problemas en esta línea."
            }
            "Uma chamada transfere a execução para uma função ou operação e depois continua no ponto seguinte." => {
                "Una llamada transfiere la ejecución a una función u operación y luego continúa en el siguiente punto."
            }
            "Uma biblioteca fornece operações que o programa pode reutilizar." => {
                "Una biblioteca proporciona operaciones que el programa puede reutilizar."
            }
            "A importação torna disponíveis nomes e operações definidos em outra biblioteca." => {
                "Una importación pone a disposición nombres y operaciones definidos en otra biblioteca."
            }
            "iniciar configura e abre a janela do jogo com título, dimensões, funções de atualização e desenho, e taxa de quadros." => {
                "iniciar configura y abre la ventana del juego con título, dimensiones, funciones de actualización y dibujo, y frecuencia de fotogramas."
            }
            "O laço do jogo chama a atualização e o desenho repetidamente enquanto a janela estiver aberta." => {
                "El bucle del juego llama repetidamente a las funciones de actualización y dibujo mientras la ventana está abierta."
            }
            "Altere a largura e a altura da chamada e observe como o espaço do jogo muda." => {
                "Cambia el ancho y el alto de la llamada y observa cómo cambia el espacio del juego."
            }
            "incluir registra uma função de desenho para compor a cena do jogo." => {
                "incluir registra una función de dibujo que forma parte de la escena del juego."
            }
            "Separar cada elemento em uma função de desenho ajuda a organizar a cena." => {
                "Separar cada elemento en una función de dibujo ayuda a organizar la escena."
            }
            "Comente temporariamente esta inclusão e observe qual elemento deixa de aparecer." => {
                "Comenta temporalmente esta inclusión y observa qué elemento deja de aparecer."
            }
            "setColor define a cor usada pelas próximas operações gráficas deste contexto." => {
                "setColor define el color usado por las siguientes operaciones gráficas de este contexto."
            }
            "A cor é um estado do contexto gráfico e permanece ativa até outra chamada de setColor." => {
                "El color es un estado del contexto gráfico y permanece activo hasta otra llamada a setColor."
            }
            "Troque o argumento de cor por outra constante Cor e observe os próximos desenhos." => {
                "Cambia el argumento de color por otra constante Cor y observa los siguientes dibujos."
            }
            "fillRect desenha um retângulo preenchido usando posição x/y e largura/altura." => {
                "fillRect dibuja un rectángulo relleno usando la posición x/y y el ancho/alto."
            }
            "As coordenadas indicam a posição do retângulo; largura e altura definem seu tamanho." => {
                "Las coordenadas indican la posición del rectángulo; el ancho y el alto definen su tamaño."
            }
            "Aumente os dois últimos argumentos e observe o tamanho do retângulo." => {
                "Aumenta los dos últimos argumentos y observa el tamaño del rectángulo."
            }
            "drawString desenha o texto informado nas coordenadas x/y da área gráfica." => {
                "drawString dibuja el texto indicado en las coordenadas x/y del área gráfica."
            }
            "A posição do texto é definida por coordenadas dentro da tela do jogo." => {
                "La posición del texto se define mediante coordenadas dentro de la pantalla del juego."
            }
            "Mude o texto ou suas coordenadas e observe o resultado na tela." => {
                "Cambia el texto o sus coordenadas y observa el resultado en la pantalla."
            }
            "Esta função termina sem devolver um valor." => {
                "Esta función termina sin devolver un valor."
            }
            "O retorno entrega um valor do tipo" => "El retorno entrega un valor del tipo",
            "ao ponto que chamou a função" => "al punto que llamó a la función",
            "O retorno devolve o controle ao ponto que chamou a rotina." => {
                "El retorno devuelve el control al punto que llamó a la rutina."
            }
            "Parâmetros são os dados recebidos pela função. `vazio` informa que nenhum resultado é devolvido." => {
                "Los parámetros son los datos que recibe la función. `vazio` indica que no devuelve un resultado."
            }
            "A função produz um valor de texto, representado pelo tipo `cadeia` em Portugol Studio 2.0." => {
                "La función produce texto, representado por el tipo `cadeia` en Portugol Studio 2.0."
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
            "Chame a função e observe as ações que ela executa." => {
                "Llama a la función y observa las acciones que ejecuta."
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
            "Nenhum programa .por, .alg, .poti, .gpt, .clg ou .mapler nesta pasta. Os demais arquivos aparecem na lista." => {
                "No hay programas .por, .alg, .poti, .gpt, .clg o .mapler en esta carpeta. Los demás archivos aparecen en la lista."
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
            "Pasta aberta. Crie um arquivo .por, .alg, .poti, .gpt, .clg ou .mapler para começar." => {
                "Carpeta abierta. Crea un archivo .por, .alg, .poti, .gpt, .clg o .mapler para empezar."
            }
            "Novo arquivo criado" => "Archivo nuevo creado",
            "Já existe um arquivo com esse nome." => "Ya existe un archivo con ese nombre.",
            "Já existe um arquivo neste caminho; escolha outro nome em Salvar como." => {
                "Ya existe un archivo en esta ruta; elige otro nombre en Guardar como."
            }
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
            "O editor abre arquivos .por, .alg, .poti, .gpt, .clg e .mapler; arquivos .txt exigem confirmação como Mapler." => {
                "El editor abre archivos .por, .alg, .poti, .gpt, .clg y .mapler; los archivos .txt requieren confirmación como Mapler."
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
            "Use a extensão .por, .alg, .poti, .gpt, .clg ou .mapler." => {
                "Usa la extensión .por, .alg, .poti, .gpt, .clg o .mapler."
            }
            "Salve o código com a extensão .por, .alg, .poti, .gpt, .clg ou .mapler." => {
                "Guarda el código con la extensión .por, .alg, .poti, .gpt, .clg o .mapler."
            }
            "Abrir arquivo Mapler" => "Abrir archivo Mapler",
            "Deseja abrir este arquivo .txt como Mapler?" => {
                "¿Abrir este archivo .txt como Mapler?"
            }
            "A IDE original do Mapler usa .txt. O Potoo usa .mapler; arquivos .txt só são abertos como Mapler após esta confirmação." => {
                "La IDE original de Mapler usa .txt. Potoo usa .mapler; los archivos .txt solo se abren como Mapler tras esta confirmación."
            }
            "Abrir como Mapler" => "Abrir como Mapler",
            "Selecione uma pasta local para abrir." => "Selecciona una carpeta local para abrir.",
            "Selecione um arquivo local." => "Selecciona un archivo local.",
            "Escolha um caminho local para salvar o arquivo." => {
                "Elige una ruta local para guardar el archivo."
            }
            "Não foi possível abrir a janela de salvamento" => {
                "No se pudo abrir la ventana para guardar"
            }
            "Abrir arquivo de código" => "Abrir archivo de código",
            "Arquivos Mapler e outros dialetos (*.mapler, *.clg, *.gpt, *.alg, *.por, *.poti, *.txt para Mapler)" => {
                "Archivos Mapler y otros dialectos (*.mapler, *.clg, *.gpt, *.alg, *.por, *.poti, *.txt para Mapler)"
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
    pub(super) fn selected_index(self) -> u32 {
        match self {
            Self::System => 0,
            Self::Light => 1,
            Self::Dark => 2,
        }
    }

    pub(super) fn from_selected_index(index: u32) -> Self {
        match index {
            1 => Self::Light,
            2 => Self::Dark,
            _ => Self::System,
        }
    }

    pub(super) fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub(super) fn from_key(key: &str) -> Self {
        match key {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }
}

#[cfg(test)]
pub(super) const UI_TRANSLATION_KEYS: &[&str] = &[
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
    "Dialetos do Portugol: G-Portugol, VisuAlg, Portugol Studio 2.0 e Mapler",
    "4 variantes",
    "Mapler",
    "2 variantes",
    "Criar programa inicial (.clg)",
    "Criar programa inicial (.poti)",
    "Pressione Esc para cancelar a execução.",
    "Analisando linha…",
    "Aguardando a próxima linha…",
    "Selecionar uma pasta",
    "Abra um projeto que não aparece na lista.",
    "Abra um arquivo .por, .alg, .poti, .gpt, .clg ou .mapler.",
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
    "Criar um projeto Calango, G-Portugol, VisuAlg, Portugol Studio 2.0, Mapler ou Potigol",
    "Selecione a linguagem/pseudolinguagem do projeto",
    "Linguagem/pseudolinguagem",
    "Depuração encerrada com erro",
    "Variante do Portugol",
    "Variante do Potigol",
    "Portugol Studio 2.0",
    "G-Portugol",
    "Potigol Clássico",
    "Potigol Jerimum (Jogo 2D)",
    "Arquivo inicial .por",
    "Arquivo inicial .alg",
    "Arquivo inicial .poti",
    "Arquivo inicial .gpt",
    "Arquivo inicial .mapler",
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
    "Criar um arquivo Calango, G-Portugol, VisuAlg, Portugol Studio 2.0, Mapler ou Potigol",
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
    "Abrir em janela separada",
    "Execute um programa Potigol Jerimum para ver o jogo aqui.",
    "Analisando a linha selecionada…",
    "função",
    "palavra-chave",
    "biblioteca",
    "chamada de biblioteca",
    "membro",
    "snippet",
    "símbolo",
    "procedimento",
    "atribuição",
    "retorno",
    "Problemas nesta linha",
    "Nenhum problema conhecido nesta linha.",
    "Erro",
    "Aviso",
    "Informação",
    "Dica",
    "Erro léxico",
    "Erro de sintaxe",
    "Erro de tipo",
    "Erro de execução",
    "Análise estática",
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
    "O comando",
    "O laço",
    "exibe os valores e encerra a linha",
    "exibe os valores sem encerrar a linha",
    "A quebra de linha determina onde a próxima saída começa.",
    "Execute duas instruções de saída seguidas e compare em quais linhas aparecem.",
    "Execute duas instruções de saída seguidas e observe se ficam na mesma linha.",
    "Altere o texto e execute novamente para observar a saída.",
    "lê um inteiro",
    "lê um número real",
    "lê um texto",
    "lê um caractere",
    "lê um valor lógico",
    "lê uma lista de inteiros",
    "lê uma lista de números reais",
    "lê uma lista de textos",
    "A instrução",
    "armazena o valor informado na variável de destino",
    "fornece um valor para a expressão que a utiliza",
    "O tipo solicitado define como o valor de entrada é interpretado.",
    "A seleção compara um valor com alternativas explícitas.",
    "O bloco é executado antes da verificação da condição.",
    "A atribuição calcula o lado direito e atualiza o valor da variável à esquerda.",
    "O operador",
    "atribui o valor à variável",
    "A expressão à direita do operador produz o valor armazenado.",
    "Altere o valor à direita do operador e observe a variável.",
    "Altere o valor à direita do operador e observe o valor de",
    "é testada por",
    "e determina qual bloco será executado.",
    "compara o valor selecionado com cada caso e executa o bloco correspondente.",
    "Troque o valor de `escolha` ou de um caso e observe qual bloco executa.",
    "testa a condição antes de cada repetição",
    "verifica a condição depois de executar o bloco",
    "percorre os valores definidos para a variável de controle",
    "Altere a condição e observe quantas vezes o bloco se repete.",
    "Altere a condição e confirme que o bloco executa antes da verificação.",
    "Altere um limite do `para` e observe quantas vezes o bloco se repete.",
    "Esta função termina sem devolver um valor.",
    "O retorno entrega um valor do tipo",
    "ao ponto que chamou a função",
    "O retorno devolve o controle ao ponto que chamou a rotina.",
    "Parâmetros são os dados recebidos pela função. `vazio` informa que nenhum resultado é devolvido.",
    "A função produz um valor de texto, representado pelo tipo `cadeia` em Portugol Studio 2.0.",
    "A função devolve um resultado do tipo `{return_type}`. Seus parâmetros recebem os dados usados no cálculo.",
    "A função devolve um resultado do tipo",
    "Seus parâmetros recebem os dados usados no cálculo.",
    "Parâmetros são os dados recebidos pela função; o tipo de retorno descreve o resultado produzido.",
    "Uma rotina executa ações sem devolver um resultado; cada linguagem define como seus parâmetros são passados.",
    "O tipo define quais valores podem ser guardados; o formato indica se há um valor, um vetor ou uma matriz.",
    "Atribuir associa um novo valor a uma variável; o operador determina como o valor é calculado.",
    "O retorno leva o resultado da função de volta ao ponto que a chamou.",
    "chamada",
    "importação",
    "não identificada",
    "A chamada",
    "obtém dados de entrada para o programa",
    "A instrução de entrada fornece valores para o programa usar.",
    "Esta instrução apresenta os dados informados na saída do programa.",
    "executa a função ou operação identificada nesta linha",
    "executa uma operação disponível na biblioteca importada",
    "A importação",
    "disponibiliza os elementos dessa biblioteca ao programa",
    "Esta linha importa elementos para uso no programa.",
    "A análise não reconheceu esta linha; consulte a mensagem em Problemas nesta linha.",
    "Esta linha não contém uma instrução reconhecida; comentários e linhas vazias não executam código.",
    "A análise não reconheceu esta construção; não foi possível verificar problemas.",
    "Uma chamada transfere a execução para uma função ou operação e depois continua no ponto seguinte.",
    "Uma biblioteca fornece operações que o programa pode reutilizar.",
    "A importação torna disponíveis nomes e operações definidos em outra biblioteca.",
    "iniciar configura e abre a janela do jogo com título, dimensões, funções de atualização e desenho, e taxa de quadros.",
    "O laço do jogo chama a atualização e o desenho repetidamente enquanto a janela estiver aberta.",
    "Altere a largura e a altura da chamada e observe como o espaço do jogo muda.",
    "incluir registra uma função de desenho para compor a cena do jogo.",
    "Separar cada elemento em uma função de desenho ajuda a organizar a cena.",
    "Comente temporariamente esta inclusão e observe qual elemento deixa de aparecer.",
    "setColor define a cor usada pelas próximas operações gráficas deste contexto.",
    "A cor é um estado do contexto gráfico e permanece ativa até outra chamada de setColor.",
    "Troque o argumento de cor por outra constante Cor e observe os próximos desenhos.",
    "fillRect desenha um retângulo preenchido usando posição x/y e largura/altura.",
    "As coordenadas indicam a posição do retângulo; largura e altura definem seu tamanho.",
    "Aumente os dois últimos argumentos e observe o tamanho do retângulo.",
    "drawString desenha o texto informado nas coordenadas x/y da área gráfica.",
    "A posição do texto é definida por coordenadas dentro da tela do jogo.",
    "Mude o texto ou suas coordenadas e observe o resultado na tela.",
    "Troque o valor atribuído e acompanhe a próxima leitura da variável.",
    "Altere o valor devolvido e veja como a chamada da função reage.",
    "Altere o valor inicial ou um elemento do vetor e observe o efeito no resultado.",
    "Chame a função e observe as ações que ela executa.",
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
    "Nenhum programa .por, .alg, .poti, .gpt, .clg ou .mapler nesta pasta. Os demais arquivos aparecem na lista.",
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
    "Pasta aberta. Crie um arquivo .por, .alg, .poti, .gpt, .clg ou .mapler para começar.",
    "Novo arquivo criado",
    "Já existe um arquivo com esse nome.",
    "Já existe um arquivo neste caminho; escolha outro nome em Salvar como.",
    "Não foi possível abrir o arquivo",
    "Não foi possível abrir a pasta",
    "Não foi possível salvar o arquivo",
    "Não foi possível criar o arquivo",
    "O editor abre arquivos .por, .alg, .poti, .gpt, .clg e .mapler; arquivos .txt exigem confirmação como Mapler.",
    "Abra uma pasta de projeto antes de criar arquivos.",
    "Crie ou abra um arquivo antes de salvar.",
    "Não foi possível salvar as configurações",
    "Não foi possível salvar a sessão",
    "Use um nome simples, sem pastas ou caracteres especiais.",
    "Use a extensão .por, .alg, .poti, .gpt, .clg ou .mapler.",
    "Salve o código com a extensão .por, .alg, .poti, .gpt, .clg ou .mapler.",
    "Selecione uma pasta local para abrir.",
    "Selecione um arquivo local.",
    "Escolha um caminho local para salvar o arquivo.",
    "Não foi possível abrir a janela de salvamento",
    "Abrir arquivo de código",
    "Arquivos Mapler e outros dialetos (*.mapler, *.clg, *.gpt, *.alg, *.por, *.poti, *.txt para Mapler)",
    "Abrir arquivo Mapler",
    "Deseja abrir este arquivo .txt como Mapler?",
    "A IDE original do Mapler usa .txt. O Potoo usa .mapler; arquivos .txt só são abertos como Mapler após esta confirmação.",
    "Abrir como Mapler",
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
pub(super) fn translate_known_text(from: UiLanguage, to: UiLanguage, text: &str) -> Option<String> {
    UI_TRANSLATION_KEYS
        .iter()
        .find_map(|key| (tr(from, key) == text).then(|| tr(to, key).to_string()))
}

pub(super) fn system_theme_is_dark() -> bool {
    gtk::Settings::default().is_some_and(|settings| {
        settings.property::<gtk::InterfaceColorScheme>("gtk-interface-color-scheme")
            == gtk::InterfaceColorScheme::Dark
    })
}

pub(super) fn theme_is_dark(theme: ThemePreference) -> bool {
    match theme {
        ThemePreference::System => system_theme_is_dark(),
        ThemePreference::Light => false,
        ThemePreference::Dark => true,
    }
}
