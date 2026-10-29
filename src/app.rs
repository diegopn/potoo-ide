use crate::components::{
    content_panel::{self, Language, SourceDocument, SyntaxTags},
    header, icon_label_button, sidebar,
    sidebar::ProjectFile,
};
use gtk::prelude::*;
use relm4::prelude::*;
use std::{
    cell::Cell,
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
            "Abrir menu" => "Open menu",
            "Abrir pasta" => "Open folder",
            "Abrir pasta de projeto" => "Open project folder",
            "Abrir arquivo" => "Open file",
            "Salvar arquivo" => "Save file",
            "Salvar" => "Save",
            "Salvar como" => "Save as",
            "Linguagem para novos arquivos" => "Language for new files",
            "Configurações" => "Settings",
            "Executar" => "Run",
            "Arquivos" => "Files",
            "Criar arquivo" => "Create file",
            "Criar item no projeto" => "Create project item",
            "Novo programa" => "New program",
            "Criar um programa Portugol ou VisuAlg" => "Create a Portugol or VisuAlg program",
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
            "Use a aparência do sistema ou escolha um tema." => {
                "Use the system appearance or choose a theme."
            }
            "Editor" => "Editor",
            "Tamanho da fonte" => "Font size",
            "Ajuste a leitura do código." => "Adjust code readability.",
            "Numeração de linhas" => "Line numbers",
            "Mostra a linha atual no editor." => "Show line numbers in the editor.",
            "Quebra automática de linha" => "Word wrap",
            "Ajusta linhas longas à largura do editor." => "Wrap long lines to fit the editor.",
            "Idioma da interface" => "Interface language",
            "O idioma escolhido será aplicado ao reabrir o Potoo." => {
                "The selected language will apply the next time you open Potoo."
            }
            "Recuo" => "Indentation",
            "Espaços por nível de recuo." => "Spaces inserted for each indentation level.",
            "2 espaços" => "2 spaces",
            "4 espaços" => "4 spaces",
            "Abra uma pasta de projeto para navegar pelos arquivos." => {
                "Open a project folder to browse its files."
            }
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
            "Nome do arquivo" => "File name",
            "Nome do arquivo, por exemplo: exercicio-1" => "File name, for example: exercise-1",
            "Linguagem" => "Language",
            "Cancelar" => "Cancel",
            "Alterações não salvas" => "Unsaved changes",
            "Descartar e abrir" => "Discard and open",
            "Este arquivo tem alterações não salvas" => "This file has unsaved changes",
            "Salve o arquivo antes de fechar ou descarte as alterações para continuar." => {
                "Save this file before closing it, or discard the changes to continue."
            }
            "Continuar editando" => "Keep editing",
            "Descartar alterações" => "Discard changes",
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
            "novo-programa" => "new-program",
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
            "Abrir menu" => "Abrir menú",
            "Abrir pasta" => "Abrir carpeta",
            "Abrir pasta de projeto" => "Abrir carpeta del proyecto",
            "Abrir arquivo" => "Abrir archivo",
            "Salvar arquivo" => "Guardar archivo",
            "Salvar" => "Guardar",
            "Salvar como" => "Guardar como",
            "Linguagem para novos arquivos" => "Lenguaje para archivos nuevos",
            "Configurações" => "Configuración",
            "Executar" => "Ejecutar",
            "Arquivos" => "Archivos",
            "Criar arquivo" => "Crear archivo",
            "Criar item no projeto" => "Crear elemento del proyecto",
            "Novo programa" => "Nuevo programa",
            "Criar um programa Portugol ou VisuAlg" => "Crear un programa Portugol o VisuAlg",
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
            "Use a aparência do sistema ou escolha um tema." => {
                "Usa la apariencia del sistema o elige un tema."
            }
            "Editor" => "Editor",
            "Tamanho da fonte" => "Tamaño de fuente",
            "Ajuste a leitura do código." => "Ajusta la lectura del código.",
            "Numeração de linhas" => "Números de línea",
            "Mostra a linha atual no editor." => "Muestra los números de línea en el editor.",
            "Quebra automática de linha" => "Ajuste de línea",
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
            "Abra uma pasta de projeto para navegar pelos arquivos." => {
                "Abre una carpeta de proyecto para explorar sus archivos."
            }
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
            "Nome do arquivo" => "Nombre del archivo",
            "Nome do arquivo, por exemplo: exercicio-1" => {
                "Nombre del archivo, por ejemplo: ejercicio-1"
            }
            "Linguagem" => "Lenguaje",
            "Cancelar" => "Cancelar",
            "Alterações não salvas" => "Cambios sin guardar",
            "Descartar e abrir" => "Descartar y abrir",
            "Este arquivo tem alterações não salvas" => "Este archivo tiene cambios sin guardar",
            "Salve o arquivo antes de fechar ou descarte as alterações para continuar." => {
                "Guarda este archivo antes de cerrarlo o descarta los cambios para continuar."
            }
            "Continuar editando" => "Seguir editando",
            "Descartar alterações" => "Descartar cambios",
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
            "novo-programa" => "nuevo-programa",
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

    fn color_scheme(self) -> adw::ColorScheme {
        match self {
            Self::System => adw::ColorScheme::Default,
            Self::Light => adw::ColorScheme::ForceLight,
            Self::Dark => adw::ColorScheme::ForceDark,
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
            font_size: 13,
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
    LanguageSelected(Language),
    InterfaceLanguageSelected(UiLanguage),
    ProjectSelected(PathBuf),
    ProjectSwitchDecision(bool),
    CreateFile(String, Language),
    CreateFolder(String),
    OpenFile(PathBuf),
    ActivateDocument(u64),
    CloseDocument(u64),
    CloseDocumentDecision(u64, bool),
    SaveCurrent,
    SaveAsCurrent,
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
    Preferences {
        theme: key_file
            .string("editor", "theme")
            .map(|value| ThemePreference::from_key(&value))
            .unwrap_or(defaults.theme),
        font_size: key_file
            .integer("editor", "font-size")
            .ok()
            .filter(|size| matches!(size, 13 | 15 | 17))
            .map(|size| size as u32)
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

    fn create_project_file(&mut self, name: String, language: Language) {
        let Some(root) = self.project_root.clone() else {
            self.output_status = self
                .text("Abra uma pasta de projeto antes de criar itens.")
                .to_string();
            return;
        };
        let (path, language) = match new_source_path(&root, &name, language, self.ui_language) {
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

    fn create_project_folder(&mut self, name: String) {
        let Some(root) = self.project_root.clone() else {
            self.output_status = self
                .text("Abra uma pasta de projeto antes de criar itens.")
                .to_string();
            return;
        };
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
            self.output_status = self.text("Digite um nome válido para a pasta.").to_string();
            return;
        }

        match fs::create_dir(root.join(name)) {
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
                self.output_status = format!(
                    "{}: {error}",
                    self.text("Não foi possível criar a pasta")
                );
            }
        }
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

    fn request_save(&mut self, save_as: bool) {
        match self
            .active_document()
            .map(|document| (document.id, document.path.clone()))
        {
            Some((id, Some(path))) if !save_as => self.save_document(id, path),
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
    theme_picker: gtk::DropDown,
    font_size_picker: gtk::DropDown,
    line_numbers_switch: gtk::Switch,
    word_wrap_switch: gtk::Switch,
    indent_width_picker: gtk::DropDown,
    ui_language_picker: gtk::DropDown,
}

pub struct MainUi {
    window: gtk::ApplicationWindow,
    content: gtk::Box,
    language_picker: gtk::DropDown,
    settings: SettingsUi,
    theme_options: Vec<(ThemePreference, gtk::ToggleButton)>,
    project_title: gtk::Label,
    project_label: gtk::Label,
    file_list: gtk::Box,
    new_file_button: gtk::MenuButton,
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
    output_status: gtk::Label,
    displayed_language: Option<Language>,
    displayed_new_file_language: Language,
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
        let style_manager = adw::StyleManager::default();
        style_manager.set_color_scheme(preferences.theme.color_scheme());
        let sender_for_system_theme = sender.clone();
        style_manager.connect_dark_notify(move |manager| {
            sender_for_system_theme.input(MainWindowMsg::SystemThemeChanged(manager.is_dark()));
        });

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
            pending_save_as_id: None,
            pending_close_app: false,
            close_app_ready: false,
            ui_language: preferences.ui_language,
            preferred_ui_language: preferences.ui_language,
            theme: preferences.theme,
            dark: style_manager.is_dark(),
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
        let widgets = build_window_content(root.clone(), sender, preferences);
        root.set_child(Some(&widgets.content));

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
                | MainWindowMsg::CreateFile(_, _)
                | MainWindowMsg::CreateFolder(_)
                | MainWindowMsg::OpenFile(_)
                | MainWindowMsg::ActivateDocument(_)
                | MainWindowMsg::CloseDocument(_)
                | MainWindowMsg::CloseDocumentDecision(_, _)
                | MainWindowMsg::SaveCurrent
                | MainWindowMsg::SaveAsCurrent
                | MainWindowMsg::SaveAsSelected(_, _)
        );
        match message {
            MainWindowMsg::LanguageSelected(language) => {
                self.new_file_language = language;
                self.save_preferences();
            }
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
            MainWindowMsg::CreateFile(name, language) => self.create_project_file(name, language),
            MainWindowMsg::CreateFolder(name) => self.create_project_folder(name),
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
            MainWindowMsg::SaveCurrent => self.request_save(false),
            MainWindowMsg::SaveAsCurrent => self.request_save(true),
            MainWindowMsg::SaveAsSelected(id, path) => {
                self.pending_save_as_id = None;
                self.save_document(id, path);
            }
            MainWindowMsg::SaveAsCanceled(id) => {
                if self.pending_save_as_id == Some(id) {
                    self.pending_save_as_id = None;
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
                let style_manager = adw::StyleManager::default();
                style_manager.set_color_scheme(theme.color_scheme());
                self.dark = match theme {
                    ThemePreference::System => style_manager.is_dark(),
                    ThemePreference::Light => false,
                    ThemePreference::Dark => true,
                };
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
        if ui.displayed_new_file_language != self.new_file_language {
            ui.language_picker
                .set_selected(self.new_file_language.selected_index());
            ui.displayed_new_file_language = self.new_file_language;
        }

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
            ui.settings
                .theme_picker
                .set_selected(self.theme.selected_index());
            for (theme, button) in &ui.theme_options {
                button.set_active(*theme == self.theme);
            }
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
                self.ui_language,
                &ui.expanded_folders,
                sender.clone(),
            );
            ui.displayed_project_files = project_file_signature;
            ui.displayed_project_open = project_open;
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
                show_project_switch_confirmation(&ui.window, self.ui_language, sender.clone());
            }
        }

        if ui.displayed_pending_close_document_id != self.pending_close_document_id {
            ui.displayed_pending_close_document_id = self.pending_close_document_id;
            if let Some(id) = self.pending_close_document_id {
                show_close_document_confirmation(&ui.window, id, self.ui_language, sender.clone());
            }
        }

        if ui.displayed_pending_close_app != self.pending_close_app {
            ui.displayed_pending_close_app = self.pending_close_app;
            if self.pending_close_app {
                show_close_app_confirmation(&ui.window, self.ui_language, sender.clone());
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
    sender: ComponentSender<MainWindow>,
    preferences: Preferences,
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

    let header = header::build_header(&window, ui_language, preferences.theme, sender.clone());
    let project_title = header.project_title.clone();
    let settings_button = header.settings_button.clone();
    let menu_open_project_button = header.open_project_button.clone();
    let menu_open_file_button = header.open_file_button.clone();
    let menu_save_button = header.save_button.clone();
    let menu_save_as_button = header.save_as_button.clone();
    let menu_popover = header.popover.clone();
    let theme_options = header.theme_options.clone();
    let run_button = header.run_button.clone();

    let settings = build_settings_window(&window, sender.clone(), preferences);

    let file_sidebar = sidebar::build_file_sidebar(ui_language, preferences.default_language);
    let expanded_folders = file_sidebar.expanded_folders.clone();
    let sidebar_widget = file_sidebar.widget;
    let project_label = file_sidebar.project_label;
    let file_list = file_sidebar.file_list;
    let new_file_button = file_sidebar.new_file_button;
    let new_program_button = file_sidebar.new_program_button;
    let new_folder_button = file_sidebar.new_folder_button;
    let new_item_popover = file_sidebar.new_item_popover;
    let open_project_button = file_sidebar.open_project_button;
    let open_file_button = file_sidebar.open_file_button;
    let language_picker = file_sidebar.language_picker;

    let code_buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
    code_buffer.set_text(Language::Portugol.sample());
    let syntax_tags = content_panel::make_syntax_tags(&code_buffer);
    let initial_dark = adw::StyleManager::default().is_dark();
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
    line_numbers_separator.set_visible(preferences.line_numbers_visible);
    editor_row.append(&line_numbers_separator);
    editor_row.append(&code_scroll);

    let editor_tabs = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    editor_tabs.set_hexpand(true);
    let tab_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    tab_bar.add_css_class("potoo-editor-tabbar");
    tab_bar.append(&editor_tabs);

    let editor = gtk::Box::new(gtk::Orientation::Vertical, 0);
    editor.add_css_class("potoo-editor");
    editor.append(&tab_bar);
    editor.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    editor.append(&editor_row);

    let (output_notebook, output_buffer) = content_panel::build_output_panel(ui_language);
    let output_status = gtk::Label::new(Some(tr(ui_language, "Pronto para executar")));
    output_status.set_xalign(0.0);
    output_status.add_css_class("potoo-muted");
    let output_status_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    output_status_row.add_css_class("potoo-output-status-row");
    output_status_row.set_margin_start(12);
    output_status_row.set_margin_end(12);
    output_status_row.set_margin_bottom(8);
    output_status_row.append(&gtk::Image::from_icon_name("emblem-synchronizing-symbolic"));
    output_status_row.append(&output_status);

    let output_area = gtk::Box::new(gtk::Orientation::Vertical, 0);
    output_area.add_css_class("potoo-output-area");
    output_area.append(&output_notebook);
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
            paned.set_position(height.saturating_sub(200).max(0));
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
            paned.set_position(width.saturating_sub(370).max(440));
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
    workspace.set_position(250);
    workspace.set_vexpand(true);

    let status_language = gtk::Label::new(Some("Portugol"));
    let status_details = gtk::Label::new(Some(&format!(
        "UTF-8  ·  {}: {}",
        tr(ui_language, "Espaços"),
        preferences.indent_width
    )));
    status_language.add_css_class("potoo-status-primary");
    status_details.add_css_class("potoo-muted");
    let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    status_bar.add_css_class("potoo-statusbar");
    status_bar.set_margin_start(12);
    status_bar.set_margin_end(12);
    status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    status_bar.append(&status_language);
    status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    status_bar.append(&status_details);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&workspace);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
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

    let sender_for_language = sender.clone();
    language_picker.connect_selected_notify(move |picker| {
        let selected = if picker.selected() == 1 {
            Language::Visualg
        } else {
            Language::Portugol
        };
        sender_for_language.input(MainWindowMsg::LanguageSelected(selected));
    });

    let sender_for_open_project = sender.clone();
    let window_for_open_project = window.clone();
    open_project_button.connect_clicked(move |_| {
        show_open_project_dialog(
            &window_for_open_project,
            ui_language,
            sender_for_open_project.clone(),
        );
    });

    let sender_for_open_file = sender.clone();
    let window_for_open_file = window.clone();
    open_file_button.connect_clicked(move |_| {
        show_open_file_dialog(
            &window_for_open_file,
            ui_language,
            sender_for_open_file.clone(),
        );
    });

    let sender_for_menu_open_project = sender.clone();
    let window_for_menu_open_project = window.clone();
    let menu_popover_for_open_project = menu_popover.clone();
    menu_open_project_button.connect_clicked(move |_| {
        menu_popover_for_open_project.popdown();
        show_open_project_dialog(
            &window_for_menu_open_project,
            ui_language,
            sender_for_menu_open_project.clone(),
        );
    });

    let sender_for_menu_open_file = sender.clone();
    let window_for_menu_open_file = window.clone();
    let menu_popover_for_open_file = menu_popover.clone();
    menu_open_file_button.connect_clicked(move |_| {
        menu_popover_for_open_file.popdown();
        show_open_file_dialog(
            &window_for_menu_open_file,
            ui_language,
            sender_for_menu_open_file.clone(),
        );
    });

    let sender_for_menu_save = sender.clone();
    let menu_popover_for_save = menu_popover.clone();
    menu_save_button.connect_clicked(move |_| {
        menu_popover_for_save.popdown();
        sender_for_menu_save.input(MainWindowMsg::SaveCurrent);
    });

    let sender_for_menu_save_as = sender.clone();
    let menu_popover_for_save_as = menu_popover.clone();
    menu_save_as_button.connect_clicked(move |_| {
        menu_popover_for_save_as.popdown();
        sender_for_menu_save_as.input(MainWindowMsg::SaveAsCurrent);
    });

    let sender_for_new_file = sender.clone();
    let window_for_new_file = window.clone();
    let language_picker_for_new_file = language_picker.clone();
    let popover_for_new_file = new_item_popover.clone();
    new_program_button.connect_clicked(move |_| {
        popover_for_new_file.popdown();
        show_new_file_dialog(
            &window_for_new_file,
            &language_picker_for_new_file,
            ui_language,
            sender_for_new_file.clone(),
        );
    });

    let sender_for_new_folder = sender.clone();
    let window_for_new_folder = window.clone();
    let popover_for_new_folder = new_item_popover.clone();
    new_folder_button.connect_clicked(move |_| {
        popover_for_new_folder.popdown();
        show_new_folder_dialog(
            &window_for_new_folder,
            ui_language,
            sender_for_new_folder.clone(),
        );
    });

    let sender_for_settings = sender.clone();
    settings_button.connect_clicked(move |_| {
        menu_popover.popdown();
        sender_for_settings.input(MainWindowMsg::OpenSettings);
    });

    let sender_for_run = sender.clone();
    let output_notebook_for_run = output_notebook.clone();
    run_button.connect_clicked(move |_| {
        output_notebook_for_run.set_current_page(Some(0));
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
        if key == gtk::gdk::Key::s && primary_modifier {
            if modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK) {
                sender_for_shortcuts.input(MainWindowMsg::SaveAsCurrent);
            } else {
                sender_for_shortcuts.input(MainWindowMsg::SaveCurrent);
            }
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
        language_picker,
        settings,
        theme_options,
        project_title,
        project_label,
        file_list,
        new_file_button,
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
        output_status,
        displayed_language: Some(Language::Portugol),
        displayed_new_file_language: preferences.default_language,
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
) -> SettingsUi {
    let language = preferences.ui_language;
    let window = gtk::Window::new();
    window.set_title(Some(tr(language, "Configurações")));
    window.set_default_size(620, 700);
    window.set_resizable(true);
    window.set_modal(true);
    window.set_transient_for(Some(parent));
    window.add_css_class("potoo-window");
    window.add_css_class("potoo-settings-window");
    if adw::StyleManager::default().is_dark() {
        window.add_css_class("potoo-dark");
    }

    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.add_css_class("potoo-settings-page");
    page.set_margin_start(18);
    page.set_margin_end(18);
    page.set_margin_top(16);
    page.set_margin_bottom(16);

    let description = gtk::Label::new(Some(tr(language, "Personalize a aparência e o editor.")));
    description.set_xalign(0.0);
    description.add_css_class("potoo-settings-description");
    description.set_margin_bottom(2);
    page.append(&description);

    let appearance = settings_section(tr(language, "Aparência"));
    let theme_picker = gtk::DropDown::from_strings(&[
        tr(language, "Seguir o sistema"),
        tr(language, "Claro"),
        tr(language, "Escuro"),
    ]);
    theme_picker.add_css_class("potoo-settings-picker");
    theme_picker.set_selected(preferences.theme.selected_index());
    appearance.append(&settings_row(
        tr(language, "Tema"),
        tr(language, "Use a aparência do sistema ou escolha um tema."),
        &theme_picker,
    ));
    page.append(&appearance);

    let editor = settings_section(tr(language, "Editor"));
    let font_size_picker = gtk::DropDown::from_strings(&["13 px", "15 px", "17 px"]);
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
        tr(language, "Mostra a linha atual no editor."),
        &line_numbers_switch,
    ));

    let word_wrap_switch = gtk::Switch::new();
    word_wrap_switch.add_css_class("potoo-settings-switch");
    word_wrap_switch.set_active(preferences.word_wrap);
    editor.append(&settings_row(
        tr(language, "Quebra automática de linha"),
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
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&page)
        .build();
    window.set_child(Some(&scroll));

    let sender_for_theme = sender.clone();
    theme_picker.connect_selected_notify(move |picker| {
        sender_for_theme.input(MainWindowMsg::ThemeSelected(
            ThemePreference::from_selected_index(picker.selected()),
        ));
    });

    let sender_for_font_size = sender.clone();
    font_size_picker.connect_selected_notify(move |picker| {
        let font_size = match picker.selected() {
            1 => 15,
            2 => 17,
            _ => 13,
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

    window.connect_close_request(move |_| {
        sender.input(MainWindowMsg::CloseSettings);
        gtk::glib::Propagation::Stop
    });

    SettingsUi {
        window,
        theme_picker,
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

fn settings_row<W: IsA<gtk::Widget>>(title: &str, description: &str, control: &W) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    row.add_css_class("potoo-settings-row");
    control.set_valign(gtk::Align::Center);

    let labels = gtk::Box::new(gtk::Orientation::Vertical, 4);
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
        17 => 2,
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

fn show_new_file_dialog(
    parent: &gtk::ApplicationWindow,
    language_picker: &gtk::DropDown,
    ui_language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(ui_language, "Criar arquivo")));
    dialog.set_default_size(420, 300);
    dialog.set_resizable(false);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_css_class("potoo-window");
    dialog.add_css_class("potoo-dialog-window");
    if adw::StyleManager::default().is_dark() {
        dialog.add_css_class("potoo-dark");
    }

    let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
    content.add_css_class("potoo-dialog-surface");
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_margin_top(24);
    content.set_margin_bottom(20);
    let title = gtk::Label::new(Some(tr(ui_language, "Criar arquivo")));
    title.set_xalign(0.0);
    title.add_css_class("potoo-settings-title");
    content.append(&title);
    let description = gtk::Label::new(Some(tr(
        ui_language,
        "Escolha um nome e a linguagem do novo arquivo.",
    )));
    description.set_xalign(0.0);
    description.add_css_class("potoo-settings-description");
    content.append(&description);
    content.append(&gtk::Label::new(Some(tr(ui_language, "Nome do arquivo"))));
    let name = gtk::Entry::new();
    name.set_placeholder_text(Some(tr(
        ui_language,
        "Nome do arquivo, por exemplo: exercicio-1",
    )));
    name.set_text(tr(ui_language, "novo-programa"));
    content.append(&name);
    content.append(&gtk::Label::new(Some(tr(ui_language, "Linguagem"))));
    let language = gtk::DropDown::from_strings(&["Portugol (.por)", "VisuAlg (.alg)"]);
    language.add_css_class("potoo-settings-picker");
    language.set_selected(language_picker.selected());
    content.append(&language);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_halign(gtk::Align::End);
    let cancel_button = gtk::Button::with_label(tr(ui_language, "Cancelar"));
    let create_button = gtk::Button::with_label(tr(ui_language, "Criar arquivo"));
    create_button.add_css_class("potoo-run-button");
    actions.append(&cancel_button);
    actions.append(&create_button);
    content.append(&actions);
    dialog.set_child(Some(&content));

    let dialog_for_cancel = dialog.clone();
    cancel_button.connect_clicked(move |_| dialog_for_cancel.close());
    let dialog_for_create = dialog.clone();
    let name_for_create = name.clone();
    create_button.connect_clicked(move |_| {
        let selected_language = if language.selected() == 1 {
            Language::Visualg
        } else {
            Language::Portugol
        };
        sender.input(MainWindowMsg::CreateFile(
            name_for_create.text().to_string(),
            selected_language,
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
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(ui_language, "Nova pasta")));
    dialog.set_default_size(400, 220);
    dialog.set_resizable(false);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_css_class("potoo-window");
    dialog.add_css_class("potoo-dialog-window");
    if adw::StyleManager::default().is_dark() {
        dialog.add_css_class("potoo-dark");
    }

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.add_css_class("potoo-dialog-surface");
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_margin_top(22);
    content.set_margin_bottom(18);
    let title = gtk::Label::new(Some(tr(ui_language, "Nova pasta")));
    title.set_xalign(0.0);
    title.add_css_class("potoo-settings-title");
    content.append(&title);
    let description = gtk::Label::new(Some(tr(
        ui_language,
        "Organize seus programas em uma pasta do projeto.",
    )));
    description.set_xalign(0.0);
    description.add_css_class("potoo-settings-description");
    content.append(&description);
    content.append(&gtk::Label::new(Some(tr(ui_language, "Nome da pasta"))));
    let name = gtk::Entry::new();
    name.set_placeholder_text(Some(tr(
        ui_language,
        "Nome da pasta, por exemplo: exercicios",
    )));
    content.append(&name);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_halign(gtk::Align::End);
    let cancel_button = gtk::Button::with_label(tr(ui_language, "Cancelar"));
    let create_button = gtk::Button::with_label(tr(ui_language, "Criar pasta"));
    create_button.add_css_class("potoo-run-button");
    actions.append(&cancel_button);
    actions.append(&create_button);
    content.append(&actions);
    dialog.set_child(Some(&content));

    let dialog_for_cancel = dialog.clone();
    cancel_button.connect_clicked(move |_| dialog_for_cancel.close());
    let dialog_for_create = dialog.clone();
    let name_for_create = name.clone();
    create_button.connect_clicked(move |_| {
        sender.input(MainWindowMsg::CreateFolder(name_for_create.text().to_string()));
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
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::AlertDialog::builder()
        .message(tr(language, "Alterações não salvas"))
        .detail(tr(language, "Salve os arquivos abertos antes de trocar de pasta ou descarte as alterações para continuar."))
        .buttons([tr(language, "Cancelar"), tr(language, "Descartar e abrir")])
        .cancel_button(0)
        .default_button(0)
        .modal(true)
        .build();
    dialog.choose(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |response| {
            sender.input(MainWindowMsg::ProjectSwitchDecision(matches!(
                response,
                Ok(1)
            )));
        },
    );
}

fn show_close_document_confirmation(
    parent: &gtk::ApplicationWindow,
    id: u64,
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::AlertDialog::builder()
        .message(tr(language, "Este arquivo tem alterações não salvas"))
        .detail(tr(
            language,
            "Salve o arquivo antes de fechar ou descarte as alterações para continuar.",
        ))
        .buttons([
            tr(language, "Continuar editando"),
            tr(language, "Descartar alterações"),
        ])
        .cancel_button(0)
        .default_button(0)
        .modal(true)
        .build();
    dialog.choose(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |response| {
            sender.input(MainWindowMsg::CloseDocumentDecision(
                id,
                matches!(response, Ok(1)),
            ));
        },
    );
}

fn show_close_app_confirmation(
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::AlertDialog::builder()
        .message(tr(language, "Há alterações não salvas"))
        .detail(tr(
            language,
            "Salve os arquivos antes de sair ou descarte as alterações para fechar o Potoo IDE.",
        ))
        .buttons([
            tr(language, "Continuar editando"),
            tr(language, "Descartar e sair"),
        ])
        .cancel_button(0)
        .default_button(0)
        .modal(true)
        .build();
    dialog.choose(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |response| {
            sender.input(MainWindowMsg::CloseAppDecision(matches!(response, Ok(1))));
        },
    );
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
