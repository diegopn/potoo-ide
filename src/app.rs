use crate::components::{
    content_panel::{self, Language, SourceDocument, SyntaxTags},
    header, icon, sidebar,
    sidebar::ProjectFile,
};
use gtk::prelude::*;
use relm4::prelude::*;
use std::{
    cell::{Cell, RefCell},
    fs,
    fs::OpenOptions,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

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

    fn from_key(key: &str) -> Self {
        match key {
            "en" => Self::English,
            "es" => Self::Spanish,
            _ => Self::Portuguese,
        }
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
            "Abrir pasta" => "Open folder",
            "Abrir pasta de projeto" => "Open project folder",
            "Abrir arquivo" => "Open file",
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
            "Arquivos" => "Files",
            "Criar arquivo" => "Create file",
            "Criar item no projeto" => "Create project item",
            "Novo projeto" => "New project",
            "Criar projeto" => "Create project",
            "Criar um projeto Portugol ou VisuAlg" => "Create a Portugol or VisuAlg project",
            "Selecione a pseudolinguagem do projeto" => "Select the project's pseudolanguage",
            "Arquivo inicial .por" => "Initial .por file",
            "Arquivo inicial .alg" => "Initial .alg file",
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
            "Depurador pronto" => "Debugger ready",
            "Execute o programa passo a passo para acompanhar as variáveis." => {
                "Step through the program to follow its variables."
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
            "Criar um arquivo Portugol ou VisuAlg" => "Create a Portugol or VisuAlg file",
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
            "Linha 7 · decisão" => "Line 7 · decision",
            "Uma condição verifica se uma expressão é verdadeira antes de executar um bloco." => {
                "A condition checks whether an expression is true before running a block."
            }
            "Experimente" => "Try it",
            "Troque o valor 18 por 16 e execute de novo. O resultado muda?" => {
                "Change 18 to 16 and run it again. Does the result change?"
            }
            "Conceito: condição" => "Concept: condition",
            "Uma comparação como idade >= 18 resulta em verdadeiro ou falso." => {
                "A comparison such as age >= 18 evaluates to true or false."
            }
            "Saída" => "Output",
            "Problemas" => "Problems",
            "Depurador" => "Debugger",
            "Interface" => "Interface",
            "Espaços" => "Spaces",
            "A saída aparecerá aqui após a execução." => {
                "Program output will appear here after you run it."
            }
            "Pronto para executar" => "Ready to run",
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
            "O idioma escolhido será aplicado ao reabrir o Potoo." => {
                "The selected language will apply the next time you open Potoo."
            }
            "Recuo" => "Indentation",
            "Espaços por nível de recuo." => "Spaces inserted for each indentation level.",
            "2 espaços" => "2 spaces",
            "4 espaços" => "4 spaces",
            "Esta pasta ainda não tem arquivos .por ou .alg." => {
                "This folder does not contain any .por or .alg files yet."
            }
            "Nenhum programa .por ou .alg encontrado. Use + para criar um." => {
                "No .por or .alg programs found. Use + to create one."
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
            "Os erros de sintaxe e execução aparecerão aqui." => {
                "Syntax and runtime errors will appear here."
            }
            "Acompanhe a execução linha por linha e os valores das variáveis." => {
                "Follow execution line by line and inspect variable values."
            }
            "Arquivo aberto" => "File opened",
            "Arquivo salvo" => "File saved",
            "Pasta aberta. Crie um arquivo .por ou .alg para começar." => {
                "Folder opened. Create a .por or .alg file to get started."
            }
            "Novo arquivo criado" => "New file created",
            "Já existe um arquivo com esse nome." => "A file with that name already exists.",
            "Não foi possível abrir o arquivo" => "Could not open file",
            "Não foi possível abrir a pasta" => "Could not open folder",
            "Não foi possível salvar o arquivo" => "Could not save file",
            "Não foi possível criar o arquivo" => "Could not create file",
            "Abra um arquivo .por ou .alg." => "Open a .por or .alg file.",
            "Abra uma pasta de projeto antes de criar arquivos." => {
                "Open a project folder before creating files."
            }
            "Crie ou abra um arquivo antes de salvar." => "Create or open a file before saving.",
            "A execução real depende da integração com o interpretador oficial selecionado." => {
                "Running code requires connecting the selected language interpreter."
            }
            "Interpretador ainda não conectado nesta prévia" => {
                "Interpreter not connected in this preview yet"
            }
            "A execução real usa o interpretador da linguagem selecionada" => {
                "Running code uses the selected language interpreter"
            }
            "O idioma será aplicado ao reabrir o Potoo." => {
                "The language change will apply when you reopen Potoo."
            }
            "Não foi possível salvar as configurações" => "Could not save settings",
            "Não foi possível salvar a sessão" => "Could not save session",
            "Use um nome simples, sem pastas ou caracteres especiais." => {
                "Use a simple name without folders or special characters."
            }
            "Use a extensão .por ou .alg." => "Use the .por or .alg extension.",
            "Salve o código com a extensão .por ou .alg." => {
                "Save the code with the .por or .alg extension."
            }
            "Selecione uma pasta local para abrir." => "Choose a local folder to open.",
            "Selecione um arquivo local." => "Choose a local file.",
            "Escolha um caminho local para salvar o arquivo." => {
                "Choose a local path to save the file."
            }
            "Não foi possível abrir a janela de salvamento" => "Could not open the save dialog",
            "Abrir arquivo de código" => "Open source file",
            "Portugol e VisuAlg (*.por, *.alg)" => "Portugol and VisuAlg (*.por, *.alg)",
            "Não foi possível abrir o arquivo de código" => "Could not open the source file",
            "novo-arquivo" => "new-file",
            "caminho de configuração inválido" => "invalid settings path",
            "Não foi possível ler a sessão anterior. Os arquivos originais continuam intactos." => {
                "The previous session could not be read. Your original files are unchanged."
            }
            "Salvar arquivo (⌘S)" => "Save file (⌘S)",
            "Salvar arquivo (Ctrl+S)" => "Save file (Ctrl+S)",
            "Salvar código com a extensão .por ou .alg." => {
                "Save code with the .por or .alg extension."
            }
            _ => portuguese,
        },
        UiLanguage::Spanish => match portuguese {
            "Meu primeiro programa" => "Mi primer programa",
            "Abrir pasta" => "Abrir carpeta",
            "Abrir pasta de projeto" => "Abrir carpeta del proyecto",
            "Abrir arquivo" => "Abrir archivo",
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
            "Arquivos" => "Archivos",
            "Criar arquivo" => "Crear archivo",
            "Criar item no projeto" => "Crear elemento del proyecto",
            "Novo projeto" => "Nuevo proyecto",
            "Criar projeto" => "Crear proyecto",
            "Criar um projeto Portugol ou VisuAlg" => "Crear un proyecto Portugol o VisuAlg",
            "Selecione a pseudolinguagem do projeto" => "Selecciona el pseudolenguaje del proyecto",
            "Arquivo inicial .por" => "Archivo inicial .por",
            "Arquivo inicial .alg" => "Archivo inicial .alg",
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
            "Depurador pronto" => "Depurador listo",
            "Execute o programa passo a passo para acompanhar as variáveis." => {
                "Ejecuta el programa paso a paso para seguir sus variables."
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
            "Criar um arquivo Portugol ou VisuAlg" => "Crear un archivo Portugol o VisuAlg",
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
            "Linha 7 · decisão" => "Línea 7 · decisión",
            "Uma condição verifica se uma expressão é verdadeira antes de executar um bloco." => {
                "Una condición comprueba si una expresión es verdadera antes de ejecutar un bloque."
            }
            "Experimente" => "Pruébalo",
            "Troque o valor 18 por 16 e execute de novo. O resultado muda?" => {
                "Cambia 18 por 16 y ejecútalo de nuevo. ¿Cambia el resultado?"
            }
            "Conceito: condição" => "Concepto: condición",
            "Uma comparação como idade >= 18 resulta em verdadeiro ou falso." => {
                "Una comparación como edad >= 18 da como resultado verdadero o falso."
            }
            "Saída" => "Salida",
            "Problemas" => "Problemas",
            "Depurador" => "Depurador",
            "Interface" => "Interfaz",
            "Espaços" => "Espacios",
            "A saída aparecerá aqui após a execução." => {
                "La salida aparecerá aquí después de ejecutar el programa."
            }
            "Pronto para executar" => "Listo para ejecutar",
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
            "O idioma escolhido será aplicado ao reabrir o Potoo." => {
                "El idioma elegido se aplicará la próxima vez que abras Potoo."
            }
            "Recuo" => "Sangría",
            "Espaços por nível de recuo." => "Espacios insertados por cada nivel de sangría.",
            "2 espaços" => "2 espacios",
            "4 espaços" => "4 espacios",
            "Esta pasta ainda não tem arquivos .por ou .alg." => {
                "Esta carpeta todavía no tiene archivos .por o .alg."
            }
            "Nenhum programa .por ou .alg encontrado. Use + para criar um." => {
                "No se encontraron programas .por o .alg. Usa + para crear uno."
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
            "Os erros de sintaxe e execução aparecerão aqui." => {
                "Los errores de sintaxis y ejecución aparecerán aquí."
            }
            "Acompanhe a execução linha por linha e os valores das variáveis." => {
                "Sigue la ejecución línea por línea y consulta los valores de las variables."
            }
            "Arquivo aberto" => "Archivo abierto",
            "Arquivo salvo" => "Archivo guardado",
            "Pasta aberta. Crie um arquivo .por ou .alg para começar." => {
                "Carpeta abierta. Crea un archivo .por o .alg para empezar."
            }
            "Novo arquivo criado" => "Archivo nuevo creado",
            "Já existe um arquivo com esse nome." => "Ya existe un archivo con ese nombre.",
            "Não foi possível abrir o arquivo" => "No se pudo abrir el archivo",
            "Não foi possível abrir a pasta" => "No se pudo abrir la carpeta",
            "Não foi possível salvar o arquivo" => "No se pudo guardar el archivo",
            "Não foi possível criar o arquivo" => "No se pudo crear el archivo",
            "Abra um arquivo .por ou .alg." => "Abre un archivo .por o .alg.",
            "Abra uma pasta de projeto antes de criar arquivos." => {
                "Abre una carpeta de proyecto antes de crear archivos."
            }
            "Crie ou abra um arquivo antes de salvar." => {
                "Crea o abre un archivo antes de guardar."
            }
            "A execução real depende da integração com o interpretador oficial selecionado." => {
                "La ejecución requiere conectar el intérprete del lenguaje seleccionado."
            }
            "Interpretador ainda não conectado nesta prévia" => {
                "El intérprete todavía no está conectado en esta vista previa"
            }
            "A execução real usa o interpretador da linguagem selecionada" => {
                "La ejecución usa el intérprete del lenguaje seleccionado"
            }
            "O idioma será aplicado ao reabrir o Potoo." => {
                "El cambio de idioma se aplicará al volver a abrir Potoo."
            }
            "Não foi possível salvar as configurações" => "No se pudo guardar la configuración",
            "Não foi possível salvar a sessão" => "No se pudo guardar la sesión",
            "Use um nome simples, sem pastas ou caracteres especiais." => {
                "Usa un nombre sencillo, sin carpetas ni caracteres especiales."
            }
            "Use a extensão .por ou .alg." => "Usa la extensión .por o .alg.",
            "Salve o código com a extensão .por ou .alg." => {
                "Guarda el código con la extensión .por o .alg."
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
            "Portugol e VisuAlg (*.por, *.alg)" => "Portugol y VisuAlg (*.por, *.alg)",
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
            "Salvar código com a extensão .por ou .alg." => {
                "Guarda el código con la extensión .por o .alg."
            }
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
            ui_language: UiLanguage::Portuguese,
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

#[derive(Debug)]
pub(crate) enum MainWindowMsg {
    InterfaceLanguageSelected(UiLanguage),
    ProjectSelected(PathBuf),
    ProjectSwitchDecision(bool),
    CreateProject(PathBuf, String, Language),
    CreateFile(String, Language, PathBuf),
    CreateFolder(String, PathBuf),
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
    SourceChanged(String),
    PersistRecovery(u64),
}

pub(crate) struct MainWindow {
    new_file_language: Language,
    documents: Vec<SourceDocument>,
    active_document_id: Option<u64>,
    next_document_id: u64,
    project_root: Option<PathBuf>,
    project_name: String,
    project_files: Vec<ProjectFile>,
    pending_project: Option<PathBuf>,
    pending_close_document_id: Option<u64>,
    pending_close_after_save_id: Option<u64>,
    pending_save_as_id: Option<u64>,
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
}

impl MainWindow {
    fn text(&self, portuguese: &'static str) -> &'static str {
        tr(self.ui_language, portuguese)
    }

    fn save_preferences(&mut self) {
        let preferences = Preferences {
            theme: self.theme,
            font_size: self.font_size,
            line_numbers_visible: self.line_numbers_visible,
            word_wrap: self.word_wrap,
            indent_width: self.indent_width,
            default_language: self.new_file_language,
            ui_language: self.preferred_ui_language,
        };
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
        ui_language: key_file
            .string("interface", "language")
            .map(|value| UiLanguage::from_key(&value))
            .unwrap_or(defaults.ui_language),
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

    fn open_document(&mut self, path: PathBuf) {
        if let Some(document) = self
            .documents
            .iter()
            .find(|doc| doc.path.as_ref() == Some(&path))
        {
            self.active_document_id = Some(document.id);
            return;
        }

        let Some(language) = Language::from_path(&path) else {
            self.output_status = self.text("Abra um arquivo .por ou .alg.").to_string();
            return;
        };
        match fs::read_to_string(&path) {
            Ok(contents) => {
                if self.project_root.is_none() {
                    self.project_root = path.parent().map(Path::to_path_buf);
                    if let Some(root) = &self.project_root {
                        self.project_name = root
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| root.display().to_string());
                        self.project_files = scan_project_files(root).unwrap_or_default();
                    }
                }
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                self.create_document(Some(path), name, language, contents.clone(), contents);
                self.output_status = self.text("Arquivo aberto").to_string();
            }
            Err(error) => {
                self.output_status =
                    format!("{}: {error}", self.text("Não foi possível abrir o arquivo"));
            }
        }
    }

    fn load_project(&mut self, root: PathBuf) {
        let files = match scan_project_files(&root) {
            Ok(files) => files,
            Err(error) => {
                self.output_status =
                    format!("{}: {error}", self.text("Não foi possível abrir a pasta"));
                return;
            }
        };

        self.project_name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string());
        self.project_root = Some(root);
        self.project_files = files;
        self.documents.clear();
        self.active_document_id = None;
        let root_file = self.project_root.as_ref().and_then(|root| {
            self.project_files
                .iter()
                .find(|file| !file.is_directory && file.path.parent() == Some(root.as_path()))
                .map(|file| file.path.clone())
        });
        let first_source_file = root_file.or_else(|| {
            self.project_files
                .iter()
                .find(|file| !file.is_directory)
                .map(|file| file.path.clone())
        });
        if let Some(path) = first_source_file {
            self.open_document(path);
        } else {
            self.output_status = self
                .text("Pasta aberta. Crie um arquivo .por ou .alg para começar.")
                .to_string();
        }
    }

    fn save_document(&mut self, id: u64, path: PathBuf) {
        let Some(index) = self.documents.iter().position(|doc| doc.id == id) else {
            return;
        };
        let (path, language) =
            match supported_save_path(path, self.documents[index].language, self.ui_language) {
                Ok(path) => path,
                Err(error) => {
                    self.output_status = error;
                    return;
                }
            };
        match fs::write(&path, self.documents[index].contents.as_bytes()) {
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
                if let Some(root) = &self.project_root {
                    if let Ok(files) = scan_project_files(root) {
                        self.project_files = files;
                    }
                }
                self.output_status = self.text("Arquivo salvo").to_string();
            }
            Err(error) => {
                self.output_status = format!(
                    "{}: {error}",
                    self.text("Não foi possível salvar o arquivo")
                );
            }
        }
    }

    fn create_project_file(&mut self, name: String, language: Language, relative_folder: PathBuf) {
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
        let (path, language) = match new_source_path(&folder, &name, language, self.ui_language) {
            Ok(source) => source,
            Err(error) => {
                self.output_status = error;
                return;
            }
        };

        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                self.create_document(
                    Some(path.clone()),
                    name,
                    language,
                    String::new(),
                    String::new(),
                );
                self.new_file_language = language;
                if let Ok(files) = scan_project_files(&root) {
                    self.project_files = files;
                }
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

    fn create_project_folder(&mut self, name: String, relative_folder: PathBuf) {
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
                if let Ok(files) = scan_project_files(&root) {
                    self.project_files = files;
                }
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

    fn create_project(
        &mut self,
        parent: PathBuf,
        name: String,
        language: Language,
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

        let source_path = root.join(language.file_name());
        if let Err(error) = fs::write(&source_path, language.sample()) {
            self.output_status =
                format!("{}: {error}", self.text("Não foi possível criar o projeto"));
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

    fn request_save(&mut self) {
        match self
            .active_document()
            .map(|document| (document.id, document.path.clone()))
        {
            Some((id, Some(path))) => self.save_document(id, path),
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
    dark_theme: Rc<Cell<bool>>,
    default_new_language: Rc<Cell<Language>>,
    project_root_for_new_items: Rc<RefCell<Option<PathBuf>>>,
    settings: SettingsUi,
    project_title: gtk::Label,
    project_row: gtk::Box,
    project_label: gtk::Label,
    file_list: gtk::Box,
    new_file_button: gtk::MenuButton,
    empty_state: gtk::Widget,
    expanded_folders: sidebar::ExpandedFolders,
    editor_tabs: gtk::Box,
    status_language: gtk::Label,
    status_details: gtk::Label,
    indent_width: Rc<Cell<u32>>,
    code_buffer: gtk::TextBuffer,
    code_view: gtk::TextView,
    syntax_tags: SyntaxTags,
    line_numbers: gtk::Label,
    line_numbers_separator: gtk::Separator,
    output_buffer: gtk::TextBuffer,
    output_view: gtk::TextView,
    output_status: gtk::Label,
    displayed_language: Option<Language>,
    displayed_active_document_id: Option<u64>,
    displayed_project_name: String,
    displayed_project_files: Vec<(PathBuf, bool)>,
    displayed_project_open: bool,
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
    displayed_output_status: String,
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
        root.set_default_size(1440, 900);
        root.maximize();
        root.add_css_class("potoo-window");
        let preferences = load_preferences();
        let saved_session = load_session();
        let unreadable_session = session_file().exists() && saved_session.is_none();
        let restored_root = saved_session
            .as_ref()
            .and_then(|session| session.project_root.clone())
            .filter(|path| path.is_dir());
        let (mut documents, mut active_document_id, recovered_changes) = saved_session
            .map(restore_session)
            .unwrap_or_else(|| (Vec::new(), None, false));
        if documents.is_empty() && restored_root.is_none() {
            let sample = Language::Portugol.sample().to_string();
            documents.push(SourceDocument {
                id: 1,
                path: None,
                name: Language::Portugol.file_name().to_string(),
                language: Language::Portugol,
                contents: sample.clone(),
                saved_contents: sample,
            });
            active_document_id = Some(1);
        }
        let project_files = restored_root
            .as_deref()
            .and_then(|root| scan_project_files(root).ok())
            .unwrap_or_default();
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

        let mut model = Self {
            new_file_language: preferences.default_language,
            next_document_id: documents.len() as u64 + 1,
            documents,
            active_document_id,
            project_root: restored_root,
            project_name,
            project_files,
            pending_project: None,
            pending_close_document_id: None,
            pending_close_after_save_id: None,
            pending_save_as_id: None,
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
        };
        let mut widgets = build_window_content(
            root.clone(),
            model.project_root.clone(),
            sender.clone(),
            preferences,
            initial_dark,
        );
        root.set_child(Some(&widgets.content));
        content_panel::rebuild_editor_tabs(
            &widgets.editor_tabs,
            &model.documents,
            model.active_document_id,
            model.ui_language,
            sender,
        );
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
                | MainWindowMsg::ProjectSwitchDecision(_)
                | MainWindowMsg::CreateProject(_, _, _)
                | MainWindowMsg::CreateFile(_, _, _)
                | MainWindowMsg::CreateFolder(_, _)
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
                self.preferred_ui_language = language;
                self.save_preferences();
                self.output_status = self
                    .text("O idioma será aplicado ao reabrir o Potoo.")
                    .to_string();
            }
            MainWindowMsg::ProjectSelected(path) => {
                if self.documents.iter().any(SourceDocument::is_dirty) {
                    self.pending_project = Some(path);
                } else {
                    self.load_project(path);
                }
            }
            MainWindowMsg::ProjectSwitchDecision(discard_changes) => {
                if discard_changes {
                    if let Some(path) = self.pending_project.take() {
                        self.load_project(path);
                    }
                } else {
                    self.pending_project = None;
                }
            }
            MainWindowMsg::CreateProject(parent, name, language) => {
                self.create_project(parent, name, language, sender.clone());
            }
            MainWindowMsg::CreateFile(name, language, relative_folder) => {
                self.create_project_file(name, language, relative_folder)
            }
            MainWindowMsg::CreateFolder(name, relative_folder) => {
                self.create_project_folder(name, relative_folder)
            }
            MainWindowMsg::OpenFile(path) => self.open_document(path),
            MainWindowMsg::ActivateDocument(id) => {
                if self.documents.iter().any(|doc| doc.id == id) {
                    self.active_document_id = Some(id);
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
                        self.save_document(id, path);
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
            MainWindowMsg::SaveCurrent => self.request_save(),
            MainWindowMsg::SaveAsSelected(id, path) => {
                self.pending_save_as_id = None;
                self.save_document(id, path);
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
                if self.pending_save_as_id == Some(id) {
                    self.pending_save_as_id = None;
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
                self.output_text = self.text("A execução real depende da integração com o interpretador oficial selecionado.").to_string();
                self.output_status = self
                    .text("Interpretador ainda não conectado nesta prévia")
                    .to_string();
            }
            MainWindowMsg::SourceChanged(contents) => {
                if let Some(id) = self.active_document_id {
                    if let Some(document) = self.documents.iter_mut().find(|doc| doc.id == id) {
                        document.contents = contents;
                    }
                }
                self.recovery_generation += 1;
                let generation = self.recovery_generation;
                let sender_for_recovery = sender.clone();
                gtk::glib::timeout_add_local_once(Duration::from_millis(800), move || {
                    sender_for_recovery.input(MainWindowMsg::PersistRecovery(generation));
                });
            }
            MainWindowMsg::PersistRecovery(generation) => {
                if generation == self.recovery_generation {
                    self.persist_session(false, false);
                }
            }
        }

        if persist_after {
            self.persist_session(false, false);
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        let ui = widgets;
        ui.project_root_for_new_items
            .replace(self.project_root.clone());

        let active_document = self.active_document();
        let active_document_id = active_document.map(|document| document.id);
        let active_language = active_document.map(|document| document.language);
        if ui.displayed_active_document_id != active_document_id {
            ui.code_buffer.set_text(
                active_document
                    .map(|document| document.contents.as_str())
                    .unwrap_or_default(),
            );
            ui.displayed_active_document_id = active_document_id;
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

        if ui.displayed_preferred_ui_language != self.preferred_ui_language {
            ui.settings
                .ui_language_picker
                .set_selected(self.preferred_ui_language.selected_index());
            ui.displayed_preferred_ui_language = self.preferred_ui_language;
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

        if ui.displayed_project_name != self.project_name {
            ui.project_title.set_text(&self.project_name);
            ui.project_label.set_text(&self.project_name);
            ui.displayed_project_name.clone_from(&self.project_name);
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

        content_panel::update_line_numbers(&ui.code_buffer, &ui.line_numbers);
        content_panel::highlight_buffer(
            &ui.code_buffer,
            active_language.unwrap_or(self.new_file_language),
            &ui.syntax_tags,
        );
    }
}

fn build_window_content(
    window: gtk::ApplicationWindow,
    initial_project_root: Option<PathBuf>,
    sender: ComponentSender<MainWindow>,
    preferences: Preferences,
    initial_dark: bool,
) -> MainUi {
    let ui_language = preferences.ui_language;
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

    let header = header::build_header(&window, ui_language);
    let project_title = header.project_title.clone();
    let run_button = header.run_button.clone();

    let dark_theme = Rc::new(Cell::new(initial_dark));
    let settings = build_settings_window(&window, sender.clone(), preferences, initial_dark);

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
    let new_folder_button = file_sidebar.new_folder_button;
    let new_item_popover = file_sidebar.new_item_popover;
    let open_project_button = file_sidebar.open_project_button;
    let open_file_button = file_sidebar.open_file_button;
    let open_popover = file_sidebar.open_popover;
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
    code_view.add_css_class("potoo-code-editor");
    code_view.add_css_class(&format!("potoo-font-{}", preferences.font_size));

    let code_scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&code_view)
        .build();

    let editor_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    editor_row.add_css_class("potoo-editor-row");
    editor_row.append(&line_numbers);
    let line_numbers_separator = gtk::Separator::new(gtk::Orientation::Vertical);
    line_numbers_separator.add_css_class("potoo-gutter-separator");
    line_numbers_separator.set_visible(preferences.line_numbers_visible);
    editor_row.append(&line_numbers_separator);
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

    let (output_panel, output_buffer, output_tab_output, output_view) =
        content_panel::build_output_panel(ui_language);
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
    output_area.append(&output_status_row);

    let editor_output = gtk::Paned::new(gtk::Orientation::Vertical);
    editor_output.set_wide_handle(false);
    editor_output.set_start_child(Some(&editor));
    editor_output.set_end_child(Some(&output_area));
    editor_output.set_resize_start_child(true);
    editor_output.set_resize_end_child(false);
    editor_output.set_shrink_start_child(false);
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

    let learning_panel = content_panel::build_learning_panel(ui_language);
    let editor_and_learning = gtk::Paned::new(gtk::Orientation::Horizontal);
    editor_and_learning.set_wide_handle(false);
    editor_and_learning.set_start_child(Some(&editor_output));
    editor_and_learning.set_end_child(Some(&learning_panel));
    editor_and_learning.set_resize_start_child(true);
    editor_and_learning.set_resize_end_child(false);
    editor_and_learning.set_shrink_start_child(false);
    editor_and_learning.set_shrink_end_child(false);
    editor_and_learning.set_position(820);
    editor_and_learning.connect_map(|paned| {
        let paned = paned.clone();
        gtk::glib::idle_add_local(move || {
            let width = paned.width();
            if width <= 0 {
                return gtk::glib::ControlFlow::Continue;
            }
            paned.set_position(width.saturating_sub(306).max(440));
            gtk::glib::ControlFlow::Break
        });
    });

    let workspace = gtk::Paned::new(gtk::Orientation::Horizontal);
    workspace.set_wide_handle(false);
    workspace.set_start_child(Some(&sidebar_widget));
    workspace.set_end_child(Some(&editor_and_learning));
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
    let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    status_bar.add_css_class("potoo-statusbar");
    status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    status_bar.append(&status_language);
    status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    status_bar.append(&status_details);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&workspace);
    content.append(&status_bar);

    let content_root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content_root.add_css_class("potoo-root");
    if let Some(header_widget) = &header.widget {
        content_root.append(header_widget);
    }
    content_root.append(&content);
    if initial_dark {
        window.add_css_class("potoo-dark");
    }

    let sender_for_open_project = sender.clone();
    let window_for_open_project = window.clone();
    let open_popover_for_project = open_popover.clone();
    open_project_button.connect_clicked(move |_| {
        open_popover_for_project.popdown();
        show_open_project_dialog(
            &window_for_open_project,
            ui_language,
            sender_for_open_project.clone(),
        );
    });

    let sender_for_open_file = sender.clone();
    let window_for_open_file = window.clone();
    let open_popover_for_file = open_popover.clone();
    open_file_button.connect_clicked(move |_| {
        open_popover_for_file.popdown();
        show_open_file_dialog(
            &window_for_open_file,
            ui_language,
            sender_for_open_file.clone(),
        );
    });

    let sender_for_empty_open_project = sender.clone();
    let window_for_empty_open_project = window.clone();
    empty_open_project_button.connect_clicked(move |_| {
        show_open_project_dialog(
            &window_for_empty_open_project,
            ui_language,
            sender_for_empty_open_project.clone(),
        );
    });

    let sender_for_empty_open_file = sender.clone();
    let window_for_empty_open_file = window.clone();
    empty_open_file_button.connect_clicked(move |_| {
        show_open_file_dialog(
            &window_for_empty_open_file,
            ui_language,
            sender_for_empty_open_file.clone(),
        );
    });

    let sender_for_new_project = sender.clone();
    let window_for_new_project = window.clone();
    let default_new_language_for_project = default_new_language.clone();
    let dark_theme_for_project = dark_theme.clone();
    new_project_button.connect_clicked(move |_| {
        show_new_project_dialog(
            &window_for_new_project,
            ui_language,
            default_new_language_for_project.get(),
            dark_theme_for_project.get(),
            sender_for_new_project.clone(),
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
    let dark_theme_for_new_file = dark_theme.clone();
    let popover_for_new_file = new_item_popover.clone();
    new_program_button.connect_clicked(move |_| {
        popover_for_new_file.popdown();
        let Some(project_root) = project_root_for_new_file.borrow().clone() else {
            return;
        };
        show_new_file_dialog(
            &window_for_new_file,
            default_new_language_for_new_file.get(),
            ui_language,
            project_directory_options(&project_root, ui_language),
            dark_theme_for_new_file.get(),
            sender_for_new_file.clone(),
        );
    });

    let sender_for_new_folder = sender.clone();
    let window_for_new_folder = window.clone();
    let project_root_for_new_folder = project_root_for_new_items.clone();
    let dark_theme_for_new_folder = dark_theme.clone();
    let popover_for_new_folder = new_item_popover.clone();
    new_folder_button.connect_clicked(move |_| {
        popover_for_new_folder.popdown();
        let Some(project_root) = project_root_for_new_folder.borrow().clone() else {
            return;
        };
        show_new_folder_dialog(
            &window_for_new_folder,
            ui_language,
            project_directory_options(&project_root, ui_language),
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

    let sender_for_source = sender.clone();
    code_buffer.connect_changed(move |buffer| {
        let contents = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
        sender_for_source.input(MainWindowMsg::SourceChanged(contents.to_string()));
    });

    let key_controller = gtk::EventControllerKey::new();
    let sender_for_shortcuts = sender;
    let window_for_shortcuts = window.clone();
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
                ui_language,
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
        dark_theme,
        default_new_language,
        project_root_for_new_items,
        settings,
        project_title,
        project_row,
        project_label,
        file_list,
        new_file_button,
        empty_state,
        expanded_folders,
        editor_tabs,
        status_language,
        status_details,
        indent_width,
        code_buffer,
        code_view,
        syntax_tags,
        line_numbers,
        line_numbers_separator,
        output_buffer,
        output_view,
        output_status,
        displayed_language: Some(Language::Portugol),
        displayed_active_document_id: Some(0),
        displayed_project_name: String::new(),
        displayed_project_files: Vec::new(),
        displayed_project_open: false,
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
        displayed_output_status: tr(ui_language, "Pronto para executar").to_string(),
    }
}

fn build_settings_window(
    parent: &gtk::ApplicationWindow,
    sender: ComponentSender<MainWindow>,
    preferences: Preferences,
    initial_dark: bool,
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
            "O idioma escolhido será aplicado ao reabrir o Potoo.",
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
    about_button.connect_clicked(move |_| {
        stack_for_about.set_visible_child_name("about");
        window_for_about.set_default_size(740, 560);
        back_for_about.set_visible(true);
        title_for_about.set_text(tr(language, "Sobre o Potoo IDE"));
        window_for_about.set_title(Some(tr(language, "Sobre o Potoo IDE")));
    });

    let stack_for_back = stack.clone();
    let window_for_back = window.clone();
    let title_for_back = title_label.clone();
    let back_for_back = back_button.clone();
    back_button.connect_clicked(move |_| {
        stack_for_back.set_visible_child_name("preferences");
        window_for_back.set_default_size(740, 685);
        back_for_back.set_visible(false);
        title_for_back.set_text(tr(language, "Configurações"));
        window_for_back.set_title(Some(tr(language, "Configurações")));
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
    window.connect_close_request(move |_| {
        stack_for_close.set_visible_child_name("preferences");
        window_for_close.set_default_size(740, 685);
        back_for_close.set_visible(false);
        title_for_close.set_text(tr(language, "Configurações"));
        window_for_close.set_title(Some(tr(language, "Configurações")));
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
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title(tr(language, "Abrir pasta de projeto"));
    dialog.set_modal(true);
    dialog.select_folder(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |result| match result {
            Ok(folder) => match folder.path() {
                Some(path) => sender.input(MainWindowMsg::ProjectSelected(path)),
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
    filter.set_name(Some(tr(language, "Portugol e VisuAlg (*.por, *.alg)")));
    filter.add_suffix("por");
    filter.add_suffix("alg");
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

fn show_new_project_dialog(
    parent: &gtk::ApplicationWindow,
    ui_language: UiLanguage,
    default_language: Language,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(ui_language, "Criar projeto")));
    dialog.set_default_size(680, 450);
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
    let language_label = gtk::Label::new(Some(tr(
        ui_language,
        "Selecione a pseudolinguagem do projeto",
    )));
    language_label.set_xalign(0.0);
    language_label.add_css_class("potoo-project-field-label");
    let language_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    language_row.set_margin_top(1);
    let portugol_button =
        language_option_button("Portugol", tr(ui_language, "Arquivo inicial .por"));
    let visualg_button = language_option_button("VisuAlg", tr(ui_language, "Arquivo inicial .alg"));
    for button in [&portugol_button, &visualg_button] {
        button.add_css_class("potoo-project-language-choice");
        button.set_hexpand(true);
    }
    match default_language {
        Language::Portugol => portugol_button.add_css_class("selected"),
        Language::Visualg => visualg_button.add_css_class("selected"),
    }
    language_row.append(&portugol_button);
    language_row.append(&visualg_button);
    language_field.append(&language_label);
    language_field.append(&language_row);
    content.append(&language_field);

    let selected_language = Rc::new(Cell::new(default_language));
    let visualg_button_for_portugol = visualg_button.clone();
    let selected_language_for_portugol = selected_language.clone();
    portugol_button.connect_clicked(move |button| {
        button.add_css_class("selected");
        visualg_button_for_portugol.remove_css_class("selected");
        selected_language_for_portugol.set(Language::Portugol);
    });
    let portugol_button_for_visualg = portugol_button.clone();
    let selected_language_for_visualg = selected_language.clone();
    visualg_button.connect_clicked(move |button| {
        button.add_css_class("selected");
        portugol_button_for_visualg.remove_css_class("selected");
        selected_language_for_visualg.set(Language::Visualg);
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
    let actions_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    actions_spacer.set_hexpand(true);
    let cancel_button = gtk::Button::with_label(tr(ui_language, "Cancelar"));
    cancel_button.add_css_class("potoo-project-cancel-button");
    let create_button = gtk::Button::with_label(tr(ui_language, "Criar projeto"));
    create_button.add_css_class("potoo-run-button");
    actions.append(&actions_spacer);
    actions.append(&cancel_button);
    actions.append(&create_button);
    dialog_content.append(&content);
    dialog_content.append(&actions);
    dialog.set_child(Some(&dialog_content));

    let dialog_for_cancel = dialog.clone();
    cancel_button.connect_clicked(move |_| dialog_for_cancel.close());

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
    let selected_language_for_create = selected_language.clone();
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
            selected_language_for_create.get(),
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

fn project_directory_options(root: &Path, language: UiLanguage) -> Vec<(String, PathBuf)> {
    let root_label = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| tr(language, "Pasta do projeto").to_string());
    let mut folders = vec![(root_label, PathBuf::new())];
    if let Ok(files) = scan_project_files(root) {
        folders.extend(
            files
                .into_iter()
                .filter(|file| file.is_directory)
                .map(|file| {
                    (
                        file.relative_path.clone(),
                        PathBuf::from(file.relative_path),
                    )
                }),
        );
    }
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
    dialog.set_default_size(680, 450);
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
    let language_label = gtk::Label::new(Some(tr(ui_language, "Linguagem")));
    language_label.set_xalign(0.0);
    language_label.add_css_class("potoo-project-field-label");
    let language_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    language_row.set_margin_top(1);
    let portugol_button = language_option_button("Portugol", ".por");
    let visualg_button = language_option_button("VisuAlg", ".alg");
    for button in [&portugol_button, &visualg_button] {
        button.add_css_class("potoo-project-language-choice");
        button.set_hexpand(true);
    }
    match default_language {
        Language::Portugol => portugol_button.add_css_class("selected"),
        Language::Visualg => visualg_button.add_css_class("selected"),
    }
    language_row.append(&portugol_button);
    language_row.append(&visualg_button);
    language_field.append(&language_label);
    language_field.append(&language_row);
    content.append(&language_field);
    let selected_language = Rc::new(Cell::new(default_language));
    let visualg_for_portugol = visualg_button.clone();
    let selected_for_portugol = selected_language.clone();
    portugol_button.connect_clicked(move |button| {
        button.add_css_class("selected");
        visualg_for_portugol.remove_css_class("selected");
        selected_for_portugol.set(Language::Portugol);
    });
    let portugol_for_visualg = portugol_button.clone();
    let selected_for_visualg = selected_language.clone();
    visualg_button.connect_clicked(move |button| {
        button.add_css_class("selected");
        portugol_for_visualg.remove_css_class("selected");
        selected_for_visualg.set(Language::Visualg);
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
    let selected_language_for_create = selected_language.clone();
    let folder_picker_for_create = folder_picker.clone();
    create_button.connect_clicked(move |_| {
        let relative_folder = folder_options
            .get(folder_picker_for_create.selected() as usize)
            .map(|(_, path)| path.clone())
            .unwrap_or_default();
        sender.input(MainWindowMsg::CreateFile(
            name_for_create.text().to_string(),
            selected_language_for_create.get(),
            relative_folder,
        ));
        dialog_for_create.close();
    });
    let create_button_for_entry = create_button.clone();
    name.connect_activate(move |_| create_button_for_entry.emit_clicked());
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
    on_response: impl Fn(usize) + 'static,
) {
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(language, title)));
    dialog.set_default_size(520, 240);
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
        button.add_css_class(if Some(index) == primary_action {
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
        Some(_) => Language::from_path(requested_path)
            .ok_or_else(|| tr(ui_language, "Use a extensão .por ou .alg.").to_string())?,
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

fn supported_save_path(
    path: PathBuf,
    default_language: Language,
    ui_language: UiLanguage,
) -> Result<(PathBuf, Language), String> {
    match path.extension() {
        Some(_) => Language::from_path(&path)
            .map(|language| (path, language))
            .ok_or_else(|| {
                tr(ui_language, "Salve o código com a extensão .por ou .alg.").to_string()
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
        } else if file_type.is_file() && Language::from_path(&path).is_some() {
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
